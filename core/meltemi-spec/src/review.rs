// SPDX-License-Identifier: Apache-2.0

//! Review-time semantic detectors (revision-specs-ux): duplicate requirement
//! names, no-op MODIFIED deltas, and dangling `Requirement:` references. These
//! complement the structural [`crate::validate`] and delta [`crate::delta`]
//! checks; they are pure and designed for **zero false positives**.

use crate::delta::{DeltaOp, DeltaSpec, apply_delta};
use crate::diagnostic::{Diagnostic, Rule};
use crate::model::{Requirement, Spec};

/// Normalizes a requirement name for duplicate detection: trimmed, lowercased,
/// internal whitespace runs collapsed to a single space.
#[must_use]
pub fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// All review diagnostics for applying `delta` onto `living`: duplicates,
/// no-op modifications, and dangling references, anchored to the delta.
#[must_use]
pub fn review_diagnostics(living: &Spec, delta: &DeltaSpec) -> Vec<Diagnostic> {
    let mut diags = duplicate_names(living, delta);
    diags.extend(noop_modifications(living, delta));
    diags.extend(dangling_references(living, delta));
    diags
}

/// ADDED requirements whose normalized name collides with an already-present
/// requirement (living, or an earlier ADDED in the same delta).
#[must_use]
pub fn duplicate_names(living: &Spec, delta: &DeltaSpec) -> Vec<Diagnostic> {
    let mut seen: std::collections::HashSet<String> = living
        .requirements
        .iter()
        .map(|r| normalize_name(&r.name))
        .collect();
    let mut diags = Vec::new();
    for op in &delta.operations {
        if let DeltaOp::Added(req) = op {
            let norm = normalize_name(&req.name);
            if !seen.insert(norm) {
                diags.push(Diagnostic::new(
                    delta.source.clone(),
                    req.line,
                    Rule::DuplicateRequirementName,
                    format!(
                        "added requirement `{}` duplicates an existing name (normalized)",
                        req.name
                    ),
                ));
            }
        }
    }
    diags
}

/// MODIFIED requirements whose statement and scenarios are identical to the
/// living requirement they target — the operation changes nothing.
#[must_use]
pub fn noop_modifications(living: &Spec, delta: &DeltaSpec) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for op in &delta.operations {
        if let DeltaOp::Modified(new) = op
            && let Some(existing) = living.requirements.iter().find(|r| r.name == new.name)
            && requirements_equal(existing, new)
        {
            diags.push(Diagnostic::new(
                delta.source.clone(),
                new.line,
                Rule::ModifiedNoOp,
                format!(
                    "modified requirement `{}` is identical to the living one",
                    new.name
                ),
            ));
        }
    }
    diags
}

/// Explicit `Requirement: X` references (in any requirement's description) that
/// name a requirement absent from the merged spec.
#[must_use]
pub fn dangling_references(living: &Spec, delta: &DeltaSpec) -> Vec<Diagnostic> {
    let (merged, _) = apply_delta(living, delta);
    let present: std::collections::HashSet<String> = merged
        .requirements
        .iter()
        .map(|r| normalize_name(&r.name))
        .collect();

    let mut diags = Vec::new();
    // Scan the delta's own added/modified requirement descriptions for
    // references, so the diagnostic anchors in the delta the author is editing.
    for op in &delta.operations {
        let req = match op {
            DeltaOp::Added(r) | DeltaOp::Modified(r) => r,
            _ => continue,
        };
        for referenced in references_in(&req.description) {
            if !present.contains(&normalize_name(&referenced)) {
                diags.push(Diagnostic::new(
                    delta.source.clone(),
                    req.line,
                    Rule::DanglingReference,
                    format!(
                        "requirement `{}` references `{referenced}`, which does not exist after the delta",
                        req.name
                    ),
                ));
            }
        }
    }
    diags
}

/// Extracts `Requirement: X` references from prose (up to end of line or a
/// closing sentence punctuation).
fn references_in(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let needle = "Requirement:";
    let mut rest = text;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|&c| c != '\n' && c != '.' && c != ',' && c != ')' && c != '»' && c != '"')
            .collect();
        let name = name.trim().trim_matches('`').trim();
        if !name.is_empty() {
            refs.push(name.to_string());
        }
        rest = after;
    }
    refs
}

