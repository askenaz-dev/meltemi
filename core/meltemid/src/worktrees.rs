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
//!
//! A second, orthogonal axis (rama-por-change D1): the change **workshop** —
//! the worktree where a whole change lives while it is open, on a branch
//! carrying the bare change name. The race axis answers "how do N agents
//! compete on this task"; the workshop answers "where does this change live".
//! Same machinery, same registry, same ownership rules.

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

/// The registry `task` marker for change workshops (never a real task id).
const WORKSPACE_TASK: &str = "workspace";

/// The workshop's stable (relative path, branch) names (rama-por-change
/// D2/D3). The branch carries the **bare change name** — it is the human
/// branch of the change, the one the maintainer merges — so ownership comes
/// from the registry, not from a namespace. A chosen branch keeps its own
/// name and gets its own directory, so workshops on different branches
/// coexist.
#[must_use]
pub fn workspace_names(change: &str, branch: Option<&str>) -> (PathBuf, String) {
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
    let c = slug(change);
    let branch_name = branch.map_or_else(|| c.clone(), str::to_string);
    let dir = if branch_name == c {
        WORKSPACE_TASK.to_string()
    } else {
        format!("{WORKSPACE_TASK}-{}", slug(&branch_name))
    };
    let rel = Path::new(".meltemi").join("worktrees").join(&c).join(dir);
    (rel, branch_name)
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

/// The outcome of asking for a change workshop: the worktree, whether it was
/// a re-encounter, and the base branch it grows from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub worktree: ManagedWorktree,
    pub reencountered: bool,
    pub base_branch: String,
}

/// "Give me the workshop", not "create it" (rama-por-change D4): idempotent
/// by default — an existing managed workshop is returned with re-encounter
/// declared, a missing one is created from the tip of the default branch
/// (detected, never assumed — not from HEAD, which depends on where the asker
/// stands). Naming `branch` is consent to mount an existing branch; `unique`
/// mints a fresh suffixed workshop and is never a re-encounter.
pub fn workspace(
    repo_root: &Path,
    change: &str,
    branch: Option<&str>,
    unique: bool,
) -> Result<Workspace, String> {
    if branch.is_some() && unique {
        return Err("`branch` and `unique` are mutually exclusive: naming an exact branch and asking for a suffix that alters it means nothing".to_string());
    }
    let base = git::default_branch(repo_root)
        .ok_or_else(|| "could not detect the default branch of the repository".to_string())?;

    if unique {
        // The suffix's only duty is to not collide (D4); re-mint until free.
        let mut branch_name = format!("{change}-{}", short_suffix());
        while git::branch_exists(repo_root, &branch_name) {
            branch_name = format!("{change}-{}", short_suffix());
        }
        let (rel, _) = workspace_names(change, Some(&branch_name));
        return mount(repo_root, change, &rel, &branch_name, &base).map(|worktree| Workspace {
            worktree,
            reencountered: false,
            base_branch: base,
        });
    }

    let (rel, branch_name) = workspace_names(change, branch);
    let abs_str = repo_root.join(&rel).to_string_lossy().into_owned();
    if let Some(existing) = list(repo_root).into_iter().find(|w| w.path == abs_str) {
        return Ok(Workspace {
            worktree: existing,
            reencountered: true,
            base_branch: base,
        });
    }

    // The refusal protects the implicit path only: a homonymous branch the
    // daemon did not create is refused untouched. Naming it is consent, and a
    // branch the daemon itself created earlier (workshops keep their branch
    // when retired, D6) is not foreign.
    if branch.is_none()
        && git::branch_exists(repo_root, &branch_name)
        && !ever_recorded_branch(repo_root, &branch_name)
    {
        return Err(format!(
            "branch `{branch_name}` already exists and Meltemi did not create it; \
             name it explicitly to mount the workshop on it, or rename/remove it"
        ));
    }

    mount(repo_root, change, &rel, &branch_name, &base).map(|worktree| Workspace {
        worktree,
        reencountered: false,
        base_branch: base,
    })
}

