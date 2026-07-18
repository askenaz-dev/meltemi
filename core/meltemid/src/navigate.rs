// SPDX-License-Identifier: Apache-2.0

//! Read-only navigation of the method tree (navegacion-del-metodo): list and
//! show changes and living-truth specs, and validate without archiving. Every
//! handler here aggregates **already-persisted** state and MUST NOT mutate the
//! `.meltemi/` tree — the review checklist is read via [`crate::review::checklist`]
//! (never the mutating handler), and the test-link scan runs **once** per
//! listing, not once per change.

use std::path::{Path, PathBuf};

use serde_json::Value;

use meltemi_proto::error_codes;

use crate::rpc::RpcError;

fn require_dir(project_root: &str) -> Result<PathBuf, RpcError> {
    let root = PathBuf::from(project_root);
    if root.is_dir() {
        Ok(root)
    } else {
        Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ))
    }
}

fn not_found(kind_of: &str, name: &str, list_hint: &str) -> RpcError {
    RpcError::application(
        error_codes::ARTIFACT_NOT_FOUND,
        format!("{kind_of} not found"),
        "artifact_not_found",
        format!("no {kind_of} named `{name}` under `.meltemi/`"),
        Some(format!("List them with `{list_hint}`.")),
    )
}

fn changes_dir(root: &Path) -> PathBuf {
    root.join(".meltemi").join("changes")
}

fn archive_dir(root: &Path) -> PathBuf {
    changes_dir(root).join("archive")
}

fn specs_dir(root: &Path) -> PathBuf {
    root.join(".meltemi").join("specs")
}

/// Splits an archived directory name `YYYY-MM-DD-<change>` into `(name, date)`.
/// A name without the 11-char date prefix returns `(dirname, "")`.
fn parse_archived(dirname: &str) -> (String, String) {
    let bytes = dirname.as_bytes();
    let dated = bytes.len() > 11
        && bytes[..10].iter().enumerate().all(|(i, &b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        && bytes[10] == b'-';
    if dated {
        (dirname[11..].to_string(), dirname[..10].to_string())
    } else {
        (dirname.to_string(), String::new())
    }
}

/// Aggregates one change's read-only state. `linked` is the precomputed set of
/// scenario names named by a test source (computed once for the whole listing).
fn aggregate(
    repo_root: &Path,
    change_dir: &Path,
    name: &str,
    archived: bool,
    archived_at: Option<String>,
    linked: &std::collections::HashSet<String>,
) -> meltemi_proto::ChangeInfo {
    use meltemi_proto::{ChangeArtifacts, ChangeInfo};

    let artifacts = ChangeArtifacts {
        proposal: change_dir.join("proposal.md").is_file(),
        design: change_dir.join("design.md").is_file(),
        tasks: change_dir.join("tasks.md").is_file(),
        specs: change_dir.join("specs").is_dir(),
    };

    let (tasks_done, tasks_total) = std::fs::read_to_string(change_dir.join("tasks.md"))
        .map(|md| {
            let tasks = crate::implement::parse_tasks(&md);
            (
                tasks.iter().filter(|t| t.done).count() as u32,
                tasks.len() as u32,
            )
        })
        .unwrap_or((0, 0));

    // Review and verify readers are bound to the active `changes/<name>` paths;
    // archived history reports 0/0 (its artifacts are frozen, not decidable).
    let (review_decided, review_total, verified, verify_total) = if archived {
        (0, 0, 0, 0)
    } else {
        let checklist = crate::review::checklist(change_dir);
        let verifications = crate::verify::verify_change_with(repo_root, name, linked);
        (
            checklist.iter().filter(|i| i.state != "pending").count() as u32,
            checklist.len() as u32,
            verifications
                .iter()
                .filter(|v| v.status != crate::verify::ScenarioStatus::Unverified)
                .count() as u32,
            verifications.len() as u32,
        )
    };

    ChangeInfo {
        name: name.to_string(),
        archived,
        archived_at,
        artifacts,
        tasks_done,
        tasks_total,
        review_decided,
        review_total,
        verified,
        verify_total,
    }
}

/// `change/list`: active changes (name asc) then archived (most recent first),
/// each with aggregated read-only state.
pub async fn handle_change_list(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{ChangeListParams, ChangeListResult};

    let params: ChangeListParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("change/list: {e}")))?;
    let root = require_dir(&params.project_root)?;

    // The test-link scan is the expensive part — compute it ONCE for all changes.
    let linked = crate::verify::linked_scenarios(&root);

    let mut active: Vec<meltemi_proto::ChangeInfo> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(changes_dir(&root)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "archive" || !entry.path().is_dir() {
                continue;
            }
            active.push(aggregate(&root, &entry.path(), &name, false, None, &linked));
        }
    }
    active.sort_by(|a, b| a.name.cmp(&b.name));

    let mut archived: Vec<meltemi_proto::ChangeInfo> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(archive_dir(&root)) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let dirname = entry.file_name().to_string_lossy().into_owned();
            let (name, date) = parse_archived(&dirname);
            archived.push(aggregate(
                &root,
                &entry.path(),
                &name,
                true,
                Some(date),
                &linked,
            ));
        }
    }
    // Most recent first: the dated prefix sorts chronologically.
    archived.sort_by(|a, b| b.archived_at.cmp(&a.archived_at).then(a.name.cmp(&b.name)));

    let mut changes = active;
    changes.append(&mut archived);
    if let Some(limit) = params.limit {
        changes.truncate(limit as usize);
    }
    Ok(serde_json::to_value(ChangeListResult { changes }).expect("ChangeListResult serializes"))
}