/// Whether two requirements are semantically equal (name, statement, scenarios
/// with steps), ignoring the source line.
fn requirements_equal(a: &Requirement, b: &Requirement) -> bool {
    a.name == b.name
        && a.description.trim() == b.description.trim()
        && a.scenarios.len() == b.scenarios.len()
        && a.scenarios.iter().zip(&b.scenarios).all(|(x, y)| {
            x.name == y.name
                && x.steps.len() == y.steps.len()
                && x.steps
                    .iter()
                    .zip(&y.steps)
                    .all(|(sx, sy)| sx.marker == sy.marker && sx.text.trim() == sy.text.trim())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_parser::parse_spec;

    fn living() -> Spec {
        parse_spec(
            "cap",
            "## Requirements\n\n### Requirement: The Alpha One\n\
             The system SHALL a. See Requirement: The Beta One.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n\n\
             ### Requirement: The Beta One\nThe system SHALL b.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
            "specs/cap/spec.md",
        )
    }

    #[test]
    fn normalization_collapses_case_and_whitespace() {
        assert_eq!(normalize_name("  The   Alpha  One "), "the alpha one");
    }

    #[test]
    fn duplicate_normalized_name_is_flagged() {
        // Scenario: Duplicado detectado (differs only in case/spacing).
        let delta = crate::delta::parse_delta(
            "cap",
            "## ADDED Requirements\n### Requirement: the   ALPHA one\n\
             The system SHALL a.\n#### Scenario: s\n- **WHEN** a\n- **THEN** b\n",
            "d.md",
        );
        let diags = duplicate_names(&living(), &delta);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::DuplicateRequirementName);
    }

    #[test]
    fn distinct_added_name_is_not_flagged() {
        let delta = crate::delta::parse_delta(
            "cap",
            "## ADDED Requirements\n### Requirement: The Gamma One\n\
             The system SHALL g.\n#### Scenario: s\n- **WHEN** a\n- **THEN** b\n",
            "d.md",
        );
        assert!(
            duplicate_names(&living(), &delta).is_empty(),
            "no false positive"
        );
    }

    #[test]
    fn identical_modified_is_a_noop() {
        // Scenario: No-op señalado (replicating the living requirement exactly).
        let delta = crate::delta::parse_delta(
            "cap",
            "## MODIFIED Requirements\n### Requirement: The Beta One\nThe system SHALL b.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
            "d.md",
        );
        let diags = noop_modifications(&living(), &delta);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::ModifiedNoOp);
    }

    #[test]
    fn changed_modified_is_not_a_noop() {
        let delta = crate::delta::parse_delta(
            "cap",
            "## MODIFIED Requirements\n### Requirement: The Beta One\nThe system SHALL b differently.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
            "d.md",
        );
        assert!(
            noop_modifications(&living(), &delta).is_empty(),
            "a real change is not a no-op"
        );
    }

    #[test]
    fn removing_a_referenced_requirement_dangles() {
        // Scenario: Referencia rota — The Alpha One references The Beta One,
        // which this delta removes.
        let delta = crate::delta::parse_delta(
            "cap",
            "## REMOVED Requirements\n### Requirement: The Beta One\n\
             **Reason**: obsolete\n**Migration**: none\n\n\
             ## MODIFIED Requirements\n### Requirement: The Alpha One\n\
             The system SHALL a. See Requirement: The Beta One.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
            "d.md",
        );
        let diags = dangling_references(&living(), &delta);
        assert!(
            diags.iter().any(|d| d.rule == Rule::DanglingReference),
            "the dangling reference is flagged: {diags:?}"
        );
    }

    #[test]
    fn intact_reference_is_not_flagged() {
        // A MODIFIED that references a still-present requirement → no diagnostic.
        let delta = crate::delta::parse_delta(
            "cap",
            "## MODIFIED Requirements\n### Requirement: The Alpha One\n\
             The system SHALL a. See Requirement: The Beta One.\n\
             #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
            "d.md",
        );
        assert!(
            dangling_references(&living(), &delta).is_empty(),
            "no false positive"
        );
    }
}
