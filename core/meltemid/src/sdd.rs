// SPDX-License-Identifier: Apache-2.0

//! SDD authoring cycle (ciclo-sdd-autoria D1/D3).
//!
//! A change in authoring advances `proposal → specs → design → tasks`. Each
//! artifact is (1) drafted by the agent, (2) validated by the spec engine
//! (structure + EARS + dry-run delta application), (3) gated by a human with
//! three outcomes: approve, comment-to-rework, abort. The cycle **state lives
//! in the change directory**, not memory, so it survives daemon restarts and
//! is inspectable. Two modes: `spec-full` (a gate per artifact) and
//! `fast-forward` (all four artifacts, one final gate) — the latter eligible
//! only for a small change with no new capabilities and no MODIFIED/REMOVED
//! deltas, unless a human explicitly forces it (recorded).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The artifacts of a change, in cycle order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Artifact {
    Proposal,
    Specs,
    Design,
    Tasks,
}

impl Artifact {
    /// The next artifact in the cycle, or `None` after `tasks`.
    #[must_use]
    pub fn next(self) -> Option<Artifact> {
        match self {
            Artifact::Proposal => Some(Artifact::Specs),
            Artifact::Specs => Some(Artifact::Design),
            Artifact::Design => Some(Artifact::Tasks),
            Artifact::Tasks => None,
        }
    }

    /// The file the artifact is written to (relative to the change dir).
    #[must_use]
    pub fn file_name(self) -> &'static str {
        match self {
            Artifact::Proposal => "proposal.md",
            Artifact::Specs => "specs",
            Artifact::Design => "design.md",
            Artifact::Tasks => "tasks.md",
        }
    }
}

/// The authoring mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// A gate per artifact.
    SpecFull,
    /// All artifacts, one final gate.
    FastForward,
}

/// A human gate decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// Approve: advance the cycle.
    Approve,
    /// Comment: rework the current artifact (no gate consumed).
    Comment(String),
    /// Abort the cycle.
    Abort,
}

/// One recorded cycle decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    /// The artifact the decision was about (or the final gate in fast-forward).
    pub artifact: Option<Artifact>,
    /// The decision kind: approve, comment, abort.
    pub kind: String,
    /// The comment, when the decision was a comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// The persisted state of a change's authoring cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleState {
    /// The change name.
    pub change_name: String,
    /// The authoring mode.
    pub mode: Mode,
    /// Whether the mode was forced by a human against the eligibility criterion.
    #[serde(default)]
    pub forced_mode: bool,
    /// The artifact currently being authored; `None` when the cycle is done or
    /// aborted.
    pub current: Option<Artifact>,
    /// Whether a gate is pending on the current artifact (or the final gate).
    #[serde(default)]
    pub gate_pending: bool,
    /// Whether the cycle was aborted.
    #[serde(default)]
    pub aborted: bool,
    /// The recorded decisions, in order.
    #[serde(default)]
    pub decisions: Vec<Decision>,
}

impl CycleState {
    /// Starts a new cycle at the proposal, with no gate pending yet.
    #[must_use]
    pub fn new(change_name: impl Into<String>, mode: Mode, forced_mode: bool) -> Self {
        Self {
            change_name: change_name.into(),
            mode,
            forced_mode,
            current: Some(Artifact::Proposal),
            gate_pending: false,
            aborted: false,
            decisions: Vec::new(),
        }
    }

    /// Marks the current artifact validated and awaiting a gate.
    pub fn open_gate(&mut self) {
        if self.current.is_some() {
            self.gate_pending = true;
        }
    }

    /// Whether the cycle finished all artifacts (not aborted).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.aborted && self.current.is_none()
    }

    /// Applies a gate decision, returning what the caller should do next.
    /// `approve` in spec-full advances one artifact; in fast-forward it
    /// completes the cycle. `comment` keeps the same artifact (rework, no gate
    /// consumed). `abort` ends the cycle.
    pub fn decide(&mut self, outcome: GateOutcome) -> GateStep {
        let artifact = self.current;
        match outcome {
            GateOutcome::Comment(comment) => {
                // Comment does not consume the gate: stay on the artifact.
                self.gate_pending = false;
                self.decisions.push(Decision {
                    artifact,
                    kind: "comment".into(),
                    comment: Some(comment.clone()),
                });
                GateStep::Rework(comment)
            }
            GateOutcome::Abort => {
                self.gate_pending = false;
                self.aborted = true;
                self.current = None;
                self.decisions.push(Decision {
                    artifact,
                    kind: "abort".into(),
                    comment: None,
                });
                GateStep::Aborted
            }
            GateOutcome::Approve => {
                self.gate_pending = false;
                self.decisions.push(Decision {
                    artifact,
                    kind: "approve".into(),
                    comment: None,
                });
                match self.mode {
                    // One final gate: approval completes the whole cycle.
                    Mode::FastForward => {
                        self.current = None;
                        GateStep::Completed
                    }
                    // A gate per artifact: advance to the next.
                    Mode::SpecFull => {
                        self.current = self.current.and_then(Artifact::next);
                        match self.current {
                            Some(next) => GateStep::Advanced(next),
                            None => GateStep::Completed,
                        }
                    }
                }
            }
        }
    }

    /// The state file path within the change directory.
    #[must_use]
    pub fn state_path(change_dir: &Path) -> PathBuf {
        change_dir.join(".cycle-state.json")
    }

    /// Persists the state into the change directory (survives restarts, D1).
    pub fn save(&self, change_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(change_dir)?;
        let json = serde_json::to_string_pretty(self).expect("CycleState serializes");
        std::fs::write(Self::state_path(change_dir), json)
    }

    /// Loads a persisted cycle from a change directory, if present.
    #[must_use]
    pub fn load(change_dir: &Path) -> Option<CycleState> {
        let text = std::fs::read_to_string(Self::state_path(change_dir)).ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// What the caller should do after a gate decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateStep {
    /// Advance to the next artifact (spec-full approval).
    Advanced(Artifact),
    /// The cycle completed.
    Completed,
    /// Rework the current artifact with this comment.
    Rework(String),
    /// The cycle was aborted.
    Aborted,
}

