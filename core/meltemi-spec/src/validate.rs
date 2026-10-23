// SPDX-License-Identifier: Apache-2.0

//! Structural validation of the model against `artifact-format` (design M5).
//!
//! This layer checks only the structural subset — the rules that need no EARS
//! semantics and no delta application (those are `motor-ears-deltas`): every
//! requirement has a scenario, delta headers are canonical, capability names
//! are kebab-case, and rumbo front-matter is present and well-formed.

use crate::delta::{DeltaOp, DeltaSpec};
use crate::diagnostic::{Diagnostic, Rule};
use crate::model::{Inclusion, MeltemiTree, RumboFile, Spec};

/// Validates a single spec, returning every structural problem found.
#[must_use]
pub fn validate_spec(spec: &Spec) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    if !is_kebab_case(&spec.capability) {
        out.push(Diagnostic::new(
            spec.source.clone(),
            0,
            Rule::NonKebabCapability,
            format!("capability `{}` is not kebab-case", spec.capability),
        ));
    }

    for delta in &spec.deltas {
        if delta.operation.is_none() {
            out.push(Diagnostic::new(
                spec.source.clone(),
                delta.line,
                Rule::UnknownDeltaHeader,
                format!(
                    "`## {} Requirements` is not a canonical delta header (ADDED/MODIFIED/REMOVED/RENAMED)",
                    delta.raw
                ),
            ));
        }
    }

    for req in &spec.requirements {
        // A REMOVED entry names a requirement that ceases to exist. It is held
        // to what the format asks of it, its Reason and its Migration, and not
        // to the rules of a requirement: asking it for a scenario would ask the
        // delta to prove what it is withdrawing (release-con-el-push D11).
        if spec.is_removed_entry(req) {
            if !has_field(&req.description, "Reason") || !has_field(&req.description, "Migration") {
                out.push(Diagnostic::new(
                    spec.source.clone(),
                    req.line,
                    Rule::RemovedWithoutReasonOrMigration,
                    format!(
                        "removed requirement `{}` lacks Reason or Migration",
                        req.name
                    ),
                ));
            }
            continue;
        }
        if req.scenarios.is_empty() {
            out.push(Diagnostic::new(
                spec.source.clone(),
                req.line,
                Rule::RequirementWithoutScenario,
                format!("requirement `{}` has no scenario", req.name),
            ));
        }
    }

    // Semantic EARS rules (design D2/D6), after the structural ones.
    crate::ears::validate_ears(spec, &mut out);

    out
}

/// Validates the requirement blocks a change's delta brings — its ADDED and
/// MODIFIED ones — by the rules of a requirement. Each must be a complete
/// block, so a fragment without its normative statement or without its
/// scenarios is reported (`artifact-format`, «Reglas de contenido de delta»).
/// REMOVED and RENAMED entries are not requirements: the merge checks them.
///
/// This is what `meltemi validate <change>` runs before the dry-run merge.
#[must_use]
pub fn validate_delta_blocks(delta: &DeltaSpec) -> Vec<Diagnostic> {
    let requirements = delta
        .operations
        .iter()
        .filter_map(|op| match op {
            DeltaOp::Added(requirement) | DeltaOp::Modified(requirement) => {
                Some(requirement.clone())
            }
            DeltaOp::Removed { .. } | DeltaOp::Renamed { .. } => None,
        })
        .collect();
    validate_spec(&Spec {
        capability: delta.capability.clone(),
        requirements,
        deltas: Vec::new(),
        source: delta.source.clone(),
    })
}

/// Validates a single rumbo file's front-matter.
#[must_use]
pub fn validate_rumbo(rumbo: &RumboFile) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    match &rumbo.inclusion {
        None => out.push(Diagnostic::new(
            rumbo.path.clone(),
            0,
            Rule::MissingFrontMatter,
            "rumbo file has no valid `inclusion` front-matter (siempre | por-patrón | manual)",
        )),
        Some(Inclusion::OnMatch(globs)) if globs.is_empty() => out.push(Diagnostic::new(
            rumbo.path.clone(),
            0,
            Rule::OnMatchWithoutFileMatch,
            "`inclusion: por-patrón` requires a non-empty `fileMatch`",
        )),
        _ => {}
    }
    out
}

/// Validates a whole `.meltemi/` tree: living specs, rumbo files, and the
/// delta specs of active changes. Archived changes are historical and not
/// validated.
#[must_use]
pub fn validate_tree(tree: &MeltemiTree) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for spec in &tree.specs {
        out.extend(validate_spec(spec));
    }
    for rumbo in &tree.rumbo {
        out.extend(validate_rumbo(rumbo));
    }
    for change in &tree.changes {
        for delta in &change.deltas {
            out.extend(validate_spec(delta));
        }
    }
    out
}

/// Whether a requirement's description carries the field `name`, in any of
/// the three spellings `delta::field` accepts. The parser joins a
/// description's lines with spaces, so the field may start the text or follow
/// a space; it is never looked for line by line.
fn has_field(description: &str, name: &str) -> bool {
    [
        format!("**{name}**:"),
        format!("**{name}:**"),
        format!("{name}:"),
    ]
    .iter()
    .any(|prefix| {
        description.starts_with(prefix.as_str()) || description.contains(&format!(" {prefix}"))
    })
}

