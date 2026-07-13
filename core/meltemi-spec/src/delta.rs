// SPDX-License-Identifier: Apache-2.0

//! Delta parsing and application (design D3/D4) — the mechanics of `/archive`.
//!
//! A delta spec groups requirements under operation headers
//! (`## ADDED/MODIFIED/REMOVED/RENAMED Requirements`). This module parses that
//! into structured operations and applies them onto a living spec, producing
//! the merged spec plus diagnostics for invalid deltas (duplicate adds,
//! dangling modifies/removes/renames, removals missing Reason/Migration).

use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Rule};
use crate::model::{DeltaOperation, Requirement, Spec};
use crate::spec_parser::parse_spec;

/// A parsed delta spec: an ordered list of operations for one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaSpec {
    pub capability: String,
    pub operations: Vec<DeltaOp>,
    pub source: PathBuf,
}

/// A single delta operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaOp {
    /// Add a new requirement.
    Added(Requirement),
    /// Replace an existing requirement's full block.
    Modified(Requirement),
    /// Remove a requirement, with its reason and migration.
    Removed {
        name: String,
        reason: Option<String>,
        migration: Option<String>,
        line: usize,
    },
    /// Rename a requirement.
    Renamed {
        from: String,
        to: String,
        line: usize,
    },
}

/// Parses a delta spec's contents into structured operations.
#[must_use]
pub fn parse_delta(
    capability: impl Into<String>,
    content: &str,
    source: impl Into<PathBuf>,
) -> DeltaSpec {
    let capability = capability.into();
    let source = source.into();
    let mut operations = Vec::new();

    let mut current_op: Option<DeltaOperation> = None;
    let mut block: Option<Block> = None;
    let mut pending_rename_from: Option<String> = None;

    for (index, raw) in content.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end();

        if let Some((level, text)) = atx_heading(line) {
            if level == 2
                && let Some(op) = delta_operation(text)
            {
                flush_block(&mut block, current_op, &source, &mut operations);
                current_op = Some(op);
                pending_rename_from = None;
                continue;
            }
            if level == 3
                && let Some(name) = text.strip_prefix("Requirement:")
            {
                flush_block(&mut block, current_op, &source, &mut operations);
                block = Some(Block {
                    name: name.trim().to_string(),
                    line: line_no,
                    body: String::new(),
                });
                continue;
            }
        }

        if let Some(b) = block.as_mut() {
            b.body.push_str(line);
            b.body.push('\n');
        } else if current_op == Some(DeltaOperation::Renamed) {
            if let Some(name) = rename_field(line, "FROM") {
                pending_rename_from = Some(name);
            } else if let Some(to) = rename_field(line, "TO")
                && let Some(from) = pending_rename_from.take()
            {
                operations.push(DeltaOp::Renamed {
                    from,
                    to,
                    line: line_no,
                });
            }
        }
    }
    flush_block(&mut block, current_op, &source, &mut operations);

    DeltaSpec {
        capability,
        operations,
        source,
    }
}

/// Applies a delta onto a living spec, returning the merged spec and any
/// diagnostics for invalid operations. Deterministic: living requirements keep
/// their order, additions go to the end.
#[must_use]
pub fn apply_delta(living: &Spec, delta: &DeltaSpec) -> (Spec, Vec<Diagnostic>) {
    let mut requirements = living.requirements.clone();
    let mut diags = Vec::new();

    for op in &delta.operations {
        match op {
            DeltaOp::Added(req) => {
                if requirements.iter().any(|r| r.name == req.name) {
                    diags.push(diag(
                        delta,
                        req.line,
                        Rule::AddedRequirementExists,
                        format!("added requirement `{}` already exists", req.name),
                    ));
                } else {
                    requirements.push(req.clone());
                }
            }
            DeltaOp::Modified(req) => match position(&requirements, &req.name) {
                Some(pos) => requirements[pos] = req.clone(),
                None => diags.push(diag(
                    delta,
                    req.line,
                    Rule::ModifiedRequirementMissing,
                    format!("modified requirement `{}` does not exist", req.name),
                )),
            },
            DeltaOp::Removed {
                name,
                reason,
                migration,
                line,
            } => {
                if reason.is_none() || migration.is_none() {
                    diags.push(diag(
                        delta,
                        *line,
                        Rule::RemovedWithoutReasonOrMigration,
                        format!("removed requirement `{name}` lacks Reason or Migration"),
                    ));
                }
                match position(&requirements, name) {
                    Some(pos) => {
                        requirements.remove(pos);
                    }
                    None => diags.push(diag(
                        delta,
                        *line,
                        Rule::RemovedRequirementMissing,
                        format!("removed requirement `{name}` does not exist"),
                    )),
                }
            }
            DeltaOp::Renamed { from, to, line } => {
                if position(&requirements, to).is_some() {
                    diags.push(diag(
                        delta,
                        *line,
                        Rule::RenamedToExists,
                        format!("renamed target `{to}` is already in use"),
                    ));
                } else if let Some(pos) = position(&requirements, from) {
                    requirements[pos].name = to.clone();
                } else {
                    diags.push(diag(
                        delta,
                        *line,
                        Rule::RenamedFromMissing,
                        format!("renamed source `{from}` does not exist"),
                    ));
                }
            }
        }
    }

    let merged = Spec {
        capability: living.capability.clone(),
        requirements,
        deltas: Vec::new(),
        source: living.source.clone(),
    };
    (merged, diags)
}