/// The proportionality criterion (D3): a change is eligible for fast-forward
/// only when it introduces **no new capability** and its deltas carry **no
/// MODIFIED and no REMOVED** operations (small ADDED-only, or docs). Anything
/// else requires spec-full unless a human forces fast-forward.
#[must_use]
pub fn fast_forward_eligible(
    deltas: &[meltemi_spec::Spec],
    living_capabilities: &[String],
) -> bool {
    for delta in deltas {
        // A new capability (a delta whose capability is not in living truth).
        if !living_capabilities.contains(&delta.capability) {
            return false;
        }
        for section in &delta.deltas {
            match section.operation {
                Some(meltemi_spec::DeltaOperation::Modified)
                | Some(meltemi_spec::DeltaOperation::Removed) => return false,
                _ => {}
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_full_advances_artifact_by_artifact_on_approval() {
        // Scenario: Gate aprueba y avanza.
        let mut s = CycleState::new("add-thing", Mode::SpecFull, false);
        assert_eq!(s.current, Some(Artifact::Proposal));
        s.open_gate();
        assert!(s.gate_pending);
        assert_eq!(
            s.decide(GateOutcome::Approve),
            GateStep::Advanced(Artifact::Specs)
        );
        assert!(!s.gate_pending, "the gate was consumed");
        assert_eq!(
            s.decide(GateOutcome::Approve),
            GateStep::Advanced(Artifact::Design)
        );
        assert_eq!(
            s.decide(GateOutcome::Approve),
            GateStep::Advanced(Artifact::Tasks)
        );
        assert_eq!(s.decide(GateOutcome::Approve), GateStep::Completed);
        assert!(s.is_complete());
        // Four approvals recorded.
        assert_eq!(
            s.decisions.iter().filter(|d| d.kind == "approve").count(),
            4
        );
    }

    #[test]
    fn comment_reworks_without_advancing_or_consuming_the_gate() {
        // Scenario: Comentario reelabora sin reiniciar.
        let mut s = CycleState::new("add-thing", Mode::SpecFull, false);
        s.open_gate();
        let step = s.decide(GateOutcome::Comment("tighten the scope".into()));
        assert_eq!(step, GateStep::Rework("tighten the scope".into()));
        assert_eq!(
            s.current,
            Some(Artifact::Proposal),
            "still the same artifact"
        );
        assert!(!s.gate_pending);
        assert_eq!(s.decisions.last().unwrap().kind, "comment");
    }

    #[test]
    fn abort_ends_the_cycle() {
        let mut s = CycleState::new("add-thing", Mode::SpecFull, false);
        s.open_gate();
        assert_eq!(s.decide(GateOutcome::Abort), GateStep::Aborted);
        assert!(s.aborted && s.current.is_none() && !s.is_complete());
    }

    #[test]
    fn fast_forward_completes_on_a_single_final_gate() {
        // Scenario: Fast-forward con gate único.
        let mut s = CycleState::new("tweak", Mode::FastForward, false);
        s.open_gate();
        assert_eq!(s.decide(GateOutcome::Approve), GateStep::Completed);
        assert!(s.is_complete());
        assert_eq!(s.decisions.len(), 1, "one final gate");
    }

    #[test]
    fn state_survives_a_round_trip_to_disk() {
        // Scenario: El estado sobrevive un reinicio.
        let dir = std::env::temp_dir().join(format!("meltemi-sdd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = CycleState::new("add-thing", Mode::SpecFull, false);
        s.open_gate();
        s.decide(GateOutcome::Approve); // now on specs
        s.save(&dir).unwrap();

        let loaded = CycleState::load(&dir).expect("state reloads");
        assert_eq!(loaded.current, Some(Artifact::Specs));
        assert_eq!(loaded.decisions.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    fn delta(capability: &str, op: Option<meltemi_spec::DeltaOperation>) -> meltemi_spec::Spec {
        meltemi_spec::Spec {
            capability: capability.into(),
            requirements: vec![],
            deltas: vec![meltemi_spec::DeltaSection {
                operation: op,
                raw: "ADDED".into(),
                line: 1,
            }],
            source: PathBuf::from("x"),
        }
    }

    #[test]
    fn fast_forward_eligibility_follows_the_written_criterion() {
        use meltemi_spec::DeltaOperation;
        let living = vec!["cli-contract".to_string()];

        // ADDED-only to an existing capability → eligible.
        assert!(fast_forward_eligible(
            &[delta("cli-contract", Some(DeltaOperation::Added))],
            &living
        ));
        // A MODIFIED delta → not eligible (Scenario: cambio grande exige spec-full).
        assert!(!fast_forward_eligible(
            &[delta("cli-contract", Some(DeltaOperation::Modified))],
            &living
        ));
        // A REMOVED delta → not eligible.
        assert!(!fast_forward_eligible(
            &[delta("cli-contract", Some(DeltaOperation::Removed))],
            &living
        ));
        // A new capability → not eligible.
        assert!(!fast_forward_eligible(
            &[delta("brand-new-cap", Some(DeltaOperation::Added))],
            &living
        ));
    }
}