/// Whether `s` matches `^[a-z0-9]+(-[a-z0-9]+)*$`.
fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') || s.ends_with('-') || s.contains("--") {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::parse_rumbo;
    use crate::spec_parser::parse_spec;
    use std::path::PathBuf;

    #[test]
    fn rumbo_without_inclusion_is_flagged() {
        let rf = parse_rumbo(PathBuf::from("r.md"), "# sin front-matter\n");
        let diags = validate_rumbo(&rf);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::MissingFrontMatter);
    }

    #[test]
    fn on_match_without_globs_is_flagged() {
        let rf = parse_rumbo(PathBuf::from("r.md"), "---\ninclusion: por-patrón\n---\n");
        let diags = validate_rumbo(&rf);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::OnMatchWithoutFileMatch);
    }

    #[test]
    fn conformant_rumbo_has_no_diagnostics() {
        let rf = parse_rumbo(PathBuf::from("r.md"), "---\ninclusion: siempre\n---\n");
        assert!(validate_rumbo(&rf).is_empty());
    }

    #[test]
    fn conformant_spec_has_no_diagnostics() {
        let spec = parse_spec(
            "my-cap",
            "### Requirement: R\nThe system SHALL do it.\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n",
            "spec.md",
        );
        assert!(validate_spec(&spec).is_empty());
    }

    #[test]
    fn requirement_without_scenario_is_flagged() {
        let spec = parse_spec(
            "my-cap",
            "### Requirement: R\nThe system SHALL do it, con prosa y sin escenario.\n",
            "spec.md",
        );
        let diags = validate_spec(&spec);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::RequirementWithoutScenario);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn a_removed_entry_is_held_to_its_reason_and_migration_not_to_a_scenario() {
        // Scenario: Requisito eliminado con razón y migración
        let spec = parse_spec(
            "my-cap",
            "## REMOVED Requirements\n\n### Requirement: Old\n**Reason**: superseded.\n**Migration**: use New.\n",
            "spec.md",
        );
        assert!(
            validate_spec(&spec).is_empty(),
            "a removed entry needs no scenario and no normative verb"
        );
    }

    #[test]
    fn a_removed_entry_without_migration_is_still_invalid() {
        // Scenario: Requisito eliminado sin migración
        let spec = parse_spec(
            "my-cap",
            "## REMOVED Requirements\n### Requirement: Old\n**Reason**: superseded.\n",
            "spec.md",
        );
        let diags = validate_spec(&spec);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].rule, Rule::RemovedWithoutReasonOrMigration);
        assert_eq!(diags[0].line, 2);
    }

    #[test]
    fn only_the_removed_section_is_exempt_from_the_requirement_rules() {
        // An ADDED requirement without a scenario is still invalid when a
        // REMOVED section follows it in the same delta.
        let spec = parse_spec(
            "my-cap",
            "## ADDED Requirements\n### Requirement: New\nThe system SHALL do it.\n\n## REMOVED Requirements\n### Requirement: Old\n**Reason**: superseded.\n**Migration**: use New.\n",
            "spec.md",
        );
        let diags = validate_spec(&spec);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].rule, Rule::RequirementWithoutScenario);
        assert_eq!(diags[0].line, 2);
    }

    #[test]
    fn a_modified_fragment_is_reported_as_an_invalid_delta() {
        // Scenario: Requisito modificado con bloque parcial
        // A MODIFIED block replaces the whole requirement, so a fragment would
        // silently drop what it left out. The change's validation holds every
        // modified block to the rules of a requirement: a statement without its
        // scenarios, or scenarios without their statement, is reported. A
        // fragment that keeps some scenarios and drops others is not caught
        // here; that one is the review's (release-con-el-push 4.3).
        use crate::delta::parse_delta;
        let statement_only = parse_delta(
            "my-cap",
            "## MODIFIED Requirements\n### Requirement: R\nThe system SHALL do it differently.\n",
            "spec.md",
        );
        let diags = validate_delta_blocks(&statement_only);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::RequirementWithoutScenario),
            "{diags:?}"
        );

        let scenarios_only = parse_delta(
            "my-cap",
            "## MODIFIED Requirements\n### Requirement: R\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n",
            "spec.md",
        );
        let diags = validate_delta_blocks(&scenarios_only);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::RequirementWithoutNormativeVerb),
            "{diags:?}"
        );

        let complete = parse_delta(
            "my-cap",
            "## MODIFIED Requirements\n### Requirement: R\nThe system SHALL do it differently.\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n\n## REMOVED Requirements\n### Requirement: Old\n**Reason**: superseded.\n**Migration**: use R.\n",
            "spec.md",
        );
        assert!(
            validate_delta_blocks(&complete).is_empty(),
            "a complete block, and a removed entry the merge checks, pass"
        );
    }

    #[test]
    fn non_canonical_delta_header_is_flagged() {
        let spec = parse_spec("my-cap", "## AÑADIDO Requirements\n", "spec.md");
        let diags = validate_spec(&spec);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::UnknownDeltaHeader);
    }

    #[test]
    fn non_kebab_capability_is_flagged() {
        let spec = parse_spec(
            "My Cap",
            "### Requirement: R\n#### Scenario: S\n- **WHEN** a\n",
            "spec.md",
        );
        let diags = validate_spec(&spec);
        assert!(diags.iter().any(|d| d.rule == Rule::NonKebabCapability));
    }

    #[test]
    fn kebab_case_recognizer() {
        assert!(is_kebab_case("acp-session"));
        assert!(is_kebab_case("propose-flow"));
        assert!(is_kebab_case("cap9"));
        assert!(!is_kebab_case("My-Cap"));
        assert!(!is_kebab_case("has space"));
        assert!(!is_kebab_case("-lead"));
        assert!(!is_kebab_case("double--dash"));
        assert!(!is_kebab_case(""));
    }
}
