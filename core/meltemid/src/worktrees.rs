// SPDX-License-Identifier: Apache-2.0

//! Managed worktree lifecycle and N×M assignment (orquestacion-worktrees
//! D2/D4/D6).
//!
//! Worktrees the daemon creates live at
//! `<repo>/.meltemi/worktrees/<change>/<task>-<agent>` on branches
//! `meltemi/<change>/<task>-<agent>`, recorded in the daemon's own registry.
//! **The daemon never touches a worktree it did not create**, and removing one
//! with uncommitted changes requires explicit confirmation. Tasks that declare
//! overlapping files are serialized (not run in parallel), reported.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::git;

/// A worktree the daemon created and owns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorktree {
    pub change: String,
    pub task: String,
    pub agent: String,
    /// Absolute path of the worktree.
    pub path: String,
    /// The managed branch name.
    pub branch: String,
    /// The base revision the worktree was created from (fixed for the race).
    pub base_rev: String,
}

/// The stable (relative path, branch) names for an assignment.
#[must_use]
pub fn names(change: &str, task: &str, agent: &str) -> (PathBuf, String) {
    let slug = |s: &str| {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
    };
    let (c, t, a) = (slug(change), slug(task), slug(agent));
    let rel = Path::new(".meltemi")
        .join("worktrees")
        .join(&c)
        .join(format!("{t}-{a}"));
    let branch = format!("meltemi/{c}/{t}-{a}");
    (rel, branch)
}

fn registry_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".meltemi")
        .join("worktrees")
        .join("registry.jsonl")
}

/// Records a managed worktree in the daemon's own registry (append-only).
fn record(repo_root: &Path, wt: &ManagedWorktree) -> std::io::Result<()> {
    use std::io::Write;
    let path = registry_path(repo_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(wt).expect("ManagedWorktree serializes");
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())
}

/// The managed worktrees the daemon created for this repo (its own registry
/// only — never other worktrees). Folds by path, dropping removed ones.
#[must_use]
pub fn list(repo_root: &Path) -> Vec<ManagedWorktree> {
    let Ok(contents) = std::fs::read_to_string(registry_path(repo_root)) else {
        return Vec::new();
    };
    let mut live: Vec<ManagedWorktree> = Vec::new();
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        if let Some(removed) = line.strip_prefix("REMOVED ") {
            live.retain(|w| w.path != removed.trim());
        } else if let Ok(wt) = serde_json::from_str::<ManagedWorktree>(line) {
            live.retain(|w| w.path != wt.path);
            live.push(wt);
        }
    }
    live
}

/// Whether a path is a worktree this daemon manages (guards cleanup).
#[must_use]
pub fn is_managed(repo_root: &Path, path: &Path) -> bool {
    let target = path.to_string_lossy();
    list(repo_root).iter().any(|w| w.path == target)
}

/// Creates a managed worktree for an assignment from `base_rev` (the common,
/// fixed base). Reuses an existing managed worktree with the same name.
pub fn create(
    repo_root: &Path,
    change: &str,
    task: &str,
    agent: &str,
    base_rev: &str,
) -> Result<ManagedWorktree, String> {
    let (rel, branch) = names(change, task, agent);
    let abs = repo_root.join(&rel);
    let abs_str = abs.to_string_lossy().into_owned();

    if let Some(existing) = list(repo_root).into_iter().find(|w| w.path == abs_str) {
        return Ok(existing);
    }

    // `git worktree add -b <branch> <path> <base>` creates the isolated tree.
    git::run(
        repo_root,
        &["worktree", "add", "-b", &branch, &abs_str, base_rev],
    )?;
    let wt = ManagedWorktree {
        change: change.to_string(),
        task: task.to_string(),
        agent: agent.to_string(),
        path: abs_str,
        branch,
        base_rev: base_rev.to_string(),
    };
    record(repo_root, &wt).map_err(|e| e.to_string())?;
    Ok(wt)
}

/// Removes a managed worktree. Refuses a worktree the daemon does not own, and
/// requires `force` when the worktree has uncommitted changes.
pub fn remove(repo_root: &Path, path: &Path, force: bool) -> Result<(), String> {
    if !is_managed(repo_root, path) {
        return Err("refusing to remove a worktree Meltemi did not create".to_string());
    }
    if !force && git::is_dirty(path) {
        return Err(
            "the worktree has uncommitted changes; confirm to remove it (force)".to_string(),
        );
    }
    let path_str = path.to_string_lossy();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    git::run(repo_root, &args)?;

    // Mark it removed in the registry (append-only tombstone).
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(registry_path(repo_root))
    {
        let _ = writeln!(f, "REMOVED {path_str}");
    }
    Ok(())
}

/// The managed worktrees competing on one `(change, task)` — the racers whose
/// results are compared against the common base.
#[must_use]
pub fn competitors(repo_root: &Path, change: &str, task: &str) -> Vec<ManagedWorktree> {
    list(repo_root)
        .into_iter()
        .filter(|w| w.change == change && w.task == task)
        .collect()
}

