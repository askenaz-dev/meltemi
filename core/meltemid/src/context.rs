// SPDX-License-Identifier: Apache-2.0

//! Context projection writer (proyeccion-contexto D2/D3).
//!
//! Compiles the project's `.meltemi/` artifacts (via the pure
//! [`meltemi_spec::project`]) and writes the result into every declared
//! target file, **inside a managed block** that carries a fingerprint of the
//! content. Everything outside the block is preserved byte for byte; a target
//! without the block gets it appended. Writes are atomic (temp + rename) and
//! idempotent: unchanged sources leave the file untouched.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use meltemi_proto::ContextTarget;
use meltemi_spec::{ActiveChange, MeltemiTree, ProjectionSources, project};

/// The embedded target map (design D3): the instruction files each agent reads.
pub const EMBEDDED_TARGETS: &str = include_str!("../data/context-targets.toml");

/// The begin marker of a managed block; carries the content fingerprint.
const BEGIN_PREFIX: &str = "<!-- meltemi:context:begin sha256=";
const BEGIN_SUFFIX: &str = " -->";
/// The end marker of a managed block.
const END_MARKER: &str = "<!-- meltemi:context:end -->";

#[derive(Debug, Deserialize)]
struct RawTargets {
    version: String,
    #[serde(default)]
    target: Vec<RawTarget>,
}

#[derive(Debug, Deserialize)]
struct RawTarget {
    path: String,
}

/// A parsed target map.
#[derive(Debug, Clone)]
pub struct Targets {
    pub version: String,
    /// Destination file paths (relative to the project root), `AGENTS.md` first.
    pub paths: Vec<String>,
}

/// Parses the target map, guaranteeing `AGENTS.md` is present as the base.
pub fn parse_targets(text: &str) -> Result<Targets, String> {
    let raw: RawTargets = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut paths: Vec<String> = raw
        .target
        .into_iter()
        .map(|t| t.path)
        .filter(|p| !p.trim().is_empty())
        .collect();
    if !paths.iter().any(|p| p == "AGENTS.md") {
        paths.insert(0, "AGENTS.md".to_string());
    }
    Ok(Targets {
        version: raw.version,
        paths,
    })
}

/// The embedded target map (unit-tested to parse).
pub fn embedded_targets() -> Targets {
    parse_targets(EMBEDDED_TARGETS).expect("embedded context targets are valid")
}

/// Compiles the projection for a project root, reading the pieces the model
/// does not hold (constitution and proposal bodies) from disk.
pub fn compile(project_root: &Path) -> std::io::Result<String> {
    let tree = MeltemiTree::discover(project_root)?;
    let constitution = tree
        .constitution
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());

    // The active change (if exactly one is in progress) and its proposal body.
    let active = tree.changes.first();
    let proposal = active.and_then(|c| std::fs::read_to_string(c.path.join("proposal.md")).ok());
    let active_change = active.map(|c| ActiveChange {
        name: &c.name,
        proposal: proposal.as_deref(),
        deltas: &c.deltas,
    });

    let sources = ProjectionSources {
        constitution: constitution.as_deref(),
        rumbo: &tree.rumbo,
        active_change,
    };
    Ok(project(&sources))
}

/// The outcome of writing one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub path: String,
    pub fingerprint: String,
    pub wrote: bool,
}

impl From<Written> for ContextTarget {
    fn from(w: Written) -> Self {
        ContextTarget {
            path: w.path,
            fingerprint: w.fingerprint,
            written: w.wrote,
        }
    }
}

/// Compiles once and writes every declared target under `project_root`,
/// reporting each destination and whether it changed.
pub fn project_and_write(project_root: &Path) -> std::io::Result<Vec<Written>> {
    let content = compile(project_root)?;
    let fingerprint = fingerprint(&content);
    let targets = embedded_targets();

    let mut written = Vec::with_capacity(targets.paths.len());
    for rel in &targets.paths {
        let path = project_root.join(rel);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let wrote = match plan_block(&existing, &content, &fingerprint) {
            BlockPlan::Unchanged => false,
            BlockPlan::Write(next) => {
                atomic_write(&path, &next)?;
                true
            }
        };
        written.push(Written {
            path: rel.clone(),
            fingerprint: fingerprint.clone(),
            wrote,
        });
    }
    Ok(written)
}