/// `change/show`: a change's artifacts and per-capability deltas, verbatim.
pub async fn handle_change_show(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{ChangeArtifact, ChangeDelta, ChangeShowParams, ChangeShowResult};

    let params: ChangeShowParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("change/show: {e}")))?;
    let root = require_dir(&params.project_root)?;

    // Active first; else the archived copy (exact name after the date prefix).
    let active = changes_dir(&root).join(&params.change);
    let dir = if active.is_dir() {
        active
    } else {
        find_archived(&root, &params.change)
            .ok_or_else(|| not_found("change", &params.change, "change/list"))?
    };

    let mut artifacts = Vec::new();
    for name in ["proposal", "design", "tasks"] {
        if let Ok(content) = std::fs::read_to_string(dir.join(format!("{name}.md"))) {
            artifacts.push(ChangeArtifact {
                name: name.to_string(),
                content,
            });
        }
    }

    let mut deltas = Vec::new();
    if let Ok(caps) = std::fs::read_dir(dir.join("specs")) {
        let mut caps: Vec<_> = caps.flatten().filter(|e| e.path().is_dir()).collect();
        caps.sort_by_key(std::fs::DirEntry::file_name);
        for cap in caps {
            let capability = cap.file_name().to_string_lossy().into_owned();
            if let Ok(content) = std::fs::read_to_string(cap.path().join("spec.md")) {
                deltas.push(ChangeDelta {
                    capability,
                    content,
                });
            }
        }
    }

    let result = ChangeShowResult {
        name: params.change,
        artifacts,
        deltas,
    };
    Ok(serde_json::to_value(result).expect("ChangeShowResult serializes"))
}

/// Finds the most recent archived directory whose name (after the date prefix)
/// exactly equals `change`.
fn find_archived(root: &Path, change: &str) -> Option<PathBuf> {
    let mut best: Option<(String, PathBuf)> = None;
    for entry in std::fs::read_dir(archive_dir(root)).ok()?.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let dirname = entry.file_name().to_string_lossy().into_owned();
        let (name, date) = parse_archived(&dirname);
        if name == change && best.as_ref().is_none_or(|(d, _)| date > *d) {
            best = Some((date, entry.path()));
        }
    }
    best.map(|(_, p)| p)
}

/// `spec/list`: the living-truth capabilities with requirement/scenario counts.
pub async fn handle_spec_list(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SpecInfo, SpecListParams, SpecListResult};

    let params: SpecListParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("spec/list: {e}")))?;
    let root = require_dir(&params.project_root)?;

    let mut specs: Vec<SpecInfo> = Vec::new();
    if let Ok(caps) = std::fs::read_dir(specs_dir(&root)) {
        for cap in caps.flatten() {
            if !cap.path().is_dir() {
                continue;
            }
            let capability = cap.file_name().to_string_lossy().into_owned();
            let Ok(content) = std::fs::read_to_string(cap.path().join("spec.md")) else {
                continue;
            };
            let spec = meltemi_spec::parse_spec(&capability, &content, cap.path().join("spec.md"));
            specs.push(SpecInfo {
                capability,
                requirements: spec.requirements.len() as u32,
                scenarios: spec
                    .requirements
                    .iter()
                    .map(|r| r.scenarios.len() as u32)
                    .sum(),
            });
        }
    }
    specs.sort_by(|a, b| a.capability.cmp(&b.capability));
    Ok(serde_json::to_value(SpecListResult { specs }).expect("SpecListResult serializes"))
}

