// SPDX-License-Identifier: Apache-2.0

//! The agents guide is prose with judgement, but its FACTS come from the
//! registry (flota-deteccion-guia design D7). This test verifies the bijection
//! in both directions and the accuracy of every fact it states, so the guide
//! cannot age in silence — the failure mode that motivated the change.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("core/meltemid lives two levels under the repo root")
        .to_path_buf()
}

fn guide() -> String {
    std::fs::read_to_string(repo_root().join("docs/agentes.md")).expect("read docs/agentes.md")
}

fn registry() -> meltemid::fleet::Registry {
    let text = std::fs::read_to_string(repo_root().join("core/meltemid/data/fleet-registry.toml"))
        .expect("read the registry snapshot");
    meltemid::fleet::parse_registry(&text).expect("the snapshot parses")
}

/// The inline-code spans of a guide section (`` `like this` ``), which is how the
/// guide names binaries. Comparing these as tokens is what makes a superset name
/// like `cursor-agent` fail against a registry that probes `agent`.
fn inline_code_spans(section: &str) -> std::collections::BTreeSet<String> {
    let mut spans = std::collections::BTreeSet::new();
    let mut rest = section;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        let span = rest[..close].trim();
        if !span.is_empty() {
            spans.insert(span.to_string());
        }
        rest = &rest[close + 1..];
    }
    spans
}

// Scenario: Guía y registro en biyección
#[test]
// Scenario: Cada entrada del registro tiene su sección con nivel y binarios
// Scenario: Entrada nueva o renombrada falla la verificación
fn every_registry_entry_has_a_section_and_every_section_an_entry() {
    let guide = guide();
    let registry = registry();

    // Registry → guide.
    for agent in &registry.agents {
        let heading = format!("### {} — {}", agent.id, agent.name);
        assert!(
            guide.contains(&heading),
            "docs/agentes.md is missing the section `{heading}`"
        );
    }

    // Guide → registry: every `### id — name` heading names a real entry.
    for line in guide.lines() {
        let Some(rest) = line.strip_prefix("### ") else {
            continue;
        };
        let Some((id, name)) = rest.split_once(" — ") else {
            continue;
        };
        let entry = registry
            .agents
            .iter()
            .find(|agent| agent.id == id.trim())
            .unwrap_or_else(|| panic!("the guide documents `{id}`, which the registry lacks"));
        assert_eq!(
            entry.name,
            name.trim(),
            "the guide names `{id}` differently from the registry"
        );
    }
}

// Scenario: La guía declara el nivel y los binarios que el registro declara
#[test]
fn the_guide_states_the_level_and_binaries_the_registry_declares() {
    let guide = guide();
    let registry = registry();

    for agent in &registry.agents {
        // The section body runs from this heading to the next one.
        let start = guide
            .find(&format!("### {} — ", agent.id))
            .expect("section present (checked by the bijection test)");
        let body = &guide[start..];
        let body = body[4..]
            .find("\n### ")
            .map_or(body, |offset| &body[..offset + 4]);

        assert!(
            body.contains(&format!("Level {}", agent.level)),
            "the section of `{}` does not state level {}",
            agent.id,
            agent.level
        );
        // Compare TOKENS, not substrings: `cursor-agent` must not satisfy a
        // registry that probes `agent`, or the guide can tell users to look for
        // a binary Meltemi never asks about.
        let named = inline_code_spans(body);
        if let Some(bin) = &agent.bin {
            assert!(
                named.contains(bin),
                "the section of `{}` names {named:?}, not its binary `{bin}`",
                agent.id
            );
        }
        // Two-layer entries must name both layers and both install commands.
        if let Some(cli_bin) = &agent.cli_bin {
            assert!(
                named.contains(cli_bin),
                "the section of `{}` names {named:?}, not its official CLI `{cli_bin}`",
                agent.id
            );
            for command in [&agent.cli_install, &agent.adapter_install]
                .into_iter()
                .flatten()
            {
                assert!(
                    body.contains(command),
                    "the section of `{}` does not state the command `{command}`",
                    agent.id
                );
            }
            if let Some(note) = &agent.legal_note {
                // The note's own wording may be summarized, but its status must
                // be stated: a known restriction is never hidden (§2).
                let status = agent.legal_status.as_deref().unwrap_or("tolerated");
                assert!(
                    body.to_lowercase().contains(status),
                    "the section of `{}` hides its legal status `{status}` ({note})",
                    agent.id
                );
            }
        }
    }
}

// Scenario: Los ejemplos de perfiles de la guía son configuración válida
#[test]
// Scenario: Ejemplos de configuración de perfiles válidos
fn the_profile_examples_parse_as_configuration() {
    let guide = guide();
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in guide.lines() {
        if line.trim_start().starts_with("```toml") {
            current = Some(String::new());
            continue;
        }
        if line.trim_start().starts_with("```") {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    assert!(
        blocks.len() >= 2,
        "the guide shows the profile and custom-agent examples"
    );
    for block in blocks {
        let value: toml::Value = toml::from_str(&block)
            .unwrap_or_else(|e| panic!("a guide example is not valid TOML: {e}\n{block}"));
        // Every example configures the fleet, and none carries a secret.
        assert!(
            value.get("fleet").is_some(),
            "a guide example does not configure the fleet: {block}"
        );
        assert!(
            !block.to_lowercase().contains("api_key") && !block.to_lowercase().contains("token ="),
            "a guide example shows secret material: {block}"
        );
    }
}