/// Creates the workshop worktree — on the existing branch when there is one
/// (consented or daemon-created), otherwise minting it from the base tip —
/// records it, and excludes the managed root from the main tree's status (D3).
fn mount(
    repo_root: &Path,
    change: &str,
    rel: &Path,
    branch: &str,
    base: &str,
) -> Result<ManagedWorktree, String> {
    let abs = repo_root.join(rel);
    let abs_str = abs.to_string_lossy().into_owned();
    if git::branch_exists(repo_root, branch) {
        git::run(repo_root, &["worktree", "add", &abs_str, branch])?;
    } else {
        git::run(
            repo_root,
            &["worktree", "add", "-b", branch, &abs_str, base],
        )?;
    }
    let base_rev = git::run(repo_root, &["rev-parse", base])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let wt = ManagedWorktree {
        change: change.to_string(),
        task: WORKSPACE_TASK.to_string(),
        agent: String::new(),
        path: abs_str,
        branch: branch.to_string(),
        base_rev,
    };
    record(repo_root, &wt).map_err(|e| e.to_string())?;
    ensure_excluded(repo_root)?;
    Ok(wt)
}

/// One commit the landing would carry (or carried): short sha and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandedCommit {
    pub sha: String,
    pub title: String,
}

/// The outcome of `change/land`: what would land (or landed), and whether it
/// did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub branch: String,
    pub base_branch: String,
    pub commits: Vec<LandedCommit>,
    pub files: Vec<String>,
    pub landed: bool,
    pub merge_sha: Option<String>,
}

