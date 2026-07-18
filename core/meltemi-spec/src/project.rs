// SPDX-License-Identifier: Apache-2.0

//! Context projection compiler (proyeccion-contexto D1).
//!
//! [`project`] is a **pure, deterministic** function: the same sources always
//! produce byte-for-byte the same document. It compiles the constitution
//! (whole), the rumbo (honoring each file's inclusion rule — `siempre` is
//! injected, `por-patrón`/`manual` are listed as available but not injected),
//! and a summary of the active change. It performs no I/O and reads no clock,
//! so callers assemble the [`ProjectionSources`] from the parsed tree plus the
//! file bodies the model does not already hold (constitution, proposal).

use crate::model::{Inclusion, RumboFile, Spec};

/// Everything the projection needs, already read from disk by the caller.
#[derive(Debug, Default)]
pub struct ProjectionSources<'a> {
    /// The full body of `constitution.md`, when present.
    pub constitution: Option<&'a str>,
    /// The rumbo files, in a stable order (discovery sorts them by path).
    pub rumbo: &'a [RumboFile],
    /// A summary of the change in progress, when there is one.
    pub active_change: Option<ActiveChange<'a>>,
}

/// The active change to summarize in the projected context.
#[derive(Debug)]
pub struct ActiveChange<'a> {
    /// The change name (directory name).
    pub name: &'a str,
    /// The body of its `proposal.md`, when present.
    pub proposal: Option<&'a str>,
    /// Its delta specs.
    pub deltas: &'a [Spec],
}

/// Compiles the sources into the projected context document (the inner content
/// of a managed block — the marker wrapping and fingerprint are added by the
/// daemon writer). Deterministic: no timestamps, no absolute paths.
#[must_use]
pub fn project(sources: &ProjectionSources) -> String {
    let mut out = String::new();
    out.push_str("# Meltemi — contexto proyectado\n\n");
    out.push_str(
        "_Compilado desde `.meltemi/` por `meltemi project`. \
         El contenido del bloque gestionado se regenera; no editarlo a mano._\n",
    );

    if let Some(constitution) = sources.constitution {
        out.push_str("\n## Constitución\n\n");
        out.push_str(constitution.trim());
        out.push('\n');
    }

    if !sources.rumbo.is_empty() {
        out.push_str("\n## Rumbo\n");
        for file in sources.rumbo {
            project_rumbo(&mut out, file);
        }
    }

    if let Some(change) = &sources.active_change {
        project_change(&mut out, change);
    }

    out
}

/// Projects one rumbo file according to its inclusion rule.
fn project_rumbo(out: &mut String, file: &RumboFile) {
    let name = rumbo_name(file);
    match &file.inclusion {
        Some(Inclusion::Always) => {
            out.push_str(&format!("\n### {name}\n\n"));
            out.push_str(file.body.trim());
            out.push('\n');
        }
        Some(Inclusion::OnMatch(globs)) => {
            let list = globs.join("`, `");
            out.push_str(&format!(
                "\n### {name} — inclusión por patrón (`{list}`): disponible, no inyectado\n"
            ));
        }
        Some(Inclusion::Manual) => {
            out.push_str(&format!(
                "\n### {name} — inclusión manual: disponible, no inyectado\n"
            ));
        }
        // Missing/malformed front-matter: list it as available, never inject
        // (fail safe — do not spill an unclassified document into every agent).
        None => {
            out.push_str(&format!(
                "\n### {name} — inclusión sin declarar: disponible, no inyectado\n"
            ));
        }
    }
}

/// Projects the active-change summary: name, proposal, and the delta shape.
fn project_change(out: &mut String, change: &ActiveChange) {
    out.push_str(&format!("\n## Cambio activo: {}\n", change.name));
    if let Some(proposal) = change.proposal {
        out.push('\n');
        out.push_str(proposal.trim());
        out.push('\n');
    }
    if !change.deltas.is_empty() {
        out.push_str("\n### Deltas\n\n");
        for delta in change.deltas {
            let n = delta.requirements.len();
            let word = if n == 1 { "requisito" } else { "requisitos" };
            out.push_str(&format!("- `{}`: {n} {word}\n", delta.capability));
        }
    }
}

