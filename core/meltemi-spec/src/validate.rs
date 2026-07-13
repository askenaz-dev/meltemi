// SPDX-License-Identifier: Apache-2.0

//! Structural validation of the model against `artifact-format` (design M5).
//!
//! This layer checks only the structural subset — the rules that need no EARS
//! semantics and no delta application (those are `motor-ears-deltas`): every
//! requirement has a scenario, delta headers are canonical, capability names
//! are kebab-case, and rumbo front-matter is present and well-formed.

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
        if req.scenarios.is_empty() {
            out.push(Diagnostic::new(
                spec.source.clone(),
                req.line,
                Rule::RequirementWithoutScenario,
                format!("requirement `{}` has no scenario", req.name),
            ));
        }
    }

    out
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
            "### Requirement: R\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n",
            "spec.md",
        );
        assert!(validate_spec(&spec).is_empty());
    }

    #[test]
    fn requirement_without_scenario_is_flagged() {
        let spec = parse_spec(
            "my-cap",
            "### Requirement: R\nsolo prosa, sin escenario\n",
            "spec.md",
        );
        let diags = validate_spec(&spec);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::RequirementWithoutScenario);
        assert_eq!(diags[0].line, 1);
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
