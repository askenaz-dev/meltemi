// SPDX-License-Identifier: Apache-2.0

//! Verification of the `flota-deteccion-guia` scenarios the fleet's own unit
//! tests do not already cover: the remedy that travels with a missing layer, the
//! legal status shown without makeup, the passivity of detection, and the guide's
//! troubleshooting and entry point.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

fn registry() -> meltemid::fleet::Registry {
    let raw = read("core/meltemid/data/fleet-registry.toml");
    meltemid::fleet::parse_registry(&raw).expect("the shipped registry parses")
}

// Scenario: Remedio con el comando exacto por capa
#[test]
fn a_missing_layer_travels_with_the_exact_command_that_installs_it() {
    let registry = registry();
    let two_layer: Vec<_> = registry
        .agents
        .iter()
        .filter(|agent| agent.adapter.is_some())
        .collect();
    assert!(
        !two_layer.is_empty(),
        "the snapshot declares adapter-piloted agents"
    );
    for agent in two_layer {
        assert!(
            agent.cli_install.is_some(),
            "{}: the official CLI layer declares how to install it",
            agent.id
        );
        assert!(
            agent.adapter_install.is_some(),
            "{}: the adapter layer declares how to install it",
            agent.id
        );
        // A remedy is a command the user can read and run, not a hint.
        for command in [
            agent.cli_install.as_deref().unwrap_or_default(),
            agent.adapter_install.as_deref().unwrap_or_default(),
        ] {
            assert!(
                command.split_whitespace().count() >= 2,
                "{}: `{command}` does not look like a command",
                agent.id
            );
        }
    }
}

// Scenario: Meltemi no ejecuta el remedio
#[test]
fn meltemi_never_runs_the_remedy_itself() {
    // Detection and composition are pure: the whole fleet module spawns nothing.
    let fleet = read("core/meltemid/src/fleet.rs");
    let production = fleet.split("#[cfg(test)]").next().expect("production half");
    for forbidden in [
        "Command::new",
        "std::process::Command",
        "spawn(",
        "output()",
    ] {
        assert!(
            !production.contains(forbidden),
            "the fleet module must not execute anything (`{forbidden}`)"
        );
    }
    // The remedy travels as data to the surfaces, which print it.
    let proto = read("proto/meltemi-proto/src/lib.rs");
    assert!(
        proto.contains("pub remedy_command: Option<String>"),
        "the remedy command is a field of the contract, not an action"
    );
    let gui = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        gui.contains("remedyCommand") && gui.contains("navigator.clipboard"),
        "the desktop surface offers to COPY it, never to run it"
    );
    assert!(
        !gui.contains("invoke(\"run") && !gui.contains("shell"),
        "no surface has a path to execute the remedy"
    );
}

// Scenario: Rehúso de lanzamiento nombra la capa que falta
#[test]
fn a_refusal_to_launch_names_the_missing_layer() {
    let levels = read("core/meltemid/src/levels.rs");
    let level_two = levels
        .split("        2 => {")
        .nth(1)
        .expect("the level-2 launch path")
        .split("        3 =>")
        .next()
        .expect("the branch ends");
    assert!(
        level_two.contains("not_detected"),
        "an undetected layer refuses with 2001: {level_two}"
    );
    assert!(
        level_two.contains("adapter") || level_two.contains("cli"),
        "and the refusal names which layer is missing"
    );
    // The refusal never degrades to another provider.
    assert!(
        levels.contains("MUST NOT degrade to the configured agent"),
        "the no-silent-swap rule is documented where it is enforced"
    );
}

// Scenario: Nota legal declarada mostrada tal cual
// Scenario: Camino seguro señalado junto a la zona gris
#[test]
fn the_legal_note_is_shown_verbatim_next_to_the_safe_path() {
    let registry = registry();
    let with_note: Vec<_> = registry
        .agents
        .iter()
        .filter(|agent| agent.legal_note.is_some())
        .collect();
    assert!(
        !with_note.is_empty(),
        "the snapshot declares a legal note where the path is not sanctioned"
    );
    for agent in &with_note {
        let status = agent.legal_status.as_deref().unwrap_or("");
        assert!(
            matches!(status, "sanctioned" | "tolerated" | "grey"),
            "{}: the status is one of the declared three, got `{status}`",
            agent.id
        );
    }
    // A grey path must coexist with an uncaveated one, so the user is never left
    // with only the uncomfortable option — and the grey entry must SAY where the
    // safe path is, rather than leaving the reader to guess.
    let uncaveated = registry
        .agents
        .iter()
        .filter(|agent| agent.legal_note.is_none())
        .count();
    assert!(
        uncaveated > 0,
        "the catalog offers integration paths with no legal caveat at all"
    );
    let guide = read("docs/agentes.md");
    for agent in &with_note {
        if agent.legal_status.as_deref() != Some("grey") {
            continue;
        }
        let section = guide
            .split(&format!("### {} ", agent.id))
            .nth(1)
            .unwrap_or_else(|| panic!("the guide has a section for {}", agent.id))
            .split("\n### ")
            .next()
            .expect("section body");
        // Prose wraps, so compare on collapsed whitespace.
        let flat = section.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains("safe path"),
            "{}: the grey entry must point at the safe path: {flat}",
            agent.id
        );
    }
    // The surfaces render the note as it was declared, without editing it.
    let gui = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        gui.contains("{agent.legalNote}") || gui.contains("{selected.legalNote}"),
        "the desktop surface prints the declared note verbatim"
    );
    let tui = read("tui/src/shell/render.rs");
    assert!(
        tui.contains("legal_note"),
        "the terminal surface prints it too"
    );
}