/// The display name of a rumbo file: its file stem (e.g. `product`).
fn rumbo_name(file: &RumboFile) -> String {
    file.path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("rumbo")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn rumbo(stem: &str, inclusion: Option<Inclusion>, body: &str) -> RumboFile {
        RumboFile {
            path: PathBuf::from(format!(".meltemi/rumbo/{stem}.md")),
            inclusion,
            ratification: None,
            body: body.to_string(),
        }
    }

    #[test]
    fn projection_is_deterministic() {
        // Scenario: Misma entrada, mismo documento.
        let rumbo = [rumbo("product", Some(Inclusion::Always), "always body")];
        let sources = ProjectionSources {
            constitution: Some("# C\n\nprinciples"),
            rumbo: &rumbo,
            active_change: None,
        };
        assert_eq!(project(&sources), project(&sources));
    }

    #[test]
    fn always_is_injected_and_patterned_is_only_listed() {
        // Scenario: Inclusión por regla.
        let rumbo = [
            rumbo("product", Some(Inclusion::Always), "PRODUCT BODY"),
            rumbo(
                "tech",
                Some(Inclusion::OnMatch(vec!["*.rs".into()])),
                "TECH BODY",
            ),
            rumbo("structure", Some(Inclusion::Manual), "STRUCTURE BODY"),
        ];
        let sources = ProjectionSources {
            constitution: Some("CONSTITUTION"),
            rumbo: &rumbo,
            active_change: None,
        };
        let doc = project(&sources);

        assert!(
            doc.contains("CONSTITUTION"),
            "constitution is injected whole"
        );
        assert!(
            doc.contains("PRODUCT BODY"),
            "an `always` rumbo is injected"
        );
        assert!(
            !doc.contains("TECH BODY"),
            "a `por-patrón` rumbo is not injected"
        );
        assert!(
            doc.contains("tech — inclusión por patrón"),
            "a `por-patrón` rumbo is listed as available"
        );
        assert!(
            !doc.contains("STRUCTURE BODY") && doc.contains("structure — inclusión manual"),
            "a `manual` rumbo is listed, not injected"
        );
    }

    #[test]
    fn undeclared_inclusion_is_listed_never_injected() {
        let rumbo = [rumbo("mystery", None, "SECRET BODY")];
        let sources = ProjectionSources {
            constitution: None,
            rumbo: &rumbo,
            active_change: None,
        };
        let doc = project(&sources);
        assert!(!doc.contains("SECRET BODY"), "undeclared is never injected");
        assert!(doc.contains("mystery — inclusión sin declarar"));
    }

    #[test]
    fn active_change_is_summarized_with_its_deltas() {
        let deltas = [Spec {
            capability: "cli-contract".into(),
            requirements: vec![
                crate::model::Requirement {
                    name: "R1".into(),
                    description: String::new(),
                    scenarios: vec![],
                    line: 1,
                },
                crate::model::Requirement {
                    name: "R2".into(),
                    description: String::new(),
                    scenarios: vec![],
                    line: 2,
                },
            ],
            deltas: vec![],
            source: PathBuf::from("x"),
        }];
        let sources = ProjectionSources {
            constitution: None,
            rumbo: &[],
            active_change: Some(ActiveChange {
                name: "add-thing",
                proposal: Some("## Why\nbecause"),
                deltas: &deltas,
            }),
        };
        let doc = project(&sources);
        assert!(doc.contains("## Cambio activo: add-thing"));
        assert!(doc.contains("because"));
        assert!(
            doc.contains("`cli-contract`: 2 requisitos"),
            "the delta shape is summarized: {doc}"
        );
    }

    #[test]
    fn empty_sources_still_produce_a_stable_header() {
        let doc = project(&ProjectionSources::default());
        assert!(doc.contains("# Meltemi — contexto proyectado"));
    }
}