/// `spec/show`: a living capability parsed into requirements and scenarios.
pub async fn handle_spec_show(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SpecRequirement, SpecScenario, SpecShowParams, SpecShowResult, SpecStep};

    let params: SpecShowParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("spec/show: {e}")))?;
    let root = require_dir(&params.project_root)?;

    let spec_file = specs_dir(&root).join(&params.capability).join("spec.md");
    let content = std::fs::read_to_string(&spec_file)
        .map_err(|_| not_found("capability", &params.capability, "spec/list"))?;
    let spec = meltemi_spec::parse_spec(&params.capability, &content, &spec_file);

    let requirements = spec
        .requirements
        .into_iter()
        .map(|r| SpecRequirement {
            name: r.name,
            description: r.description,
            scenarios: r
                .scenarios
                .into_iter()
                .map(|s| SpecScenario {
                    name: s.name,
                    steps: s
                        .steps
                        .into_iter()
                        .map(|step| SpecStep {
                            marker: marker_word(step.marker),
                            text: step.text,
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect();

    let result = SpecShowResult {
        capability: params.capability,
        requirements,
    };
    Ok(serde_json::to_value(result).expect("SpecShowResult serializes"))
}

fn marker_word(marker: meltemi_spec::model::StepMarker) -> String {
    use meltemi_spec::model::StepMarker::*;
    match marker {
        When => "when",
        While => "while",
        If => "if",
        Then => "then",
        Where => "where",
        And => "and",
        Other => "other",
    }
    .to_string()
}

/// `sdd/validate`: validate a change (engine structure + EARS on its delta
/// requirements, plus a dry-run of its deltas against the living truth) or the
/// whole living truth when no change is named. Read-only; findings are a result.
pub async fn handle_sdd_validate(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SddValidateParams, SddValidateResult, ValidateDiagnostic};

    let params: SddValidateParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/validate: {e}")))?;
    let root = require_dir(&params.project_root)?;

    let location = |d: &meltemi_spec::Diagnostic| format!("{}:{}", d.file.display(), d.line);

    let (scope, target, diagnostics) = match &params.change {
        Some(change) => {
            let dir = changes_dir(&root).join(change);
            if !dir.is_dir() {
                return Err(not_found("change", change, "change/list"));
            }
            let mut diagnostics = Vec::new();
            let specs = dir.join("specs");
            if let Ok(caps) = std::fs::read_dir(&specs) {
                for cap in caps.flatten() {
                    if !cap.path().is_dir() {
                        continue;
                    }
                    let capability = cap.file_name().to_string_lossy().into_owned();
                    let path = cap.path().join("spec.md");
                    let Ok(md) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let delta = meltemi_spec::parse_delta(&capability, &md, &path);
                    // Structural + EARS validation of the delta's own requirements
                    // (build a synthetic spec from its Added/Modified blocks).
                    let requirements: Vec<meltemi_spec::model::Requirement> = delta
                        .operations
                        .iter()
                        .filter_map(|op| match op {
                            meltemi_spec::DeltaOp::Added(r)
                            | meltemi_spec::DeltaOp::Modified(r) => Some(r.clone()),
                            _ => None,
                        })
                        .collect();
                    let synthetic = meltemi_spec::model::Spec {
                        capability: capability.clone(),
                        requirements,
                        deltas: Vec::new(),
                        source: path.clone(),
                    };
                    for d in meltemi_spec::validate_spec(&synthetic) {
                        diagnostics.push(ValidateDiagnostic {
                            capability: capability.clone(),
                            location: location(&d),
                            message: d.message,
                        });
                    }
                    // Dry-run merge against the living truth (never written).
                    let living_path = specs_dir(&root).join(&capability).join("spec.md");
                    let living = std::fs::read_to_string(&living_path)
                        .map(|c| meltemi_spec::parse_spec(&capability, &c, &living_path))
                        .unwrap_or_else(|_| {
                            meltemi_spec::parse_spec(&capability, "", &living_path)
                        });
                    let (_, merge) = meltemi_spec::apply_delta(&living, &delta);
                    for d in merge {
                        diagnostics.push(ValidateDiagnostic {
                            capability: capability.clone(),
                            location: location(&d),
                            message: d.message,
                        });
                    }
                }
            }
            ("change".to_string(), Some(change.clone()), diagnostics)
        }
        None => {
            // Living-truth scope is `.meltemi/specs/` ONLY — never the change
            // deltas under `.meltemi/changes/` (those are validated per change).
            let mut diagnostics = Vec::new();
            if let Ok(caps) = std::fs::read_dir(specs_dir(&root)) {
                for cap in caps.flatten() {
                    if !cap.path().is_dir() {
                        continue;
                    }
                    let capability = cap.file_name().to_string_lossy().into_owned();
                    let path = cap.path().join("spec.md");
                    let Ok(md) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let spec = meltemi_spec::parse_spec(&capability, &md, &path);
                    for d in meltemi_spec::validate_spec(&spec) {
                        diagnostics.push(ValidateDiagnostic {
                            capability: capability.clone(),
                            location: location(&d),
                            message: d.message,
                        });
                    }
                }
            }
            ("living-truth".to_string(), None, diagnostics)
        }
    };

    let result = SddValidateResult {
        scope,
        target,
        clean: diagnostics.is_empty(),
        diagnostics,
    };
    Ok(serde_json::to_value(result).expect("SddValidateResult serializes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archived_dir_names_split_into_name_and_date() {
        assert_eq!(
            parse_archived("2026-07-18-flota-multiproveedor"),
            ("flota-multiproveedor".to_string(), "2026-07-18".to_string())
        );
        // A name without a date prefix keeps the whole name, empty date.
        assert_eq!(
            parse_archived("no-date-here"),
            ("no-date-here".to_string(), String::new())
        );
        // A leading token that only looks partly like a date is not stripped.
        assert_eq!(
            parse_archived("2026-13"),
            ("2026-13".to_string(), String::new())
        );
    }
}
