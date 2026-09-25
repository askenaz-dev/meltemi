// SPDX-License-Identifier: Apache-2.0

//! The agents guide and the adapter declare the same things about Codex
//! (codex-al-dia task 3.2, design D1, D2 and D5).
//!
//! The guide is where a user learns which CLI version to install and what the
//! gate does and does not cover. Those are facts the adapter decides in code,
//! so the guide is checked against the code: a floor moved by a re-anchoring,
//! or a gate value changed, fails here instead of leaving the guide to age in
//! silence.

use std::path::PathBuf;

use meltemi_adapters::codex::{SPEC, gate, version, wire};

/// The guide's section for an entry: from its heading to the next one.
fn section(guide: &str, id: &str) -> String {
    let start = guide
        .find(&format!("### {id} — "))
        .unwrap_or_else(|| panic!("the guide has a section for `{id}`"));
    let body = &guide[start..];
    let end = body[4..]
        .find("\n### ")
        .map_or(body.len(), |offset| offset + 4);
    // Prose wraps, so facts are compared on collapsed whitespace.
    body[..end].split_whitespace().collect::<Vec<_>>().join(" ")
}

fn guide() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("agentes.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The minimum CLI version a section declares, as `Minimum CLI version:
/// **<version>**`.
fn declared_minimum(section: &str) -> Option<String> {
    let rest = section.split_once("Minimum CLI version: **")?.1;
    Some(rest.split_once("**")?.0.to_string())
}

/// What is wrong with a section's minimum, measured against the adapter's
/// floor; `None` when they agree.
fn disagreement(section: &str, floor: &str) -> Option<String> {
    match declared_minimum(section) {
        None => Some("the section declares no minimum CLI version".to_string()),
        Some(declared) if declared != floor => Some(format!(
            "the guide says {declared}, the adapter's floor is {floor}"
        )),
        Some(_) => None,
    }
}

#[test]
fn the_guide_and_the_adapter_declare_the_same_minimum_version() {
    // Scenario: La guía y el adaptador declaran la misma versión mínima
    let codex = section(&guide(), "codex-cli");
    let floor = version::render(wire::SUPPORTED_FLOOR);
    assert_eq!(disagreement(&codex, &floor), None);
    assert!(
        codex.contains(&format!("{floor} or newer")),
        "the remedy for an older CLI names what to update to: {codex}"
    );
    assert!(
        codex.contains(&version::render(wire::SUPPORTED_CEILING)),
        "and where the range ends: {codex}"
    );
}

#[test]
fn a_guide_that_drifts_from_the_floor_fails_naming_both() {
    // Scenario: La guía y el adaptador declaran la misma versión mínima
    let drifted = "### codex-cli — Codex CLI - Minimum CLI version: **0.77.0**, old.";
    let wrong = disagreement(drifted, "0.156.1").expect("a divergence fails");
    assert!(
        wrong.contains("0.77.0") && wrong.contains("0.156.1"),
        "{wrong}"
    );
    assert!(disagreement("### codex-cli — Codex CLI - nothing said", "0.156.1").is_some());
}

#[test]
fn the_guide_states_the_gate_and_the_name_the_adapter_uses() {
    let codex = section(&guide(), "codex-cli");
    for fact in [
        gate::APPROVAL_POLICY,
        gate::SANDBOX_MODE,
        SPEC.name,
        "originator",
        "source",
        "web search",
        "MCP servers",
        "remote control",
    ] {
        assert!(
            codex.contains(fact),
            "the Codex section states `{fact}`: {codex}"
        );
    }
}
