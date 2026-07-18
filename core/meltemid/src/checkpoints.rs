// SPDX-License-Identifier: Apache-2.0

//! Pre-task checkpoints as technical git refs, with granular reversion and an
//! honest scope (checkpoints-rollback D1/D2/D3/D4).
//!
//! Before a task runs, the daemon snapshots its worktree — tracked state and
//! untracked-but-not-ignored files — into a commit under
//! `refs/meltemi/checkpoints/<change>/<task>-<agent>`, using a scratch index so
//! the worktree's own index is never disturbed and **no user branch moves**.
//! Reverting a task resets its worktree to that commit and removes later
//! untracked files (ignored files are left alone). The approved out-of-tree
//! operations that a restore cannot undo are accumulated per task and listed as
//! irreversible, so a reversion is never presented as total when any remain.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::git;

/// The technical ref namespace: outside `refs/heads`, invisible to the user's
/// branches, removable without a trace.
const REF_ROOT: &str = "refs/meltemi/checkpoints";

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// The technical ref for a task's checkpoint.
#[must_use]
pub fn ref_for(change: &str, task: &str, agent: &str) -> String {
    format!("{REF_ROOT}/{}/{}-{}", slug(change), slug(task), slug(agent))
}

/// A recorded checkpoint (the daemon's own metadata index — mirrors the git
/// object, and carries the exact task/agent/worktree the ref name can't encode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecord {
    pub change: String,
    pub task: String,
    pub agent: String,
    pub git_ref: String,
    pub worktree: String,
    pub created_at: String,
}

fn checkpoints_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".meltemi").join("checkpoints")
}

fn registry_path(repo_root: &Path) -> PathBuf {
    checkpoints_dir(repo_root).join("registry.jsonl")
}

fn irreversible_path(repo_root: &Path) -> PathBuf {
    checkpoints_dir(repo_root).join("irreversible.jsonl")
}

fn events_path(repo_root: &Path) -> PathBuf {
    checkpoints_dir(repo_root).join("events.jsonl")
}

/// Appends a checkpoint lifecycle event (`checkpoint_created` /
/// `checkpoint_restored`) to the project's checkpoints JSONL, so the lifecycle
/// is auditable even outside a live session (D4). Best-effort: a logging
/// failure never fails the operation it records.
pub fn log_event(repo_root: &Path, kind: meltemi_proto::SessionEventKind) {
    let event = meltemi_proto::SessionEvent {
        v: meltemi_proto::SESSION_EVENT_VERSION,
        ts: crate::clock::now_rfc3339(),
        kind,
    };
    if let Ok(line) = serde_json::to_string(&event) {
        let _ = append_line(&events_path(repo_root), &line);
    }
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

/// Creates the pre-task checkpoint of `worktree` for `(change, task, agent)`.
/// Snapshots into a scratch index so nothing in the worktree is disturbed and
/// no user branch moves; records the checkpoint in the daemon's registry.
pub fn create(
    repo_root: &Path,
    worktree: &Path,
    change: &str,
    task: &str,
    agent: &str,
) -> Result<CheckpointRecord, String> {
    let git_ref = ref_for(change, task, agent);

    // A scratch index in the checkpoints dir keeps the worktree's own index
    // untouched while we stage tracked + untracked-non-ignored files.
    let dir = checkpoints_dir(repo_root);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let scratch = dir.join(format!("index-{}-{}", slug(task), slug(agent)));
    let scratch_str = scratch.to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&scratch);
    let env = [("GIT_INDEX_FILE", scratch_str.as_str())];

    // Seed the scratch index from HEAD (so deletions are captured), then stage
    // everything the worktree tracks or newly added (gitignore is honored).
    git::run_env(worktree, &["read-tree", "HEAD"], &env)?;
    git::run_env(worktree, &["add", "-A"], &env)?;
    let tree = git::run_env(worktree, &["write-tree"], &env)?;
    let tree = tree.trim();
    let parent = git::run(worktree, &["rev-parse", "HEAD"])?;
    let parent = parent.trim();

    let message = format!("meltemi checkpoint {change}/{task}-{agent}");
    let commit = git::run(
        worktree,
        &["commit-tree", tree, "-p", parent, "-m", &message],
    )?;
    let commit = commit.trim();
    git::run(worktree, &["update-ref", &git_ref, commit])?;
    let _ = std::fs::remove_file(&scratch);

    let record = CheckpointRecord {
        change: change.to_string(),
        task: task.to_string(),
        agent: agent.to_string(),
        git_ref,
        worktree: worktree.to_string_lossy().into_owned(),
        created_at: crate::clock::now_rfc3339(),
    };
    append_line(
        &registry_path(repo_root),
        &serde_json::to_string(&record).expect("CheckpointRecord serializes"),
    )
    .map_err(|e| e.to_string())?;
    Ok(record)
}