/// Lands the workshop branch on the default branch (rama-por-change D5) —
/// only with explicit confirmation; without it, the preview: which commits
/// would land and which files they touch. The merge is `--no-ff` so the
/// change's shape stays visible in the graph. Three honest refusals, each
/// with its remedy: a dirty workshop, a merge that conflicts (aborted
/// immediately, the default branch left intact — conflicts belong to the
/// user's git, never resolved here), and a main worktree not standing on the
/// default branch.
pub fn land(
    repo_root: &Path,
    change: &str,
    branch: Option<&str>,
    confirm: bool,
) -> Result<Landing, String> {
    let base = git::default_branch(repo_root)
        .ok_or_else(|| "could not detect the default branch of the repository".to_string())?;
    let (_, branch_name) = workspace_names(change, branch);

    let workshop = list(repo_root)
        .into_iter()
        .find(|w| w.task == WORKSPACE_TASK && w.change == change && w.branch == branch_name)
        .ok_or_else(|| {
            format!(
                "no managed workshop for change `{change}` on branch `{branch_name}`; \
                 ask for the workshop first — asking again is always a re-encounter, never an error"
            )
        })?;

    // The dirty workshop never lands: uncommitted work is neither landed nor
    // silently left behind.
    if git::is_dirty(Path::new(&workshop.path)) {
        return Err(format!(
            "the workshop at `{}` has uncommitted changes; commit or discard them before landing",
            workshop.path
        ));
    }

    let commits: Vec<LandedCommit> = git::run(
        repo_root,
        &[
            "log",
            "--format=%h\u{9}%s",
            &format!("{base}..{branch_name}"),
        ],
    )?
    .lines()
    .filter_map(|l| {
        let (sha, title) = l.split_once('\u{9}')?;
        Some(LandedCommit {
            sha: sha.to_string(),
            title: title.to_string(),
        })
    })
    .collect();
    let files: Vec<String> = git::run(
        repo_root,
        &["diff", "--name-only", &format!("{base}...{branch_name}")],
    )?
    .lines()
    .filter(|l| !l.trim().is_empty())
    .map(str::to_string)
    .collect();

    if !confirm {
        return Ok(Landing {
            branch: branch_name,
            base_branch: base,
            commits,
            files,
            landed: false,
            merge_sha: None,
        });
    }

    if commits.is_empty() {
        return Err(format!(
            "nothing to land: `{branch_name}` has no commits that `{base}` lacks"
        ));
    }
    // The merge runs in the main worktree, so it must be standing on the
    // default branch — merging into whatever happens to be checked out would
    // land the change somewhere nobody asked for.
    let standing = git::run(repo_root, &["symbolic-ref", "--short", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if standing != base {
        return Err(format!(
            "the main worktree stands on `{standing}`, not on `{base}`; check out `{base}` there before landing"
        ));
    }

    if let Err(diagnostic) = git::run(repo_root, &["merge", "--no-ff", "--no-edit", &branch_name]) {
        // The conflict is refused, never half-applied: abort immediately
        // (best-effort — a merge that never started has nothing to abort) and
        // hand the conflict to the user's git.
        let _ = git::run(repo_root, &["merge", "--abort"]);
        return Err(format!(
            "the merge did not apply cleanly and was aborted; `{base}` is intact. \
             Resolve it in your git: {diagnostic}"
        ));
    }
    let merge_sha = git::run(repo_root, &["rev-parse", "--short", "HEAD"])
        .map(|s| s.trim().to_string())
        .ok();

    Ok(Landing {
        branch: branch_name,
        base_branch: base,
        commits,
        files,
        landed: true,
        merge_sha,
    })
}

/// Whether the daemon ever created this branch for a worktree — tombstoned
/// entries count, because retiring a workshop keeps the branch (D6), and
/// re-mounting a branch the daemon itself minted is not touching the foreign.
fn ever_recorded_branch(repo_root: &Path, branch: &str) -> bool {
    let Ok(contents) = std::fs::read_to_string(registry_path(repo_root)) else {
        return false;
    };
    contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<ManagedWorktree>(l).ok())
        .any(|w| w.branch == branch)
}

/// Excludes the managed root from the main tree's git status by the local
/// route (rama-por-change D3): one entry in `.git/info/exclude`, which is not
/// versioned — never the user's `.gitignore`.
fn ensure_excluded(repo_root: &Path) -> Result<(), String> {
    const ENTRY: &str = "/.meltemi/worktrees/";
    let git_dir = git::run(repo_root, &["rev-parse", "--git-common-dir"])?;
    let git_dir = git_dir.trim();
    let dir = if Path::new(git_dir).is_absolute() {
        PathBuf::from(git_dir)
    } else {
        repo_root.join(git_dir)
    };
    let exclude = dir.join("info").join("exclude");
    let current = std::fs::read_to_string(&exclude).unwrap_or_default();
    if current.lines().any(|l| l.trim() == ENTRY) {
        return Ok(());
    }
    if let Some(parent) = exclude.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{ENTRY}").map_err(|e| e.to_string())
}

/// A short suffix whose only duty is to not collide: time, pid and a counter
/// mixed into six hex chars; the caller's loop re-mints on the improbable hit.
fn short_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static MINT: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()) ^ d.as_secs())
        .unwrap_or(0);
    let mixed = time ^ u64::from(std::process::id()) ^ MINT.fetch_add(1, Ordering::Relaxed);
    format!("{:06x}", mixed & 0x00ff_ffff)
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
    fn workspace_branch_is_the_bare_change_name() {
        // The workshop branch is the HUMAN branch of the change (D2): no
        // `meltemi/` namespace — ownership comes from the registry.
        let (path, branch) = workspace_names("rama-por-change", None);
        assert_eq!(branch, "rama-por-change");
        assert!(path.ends_with(Path::new("workspace")));
        assert!(path.starts_with(Path::new(".meltemi").join("worktrees")));
    }

    #[test]
    fn a_chosen_branch_gets_its_own_workshop_directory() {
        // Workshops on different branches coexist: the directory is keyed by
        // the branch, so idempotence applies per named branch (D4).
        let (default_path, _) = workspace_names("some-change", None);
        let (chosen_path, branch) = workspace_names("some-change", Some("hotfix/x"));
        assert_eq!(branch, "hotfix/x", "the branch name passes verbatim");
        assert_ne!(default_path, chosen_path);
        assert!(chosen_path.ends_with(Path::new("workspace-hotfix-x")));
    }

    #[test]
    fn branch_plus_unique_refuses_before_touching_git() {
        // The contradiction is refused in the daemon too, not only in the
        // schema: a caller wiring params by hand gets the same refusal.
        let dir = std::env::temp_dir();
        let err = workspace(&dir, "some-change", Some("hotfix-x"), true).unwrap_err();
        assert!(err.contains("mutually exclusive"), "{err}");
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
