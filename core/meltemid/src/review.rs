// SPDX-License-Identifier: Apache-2.0

//! Spec review checklist (revision-specs-ux). The `review` verb walks a
//! change's delta requirements as a persistent checklist (approved / commented
//! / rejected), resumable across restarts; closing requires every item
//! decided; engine diagnostics anchor to the requirement that produced them; a
//! comment dispatches a rework instruction citing the requirement and reopens
//! the specs gate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use meltemi_proto::{
    ReviewItem, SddReviewDecideParams, SddReviewParams, SddReviewResult, error_codes,
};

use crate::rpc::{Peer, RpcError};
use crate::server::DaemonState;

/// The persisted review state of a change (states keyed by capability+name).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ReviewState {
    /// `"<capability>\u{1f}<requirement>" -> state`.
    states: BTreeMap<String, String>,
}

fn state_path(change_dir: &Path) -> PathBuf {
    change_dir.join(".review-state.json")
}

fn load_state(change_dir: &Path) -> ReviewState {
    std::fs::read_to_string(state_path(change_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_state(change_dir: &Path, state: &ReviewState) -> std::io::Result<()> {
    std::fs::create_dir_all(change_dir)?;
    std::fs::write(
        state_path(change_dir),
        serde_json::to_string_pretty(state).expect("ReviewState serializes"),
    )
}

fn key(capability: &str, requirement: &str) -> String {
    format!("{capability}\u{1f}{requirement}")
}

/// Builds the checklist from the change's delta specs, folding in the persisted
/// item states and the engine's review diagnostics anchored per requirement.
fn build_checklist(change_dir: &Path) -> Vec<ReviewItem> {
    let state = load_state(change_dir);
    let specs_dir = change_dir.join("specs");
    let mut items = Vec::new();

    let Ok(caps) = std::fs::read_dir(&specs_dir) else {
        return items;
    };
    for cap in caps.filter_map(Result::ok) {
        let spec_file = cap.path().join("spec.md");
        if !spec_file.is_file() {
            continue;
        }
        let Ok(delta) = std::fs::read_to_string(&spec_file) else {
            continue;
        };
        let capability = cap.file_name().to_string_lossy().into_owned();
        let parsed = meltemi_spec::parse_delta(&capability, &delta, spec_file.clone());
        // Diagnostics against an empty living spec (the change is not yet
        // archived); anchored per requirement by name.
        let living = empty_spec(&capability);
        let diagnostics = meltemi_spec::review_diagnostics(&living, &parsed);

        for op in &parsed.operations {
            let requirement = match op {
                meltemi_spec::DeltaOp::Added(r) | meltemi_spec::DeltaOp::Modified(r) => {
                    r.name.clone()
                }
                meltemi_spec::DeltaOp::Removed { name, .. } => name.clone(),
                meltemi_spec::DeltaOp::Renamed { to, .. } => to.clone(),
            };
            let anchored: Vec<String> = diagnostics
                .iter()
                .filter(|d| d.message.contains(&requirement))
                .map(|d| d.to_string())
                .collect();
            let state_str = state
                .states
                .get(&key(&capability, &requirement))
                .cloned()
                .unwrap_or_else(|| "pending".to_string());
            items.push(ReviewItem {
                capability: capability.clone(),
                requirement,
                state: state_str,
                diagnostics: anchored,
            });
        }
    }
    items.sort_by(|a, b| {
        (a.capability.as_str(), a.requirement.as_str())
            .cmp(&(b.capability.as_str(), b.requirement.as_str()))
    });
    items
}

fn empty_spec(capability: &str) -> meltemi_spec::Spec {
    meltemi_spec::Spec {
        capability: capability.to_string(),
        requirements: vec![],
        deltas: vec![],
        source: PathBuf::from(format!("specs/{capability}/spec.md")),
    }
}

fn result(change_dir: &Path, change_name: &str) -> SddReviewResult {
    let items = build_checklist(change_dir);
    let pending = items.iter().filter(|i| i.state == "pending").count() as u32;
    SddReviewResult {
        change_name: change_name.to_string(),
        items,
        pending,
        can_close: pending == 0,
    }
}

fn change_dir(project_root: &Path, change_name: &str) -> PathBuf {
    project_root
        .join(".meltemi")
        .join("changes")
        .join(change_name)
}

/// `sdd/review`: the checklist for a change's spec deltas.
pub async fn handle_review(params: Value, _state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    let params: SddReviewParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/review: {e}")))?;
    let root = project_dir(&params.project_root)?;
    let dir = change_dir(&root, &params.change_name);
    if !dir.is_dir() {
        return Err(RpcError::invalid_params(format!(
            "no change `{}`",
            params.change_name
        )));
    }
    Ok(serde_json::to_value(result(&dir, &params.change_name)).expect("serializes"))
}

/// `sdd/review-decide`: record a decision on one item. A `comment` dispatches a
/// rework instruction to the agent citing the requirement and reopens the specs
/// gate, linking the rework in the cycle state.
pub async fn handle_review_decide(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddReviewDecideParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/review-decide: {e}")))?;
    let root = project_dir(&params.project_root)?;
    let dir = change_dir(&root, &params.change_name);
    if !dir.is_dir() {
        return Err(RpcError::invalid_params(format!(
            "no change `{}`",
            params.change_name
        )));
    }

    let item_state = match params.decision.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        "comment" => "commented",
        other => {
            return Err(RpcError::invalid_params(format!(
                "unknown review decision `{other}` (approve|comment|reject)"
            )));
        }
    };
    let mut review = load_state(&dir);
    review.states.insert(
        key(&params.capability, &params.requirement),
        item_state.to_string(),
    );
    save_state(&dir, &review).map_err(RpcError::internal)?;

    // A comment reopens the specs gate and dispatches a rework instruction
    // citing the requirement (Scenario: Comentario reabre el gate).
    if params.decision == "comment" {
        let comment = params.comment.clone().unwrap_or_default();
        let instruction = format!(
            "Rework the specs artifact. Address this review comment on \
             Requirement: {} — {comment}",
            params.requirement
        );
        // Reopen the specs gate via the SDD gate flow: comment on the current
        // gate reworks without consuming it.
        let gate_params = serde_json::json!({
            "projectRoot": params.project_root,
            "changeName": params.change_name,
            "decision": "comment",
            "comment": instruction,
        });
        // Best-effort: only if a cycle exists with a pending gate.
        let _ = crate::sdd_flow::handle_gate(gate_params, state, peer).await;
    }

    Ok(serde_json::to_value(result(&dir, &params.change_name)).expect("serializes"))
}

fn project_dir(root: &str) -> Result<PathBuf, RpcError> {
    let path = PathBuf::from(root);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{root}` is not an existing directory"),
            Some("Pass an existing repository root.".into()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_delta(dir: &Path, cap: &str, body: &str) {
        let cap_dir = dir.join("specs").join(cap);
        std::fs::create_dir_all(&cap_dir).unwrap();
        std::fs::write(cap_dir.join("spec.md"), body).unwrap();
    }

    #[test]
    fn checklist_reflects_states_and_close_requires_all_decided() {
        // Scenarios: Review reanudable; Cierre exige decisión total.
        let dir = std::env::temp_dir().join(format!("meltemi-rev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_delta(
            &dir,
            "cap",
            "## ADDED Requirements\n### Requirement: One\nThe system SHALL a.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n\n\
             ### Requirement: Two\nThe system SHALL b.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
        );

        // Fresh: two pending items, cannot close.
        let r = result(&dir, "c");
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.pending, 2);
        assert!(!r.can_close);

        // Decide one; the persisted state survives a reload (resumable).
        let mut state = load_state(&dir);
        state.states.insert(key("cap", "One"), "approved".into());
        save_state(&dir, &state).unwrap();
        let r = result(&dir, "c");
        assert_eq!(r.pending, 1, "one still pending → cannot close");
        assert!(!r.can_close);
        assert_eq!(
            r.items
                .iter()
                .find(|i| i.requirement == "One")
                .unwrap()
                .state,
            "approved"
        );

        // Decide the second: now closable.
        let mut state = load_state(&dir);
        state.states.insert(key("cap", "Two"), "rejected".into());
        save_state(&dir, &state).unwrap();
        let r = result(&dir, "c");
        assert!(r.can_close && r.pending == 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
