// SPDX-License-Identifier: Apache-2.0

//! Repository context (gestion-contexto-repo): the repo map (`repo/map`) and
//! deterministic `@` reference expansion in prompts.
//!
//! - The map walks the repo with the `ignore` crate, honoring nested
//!   `.gitignore`, with per-file sizes and a declared truncation budget (D2).
//! - `@path` in a prompt injects a fenced, path-identified file; `@dir/`
//!   injects a listing (not contents); `@@` is a literal `@`. Expansion is
//!   deterministic with explicit per-file and per-prompt limits; overflow is
//!   marked in the prompt itself, and a missing reference is flagged without
//!   aborting the turn (D3).

use std::path::Path;

use ignore::WalkBuilder;

use meltemi_proto::{RefExpansion, RepoEntry, RepoMapResult};

/// Builds the repository map honoring nested `.gitignore`, sorted by path,
/// with a declared truncation budget.
pub fn build_map(root: &Path, depth: Option<u32>, limit: Option<u32>) -> RepoMapResult {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        // Honor `.gitignore` even when the root is not itself a git repo (a
        // fixture, or a subdirectory), so ignore rules always apply.
        .require_git(false)
        .parents(true);
    if let Some(d) = depth {
        // `ignore` depth counts the root as 0; +1 so `depth=0` lists the root's
        // immediate entries.
        builder.max_depth(Some(d as usize + 1));
    }

    let mut entries = Vec::new();
    for result in builder.build() {
        let Ok(dirent) = result else { continue };
        let path = dirent.path();
        if path == root {
            continue; // skip the root itself
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let is_dir = dirent.file_type().is_some_and(|t| t.is_dir());
        let size = if is_dir {
            0
        } else {
            dirent.metadata().map(|m| m.len()).unwrap_or(0)
        };
        entries.push(RepoEntry {
            path: rel.to_string_lossy().replace('\\', "/"),
            is_dir,
            size,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let limit = limit.map(|l| l as usize).unwrap_or(usize::MAX);
    let total = entries.len();
    let (truncated, omitted) = if total > limit {
        entries.truncate(limit);
        (true, (total - limit) as u32)
    } else {
        (false, 0)
    };
    RepoMapResult {
        entries,
        truncated,
        omitted,
    }
}

/// Prefix-matched map entries for `@` autocomplete in the composer (D4).
#[must_use]
pub fn autocomplete<'a>(map: &'a RepoMapResult, prefix: &str) -> Vec<&'a str> {
    map.entries
        .iter()
        .filter(|e| e.path.starts_with(prefix))
        .map(|e| e.path.as_str())
        .collect()
}

/// Per-file and per-prompt byte budgets for expansion.
#[derive(Debug, Clone, Copy)]
pub struct ExpandLimits {
    pub per_file: usize,
    pub per_prompt: usize,
}

impl Default for ExpandLimits {
    fn default() -> Self {
        Self {
            per_file: 32 * 1024,
            per_prompt: 128 * 1024,
        }
    }
}

/// Expands `@` references in `text` against `root`, returning the expanded
/// prompt and one [`RefExpansion`] per reference. `@@` is a literal `@`;
/// `@path` injects a fenced file identified by its path; `@dir/` injects a
/// listing. Missing references are flagged inline without aborting.
pub fn expand_refs(root: &Path, text: &str, limits: ExpandLimits) -> (String, Vec<RefExpansion>) {
    let mut out = String::with_capacity(text.len());
    let mut expansions = Vec::new();
    let mut budget = limits.per_prompt;
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'@' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // `@@` → literal `@`.
        if bytes.get(i + 1) == Some(&b'@') {
            out.push('@');
            i += 2;
            continue;
        }
        // Read the reference token (path chars).
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && is_ref_char(bytes[end]) {
            end += 1;
        }
        if end == start {
            out.push('@');
            i += 1;
            continue;
        }
        let reference = &text[start..end];
        let (block, expansion) = expand_one(root, reference, &mut budget, limits.per_file);
        out.push_str(&block);
        expansions.push(expansion);
        i = end;
    }
    (out, expansions)
}

/// Whether a byte can be part of an `@` reference (path-ish, no whitespace).
fn is_ref_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_')
}

/// Expands one reference, drawing from `budget`.
fn expand_one(
    root: &Path,
    reference: &str,
    budget: &mut usize,
    per_file: usize,
) -> (String, RefExpansion) {
    let target = root.join(reference);
    // A directory reference (trailing slash or an actual directory): listing.
    if reference.ends_with('/') || target.is_dir() {
        return match std::fs::read_dir(&target) {
            Ok(dir) => {
                let mut names: Vec<String> = dir
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                let listing = format!("@{reference} (directory):\n{}\n", names.join("\n"));
                let (emitted, truncated) = take_budget(&listing, budget, per_file);
                (
                    emitted.clone(),
                    RefExpansion {
                        path: reference.to_string(),
                        bytes: emitted.len() as u64,
                        not_found: false,
                        truncated,
                    },
                )
            }
            Err(_) => not_found_block(reference),
        };
    }
    match std::fs::read_to_string(&target) {
        Ok(content) => {
            let fenced = format!("@{reference}:\n```\n{content}\n```\n");
            let (emitted, truncated) = take_budget(&fenced, budget, per_file);
            (
                emitted.clone(),
                RefExpansion {
                    path: reference.to_string(),
                    bytes: emitted.len() as u64,
                    not_found: false,
                    truncated,
                },
            )
        }
        Err(_) => not_found_block(reference),
    }
}

