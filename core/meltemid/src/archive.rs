// SPDX-License-Identifier: Apache-2.0

//! The `archive` verb: fold a change's approved deltas into the living truth
//! atomically, preserve the change in the dated history, and regenerate the
//! projected context (comandos-verify-archive D2).
//!
//! The spec engine validates (dry-run `apply_delta`) so every conflict is
//! reported as a diagnostic; the fold itself is done at the text level, so the
//! exact wording of untouched requirements is preserved. The write is
//! all-or-nothing: every capability's new living spec is computed in memory
//! first, and only written once all folds succeed — a mid-fold failure leaves
//! the living truth exactly as it was.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use meltemi_spec::delta::{DeltaOp, parse_delta};
use meltemi_spec::spec_parser::parse_spec;

/// A per-capability fold result to write.
struct Fold {
    capability: String,
    living_path: PathBuf,
    new_content: String,
}

/// The report of an archive operation.
pub struct ArchiveReport {
    pub capabilities: Vec<String>,
    pub archived_to: String,
    pub projection_regenerated: bool,
}

fn changes_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".meltemi").join("changes")
}

fn specs_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".meltemi").join("specs")
}

/// Splits a spec's markdown into its preamble (everything before the first
/// `### Requirement:`) and its requirement blocks, keyed by name in order.
fn split_requirements(md: &str) -> (String, Vec<(String, String)>) {
    let mut preamble = String::new();
    let mut blocks: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in md.lines() {
        if let Some(name) = line.strip_prefix("### Requirement:") {
            if let Some((n, body)) = current.take() {
                blocks.push((n, body));
            }
            current = Some((name.trim().to_string(), format!("{line}\n")));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        } else {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    if let Some((n, body)) = current.take() {
        blocks.push((n, body));
    }
    (preamble, blocks)
}

/// The raw requirement blocks of a delta file, keyed by name (for splicing the
/// exact authored text of ADDED/MODIFIED requirements into the living spec).
fn delta_raw_blocks(delta_md: &str) -> BTreeMap<String, String> {
    let (_, blocks) = split_requirements(delta_md);
    blocks.into_iter().collect()
}

/// Folds one capability's delta into the living markdown at the text level,
/// mirroring the engine's operation semantics but preserving exact wording.
fn fold_text(living_md: &str, delta_md: &str, ops: &[DeltaOp]) -> String {
    let (preamble, mut blocks) = split_requirements(living_md);
    let raw = delta_raw_blocks(delta_md);
    let find = |blocks: &[(String, String)], name: &str| blocks.iter().position(|(n, _)| n == name);

    for op in ops {
        match op {
            DeltaOp::Added(req) => {
                if find(&blocks, &req.name).is_none()
                    && let Some(block) = raw.get(&req.name)
                {
                    blocks.push((req.name.clone(), normalize_block(block)));
                }
            }
            DeltaOp::Modified(req) => {
                if let (Some(pos), Some(block)) = (find(&blocks, &req.name), raw.get(&req.name)) {
                    blocks[pos].1 = normalize_block(block);
                }
            }
            DeltaOp::Removed { name, .. } => {
                if let Some(pos) = find(&blocks, name) {
                    blocks.remove(pos);
                }
            }
            DeltaOp::Renamed { from, to, .. } => {
                if let Some(pos) = find(&blocks, from) {
                    let body = blocks[pos].1.replacen(
                        &format!("### Requirement:{}", header_suffix(&blocks[pos].1)),
                        &format!("### Requirement: {to}"),
                        1,
                    );
                    blocks[pos] = (to.clone(), body);
                }
            }
        }
    }

    let mut out = preamble.trim_end().to_string();
    out.push('\n');
    for (_, body) in &blocks {
        out.push('\n');
        out.push_str(body.trim_end());
        out.push('\n');
    }
    out
}

/// The text after `### Requirement:` on the block's first line (to rebuild a
/// rename in place).
fn header_suffix(block: &str) -> String {
    block
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("### Requirement:"))
        .unwrap_or("")
        .to_string()
}

/// Trims trailing blank lines from a spliced block so blocks join cleanly.
fn normalize_block(block: &str) -> String {
    format!("{}\n", block.trim_end())
}

/// Validates the change's deltas against the current living truth (dry-run
/// `apply_delta`), returning all diagnostics. A non-empty result blocks the
/// archive — nothing is written.
#[must_use]
pub fn dry_run_diagnostics(repo_root: &Path, change: &str) -> Vec<String> {
    let mut diags = Vec::new();
    for (capability, delta_md, delta_path) in change_deltas(repo_root, change) {
        let delta = parse_delta(&capability, &delta_md, &delta_path);
        let living_path = specs_dir(repo_root).join(&capability).join("spec.md");
        let living = std::fs::read_to_string(&living_path)
            .map(|md| parse_spec(&capability, &md, &living_path))
            .unwrap_or_else(|_| parse_spec(&capability, "", &living_path));
        let (_, cap_diags) = meltemi_spec::delta::apply_delta(&living, &delta);
        for d in cap_diags {
            diags.push(format!("{capability}: {}", d.message));
        }
    }
    diags
}

/// The `(capability, delta_markdown, delta_path)` of each capability in a change.
fn change_deltas(repo_root: &Path, change: &str) -> Vec<(String, String, PathBuf)> {
    let specs = changes_dir(repo_root).join(change).join("specs");
    let mut out = Vec::new();
    let Ok(caps) = std::fs::read_dir(&specs) else {
        return out;
    };
    for cap in caps.flatten() {
        if !cap.path().is_dir() {
            continue;
        }
        let capability = cap.file_name().to_string_lossy().into_owned();
        let path = cap.path().join("spec.md");
        if let Ok(md) = std::fs::read_to_string(&path) {
            out.push((capability, md, path));
        }
    }
    out
}

/// Whether the living specs tree has uncommitted git changes (a dirty tree is
/// warned before folding).
#[must_use]
pub fn living_specs_dirty(repo_root: &Path) -> bool {
    let specs = specs_dir(repo_root);
    if !specs.is_dir() {
        return false;
    }
    crate::git::run(
        repo_root,
        &["status", "--porcelain", "--", ".meltemi/specs"],
    )
    .map(|s| !s.trim().is_empty())
    .unwrap_or(false)
}

/// Archives a change: folds every capability atomically into the living truth,
/// moves the change to the dated history, and best-effort regenerates the
/// projected context. `date` is supplied by the caller (the daemon clock).
pub fn archive_change(repo_root: &Path, change: &str, date: &str) -> Result<ArchiveReport, String> {
    // 1. Compute every fold in memory (atomic: no write until all succeed).
    let mut folds: Vec<Fold> = Vec::new();
    for (capability, delta_md, delta_path) in change_deltas(repo_root, change) {
        let delta = parse_delta(&capability, &delta_md, &delta_path);
        let living_path = specs_dir(repo_root).join(&capability).join("spec.md");
        let living_md = std::fs::read_to_string(&living_path).unwrap_or_default();
        let new_content = fold_text(&living_md, &delta_md, &delta.operations);
        folds.push(Fold {
            capability,
            living_path,
            new_content,
        });
    }
    if folds.is_empty() {
        return Err(format!("change `{change}` declares no capability deltas"));
    }

    // 2. Write all folds (only reached when every fold computed cleanly).
    let capabilities: Vec<String> = folds.iter().map(|f| f.capability.clone()).collect();
    for fold in &folds {
        if let Some(parent) = fold.living_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&fold.living_path, &fold.new_content).map_err(|e| e.to_string())?;
    }

    // 3. Preserve the change in the dated history.
    let src = changes_dir(repo_root).join(change);
    let archive_root = changes_dir(repo_root).join("archive");
    std::fs::create_dir_all(&archive_root).map_err(|e| e.to_string())?;
    let dest = archive_root.join(format!("{date}-{change}"));
    let _ = std::fs::remove_dir_all(&dest);
    move_dir(&src, &dest).map_err(|e| e.to_string())?;

    // 4. Best-effort: regenerate the projected context (if the repo declares one).
    let projection_regenerated = regenerate_projection(repo_root);

    Ok(ArchiveReport {
        capabilities,
        archived_to: dest.to_string_lossy().into_owned(),
        projection_regenerated,
    })
}

