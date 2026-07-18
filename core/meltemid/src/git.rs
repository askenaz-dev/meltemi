// SPDX-License-Identifier: Apache-2.0

//! Thin wrapper over the user's `git` (orquestacion-worktrees D1): every
//! operation spawns the `git` the user already has and parses its output — no
//! native libgit2 dependency, behavior matching what the user knows. The
//! minimum version is checked on first use with a clear diagnostic.

use std::path::Path;
use std::process::Command;

/// The minimum git version worktrees need (`git worktree` is stable well
/// before this, but `--porcelain` listing lands here).
const MIN_MAJOR_MINOR: (u32, u32) = (2, 17);

/// Runs `git <args>` in `cwd`, returning stdout on success or a diagnostic.
pub fn run(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Whether `dir` is inside a git working tree.
#[must_use]
pub fn is_repo(dir: &Path) -> bool {
    run(dir, &["rev-parse", "--is-inside-work-tree"])
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

/// Checks the installed git satisfies the minimum version, with a diagnostic.
pub fn check_version() -> Result<(), String> {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|e| format!("git is not available: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let version = text
        .split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .ok_or_else(|| "could not parse `git --version`".to_string())?;
    let mut parts = version.split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    if (major, minor) >= MIN_MAJOR_MINOR {
        Ok(())
    } else {
        Err(format!(
            "git {major}.{minor} is too old; Meltemi needs {}.{}+",
            MIN_MAJOR_MINOR.0, MIN_MAJOR_MINOR.1
        ))
    }
}

/// The current commit revision (`HEAD`), when in a repo.
#[must_use]
pub fn head_rev(cwd: &Path) -> Option<String> {
    run(cwd, &["rev-parse", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
}

/// Whether the working tree at `cwd` has uncommitted changes.
#[must_use]
pub fn is_dirty(cwd: &Path) -> bool {
    run(cwd, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// The unified diff of `cwd` against `base` (a revision), for assisted merge.
#[must_use]
pub fn diff_against(cwd: &Path, base: &str) -> String {
    run(cwd, &["diff", base]).unwrap_or_default()
}

/// The names of files changed in `cwd` against `base`.
#[must_use]
pub fn changed_files(cwd: &Path, base: &str) -> Vec<String> {
    run(cwd, &["diff", "--name-only", base])
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}