/// The checkpoints the daemon recorded for this repo, most recent first,
/// optionally filtered by change. Folds by ref, keeping the latest.
#[must_use]
pub fn list(repo_root: &Path, change: Option<&str>) -> Vec<CheckpointRecord> {
    let Ok(contents) = std::fs::read_to_string(registry_path(repo_root)) else {
        return Vec::new();
    };
    let mut by_ref: Vec<CheckpointRecord> = Vec::new();
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(record) = serde_json::from_str::<CheckpointRecord>(line) {
            by_ref.retain(|r| r.git_ref != record.git_ref);
            by_ref.push(record);
        }
    }
    by_ref.reverse(); // most recent first
    if let Some(change) = change {
        by_ref.retain(|r| r.change == change);
    }
    by_ref
}

/// Records an approved out-of-tree operation against a task, so its reversion
/// declares the operation irreversible (honest scope, D3).
pub fn record_irreversible(
    repo_root: &Path,
    change: &str,
    task: &str,
    agent: &str,
    operation: &str,
) -> Result<(), String> {
    let entry = IrreversibleEntry {
        change: change.to_string(),
        task: task.to_string(),
        agent: agent.to_string(),
        operation: operation.to_string(),
    };
    append_line(
        &irreversible_path(repo_root),
        &serde_json::to_string(&entry).expect("IrreversibleEntry serializes"),
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IrreversibleEntry {
    change: String,
    task: String,
    agent: String,
    operation: String,
}

/// The approved out-of-tree operations accumulated for a task, in order.
#[must_use]
pub fn irreversibles_for(repo_root: &Path, change: &str, task: &str, agent: &str) -> Vec<String> {
    let Ok(contents) = std::fs::read_to_string(irreversible_path(repo_root)) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<IrreversibleEntry>(l).ok())
        .filter(|e| e.change == change && e.task == task && e.agent == agent)
        .map(|e| e.operation)
        .collect()
}

/// The outcome of a reversion: the restored worktree, the ref, and the
/// approved out-of-tree operations that remain in effect.
pub struct Reverted {
    pub worktree: String,
    pub git_ref: String,
    pub irreversible: Vec<String>,
}

/// Reverts a task's worktree to its checkpoint: resets tracked state to the
/// checkpoint commit and removes later untracked files (ignored files are left
/// untouched). Other worktrees are not touched; no user branch is rewritten.
pub fn revert(repo_root: &Path, change: &str, task: &str, agent: &str) -> Result<Reverted, String> {
    let git_ref = ref_for(change, task, agent);
    let record = list(repo_root, Some(change))
        .into_iter()
        .find(|r| r.task == task && r.agent == agent)
        .ok_or_else(|| format!("no checkpoint for {change}/{task}-{agent}"))?;

    let worktree = PathBuf::from(&record.worktree);
    // Restore tracked state to the checkpoint, then clear untracked files
    // created afterwards. `-d` removes new directories; no `-x`, so ignored
    // files (builds) are preserved — never captured, never cleaned.
    git::run(&worktree, &["reset", "--hard", &git_ref])?;
    git::run(&worktree, &["clean", "-fd"])?;

    Ok(Reverted {
        worktree: record.worktree,
        git_ref,
        irreversible: irreversibles_for(repo_root, change, task, agent),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_is_technical_and_slugged_never_a_user_branch() {
        // Scenario: ref técnica fuera de las ramas del usuario.
        let r = ref_for("add-thing", "1.2", "gemini/cli");
        assert_eq!(r, "refs/meltemi/checkpoints/add-thing/1-2-gemini-cli");
        assert!(r.starts_with("refs/meltemi/checkpoints/"));
        assert!(!r.starts_with("refs/heads/"), "never a user branch");
    }

    #[test]
    fn irreversibles_accumulate_per_task_only() {
        // Scenario (unit): clasificación acumulada — filtered by task/agent.
        let dir = std::env::temp_dir().join(format!(
            "meltemi-ut-cp-{}-{:p}",
            std::process::id(),
            &0u8 as *const u8
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(irreversibles_for(&dir, "c", "1.1", "claude").is_empty());
        record_irreversible(&dir, "c", "1.1", "claude", "ran: npm publish").unwrap();
        record_irreversible(&dir, "c", "1.2", "gemini", "ran: curl evil").unwrap();

        let ours = irreversibles_for(&dir, "c", "1.1", "claude");
        assert_eq!(ours, vec!["ran: npm publish".to_string()], "only this task");
        assert!(
            irreversibles_for(&dir, "c", "9.9", "nobody").is_empty(),
            "no leakage across tasks"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