/// Moves a directory, falling back to copy+remove across filesystems.
fn move_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_dir(src, dest)?;
    std::fs::remove_dir_all(src)
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Regenerates the projected context if the repo declares projection targets;
/// returns whether anything was written. Best-effort — never fails the archive.
fn regenerate_projection(repo_root: &Path) -> bool {
    // Reuse the projection compiler only when a `.meltemi/` constitution exists;
    // a fixture without one simply skips it.
    let constitution = repo_root.join(".meltemi").join("constitution.md");
    if !constitution.is_file() {
        return false;
    }
    crate::context::project_and_write(repo_root)
        .map(|targets| targets.iter().any(|t| t.wrote))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVING: &str = "# cap Specification\n\n## Requirements\n### Requirement: Keep me\nProse.\n\n#### Scenario: A\n- **WHEN** x\n- **THEN** y\n";

    #[test]
    fn added_requirement_is_appended_preserving_others() {
        // Scenario: Fusión total o nada (the fold half).
        let delta = "## ADDED Requirements\n\n### Requirement: New thing\nNew prose.\n\n#### Scenario: B\n- **WHEN** a\n- **THEN** b\n";
        let ops = parse_delta("cap", delta, "d.md").operations;
        let folded = fold_text(LIVING, delta, &ops);
        assert!(folded.contains("### Requirement: Keep me"), "existing kept");
        assert!(
            folded.contains("### Requirement: New thing"),
            "added present"
        );
        assert!(folded.contains("New prose."));
    }

    #[test]
    fn modified_requirement_replaces_the_block() {
        let delta = "## MODIFIED Requirements\n\n### Requirement: Keep me\nUpdated prose.\n\n#### Scenario: A\n- **WHEN** x\n- **THEN** z\n";
        let ops = parse_delta("cap", delta, "d.md").operations;
        let folded = fold_text(LIVING, delta, &ops);
        assert!(
            folded.contains("Updated prose."),
            "block replaced: {folded}"
        );
        assert!(!folded.contains("Prose."), "old prose gone");
        assert_eq!(
            folded.matches("### Requirement: Keep me").count(),
            1,
            "no dup"
        );
    }

    #[test]
    fn removed_requirement_drops_the_block() {
        let delta = "## REMOVED Requirements\n\n### Requirement: Keep me\n**Reason**: obsolete\n**Migration**: none\n";
        let ops = parse_delta("cap", delta, "d.md").operations;
        let folded = fold_text(LIVING, delta, &ops);
        assert!(
            !folded.contains("### Requirement: Keep me"),
            "removed: {folded}"
        );
    }
}