// Scenario: Detección sin efectos laterales
#[test]
fn detection_has_no_side_effects_on_the_users_machine() {
    let fleet = read("core/meltemid/src/fleet.rs");
    let production = fleet.split("#[cfg(test)]").next().expect("production half");
    // It only asks the filesystem whether a path exists and is executable.
    for forbidden in [
        "fs::write",
        "fs::create_dir",
        "fs::remove",
        "OpenOptions",
        "set_permissions",
        "env::set_var",
    ] {
        assert!(
            !production.contains(forbidden),
            "detection must not write anything (`{forbidden}`)"
        );
    }
    assert!(
        production.contains("fn probe(") && production.contains("metadata"),
        "it probes with a metadata read, nothing more"
    );
}

// Scenario: La detección se refresca por consulta
#[test]
fn every_query_redetects_instead_of_serving_a_cache() {
    let fleet = read("core/meltemid/src/fleet.rs");
    let production = fleet.split("#[cfg(test)]").next().expect("production half");
    for forbidden in [
        "static CACHE",
        "OnceLock<Vec<FleetAgent>",
        "lazy_static",
        "Mutex<Vec<Fleet",
    ] {
        assert!(
            !production.contains(forbidden),
            "the catalog must not be cached across queries (`{forbidden}`)"
        );
    }
    let server = read("core/meltemid/src/server.rs");
    assert!(
        server.contains("fleet::handle_fleet_list") || server.contains("FLEET_LIST =>"),
        "the handler runs per request"
    );
}

// Scenario: Campos aditivos sin romper el contrato vigente
#[test]
fn the_new_detection_fields_are_additive() {
    let proto = read("proto/meltemi-proto/src/lib.rs");
    let agent = proto
        .split("pub struct FleetAgent {")
        .nth(1)
        .expect("FleetAgent")
        .split("\n}")
        .next()
        .expect("body");
    for field in [
        "layers",
        "install_state",
        "remedy",
        "remedy_command",
        "legal_status",
        "legal_note",
    ] {
        let declaration = agent
            .split(&format!("pub {field}:"))
            .nth(1)
            .unwrap_or_else(|| panic!("{field} is declared"));
        let head = &declaration[..declaration.len().min(120)];
        assert!(
            head.contains("Option<") || head.contains("Vec<"),
            "{field} must be optional or empty-able so an old client still parses: {head}"
        );
    }
    // The schema marks them optional too: they are absent from the required
    // list of the agent object.
    let schema = read("proto/schemas/v1/fleet.schema.json");
    let required = schema
        .split("\"fleetAgent\": {")
        .nth(1)
        .expect("the agent object")
        .split("\"required\": [")
        .nth(1)
        .expect("its required list")
        .split(']')
        .next()
        .expect("the list closes");
    for field in [
        "layers",
        "installState",
        "remedy",
        "legalStatus",
        "legalNote",
    ] {
        assert!(
            !required.contains(field),
            "{field} must not be required by the schema: {required}"
        );
    }
}

// Scenario: Solución de problemas de detección por sistema operativo
#[test]
fn the_guide_troubleshoots_detection_per_operating_system() {
    let guide = read("docs/agentes.md");
    for heading in ["### Windows", "### macOS and Linux"] {
        assert!(
            guide.contains(heading),
            "the guide has a per-OS section: {heading}"
        );
    }
    let windows = guide
        .split("### Windows")
        .nth(1)
        .expect("the Windows section")
        .split("###")
        .next()
        .expect("section body");
    // The Windows section must name the extension rule, which is the actual
    // cause of most "not detected" reports there.
    assert!(
        windows.contains(".cmd") && windows.contains(".ps1"),
        "it explains which extensions launch and which are evidence only: {windows}"
    );
    let unix = guide
        .split("### macOS and Linux")
        .nth(1)
        .expect("the unix section")
        .split("## ")
        .next()
        .expect("section body");
    assert!(
        unix.contains("PATH") && unix.contains("execute bit"),
        "and the unix section names PATH and the execute bit: {unix}"
    );
    assert!(
        guide.contains("## When something still does not work"),
        "the guide ends with a what-to-do-next section"
    );
}

// Scenario: Enlace desde el README sin nombrar productos
#[test]
fn the_readme_links_the_guide_without_selling_third_party_products() {
    let readme = read("README.md");
    assert!(
        readme.contains("docs/agentes.md"),
        "the README points at the agents guide"
    );
    // The README may not use third-party product names as an argument; the guide
    // names them because that is interoperability data.
    for product in [
        "Claude",
        "Codex",
        "Copilot",
        "Gemini",
        "Cursor",
        "OpenAI",
        "Anthropic",
    ] {
        assert!(
            !readme.contains(product),
            "the README must not name the third-party product `{product}`"
        );
    }
    let guide = read("docs/agentes.md");
    assert!(
        guide.contains("### claude-code") || guide.contains("### codex-cli"),
        "the guide does name them, as the interoperability data it is"
    );
}
