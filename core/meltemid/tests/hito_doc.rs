// SPDX-License-Identifier: Apache-2.0

//! Milestone-doc lint (hito-v01-aceptacion): the acceptance script documents
//! the criteria, the manual run for the maintainer, the budget metrics, and a
//! reproducible report template. The executable run itself is `e2e_hito`.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

#[test]
fn the_acceptance_script_documents_criteria_manual_run_and_report() {
    let doc = std::fs::read_to_string(repo_root().join("docs/hito-v01.md"))
        .expect("docs/hito-v01.md exists");

    // Scenario: Informe acompaña al tag — a per-criterion report template.
    assert!(
        doc.contains("acceptance report"),
        "the report is documented"
    );
    assert!(
        doc.contains("Verdict:"),
        "the report carries a per-criterion verdict"
    );
    for criterion in ["C1", "C2", "C3", "C4", "C5", "C6"] {
        assert!(doc.contains(criterion), "criterion {criterion} is listed");
    }

    // Scenario: Presupuestos en el informe — the §12 budgets are referenced.
    assert!(
        doc.contains("25 MB"),
        "the TUI size budget is in the acceptance criteria"
    );

    // Scenario: Corrida manual registrada — the maintainer's real-agent run.
    assert!(
        doc.contains("Manual run"),
        "the manual run with real agents is documented step by step"
    );
    // Two vendors in parallel — the milestone's defining property.
    assert!(
        doc.to_ascii_lowercase().contains("two agents")
            || doc.to_ascii_lowercase().contains("two real agents"),
        "the two-vendor parallel run is stated"
    );
}
