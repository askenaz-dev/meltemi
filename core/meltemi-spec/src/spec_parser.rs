// SPDX-License-Identifier: Apache-2.0

//! Line-oriented spec parser (design M3).
//!
//! The canonical format is line-oriented, so a scanner is enough — no Markdown
//! engine and no dependencies. It recognizes the anchor lines and preserves
//! every element's 1-based line number; anything else is prose.

use std::path::{Path, PathBuf};

use crate::model::{DeltaOperation, DeltaSection, Requirement, Scenario, Spec, Step, StepMarker};

/// Parses a spec from its file contents. `capability` comes from the
/// containing directory; `source` is the file path (for diagnostics).
#[must_use]
pub fn parse_spec(
    capability: impl Into<String>,
    content: &str,
    source: impl Into<PathBuf>,
) -> Spec {
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut deltas: Vec<DeltaSection> = Vec::new();
    let mut current: Option<Requirement> = None;
    let mut description = String::new();

    for (index, raw) in content.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end();

        if let Some((level, text)) = heading(line) {
            if level == 2
                && let Some(op) = delta_operation_word(text)
            {
                deltas.push(DeltaSection {
                    operation: DeltaOperation::classify(op),
                    raw: op.to_string(),
                    line: line_no,
                });
            } else if level == 3
                && let Some(name) = text.strip_prefix("Requirement:")
            {
                finish_requirement(&mut requirements, &mut current, &mut description);
                current = Some(Requirement {
                    name: name.trim().to_string(),
                    description: String::new(),
                    scenarios: Vec::new(),
                    line: line_no,
                });
            } else if level == 4
                && let Some(name) = text.strip_prefix("Scenario:")
                && let Some(req) = current.as_mut()
            {
                req.scenarios.push(Scenario {
                    name: name.trim().to_string(),
                    steps: Vec::new(),
                    line: line_no,
                });
            }
            continue;
        }

        if let Some(step) = parse_step(line) {
            // Steps belong to the current scenario, if any.
            if let Some(scenario) = current.as_mut().and_then(|r| r.scenarios.last_mut()) {
                scenario.steps.push(step);
            }
            continue;
        }

        // Prose before the first scenario is the requirement's description.
        if let Some(req) = current.as_ref()
            && req.scenarios.is_empty()
            && !line.trim().is_empty()
        {
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(line.trim());
        }
    }

    finish_requirement(&mut requirements, &mut current, &mut description);

    Spec {
        capability: capability.into(),
        requirements,
        deltas,
        source: source.into(),
    }
}

/// Reads and parses a spec file, deriving the capability from its parent
/// directory (`.../<capability>/spec.md`).
pub fn parse_spec_file(path: &Path) -> std::io::Result<Spec> {
    let content = std::fs::read_to_string(path)?;
    let capability = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    Ok(parse_spec(capability, &content, path.to_path_buf()))
}

/// Attaches the accumulated description and pushes the finished requirement.
fn finish_requirement(
    requirements: &mut Vec<Requirement>,
    current: &mut Option<Requirement>,
    description: &mut String,
) {
    if let Some(mut req) = current.take() {
        req.description = std::mem::take(description);
        requirements.push(req);
    } else {
        description.clear();
    }
}

/// Returns `(level, text)` for a Markdown ATX heading — the number of leading
/// `#` and the trimmed text after the required space. `None` if not a heading.
fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes == 0 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes, rest.trim()))
}

/// Returns the operation word of a `## <OP> Requirements` delta header.
/// A plain `## Requirements` (living-truth section header, no operation) or a
/// non-requirements heading returns `None` and is not treated as a delta.
fn delta_operation_word(text: &str) -> Option<&str> {
    let word = text.strip_suffix("Requirements")?.trim();
    (!word.is_empty()).then_some(word)
}

/// Parses a `- **MARKER** text` step line.
fn parse_step(line: &str) -> Option<Step> {
    let rest = line.trim_start().strip_prefix("- ")?.strip_prefix("**")?;
    let end = rest.find("**")?;
    let marker = StepMarker::classify(&rest[..end]);
    let text = rest[end + 2..].trim().to_string();
    Some(Step { marker, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# cap — título

## ADDED Requirements

### Requirement: Primero
Descripción del primero en prosa.

#### Scenario: Caso feliz
- **WHEN** ocurre algo
- **THEN** pasa lo esperado

### Requirement: Segundo

#### Scenario: Otro caso
- **WHEN** otra condición
- **THEN** otro resultado
";

    #[test]
    fn parses_requirements_scenarios_and_steps() {
        let spec = parse_spec("cap", SAMPLE, "cap/spec.md");
        assert_eq!(spec.requirements.len(), 2);
        assert_eq!(spec.deltas.len(), 1);
        assert_eq!(spec.deltas[0].operation, Some(DeltaOperation::Added));

        let first = &spec.requirements[0];
        assert_eq!(first.name, "Primero");
        assert_eq!(first.description, "Descripción del primero en prosa.");
        assert_eq!(first.scenarios.len(), 1);
        assert_eq!(first.scenarios[0].name, "Caso feliz");
        assert_eq!(first.scenarios[0].steps.len(), 2);
        assert_eq!(first.scenarios[0].steps[0].marker, StepMarker::When);
        assert_eq!(first.scenarios[0].steps[1].marker, StepMarker::Then);
    }

    #[test]
    fn scenario_must_be_exactly_four_hashes() {
        let three = "### Requirement: X\n### Scenario: no cuenta\n- **WHEN** a\n";
        let spec = parse_spec("cap", three, "s.md");
        assert_eq!(spec.requirements.len(), 1);
        assert!(
            spec.requirements[0].scenarios.is_empty(),
            "a 3-hash heading is not a scenario"
        );

        let five = "### Requirement: X\n##### Scenario: tampoco\n";
        let spec = parse_spec("cap", five, "s.md");
        assert!(spec.requirements[0].scenarios.is_empty());
    }

    #[test]
    fn plain_requirements_section_is_not_a_delta() {
        // Living-truth specs use `## Requirements` as the section header; it is
        // not a delta operation.
        let spec = parse_spec(
            "cap",
            "## Requirements\n\n### Requirement: R\n#### Scenario: S\n- **WHEN** a\n",
            "s.md",
        );
        assert!(spec.deltas.is_empty());
        assert_eq!(spec.requirements.len(), 1);
    }

    #[test]
    fn non_canonical_delta_header_is_recorded_without_operation() {
        let spec = parse_spec("cap", "## AÑADIDO Requirements\n", "s.md");
        assert_eq!(spec.deltas.len(), 1);
        assert_eq!(spec.deltas[0].operation, None);
        assert_eq!(spec.deltas[0].raw, "AÑADIDO");
    }

    #[test]
    fn line_numbers_are_preserved() {
        let spec = parse_spec("cap", SAMPLE, "s.md");
        assert_eq!(spec.requirements[0].line, 5);
        assert_eq!(spec.requirements[0].scenarios[0].line, 8);
    }
}
