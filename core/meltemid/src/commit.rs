// SPDX-License-Identifier: Apache-2.0

//! Atomic per-task commits with machine-readable traceability trailers
//! (git-commit-por-tarea D1/D2/D4).
//!
//! Every commit a task produces carries `Meltemi-Task: <change>/<task>` and one
//! `Meltemi-Req: <capability>/<requirement>` per requirement it implements, so
//! each line traces back to what originated it (constitution §8). The message
//! follows the repo convention (imperative title, what/why body, `(<change>
//! <task>)` reference). **A co-authorship trailer is never emitted** — the
//! author is the user's own git identity, which the daemon does not touch — and
//! any co-authorship line an agent proposes in the body is stripped.

use std::path::Path;

use crate::git;

/// Folds a common Latin accented letter to its ASCII base (so `atómico`
/// slugs to `atomico`, not `at-mico`). Anything else is returned unchanged.
fn fold_accent(ch: char) -> char {
    match ch {
        'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

/// Slugs a requirement name into a stable kebab token for the trailer, folding
/// common Latin accents so Spanish requirement names slug cleanly.
#[must_use]
pub fn slug_requirement(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true; // avoids a leading dash
    for ch in name.chars() {
        // Unicode lowercase, then fold a Latin accent to its ASCII base.
        let lowered = ch.to_lowercase().next().unwrap_or(ch);
        let ch = fold_accent(lowered);
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Removes any co-authorship trailer lines from agent-proposed text — the guard
/// that keeps authorship the user's alone, even if the agent tried to add one.
#[must_use]
pub fn strip_coauthorship(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let lower = line.trim_start().to_ascii_lowercase();
            !lower.starts_with("co-authored-by:") && !lower.starts_with("co-authored-by ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// A requirement the task implements (`capability`, `requirement`).
#[derive(Debug, Clone)]
pub struct Requirement {
    pub capability: String,
    pub requirement: String,
}

/// Builds the convention-guaranteed commit message with traceability trailers.
/// The form is Meltemi's guarantee; the agent only proposes title/body content.
#[must_use]
pub fn build_message(
    change: &str,
    task: &str,
    title: &str,
    body: Option<&str>,
    requirements: &[Requirement],
) -> String {
    let mut message = String::new();
    // Imperative title, trimmed to the convention's length budget.
    let title = title.trim();
    message.push_str(title);
    message.push('\n');

    if let Some(body) = body {
        let clean = strip_coauthorship(body);
        if !clean.is_empty() {
            message.push('\n');
            message.push_str(&clean);
            message.push('\n');
        }
    }

    // The repo's `(<change> <task>)` reference, on its own paragraph.
    message.push('\n');
    message.push_str(&format!("({change} {task})\n"));

    // The trailer block is the last paragraph, so git parses it as trailers.
    message.push('\n');
    message.push_str(&format!("Meltemi-Task: {change}/{task}\n"));
    for req in requirements {
        message.push_str(&format!(
            "Meltemi-Req: {}/{}\n",
            req.capability,
            slug_requirement(&req.requirement)
        ));
    }
    message
}

/// The `<capability>/<requirement-slug>` strings recorded for the event/result.
#[must_use]
pub fn requirement_refs(requirements: &[Requirement]) -> Vec<String> {
    requirements
        .iter()
        .map(|r| format!("{}/{}", r.capability, slug_requirement(&r.requirement)))
        .collect()
}

/// The names of files currently pending in the worktree (tracked changes and
/// untracked-non-ignored), for the supervised preview.
#[must_use]
pub fn pending_files(worktree: &Path) -> Vec<String> {
    git::run(worktree, &["status", "--porcelain"])
        .map(|s| {
            s.lines()
                .filter_map(|line| {
                    // Porcelain: `XY <path>` (or `XY <old> -> <new>` for renames).
                    let path = line.get(3..).unwrap_or("").trim();
                    let path = path.rsplit(" -> ").next().unwrap_or(path);
                    (!path.is_empty()).then(|| path.trim_matches('"').to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The committed paths that fall outside the task's declared scope. Empty when
/// the scope matches, or when the task declared no files (nothing to check
/// against — reported by the caller as unverified, never as "clean").
#[must_use]
pub fn scope_deviations(changed: &[String], declared: &[String]) -> Vec<String> {
    if declared.is_empty() {
        return Vec::new();
    }
    changed
        .iter()
        .filter(|path| {
            !declared
                .iter()
                .any(|d| *path == d || path.starts_with(&format!("{d}/")))
        })
        .cloned()
        .collect()
}

/// The applied commit's outcome.
pub struct Committed {
    pub sha: String,
    pub changed_files: Vec<String>,
}

/// Stages everything in the worktree and creates the commit with `message`,
/// honoring the user's hooks and identity. **Never** passes `--no-verify`; a
/// hook rejection surfaces verbatim. `base` (the task's checkpoint, or a
/// fallback) scopes the changed-file listing.
pub fn commit(worktree: &Path, message: &str, base: &str) -> Result<Committed, String> {
    git::run(worktree, &["add", "-A"])?;

    // Pass the message via stdin so multi-line trailers survive intact and no
    // temp file lingers. `commit -F -` reads the message from stdin.
    let output = std::process::Command::new("git")
        .args(["commit", "-F", "-"])
        .current_dir(worktree)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(message.as_bytes())?;
            }
            child.wait_with_output()
        })
        .map_err(|e| format!("could not run git commit: {e}"))?;

    if !output.status.success() {
        // Surface the hook/git output verbatim (stdout often carries hook text).
        let mut detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let out = String::from_utf8_lossy(&output.stdout);
        if !out.trim().is_empty() {
            if !detail.is_empty() {
                detail.push('\n');
            }
            detail.push_str(out.trim());
        }
        return Err(detail);
    }

    let sha = git::run(worktree, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    Ok(Committed {
        changed_files: git::changed_files(worktree, base),
        sha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_follows_the_convention_with_trailers() {
        // Scenario: Trazabilidad hasta el requisito; Forma garantizada.
        let msg = build_message(
            "add-thing",
            "2.1",
            "Add the thing",
            Some("Implements the thing the spec asks for."),
            &[Requirement {
                capability: "git-per-task".into(),
                requirement: "Commit atómico por tarea completada".into(),
            }],
        );
        let lines: Vec<&str> = msg.lines().collect();
        assert_eq!(lines[0], "Add the thing", "imperative title first");
        assert!(msg.contains("(add-thing 2.1)"), "repo reference present");
        assert!(msg.contains("Meltemi-Task: add-thing/2.1"), "task trailer");
        assert!(
            msg.contains("Meltemi-Req: git-per-task/commit-atomico-por-tarea-completada"),
            "requirement trailer slugged: {msg}"
        );
    }

    #[test]
    fn coauthorship_is_never_emitted_even_if_proposed() {
        // Scenario: Sin co-autoría jamás.
        let msg = build_message(
            "c",
            "1.1",
            "Do it",
            Some("Body line.\nCo-authored-by: Somebody <x@y.z>\nMore body."),
            &[],
        );
        let lower = msg.to_ascii_lowercase();
        assert!(
            !lower.contains("co-authored-by"),
            "the guard strips co-authorship: {msg}"
        );
        assert!(msg.contains("Body line."), "legitimate body kept");
        assert!(msg.contains("More body."));
    }

    #[test]
    fn deviations_flag_paths_outside_the_declared_scope() {
        // Scenario: Desviación declarada.
        let changed = vec!["src/a.rs".to_string(), "src/rogue.rs".to_string()];
        let declared = vec!["src/a.rs".to_string()];
        assert_eq!(scope_deviations(&changed, &declared), vec!["src/rogue.rs"]);
        // A directory declaration covers files beneath it.
        assert!(scope_deviations(&["src/a.rs".into()], &["src".into()]).is_empty());
        // No declared scope → nothing to flag (unverified, not a deviation).
        assert!(scope_deviations(&changed, &[]).is_empty());
    }

    #[test]
    fn requirement_slug_is_stable_kebab_folding_accents() {
        assert_eq!(slug_requirement("Commit atómico!"), "commit-atomico");
        assert_eq!(
            slug_requirement("  Trailers de trazabilidad  "),
            "trailers-de-trazabilidad"
        );
        // Uppercase accents fold too.
        assert_eq!(slug_requirement("Órdenes"), "ordenes");
    }
}
