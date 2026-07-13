// SPDX-License-Identifier: Apache-2.0

//! EARS validation (design D2).
//!
//! A light semantic layer on top of the structural parse: every scenario must
//! carry a recognized EARS marker, and every requirement's description must be
//! normative (`SHALL`/`MUST`). The parser already classifies step markers; this
//! module turns that into located diagnostics.

use crate::diagnostic::{Diagnostic, Rule};
use crate::model::{Scenario, Spec, StepMarker};

/// Appends EARS diagnostics for a spec's requirements and scenarios.
pub(crate) fn validate_ears(spec: &Spec, out: &mut Vec<Diagnostic>) {
    for req in &spec.requirements {
        if !has_normative_verb(&req.description) {
            out.push(Diagnostic::new(
                spec.source.clone(),
                req.line,
                Rule::RequirementWithoutNormativeVerb,
                format!(
                    "requirement `{}` has no normative verb (SHALL/MUST)",
                    req.name
                ),
            ));
        }
        for scenario in &req.scenarios {
            if !has_ears_marker(scenario) {
                out.push(Diagnostic::new(
                    spec.source.clone(),
                    scenario.line,
                    Rule::ScenarioWithoutEarsMarker,
                    format!(
                        "scenario `{}` has no EARS marker (WHEN/WHILE/IF/THEN/WHERE)",
                        scenario.name
                    ),
                ));
            }
        }
    }
}

/// Whether a scenario has at least one step with a recognized EARS marker.
fn has_ears_marker(scenario: &Scenario) -> bool {
    scenario.steps.iter().any(|s| {
        matches!(
            s.marker,
            StepMarker::When
                | StepMarker::While
                | StepMarker::If
                | StepMarker::Then
                | StepMarker::Where
        )
    })
}

/// Whether `text` contains `SHALL` or `MUST` as a whole word.
fn has_normative_verb(text: &str) -> bool {
    contains_word(text, "SHALL") || contains_word(text, "MUST")
}

/// Whole-word (ASCII-alphabetic-bounded) containment.
fn contains_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w == word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_parser::parse_spec;

    fn diags(content: &str) -> Vec<Diagnostic> {
        let spec = parse_spec("cap", content, "s.md");
        let mut out = Vec::new();
        validate_ears(&spec, &mut out);
        out
    }

    #[test]
    fn normative_requirement_with_ears_scenario_is_clean() {
        let out = diags(
            "### Requirement: R\nThe system SHALL do it.\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n",
        );
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn non_normative_requirement_is_flagged() {
        let out = diags(
            "### Requirement: R\nThe system should maybe do it.\n#### Scenario: S\n- **WHEN** a\n",
        );
        assert!(
            out.iter()
                .any(|d| d.rule == Rule::RequirementWithoutNormativeVerb)
        );
    }

    #[test]
    fn scenario_without_ears_marker_is_flagged() {
        let out = diags(
            "### Requirement: R\nThe system SHALL do it.\n#### Scenario: S\n- **NOTE** something\n",
        );
        assert!(
            out.iter()
                .any(|d| d.rule == Rule::ScenarioWithoutEarsMarker)
        );
    }

    #[test]
    fn shall_is_matched_as_a_whole_word() {
        assert!(has_normative_verb("it SHALL happen"));
        assert!(has_normative_verb("MUST occur, always"));
        assert!(!has_normative_verb("marshall the troops")); // not a normative SHALL
        assert!(!has_normative_verb("it should happen"));
    }
}