/// A requirement block being accumulated (header name + raw body lines).
struct Block {
    name: String,
    line: usize,
    body: String,
}

/// Turns an accumulated block into an operation for the current section.
fn flush_block(
    block: &mut Option<Block>,
    op: Option<DeltaOperation>,
    source: &Path,
    out: &mut Vec<DeltaOp>,
) {
    let Some(block) = block.take() else { return };
    match op {
        Some(DeltaOperation::Added) => {
            out.push(DeltaOp::Added(requirement_from_block(&block, source)))
        }
        Some(DeltaOperation::Modified) => {
            out.push(DeltaOp::Modified(requirement_from_block(&block, source)));
        }
        Some(DeltaOperation::Removed) => out.push(DeltaOp::Removed {
            name: block.name,
            reason: field(&block.body, "Reason"),
            migration: field(&block.body, "Migration"),
            line: block.line,
        }),
        // Renamed uses FROM/TO lines, not `### Requirement:` blocks; None means
        // a stray requirement outside any section — dropped.
        Some(DeltaOperation::Renamed) | None => {}
    }
}

/// Reconstructs a full [`Requirement`] from a block by reparsing it, keeping
/// the block's original line for diagnostics.
fn requirement_from_block(block: &Block, source: &Path) -> Requirement {
    let text = format!("### Requirement: {}\n{}", block.name, block.body);
    let mut req = parse_spec("", &text, source.to_path_buf())
        .requirements
        .pop()
        .unwrap_or(Requirement {
            name: block.name.clone(),
            description: String::new(),
            scenarios: Vec::new(),
            line: block.line,
        });
    req.line = block.line;
    req
}

fn position(reqs: &[Requirement], name: &str) -> Option<usize> {
    reqs.iter().position(|r| r.name == name)
}

fn diag(delta: &DeltaSpec, line: usize, rule: Rule, message: String) -> Diagnostic {
    Diagnostic::new(delta.source.clone(), line, rule, message)
}

/// Recognizes a `## <OP> Requirements` delta operation header.
fn delta_operation(text: &str) -> Option<DeltaOperation> {
    let word = text.strip_suffix("Requirements")?.trim();
    DeltaOperation::classify(word)
}