/// Applies one file from a source managed worktree into a target managed
/// worktree (the assisted-merge primitive). Both must be managed by the daemon;
/// the file is copied verbatim (the human then reviews and commits the target).
pub fn apply_file(
    repo_root: &Path,
    target: &Path,
    source: &Path,
    file: &str,
) -> Result<(), String> {
    if !is_managed(repo_root, target) || !is_managed(repo_root, source) {
        return Err("both worktrees must be managed by Meltemi".to_string());
    }
    // Reject path escapes: the file must stay within the source worktree.
    if file.contains("..") || Path::new(file).is_absolute() {
        return Err(format!("`{file}` is not a path inside the worktree"));
    }
    let from = source.join(file);
    let to = target.join(file);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&from, &to)
        .map(|_| ())
        .map_err(|e| format!("could not apply `{file}`: {e}"))
}

/// A task with the files it declares it touches (from `tasks.md` of the plan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFiles {
    pub task: String,
    pub files: Vec<String>,
}

/// A serialized batch of tasks: tasks within a batch run in parallel; batches
/// run in sequence. Overlapping tasks are placed in different batches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub tasks: Vec<String>,
    /// Why a task landed in a later batch (file overlap), for the report.
    pub serialized_reason: Option<String>,
}

/// Plans the N×M assignment: greedily packs tasks into parallel batches,
/// pushing a task to a later batch when it shares a declared file with a task
/// already in the current batch (design D6). Deterministic in task order.
#[must_use]
pub fn assignment_plan(tasks: &[TaskFiles]) -> Vec<Batch> {
    let mut batches: Vec<(
        Vec<String>,
        std::collections::HashSet<String>,
        Option<String>,
    )> = Vec::new();
    for task in tasks {
        let files: std::collections::HashSet<&str> =
            task.files.iter().map(String::as_str).collect();
        // Find the first batch with no file overlap.
        let mut placed = false;
        for batch in batches.iter_mut() {
            if files.iter().all(|f| !batch.1.contains(*f)) {
                batch.0.push(task.task.clone());
                batch.1.extend(task.files.iter().cloned());
                placed = true;
                break;
            }
        }
        if !placed {
            let overlap: Vec<&str> = batches
                .last()
                .map(|b| {
                    b.1.iter()
                        .filter(|f| files.contains(f.as_str()))
                        .map(String::as_str)
                        .collect()
                })
                .unwrap_or_default();
            let reason = (!overlap.is_empty()).then(|| {
                format!(
                    "serialized: shares {} with an earlier task",
                    overlap.join(", ")
                )
            });
            let mut set = std::collections::HashSet::new();
            set.extend(task.files.iter().cloned());
            batches.push((vec![task.task.clone()], set, reason));
        }
    }
    batches
        .into_iter()
        .map(|(tasks, _, reason)| Batch {
            tasks,
            serialized_reason: reason,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_slugged() {
        // Scenario: Creación con nomenclatura estable.
        let (path, branch) = names("add-thing", "1.2", "gemini/cli");
        assert_eq!(branch, "meltemi/add-thing/1-2-gemini-cli");
        assert!(path.ends_with(Path::new("1-2-gemini-cli")));
        assert!(path.starts_with(Path::new(".meltemi").join("worktrees")));
    }

    #[test]
    fn non_overlapping_tasks_share_one_parallel_batch() {
        // Scenario: Paralelo sin solapamiento.
        let plan = assignment_plan(&[
            TaskFiles {
                task: "a".into(),
                files: vec!["src/a.rs".into()],
            },
            TaskFiles {
                task: "b".into(),
                files: vec!["src/b.rs".into()],
            },
        ]);
        assert_eq!(plan.len(), 1, "no overlap → one parallel batch");
        assert_eq!(plan[0].tasks, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn an_unrecorded_worktree_is_never_claimed_as_managed() {
        // Scenario: un worktree ajeno SHALL NOT ser tocado jamás. With no
        // registry, the daemon manages nothing and claims no foreign path.
        let dir = std::env::temp_dir().join(format!(
            "meltemi-ut-wt-{}-{:p}",
            std::process::id(),
            &0u8 as *const u8
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(list(&dir).is_empty(), "no registry → nothing managed");
        assert!(
            !is_managed(&dir, &dir.join("somebody-elses-worktree")),
            "a path the daemon never recorded is never managed"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn overlapping_tasks_serialize_with_a_reason() {
        // Scenario: Solapamiento serializado.
        let plan = assignment_plan(&[
            TaskFiles {
                task: "a".into(),
                files: vec!["src/shared.rs".into()],
            },
            TaskFiles {
                task: "b".into(),
                files: vec!["src/shared.rs".into()],
            },
        ]);
        assert_eq!(plan.len(), 2, "overlap → serialized into two batches");
        assert!(
            plan[1]
                .serialized_reason
                .as_deref()
                .is_some_and(|r| r.contains("shared.rs")),
            "the reason names the shared file: {:?}",
            plan[1].serialized_reason
        );
    }
}
