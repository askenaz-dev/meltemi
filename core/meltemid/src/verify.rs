// SPDX-License-Identifier: Apache-2.0

//! The `verify` verb: a per-requirement checklist of a change's spec deltas,
//! where each scenario is linked to a test by the living convention "the
//! scenario name is the source of the test name" (comandos-verify-archive D1).
//!
//! A scenario is *linked* when a test source in the repository names it in a
//! `Scenario:` marker; it is *manual* when the human marked it verified (with a
//! note); otherwise it is *unverified*. Manual marks persist under the change,
//! so verification is resumable. This verb reads and records — it never mutates
//! the specs.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The verification status of one scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioStatus {
    /// A test source names this scenario (linked by the naming convention).
    Linked,
    /// The human marked it verified with a note.
    Manual,
    /// Neither — it blocks archiving until verified or excepted.
    Unverified,
}

impl ScenarioStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ScenarioStatus::Linked => "linked",
            ScenarioStatus::Manual => "manual",
            ScenarioStatus::Unverified => "unverified",
        }
    }
}

/// One scenario's verification state.
#[derive(Debug, Clone)]
pub struct ScenarioVerification {
    pub capability: String,
    pub requirement: String,
    pub scenario: String,
    pub status: ScenarioStatus,
    pub note: Option<String>,
}

/// The change directory under the target repo's `.meltemi/`.
fn change_dir(repo_root: &Path, change: &str) -> PathBuf {
    repo_root.join(".meltemi").join("changes").join(change)
}

fn manual_marks_path(repo_root: &Path, change: &str) -> PathBuf {
    change_dir(repo_root, change).join(".verify.jsonl")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManualMark {
    scenario: String,
    note: String,
}

/// The scenarios of a change's deltas, each with its capability and requirement.
#[must_use]
pub fn change_scenarios(repo_root: &Path, change: &str) -> Vec<(String, String, String)> {
    let specs = change_dir(repo_root, change).join("specs");
    let mut out = Vec::new();
    let Ok(caps) = std::fs::read_dir(&specs) else {
        return out;
    };
    for cap in caps.flatten() {
        if !cap.path().is_dir() {
            continue;
        }
        let capability = cap.file_name().to_string_lossy().into_owned();
        let spec_file = cap.path().join("spec.md");
        let Ok(content) = std::fs::read_to_string(&spec_file) else {
            continue;
        };
        let delta = meltemi_spec::delta::parse_delta(&capability, &content, &spec_file);
        for op in &delta.operations {
            let req = match op {
                meltemi_spec::delta::DeltaOp::Added(r)
                | meltemi_spec::delta::DeltaOp::Modified(r) => r,
                _ => continue,
            };
            for scenario in &req.scenarios {
                out.push((capability.clone(), req.name.clone(), scenario.name.clone()));
            }
        }
    }
    out
}

/// The set of scenario names that a test source in the repo links by naming
/// them in a `Scenario:` marker (the codebase's test↔scenario convention).
#[must_use]
pub fn linked_scenarios(repo_root: &Path) -> HashSet<String> {
    let mut linked = HashSet::new();
    let mut stack = vec![repo_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                // Skip heavy/irrelevant trees.
                if !matches!(name.as_ref(), "target" | ".git" | "node_modules") {
                    stack.push(path);
                }
            } else if name.ends_with(".rs")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                collect_scenario_markers(&content, &mut linked);
            }
        }
    }
    linked
}

/// Collects the scenario names named in `Scenario:` markers of a source file.
fn collect_scenario_markers(content: &str, into: &mut HashSet<String>) {
    for line in content.lines() {
        if let Some(idx) = line.find("Scenario:") {
            let name = line[idx + "Scenario:".len()..].trim().trim_end_matches('.');
            if !name.is_empty() {
                into.insert(name.to_string());
            }
        }
    }
}

fn load_manual(repo_root: &Path, change: &str) -> Vec<ManualMark> {
    std::fs::read_to_string(manual_marks_path(repo_root, change))
        .map(|s| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str::<ManualMark>(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Verifies a change: links each scenario to a test or a manual mark. Reads
/// only; the returned list is the checklist (resumable via persisted marks).
#[must_use]
pub fn verify_change(repo_root: &Path, change: &str) -> Vec<ScenarioVerification> {
    let linked = linked_scenarios(repo_root);
    verify_change_with(repo_root, change, &linked)
}

/// As [`verify_change`], with a precomputed linked-scenario set — so a listing
/// over many changes walks the repo for test markers only once, not per change.
#[must_use]
pub fn verify_change_with(
    repo_root: &Path,
    change: &str,
    linked: &HashSet<String>,
) -> Vec<ScenarioVerification> {
    let manual = load_manual(repo_root, change);
    change_scenarios(repo_root, change)
        .into_iter()
        .map(|(capability, requirement, scenario)| {
            let manual_note = manual
                .iter()
                .find(|m| m.scenario == scenario)
                .map(|m| m.note.clone());
            let status = if manual_note.is_some() {
                ScenarioStatus::Manual
            } else if linked.contains(&scenario) {
                ScenarioStatus::Linked
            } else {
                ScenarioStatus::Unverified
            };
            ScenarioVerification {
                capability,
                requirement,
                scenario,
                status,
                note: manual_note,
            }
        })
        .collect()
}

/// Records a manual verification of a scenario with a note (persisted, so the
/// checklist is resumable).
pub fn mark_manual(
    repo_root: &Path,
    change: &str,
    scenario: &str,
    note: &str,
) -> Result<(), String> {
    use std::io::Write;
    let path = manual_marks_path(repo_root, change);
    if !change_dir(repo_root, change).is_dir() {
        return Err(format!("change `{change}` does not exist"));
    }
    let mark = ManualMark {
        scenario: scenario.to_string(),
        note: note.to_string(),
    };
    let mut line = serde_json::to_string(&mark).expect("ManualMark serializes");
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

/// Whether every scenario of a change is verified (linked or manual).
#[must_use]
pub fn all_verified(verifications: &[ScenarioVerification]) -> bool {
    verifications
        .iter()
        .all(|v| v.status != ScenarioStatus::Unverified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_markers_are_collected_from_comments() {
        // Scenario: Escenario vinculado a su test.
        let mut set = HashSet::new();
        collect_scenario_markers(
            "// Scenario: Fusión total o nada.\nfn foo() {}\n// noise\n",
            &mut set,
        );
        assert!(
            set.contains("Fusión total o nada"),
            "trailing dot trimmed: {set:?}"
        );
        assert_eq!(set.len(), 1);
    }
}