/// Extracts a `**Field**: value` (or `Field:`) line's value from a body.
fn field(body: &str, name: &str) -> Option<String> {
    for line in body.lines() {
        let l = line.trim();
        for prefix in [
            format!("**{name}**:"),
            format!("**{name}:**"),
            format!("{name}:"),
        ] {
            if let Some(rest) = l.strip_prefix(&prefix) {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

/// Extracts the name from a `- FROM: \`name\`` / `- TO: \`name\`` rename line.
fn rename_field(line: &str, key: &str) -> Option<String> {
    let l = line.trim().trim_start_matches("- ").trim();
    for prefix in [
        format!("**{key}:**"),
        format!("**{key}**:"),
        format!("{key}:"),
    ] {
        if let Some(rest) = l.strip_prefix(&prefix) {
            return Some(rest.trim().trim_matches('`').trim().to_string());
        }
    }
    None
}

/// `(level, text)` for a Markdown ATX heading, mirroring the spec parser.
fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes == 0 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes, rest.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn living() -> Spec {
        parse_spec(
            "cap",
            "## Requirements\n\n### Requirement: A\nThe system SHALL a.\n#### Scenario: sa\n- **WHEN** x\n- **THEN** y\n\n### Requirement: B\nThe system SHALL b.\n#### Scenario: sb\n- **WHEN** x\n- **THEN** y\n",
            "specs/cap/spec.md",
        )
    }

    #[test]
    fn parses_added_and_modified_operations() {
        let delta = parse_delta(
            "cap",
            "## ADDED Requirements\n### Requirement: C\nThe system SHALL c.\n#### Scenario: sc\n- **WHEN** a\n- **THEN** b\n\n## MODIFIED Requirements\n### Requirement: A\nThe system SHALL a differently.\n#### Scenario: sa\n- **WHEN** a\n- **THEN** b\n",
            "d.md",
        );
        assert_eq!(delta.operations.len(), 2);
        assert!(matches!(delta.operations[0], DeltaOp::Added(_)));
        assert!(matches!(delta.operations[1], DeltaOp::Modified(_)));
    }

    #[test]
    fn parses_removed_with_reason_and_migration() {
        let delta = parse_delta(
            "cap",
            "## REMOVED Requirements\n### Requirement: B\n**Reason**: obsolete\n**Migration**: use A\n",
            "d.md",
        );
        assert_eq!(delta.operations.len(), 1);
        match &delta.operations[0] {
            DeltaOp::Removed {
                name,
                reason,
                migration,
                ..
            } => {
                assert_eq!(name, "B");
                assert_eq!(reason.as_deref(), Some("obsolete"));
                assert_eq!(migration.as_deref(), Some("use A"));
            }
            other => panic!("expected Removed, got {other:?}"),
        }
    }

    #[test]
    fn parses_renamed_from_to() {
        let delta = parse_delta(
            "cap",
            "## RENAMED Requirements\n- FROM: `A`\n- TO: `Alpha`\n",
            "d.md",
        );
        assert_eq!(
            delta.operations,
            vec![DeltaOp::Renamed {
                from: "A".into(),
                to: "Alpha".into(),
                line: 3,
            }]
        );
    }

    #[test]
    fn applies_added_and_modified() {
        let living = living();
        let delta = parse_delta(
            "cap",
            "## ADDED Requirements\n### Requirement: C\nThe system SHALL c.\n#### Scenario: sc\n- **WHEN** a\n- **THEN** b\n",
            "d.md",
        );
        let (merged, diags) = apply_delta(&living, &delta);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(merged.requirements.len(), 3);
        assert_eq!(merged.requirements[2].name, "C"); // appended at the end
    }

    #[test]
    fn flags_duplicate_add() {
        let living = living();
        let delta = parse_delta(
            "cap",
            "## ADDED Requirements\n### Requirement: A\nThe system SHALL a.\n#### Scenario: s\n- **WHEN** a\n",
            "d.md",
        );
        let (merged, diags) = apply_delta(&living, &delta);
        assert_eq!(merged.requirements.len(), 2, "no duplicate added");
        assert!(diags.iter().any(|d| d.rule == Rule::AddedRequirementExists));
    }

    #[test]
    fn flags_missing_modified() {
        let (_, diags) = apply_delta(
            &living(),
            &parse_delta(
                "cap",
                "## MODIFIED Requirements\n### Requirement: Z\nThe system SHALL z.\n#### Scenario: s\n- **WHEN** a\n",
                "d.md",
            ),
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::ModifiedRequirementMissing)
        );
    }

    #[test]
    fn removes_and_flags_missing_fields() {
        let living = living();
        let (merged, diags) = apply_delta(
            &living,
            &parse_delta(
                "cap",
                "## REMOVED Requirements\n### Requirement: B\n**Reason**: x\n**Migration**: y\n",
                "d.md",
            ),
        );
        assert_eq!(merged.requirements.len(), 1);
        assert!(diags.is_empty());

        let (_, diags2) = apply_delta(
            &living,
            &parse_delta(
                "cap",
                "## REMOVED Requirements\n### Requirement: B\n**Reason**: x\n",
                "d.md",
            ),
        );
        assert!(
            diags2
                .iter()
                .any(|d| d.rule == Rule::RemovedWithoutReasonOrMigration)
        );
    }

    #[test]
    fn renames_and_flags_conflicts() {
        let (merged, diags) = apply_delta(
            &living(),
            &parse_delta(
                "cap",
                "## RENAMED Requirements\n- FROM: `A`\n- TO: `Alpha`\n",
                "d.md",
            ),
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(merged.requirements[0].name, "Alpha");

        let (_, to_exists) = apply_delta(
            &living(),
            &parse_delta(
                "cap",
                "## RENAMED Requirements\n- FROM: `A`\n- TO: `B`\n",
                "d.md",
            ),
        );
        assert!(to_exists.iter().any(|d| d.rule == Rule::RenamedToExists));

        let (_, from_missing) = apply_delta(
            &living(),
            &parse_delta(
                "cap",
                "## RENAMED Requirements\n- FROM: `Z`\n- TO: `Zeta`\n",
                "d.md",
            ),
        );
        assert!(
            from_missing
                .iter()
                .any(|d| d.rule == Rule::RenamedFromMissing)
        );
    }
}
