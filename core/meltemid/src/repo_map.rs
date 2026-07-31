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
        .parents(true)
        // `.hidden(false)` lets context dotdirs like `.meltemi/` through, but
        // the git metadirectory is never context: cut the subtree at the
        // walker so no consumer sees it and it spends no truncation budget.
        .filter_entry(|entry| entry.file_name() != std::ffi::OsStr::new(".git"));
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
        // The literal run up to the next `@` is copied as ONE `&str` slice.
        // Walking it byte by byte was the whole defect: `byte as char` decodes
        // nothing, it maps each byte to its namesake code point — Latin-1 — so
        // every multi-byte character left double-encoded. A slice cannot tear a
        // character in half; `&str` refuses to be cut off a boundary, which
        // turns what was a silent conversion into an invariant the type checks.
        //
        // `i` is always ON a boundary, and every branch below keeps it there:
        // it starts at 0, lands on an `@` after a literal run, advances 2 over
        // `@@` and 1 over a lone `@` (all ASCII), and takes the token's `end`,
        // which the scan advances one whole character at a time.
        let Some(offset) = text[i..].find('@') else {
            out.push_str(&text[i..]);
            break;
        };
        out.push_str(&text[i..i + offset]);
        i += offset;
        // `@@` → literal `@`.
        if bytes.get(i + 1) == Some(&b'@') {
            out.push('@');
            i += 2;
            continue;
        }
        // Read the reference token (path chars), one whole character at a
        // time: a filename may carry any alphabet, and stepping by `len_utf8()`
        // is what leaves `end` on a boundary for the slice below (D2).
        let start = i + 1;
        let mut end = start;
        for (offset, ch) in text[start..].char_indices() {
            if !is_ref_char(ch) {
                break;
            }
            end = start + offset + ch.len_utf8();
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

/// Whether a character can be part of an `@` reference (path-ish, no
/// whitespace). Letters and digits of ANY alphabet, so a file named with
/// accents or eñes is referenced like any other; over ASCII this is a strict
/// widening, since `is_alphanumeric` and `is_ascii_alphanumeric` agree there.
/// Non-ASCII PUNCTUATION («¿», «—», «…») is not alphanumeric, so it closes the
/// token instead of being swallowed by it — which is why the test is Unicode
/// classification and not "any byte above ASCII" (D2).
fn is_ref_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_')
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
    fn map_excludes_the_git_metadirectory() {
        // Scenario: Metadirectorio de git fuera del mapa.
        let dir = temp("gitdir");
        std::fs::create_dir_all(dir.join(".git").join("objects").join("ab")).unwrap();
        std::fs::write(dir.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::create_dir_all(dir.join(".meltemi")).unwrap();
        std::fs::write(dir.join(".meltemi").join("config.toml"), "# ctx\n").unwrap();
        std::fs::write(dir.join("keep.rs"), "fn main() {}").unwrap();

        let map = build_map(&dir, None, None);
        let paths: Vec<&str> = map.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(
            !paths.iter().any(|p| *p == ".git" || p.starts_with(".git/")),
            "the git metadirectory never appears: {paths:?}"
        );
        // The hidden dirs that ARE context stay listed.
        assert!(
            paths.contains(&".meltemi/config.toml"),
            ".meltemi stays in the map: {paths:?}"
        );
        assert!(paths.contains(&"keep.rs"), "tracked file listed: {paths:?}");
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
    fn a_prompt_in_spanish_travels_character_for_character() {
        // Scenario: Prompt en español íntegro
        let dir = temp("utf8");
        // The exact string the conducted smoke measured over the named pipe: it
        // went in at 20 characters and the session record held 24, because each
        // byte of every accent became a character of its own.
        let prompt = "acción íntegra ñandú";
        assert_eq!(prompt.chars().count(), 20, "the measured string, unchanged");
        let (out, exp) = expand_refs(&dir, prompt, ExpandLimits::default());
        assert_eq!(
            out.chars().count(),
            20,
            "not one character was invented on the way out: {out}"
        );
        assert_eq!(out, prompt, "the prompt travels character for character");
        assert!(exp.is_empty(), "no references, no expansions");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn double_at_is_a_literal() {
        // Scenario: Arroba doble literal
        let dir = temp("escape");
        let (out, exp) = expand_refs(
            &dir,
            "escríbeme a ñandú@@correo.eñe",
            ExpandLimits::default(),
        );
        assert_eq!(
            out, "escríbeme a ñandú@correo.eñe",
            "`@@` collapses to one literal `@` and the accents around it survive"
        );
        assert!(exp.is_empty(), "no expansion for a literal");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_reference_glued_to_a_multibyte_character_still_lands() {
        // Scenario: Referencia pegada a un carácter multibyte
        let dir = temp("adjacent");
        std::fs::write(dir.join("lib.rs"), "pub fn x() {}").unwrap();
        // Deliberately adversarial: `ñ` is glued to the `@` on the left and `»`
        // to the end of the token on the right, so a byte-walking expansion
        // tears one of them in half whichever side it gets right.
        let (out, exp) = expand_refs(&dir, "ñ@lib.rs» ñandú", ExpandLimits::default());
        assert!(
            out.starts_with('ñ'),
            "the character before the `@` survives: {out}"
        );
        assert!(
            out.contains("@lib.rs:") && out.contains("pub fn x()"),
            "the reference expanded exactly as it would surrounded by ASCII: {out}"
        );
        assert!(
            out.ends_with("» ñandú"),
            "the characters after the token survive: {out}"
        );
        assert_eq!(exp.len(), 1);
        assert_eq!(exp[0].path, "lib.rs");
        assert!(!exp[0].not_found);
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
    fn a_reference_to_a_path_outside_ascii_resolves() {
        // Scenario: Ruta con carácter no ASCII resuelta
        let dir = temp("acentos");
        std::fs::create_dir_all(dir.join("señales")).unwrap();
        std::fs::write(dir.join("señales").join("informé.md"), "# año\n").unwrap();
        let (out, exp) = expand_refs(&dir, "mirá @señales/informé.md", ExpandLimits::default());
        assert!(
            out.contains("@señales/informé.md:"),
            "identified by the path exactly as written: {out}"
        );
        assert!(out.contains("# año"), "the content was injected: {out}");
        assert_eq!(exp.len(), 1);
        assert_eq!(
            exp[0].path, "señales/informé.md",
            "the audit records the path the user wrote"
        );
        assert!(
            !exp[0].not_found,
            "an accented filename resolves like any other, instead of being \
             reported missing under a name nobody typed"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_ascii_punctuation_closes_the_token() {
        // Scenario: Puntuación no ASCII cierra el token
        let dir = temp("puntuacion");
        std::fs::write(dir.join("lib.rs"), "pub fn x() {}").unwrap();
        // Spanish prose is full of punctuation outside ASCII. Widening the
        // token to "any byte above ASCII" would have swallowed `¿ves` whole.
        let (out, exp) = expand_refs(&dir, "@lib.rs¿ves? —sí…", ExpandLimits::default());
        assert_eq!(exp.len(), 1);
        assert_eq!(
            exp[0].path, "lib.rs",
            "the token ended before the punctuation"
        );
        assert!(!exp[0].not_found);
        assert!(
            out.ends_with("¿ves? —sí…"),
            "the punctuation travelled as prose: {out}"
        );
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