/// The content fingerprint: hex SHA-256 of the compiled content.
fn fingerprint(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// What to do with a target file.
#[derive(Debug, PartialEq, Eq)]
enum BlockPlan {
    /// The managed block already carries this fingerprint: leave the file.
    Unchanged,
    /// Write this new file content.
    Write(String),
}

/// Plans the managed-block update for an existing file: replace the block's
/// interior (preserving everything outside byte for byte), append a block if
/// none is present, and skip entirely when the fingerprint already matches.
fn plan_block(existing: &str, content: &str, fingerprint: &str) -> BlockPlan {
    let block = render_block(content, fingerprint);
    match find_block(existing) {
        Some((start, end, current_fingerprint)) => {
            if current_fingerprint == fingerprint {
                return BlockPlan::Unchanged;
            }
            let mut next = String::with_capacity(existing.len() + block.len());
            next.push_str(&existing[..start]);
            next.push_str(&block);
            next.push_str(&existing[end..]);
            BlockPlan::Write(next)
        }
        None => {
            // Append the block, keeping existing content intact.
            let mut next = String::from(existing);
            if !next.is_empty() && !next.ends_with('\n') {
                next.push('\n');
            }
            if !next.is_empty() {
                next.push('\n');
            }
            next.push_str(&block);
            BlockPlan::Write(next)
        }
    }
}

/// Renders a full managed block (begin marker with fingerprint, content, end).
fn render_block(content: &str, fingerprint: &str) -> String {
    format!("{BEGIN_PREFIX}{fingerprint}{BEGIN_SUFFIX}\n{content}\n{END_MARKER}\n")
}

/// Locates the first well-formed managed block: returns its byte range
/// `[start, end)` (covering the begin marker line through the end marker line
/// and its trailing newline) and the fingerprint declared in the begin marker.
/// A begin without a matching end is treated as absent (the stray marker is
/// preserved as user content).
fn find_block(text: &str) -> Option<(usize, usize, String)> {
    let begin = text.find(BEGIN_PREFIX)?;
    let after_prefix = begin + BEGIN_PREFIX.len();
    let suffix_rel = text[after_prefix..].find(BEGIN_SUFFIX)?;
    let fingerprint = text[after_prefix..after_prefix + suffix_rel].to_string();
    // The end marker must come after the begin marker.
    let search_from = after_prefix + suffix_rel + BEGIN_SUFFIX.len();
    let end_rel = text[search_from..].find(END_MARKER)?;
    let mut end = search_from + end_rel + END_MARKER.len();
    // Consume a single trailing newline so the block owns its own line.
    if text[end..].starts_with('\n') {
        end += 1;
    }
    Some((begin, end, fingerprint))
}

/// Atomically writes `content` to `path`: write a sibling temp file, then
/// rename over the target (replaces on both Unix and Windows).
fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_sibling(path);
    std::fs::write(&tmp, content)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// A temp sibling path next to `path` (same directory, so rename is atomic).
fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("context");
    path.with_file_name(format!(".{name}.meltemi-tmp-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_targets_parse_with_agents_md_first() {
        let targets = embedded_targets();
        assert!(!targets.version.is_empty());
        assert_eq!(targets.paths.first().map(String::as_str), Some("AGENTS.md"));
        assert!(targets.paths.iter().any(|p| p == "CLAUDE.md"));
    }

    #[test]
    fn agents_md_is_forced_present_even_if_omitted() {
        let targets = parse_targets("version = \"v\"\n[[target]]\npath = \"OTHER.md\"\n").unwrap();
        assert!(targets.paths.contains(&"AGENTS.md".to_string()));
    }

    #[test]
    fn appends_a_block_to_a_file_without_markers() {
        let existing = "# My notes\n\nhand-written.\n";
        let plan = plan_block(existing, "COMPILED", "abc123");
        let BlockPlan::Write(next) = plan else {
            panic!("expected a write")
        };
        assert!(
            next.starts_with("# My notes\n\nhand-written.\n"),
            "user content is preserved at the top: {next:?}"
        );
        assert!(next.contains("sha256=abc123"));
        assert!(next.contains("COMPILED"));
        assert!(next.contains(END_MARKER));
    }

    #[test]
    fn replaces_only_the_block_interior_preserving_user_content() {
        // Scenario: Contenido del usuario intacto.
        let first = match plan_block("PREFIX\n\nSUFFIX\n", "V1", "fp1") {
            BlockPlan::Write(s) => s,
            _ => panic!(),
        };
        // The prefix survived; now reproject with new content.
        let second = match plan_block(&first, "V2", "fp2") {
            BlockPlan::Write(s) => s,
            _ => panic!(),
        };
        assert!(second.contains("PREFIX"), "user prefix preserved");
        assert!(second.contains("SUFFIX"), "user suffix preserved");
        assert!(
            second.contains("V2") && !second.contains("V1"),
            "block updated"
        );
        assert!(second.contains("sha256=fp2"));
    }

    #[test]
    fn idempotent_when_the_fingerprint_matches() {
        // Scenario: Idempotencia.
        let first = match plan_block("top\n", "COMPILED", "same") {
            BlockPlan::Write(s) => s,
            _ => panic!(),
        };
        assert_eq!(
            plan_block(&first, "COMPILED", "same"),
            BlockPlan::Unchanged,
            "the same fingerprint yields no write"
        );
    }

    #[test]
    fn a_begin_without_end_is_treated_as_absent() {
        // Adversarial: a stray begin marker is preserved as user content and a
        // fresh block is appended (never lose content).
        let existing = format!("{BEGIN_PREFIX}deadbeef{BEGIN_SUFFIX}\nno end here\n");
        let plan = plan_block(&existing, "COMPILED", "new");
        let BlockPlan::Write(next) = plan else {
            panic!("expected a write")
        };
        assert!(
            next.contains("no end here"),
            "the stray marker is preserved"
        );
        assert!(next.contains(END_MARKER), "a complete block is appended");
    }

    #[test]
    fn duplicate_blocks_only_the_first_is_managed() {
        // Two full blocks: the first is updated, the second is preserved as-is
        // (it lives outside the first block's range → user content).
        let doc = format!(
            "{BEGIN_PREFIX}aaa{BEGIN_SUFFIX}\nONE\n{END_MARKER}\nmiddle\n\
             {BEGIN_PREFIX}bbb{BEGIN_SUFFIX}\nTWO\n{END_MARKER}\n"
        );
        let next = match plan_block(&doc, "FRESH", "ccc") {
            BlockPlan::Write(s) => s,
            _ => panic!(),
        };
        assert!(next.contains("FRESH"), "first block updated");
        assert!(next.contains("middle"), "middle content preserved");
        assert!(next.contains("TWO"), "the second block is left intact");
    }

    #[test]
    fn compile_and_write_round_trip_on_a_fixture() {
        // Scenario: AGENTS.md del repo gestionado (in miniature).
        let dir = std::env::temp_dir().join(format!("meltemi-ctx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".meltemi").join("rumbo")).unwrap();
        std::fs::write(
            dir.join(".meltemi").join("constitution.md"),
            "# C\n\nrules\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(".meltemi").join("rumbo").join("product.md"),
            "---\ninclusion: siempre\n---\nPRODUCT RUMBO\n",
        )
        .unwrap();
        // A pre-existing AGENTS.md with hand-written content.
        std::fs::write(dir.join("AGENTS.md"), "# Hand notes\n\nkeep me.\n").unwrap();

        let written = project_and_write(&dir).unwrap();
        assert!(written.iter().any(|w| w.path == "AGENTS.md" && w.wrote));
        let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(
            agents.contains("keep me."),
            "hand-written content preserved"
        );
        assert!(agents.contains("PRODUCT RUMBO"), "rumbo projected");
        assert!(agents.contains("rules"), "constitution projected");

        // A second run is idempotent: nothing is written.
        let again = project_and_write(&dir).unwrap();
        assert!(
            again.iter().all(|w| !w.wrote),
            "re-projection without source changes writes nothing"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