/// Emits a not-found marker inline (never aborts the turn).
fn not_found_block(reference: &str) -> (String, RefExpansion) {
    (
        format!("@{reference} (not found)\n"),
        RefExpansion {
            path: reference.to_string(),
            bytes: 0,
            not_found: true,
            truncated: false,
        },
    )
}

/// Takes up to the per-file and remaining-prompt budget from `block`, marking
/// truncation visibly in the emitted text.
fn take_budget(block: &str, budget: &mut usize, per_file: usize) -> (String, bool) {
    let cap = per_file.min(*budget);
    if block.len() <= cap {
        *budget -= block.len();
        (block.to_string(), false)
    } else {
        // Truncate on a char boundary.
        let mut end = cap.min(block.len());
        while end > 0 && !block.is_char_boundary(end) {
            end -= 1;
        }
        let mut emitted = block[..end].to_string();
        emitted.push_str("\n… [truncated]\n");
        *budget = budget.saturating_sub(end);
        (emitted, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("meltemi-repomap-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn map_excludes_gitignored_and_reports_sizes() {
        // Scenario: Ignorados fuera del mapa.
        let dir = temp("map");
        std::fs::write(dir.join(".gitignore"), "secret.txt\ntarget/\n").unwrap();
        std::fs::write(dir.join("keep.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("secret.txt"), "shhh").unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("target").join("junk"), "x").unwrap();

        let map = build_map(&dir, None, None);
        let paths: Vec<&str> = map.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"keep.rs"), "tracked file listed: {paths:?}");
        assert!(
            !paths.iter().any(|p| p.contains("secret")),
            "ignored excluded"
        );
        assert!(
            !paths.iter().any(|p| p.contains("target")),
            "ignored dir excluded"
        );
        let keep = map.entries.iter().find(|e| e.path == "keep.rs").unwrap();
        assert!(keep.size > 0, "sizes reported");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn map_declares_truncation() {
        // Scenario: Truncado declarado.
        let dir = temp("trunc");
        for i in 0..10 {
            std::fs::write(dir.join(format!("f{i}.txt")), "x").unwrap();
        }
        let map = build_map(&dir, None, Some(3));
        assert_eq!(map.entries.len(), 3);
        assert!(map.truncated);
        assert!(map.omitted >= 7, "omitted count declared: {}", map.omitted);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn at_reference_injects_a_fenced_file() {
        // Scenario: Archivo inyectado con cerca.
        let dir = temp("ref");
        std::fs::write(dir.join("lib.rs"), "pub fn x() {}").unwrap();
        let (out, exp) = expand_refs(&dir, "look at @lib.rs please", ExpandLimits::default());
        assert!(
            out.contains("@lib.rs:") && out.contains("```"),
            "fenced+identified: {out}"
        );
        assert!(out.contains("pub fn x()"));
        assert_eq!(exp.len(), 1);
        assert!(exp[0].bytes > 0 && !exp[0].not_found);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn double_at_is_a_literal() {
        let dir = temp("escape");
        let (out, exp) = expand_refs(&dir, "email me@@example.com", ExpandLimits::default());
        assert!(out.contains("me@example.com"), "@@ is a literal @: {out}");
        assert!(exp.is_empty(), "no expansion for a literal");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_reference_is_flagged_without_aborting() {
        // Scenario: Referencia inexistente señalada.
        let dir = temp("missing");
        let (out, exp) = expand_refs(&dir, "see @no/existe.rs", ExpandLimits::default());
        assert!(out.contains("not found"), "marker present: {out}");
        assert!(exp[0].not_found);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn oversized_file_is_truncated_with_a_visible_mark() {
        // Scenario: Exceso truncado con marca.
        let dir = temp("big");
        std::fs::write(dir.join("big.txt"), "a".repeat(10_000)).unwrap();
        let limits = ExpandLimits {
            per_file: 100,
            per_prompt: 1000,
        };
        let (out, exp) = expand_refs(&dir, "@big.txt", limits);
        assert!(out.contains("[truncated]"), "truncation is visible: {out}");
        assert!(exp[0].truncated);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn autocomplete_matches_by_prefix() {
        // Scenario: Completar una ruta.
        let dir = temp("complete");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("lib.rs"), "x").unwrap();
        std::fs::write(dir.join("src").join("main.rs"), "x").unwrap();
        std::fs::write(dir.join("README.md"), "x").unwrap();
        let map = build_map(&dir, None, None);
        let hits = autocomplete(&map, "src/");
        assert!(hits.iter().all(|p| p.starts_with("src/")));
        assert!(hits.contains(&"src/lib.rs") && hits.contains(&"src/main.rs"));
        assert!(!hits.contains(&"README.md"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
