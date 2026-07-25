// SPDX-License-Identifier: Apache-2.0

//! Governance lint (gobernanza-comunidad): the community documents must exist
//! at the repository root with their minimum sections. This runs in CI on the
//! three platforms; a missing document or section fails the build.
//!
//! This is a documentation presence/section lint, not an agent e2e — it reads
//! the repository's own static governance files (never spawns a daemon).

use std::path::{Path, PathBuf};

/// The required documents and the section markers each must contain.
fn requirements() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            "GOVERNANCE.md",
            vec![
                "## Governance model",
                "## Amendment ratification",
                "## Becoming a maintainer",
            ],
        ),
        (
            "CONTRIBUTING.md",
            vec![
                "## Spec-driven contribution",
                "## Fast track",
                "## Quality checklist",
                "## Commit convention",
            ],
        ),
        (
            "CODE_OF_CONDUCT.md",
            vec!["## Our Pledge", "## Enforcement"],
        ),
        (
            "SECURITY.md",
            vec!["## Reporting a Vulnerability", "## Scope", "## Response"],
        ),
        (
            "CLA.md",
            vec!["Apache License 2.0", "No copyright assignment"],
        ),
        (
            ".github/PULL_REQUEST_TEMPLATE.md",
            vec!["Quality checklist", "co-authorship"],
        ),
        (".github/ISSUE_TEMPLATE/change-proposal.md", vec!["Why"]),
    ]
}

/// Returns the list of problems (missing files or sections) under `root`.
fn lint_governance(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (file, sections) in requirements() {
        let path = root.join(file);
        let Ok(content) = std::fs::read_to_string(&path) else {
            problems.push(format!("missing document: {file}"));
            continue;
        };
        for section in sections {
            if !content.contains(section) {
                problems.push(format!("{file}: missing section `{section}`"));
            }
        }
    }
    problems
}

/// The workspace root (this crate lives at `<root>/core/meltemid`).
fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

#[test]
fn governance_documents_are_present_and_complete() {
    // Scenario: Gobernanza refleja la realidad; Lint de presencia y secciones.
    let problems = lint_governance(&workspace_root());
    assert!(
        problems.is_empty(),
        "governance lint found problems:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_lint_fails_when_a_document_is_missing() {
    // Scenario: Lint de presencia y secciones
    // SHALL fallar si falta un
    // documento o una sección mínima. Negative control on an empty tree.
    let empty = std::env::temp_dir().join(format!("meltemi-gov-lint-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    let problems = lint_governance(&empty);
    assert!(
        problems.iter().any(|p| p.contains("missing document")),
        "the lint must detect missing documents"
    );
    assert!(
        problems.len() >= requirements().len(),
        "every required document is reported missing on an empty tree"
    );
    let _ = std::fs::remove_dir_all(&empty);
}
