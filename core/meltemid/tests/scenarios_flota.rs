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
        // The pilot layer declares its command, or declares that it travels in
        // Meltemi's own installers — one of the two, never neither, and never
        // both (adaptadores-propios-acp design D8).
        assert_ne!(
            agent.bundled,
            agent.adapter_install.is_some(),
            "{}: the pilot layer is bundled or installable, and says which",
            agent.id
        );
        // A remedy is a command the user can read and run, not a hint.
        for command in [&agent.cli_install, &agent.adapter_install]
            .into_iter()
            .flatten()
        {
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
    // What "no path to execute" means, named precisely rather than by a
    // substring: the bare word `shell` also occurs in `powershell`, which is
    // the NAME of a gesture the surface prints for the human to run — the
    // opposite of executing it (vincular-suscripciones). The guard therefore
    // forbids the actual doors: Tauri's shell plugin, its execute channel, and
    // any home-grown run/exec command.
    for door in [
        "plugin-shell",
        "plugin:shell",
        "shell|execute",
        "invoke(\"run",
        "invoke(\"exec",
        "Command.create",
    ] {
        assert!(
            !gui.contains(door),
            "no surface may have a path to execute a command (`{door}`)"
        );
    }
    // And the only thing done with a command string is putting it on the
    // clipboard: the copy call is present, execution is not.
    assert!(
        gui.contains("navigator.clipboard.writeText(command)"),
        "the surface copies commands; that is the whole interaction"
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

// ---- behavioural: the layers as DETECTED, not as hand-built ------------------

/// A fake executable for the current OS, in `dir`, named `name`.
fn fake_binary(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("fixture dir");
    #[cfg(windows)]
    {
        let path = dir.join(format!("{name}.cmd"));
        std::fs::write(&path, "@echo off\r\n").expect("write");
        path
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }
}

/// A two-layer catalog entry with DISTINCT install commands per layer, so a
/// swapped remedy fails the test instead of passing unnoticed.
fn two_layer_entry() -> meltemid::fleet::CatalogEntry {
    let mut entry = meltemid::fleet::CatalogEntry {
        id: "provider".into(),
        name: "Provider".into(),
        level: 2,
        ..Default::default()
    };
    entry.bin = Some("provider-acp".into());
    entry.adapter = Some("provider-acp".into());
    entry.cli_bin = Some("provider".into());
    entry.cli_install = Some("npm i -g provider-cli".into());
    entry.adapter_install = Some("cargo install provider-acp".into());
    entry
}

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mel-flota-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

// Scenario: CLI oficial presente sin adaptador
// Scenario: Adaptador presente sin CLI oficial
// Scenario: Ambas capas presentes
// Scenario: Ninguna capa presente
#[test]
fn the_two_layers_are_detected_independently_with_their_own_remedy() {
    use meltemi_proto::{FleetInstallState, FleetLayerKind};
    let entry = two_layer_entry();

    // (a) The official CLI is installed, the adapter is not.
    let cli_only = temp("cli-only");
    fake_binary(&cli_only, "provider");
    let path = std::ffi::OsString::from(cli_only.display().to_string());
    let launchable =
        |path: &std::ffi::OsString| meltemid::fleet::detect(&entry, path, None).is_some();
    let layers = meltemid::fleet::detect_layers(&entry, &path, None);
    assert_eq!(
        layers.len(),
        2,
        "both declared layers are reported: {layers:?}"
    );
    assert_eq!(
        layers[0].kind,
        FleetLayerKind::Cli,
        "the CLI layer comes first"
    );
    assert_eq!(layers[1].kind, FleetLayerKind::Adapter);
    assert!(layers[0].detected, "the installed CLI is detected");
    assert!(!layers[1].detected, "the absent adapter is not");
    let cli_path = layers[0].binary_path.as_deref().unwrap_or_default();
    assert!(
        std::path::Path::new(cli_path).is_absolute() && cli_path.contains("provider"),
        "the layer reports the absolute path of ITS binary: {cli_path:?}"
    );
    // Each layer carries its OWN install command: a swap fails here.
    assert_eq!(layers[0].install.as_deref(), Some("npm i -g provider-cli"));
    assert_eq!(
        layers[1].install.as_deref(),
        Some("cargo install provider-acp")
    );
    let (state, remedy, command) = meltemid::fleet::compose_state(&layers, launchable(&path));
    assert_eq!(state, FleetInstallState::AdapterMissing);
    assert!(
        remedy
            .unwrap_or_default()
            .to_lowercase()
            .contains("adapter"),
        "the remedy names the missing layer"
    );
    assert_eq!(command.as_deref(), Some("cargo install provider-acp"));

    // (b) The adapter is installed, the official CLI is not.
    let adapter_only = temp("adapter-only");
    fake_binary(&adapter_only, "provider-acp");
    let path = std::ffi::OsString::from(adapter_only.display().to_string());
    let layers = meltemid::fleet::detect_layers(&entry, &path, None);
    assert!(!layers[0].detected && layers[1].detected);
    let (state, remedy, command) = meltemid::fleet::compose_state(&layers, launchable(&path));
    assert_eq!(state, FleetInstallState::CliMissing);
    assert!(remedy.unwrap_or_default().to_lowercase().contains("cli"));
    assert_eq!(command.as_deref(), Some("npm i -g provider-cli"));

    // (c) Both installed: ready, with nothing to remedy.
    let both = temp("both");
    fake_binary(&both, "provider");
    fake_binary(&both, "provider-acp");
    let path = std::ffi::OsString::from(both.display().to_string());
    let layers = meltemid::fleet::detect_layers(&entry, &path, None);
    assert!(layers.iter().all(|layer| layer.detected));
    let (state, remedy, command) = meltemid::fleet::compose_state(&layers, launchable(&path));
    assert_eq!(state, FleetInstallState::Ready);
    assert!(remedy.is_none() && command.is_none());

    // (d) Neither installed: not detected, and the remedy starts at the CLI.
    let neither = std::ffi::OsString::new();
    let layers = meltemid::fleet::detect_layers(&entry, &neither, None);
    assert!(layers.iter().all(|layer| !layer.detected));
    let (state, remedy, _) = meltemid::fleet::compose_state(&layers, launchable(&neither));
    assert_eq!(state, FleetInstallState::NotDetected);
    assert!(
        remedy.is_some(),
        "even with nothing installed there is a way in"
    );

    for dir in [&cli_only, &adapter_only, &both] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

// Scenario: Rehúso de lanzamiento nombra la capa que falta
#[test]
fn the_refusal_carries_the_missing_layer_and_its_install_command() {
    // The CLI is installed, the adapter is not: launching must refuse with 2001
    // naming the adapter AND the command that installs it.
    let cli_only = temp("refusal");
    fake_binary(&cli_only, "provider");
    let path = std::ffi::OsString::from(cli_only.display().to_string());

    let registry = "version = \"refusal-fixture\"
         [[agents]]
id = \"provider\"
name = \"Provider\"
level = 2
         bin = \"provider-acp\"
adapter = \"provider-acp\"
cli-bin = \"provider\"
         cli-install = \"npm i -g provider-cli\"
         adapter-install = \"cargo install provider-acp\"
acp-args = []
";
    // Build the catalog the way the daemon does: a substituted registry file,
    // which is also how a user overrides the snapshot.
    let registry_path = cli_only.join("registry.toml");
    std::fs::write(&registry_path, registry).expect("write the fixture registry");
    let config = meltemid::config::Config {
        fleet_registry: Some(registry_path.clone()),
        ..Default::default()
    };
    let catalog = meltemid::fleet::build_catalog(&config);

    let error = meltemid::levels::resolve_id_launch(&catalog, "provider", &path, None)
        .expect_err("an incomplete agent refuses to launch");
    assert_eq!(error.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);
    let data = error.data.as_ref().expect("the refusal carries error data");
    let detail = data
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let remedy = data
        .get("remedy")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        detail.to_lowercase().contains("adapter"),
        "the refusal names the missing layer: {detail}"
    );
    assert!(
        remedy.contains("cargo install provider-acp"),
        "and the remedy carries the exact command for THAT layer: {remedy}"
    );
    assert!(
        !remedy.contains("npm i -g provider-cli"),
        "not the other layer's command: {remedy}"
    );
    let _ = std::fs::remove_dir_all(&cli_only);
}

// ---- the bundled layer: found beside the daemon, and never before the PATH --

/// A registry fixture with ONE entry whose pilot layer is declared bundled.
/// The id is deliberately anonymous: the mechanism must work for any entry that
/// declares the layer, and a test named after a product would let a per-id
/// special case pass unnoticed (adaptadores-propios-acp design D8).
fn bundled_registry(dir: &Path, id: &str) -> meltemid::fleet::Catalog {
    let text = format!(
        "version = \"bundled-fixture\"
[[agents]]
id = \"{id}\"
name = \"{id}\"
level = 2
bin = \"meltemi-{id}-acp\"
adapter = \"meltemi-{id}-acp\"
bundled = true
cli-bin = \"{id}\"
cli-install = \"npm i -g {id}\"
acp-args = []
"
    );
    let path = dir.join("registry.toml");
    std::fs::write(&path, text).expect("write the fixture registry");
    let config = meltemid::config::Config {
        fleet_registry: Some(path),
        ..Default::default()
    };
    meltemid::fleet::build_catalog(&config)
}

// Scenario: Capa empaquetada detectada junto al daemon
// Scenario: Mecanismo genérico sin casos por id
#[test]
fn a_bundled_layer_is_found_beside_the_daemon_and_says_so() {
    use meltemi_proto::{FleetInstallState, FleetLayerKind, FleetLayerSource};

    // The official CLI is on the PATH; the pilot layer is nowhere on it, and
    // sits beside the daemon — the state of a machine where the user installed
    // Meltemi and the provider CLI, and nothing else.
    let beside = temp("bundled-beside");
    let on_path = temp("bundled-path");
    fake_binary(&on_path, "vendor");
    fake_binary(&beside, "meltemi-vendor-acp");
    let path_var = std::ffi::OsString::from(on_path.display().to_string());

    // Two ids, one mechanism: the same fixture under a different id must behave
    // identically, which is what "generic" means here.
    for id in ["vendor", "another"] {
        let fixtures = temp(&format!("bundled-reg-{id}"));
        fake_binary(&on_path, id);
        fake_binary(&beside, &format!("meltemi-{id}-acp"));
        let catalog = bundled_registry(&fixtures, id);
        let entry = catalog
            .entries
            .iter()
            .find(|e| e.id == id)
            .expect("the fixture entry");

        let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&beside));
        let pilot = layers
            .iter()
            .find(|layer| layer.kind == FleetLayerKind::Adapter)
            .expect("the pilot layer");
        assert!(pilot.detected, "{id}: the bundled layer is found");
        assert!(
            pilot.bundled,
            "{id}: and the catalog says it travels with Meltemi"
        );
        assert_eq!(
            pilot.source,
            Some(FleetLayerSource::Bundled),
            "{id}: the provenance of the find is reported"
        );
        let found = pilot.binary_path.as_deref().unwrap_or_default();
        assert!(
            Path::new(found).is_absolute() && Path::new(found).starts_with(&beside),
            "{id}: with the absolute path it was found at: {found:?}"
        );
        assert!(
            pilot.install.is_none(),
            "{id}: a bundled layer offers no third-party install command"
        );

        // Composed: nothing else to install, so the entry is ready to pilot.
        let launchable = meltemid::fleet::detect(entry, &path_var, Some(&beside)).is_some();
        let (state, _, _) = meltemid::fleet::compose_state(&layers, launchable);
        assert_eq!(
            state,
            FleetInstallState::Ready,
            "{id}: Meltemi plus the official CLI is a complete installation"
        );

        // Without the bundled probe the very same entry is incomplete: the
        // probe is what finds it, not a coincidence of the PATH.
        let blind = meltemid::fleet::detect_layers(entry, &path_var, None);
        assert!(
            !blind[1].detected,
            "{id}: nothing but the sibling directory holds this binary"
        );
        let _ = std::fs::remove_dir_all(&fixtures);
    }
    for dir in [&beside, &on_path] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

// Scenario: El PATH conserva la precedencia sobre el empaquetado
#[test]
fn the_path_outranks_the_bundled_copy_and_the_launch_says_which_ran() {
    use meltemi_proto::FleetLayerSource;
    let beside = temp("precedence-beside");
    let on_path = temp("precedence-path");
    // The SAME binary name in both places: what the user installed must win.
    let installed = fake_binary(&on_path, "meltemi-vendor-acp");
    fake_binary(&beside, "meltemi-vendor-acp");
    fake_binary(&on_path, "vendor");
    let path_var = std::ffi::OsString::from(on_path.display().to_string());

    let fixtures = temp("precedence-reg");
    let catalog = bundled_registry(&fixtures, "vendor");
    let entry = catalog.entries.iter().find(|e| e.id == "vendor").unwrap();

    let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&beside));
    assert_eq!(
        layers[1].source,
        Some(FleetLayerSource::Path),
        "the PATH copy is the one reported"
    );
    assert_eq!(
        layers[1].binary_path.as_deref(),
        Some(installed.display().to_string().as_str()),
        "with its own absolute path, not the bundled one"
    );

    // And it is the binary a launch would execute — the argv the session log
    // then records as the effective binary.
    let launch = meltemid::levels::resolve_id_launch(&catalog, "vendor", &path_var, Some(&beside))
        .expect("a detected pilot layer launches");
    let argv = match launch {
        meltemid::levels::Launch::Acp { argv, .. } => argv,
        other => panic!("expected an ACP launch, got {other:?}"),
    };
    assert_eq!(
        argv.first().map(String::as_str),
        Some(installed.display().to_string().as_str())
    );
    // The launch path logs exactly that program as the effective binary.
    let server = read("core/meltemid/src/server.rs");
    assert!(
        server.contains("binary: agent_command.first().cloned().unwrap_or_default()"),
        "the session log records the program the resolution picked"
    );

    for dir in [&beside, &on_path, &fixtures] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

// ---- the shipped snapshot after the flip -----------------------------------

/// The catalog built the way the daemon builds it, from the SHIPPED snapshot.
fn shipped_catalog() -> meltemid::fleet::Catalog {
    meltemid::fleet::build_catalog(&meltemid::config::Config::default())
}

// Scenario: Entrada lista con solo Meltemi y el CLI oficial
// Scenario: La capa propia no ofrece instalación de terceros
#[test]
fn an_entry_with_its_own_adapter_is_ready_with_meltemi_and_the_official_cli() {
    use meltemi_proto::{FleetInstallState, FleetLayerKind, FleetLayerSource};
    let beside = temp("shipped-beside");
    let on_path = temp("shipped-path");
    let path_var = std::ffi::OsString::from(on_path.display().to_string());

    let catalog = shipped_catalog();
    let bundled: Vec<_> = catalog.entries.iter().filter(|e| e.bundled).collect();
    assert!(
        !bundled.is_empty(),
        "the snapshot pilots at least one entry through an adapter of our own"
    );

    for entry in bundled {
        // The user installed the provider's official CLI, and Meltemi. That is
        // the whole installation.
        fake_binary(&on_path, entry.cli_bin.as_deref().expect("a CLI layer"));
        fake_binary(&beside, entry.bin.as_deref().expect("a pilot layer"));

        let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&beside));
        let pilot = layers
            .iter()
            .find(|layer| layer.kind == FleetLayerKind::Adapter)
            .unwrap_or_else(|| panic!("{}: a pilot layer", entry.id));
        assert!(
            pilot.bundled,
            "{}: the pilot layer is declared bundled",
            entry.id
        );
        assert_eq!(pilot.source, Some(FleetLayerSource::Bundled));
        assert!(
            pilot.install.is_none(),
            "{}: and offers no third-party install command",
            entry.id
        );
        assert!(
            entry.adapter_install.is_none(),
            "{}: the registry declares none either",
            entry.id
        );

        let launchable = meltemid::fleet::detect(entry, &path_var, Some(&beside)).is_some();
        let (state, remedy, command) = meltemid::fleet::compose_state(&layers, launchable);
        assert_eq!(
            state,
            FleetInstallState::Ready,
            "{}: nothing else needs installing",
            entry.id
        );
        assert!(
            remedy.is_none() && command.is_none(),
            "{}: a ready entry has nothing to remedy",
            entry.id
        );
    }
    for dir in [&beside, &on_path] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

// Scenario: La capa propia no ofrece instalación de terceros
// Scenario: Capa empaquetada ausente remite a la instalación de Meltemi
#[test]
fn a_bundled_entry_that_is_its_own_only_layer_is_treated_like_one() {
    use meltemi_proto::{FleetInstallState, FleetLayerKind, FleetLayerSource};
    // The shape the registry does not have yet and will: a pilot binary of our
    // own with no provider CLI beneath it, so its single layer IS the bundled
    // one. Everything the two-layer entries are asserted to do, this shape must
    // do too — the mechanism is generic or it is a special case waiting to be
    // written for the next entry.
    let dir = temp("bundled-single");
    let path = dir.join("registry.toml");
    std::fs::write(
        &path,
        "version = \"bundled-single-fixture\"
[[agents]]
id = \"single\"
name = \"Single\"
level = 1
bin = \"meltemi-single-engine\"
bundled = true
acp-args = []
",
    )
    .expect("write the fixture registry");
    let catalog = meltemid::fleet::build_catalog(&meltemid::config::Config {
        fleet_registry: Some(path),
        ..Default::default()
    });
    let entry = catalog
        .entries
        .iter()
        .find(|e| e.id == "single")
        .expect("a single-layer bundled entry parses");

    // Nothing on the PATH; the binary sits beside the daemon, as an installer
    // would have put it.
    let path_var = std::ffi::OsString::from(temp("bundled-single-path").display().to_string());
    let beside = temp("bundled-single-beside");
    fake_binary(&beside, "meltemi-single-engine");
    let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&beside));
    assert_eq!(layers.len(), 1, "there is no provider CLI under this one");
    let only = &layers[0];
    assert_eq!(only.kind, FleetLayerKind::Cli);
    assert!(only.bundled, "and the one layer it has is the bundled one");
    assert_eq!(
        only.source,
        Some(FleetLayerSource::Bundled),
        "found by the same generic probe"
    );
    assert!(
        only.install.is_none(),
        "carrying no install command a surface could offer: {:?}",
        only.install
    );

    // And when Meltemi's own installation is the broken part, the remedy says
    // so — with no third-party command in it or beside it.
    let nowhere = temp("bundled-single-nowhere");
    let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&nowhere));
    let (state, remedy, command) = meltemid::fleet::compose_state(&layers, false);
    assert_eq!(state, FleetInstallState::NotDetected);
    let remedy = remedy.expect("an incomplete state always carries its remedy");
    let lower = remedy.to_lowercase();
    assert!(
        lower.contains("meltemi") && lower.contains("reinstall"),
        "the remedy is Meltemi's own installation: {remedy}"
    );
    assert!(command.is_none(), "and no command is offered: {command:?}");

    for d in [&dir, &beside, &nowhere] {
        let _ = std::fs::remove_dir_all(d);
    }
}

// Scenario: Adaptador de terceros por configuración sigue pilotable
#[test]
fn a_third_party_adapter_declared_by_the_user_is_piloted_like_any_other() {
    use meltemid::config::CustomAgent;
    let dir = temp("third-party");
    let binary = fake_binary(&dir, "somebody-elses-acp");
    let path_var = std::ffi::OsString::from(dir.display().to_string());

    let config = meltemid::config::Config {
        fleet_custom: vec![CustomAgent {
            id: "third-party".into(),
            name: "A third-party adapter".into(),
            command: vec!["somebody-elses-acp".into(), "--acp".into()],
        }],
        ..Default::default()
    };
    let catalog = meltemid::fleet::build_catalog(&config);
    let entry = catalog
        .entries
        .iter()
        .find(|e| e.id == "third-party")
        .expect("the user's entry joins the catalog");
    assert!(
        !entry.bundled,
        "the user's own declaration is not a bundled layer"
    );

    let launch = meltemid::levels::resolve_id_launch(&catalog, "third-party", &path_var, None)
        .expect("it is piloted like any other entry");
    match launch {
        meltemid::levels::Launch::Acp { argv, level } => {
            assert_eq!(level, 1, "a declared ACP command is what it says it is");
            assert_eq!(
                argv.first().map(String::as_str),
                Some(binary.display().to_string().as_str())
            );
            assert_eq!(argv.get(1).map(String::as_str), Some("--acp"));
        }
        other => panic!("expected an ACP launch, got {other:?}"),
    }

    // And no code path treats it differently for not being ours: the registry
    // simply stopped recommending one.
    let fleet = read("core/meltemid/src/fleet.rs");
    let levels = read("core/meltemid/src/levels.rs");
    for source in [&fleet, &levels] {
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production half");
        for name in ["claude", "codex", "meltemi-claude-acp", "meltemi-codex-acp"] {
            assert!(
                !production.contains(name),
                "detection must not name a product: `{name}`"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// Scenario: Capa empaquetada ausente remite a la instalación de Meltemi
// Scenario: Rehúso de lanzamiento nombra la capa que falta
#[test]
fn a_missing_bundled_layer_sends_the_user_to_meltemis_own_installer() {
    use meltemi_proto::FleetInstallState;
    // The provider's CLI is installed; Meltemi's own adapter is nowhere — the
    // shape of a broken or partial Meltemi installation.
    let dir = temp("bundled-missing");
    let catalog = bundled_registry(&dir, "vendor");
    fake_binary(&dir, "vendor");
    let path_var = std::ffi::OsString::from(dir.display().to_string());
    let entry = catalog.entries.iter().find(|e| e.id == "vendor").unwrap();

    // Nothing beside the daemon either: the probe runs and finds nothing.
    let empty = temp("bundled-missing-beside");
    let layers = meltemid::fleet::detect_layers(entry, &path_var, Some(&empty));
    let (state, remedy, command) = meltemid::fleet::compose_state(&layers, false);
    assert_eq!(state, FleetInstallState::AdapterMissing);
    let remedy = remedy.expect("an incomplete state always carries its remedy");
    let lower = remedy.to_lowercase();
    assert!(
        lower.contains("meltemi") && lower.contains("reinstall"),
        "the remedy sends the user to Meltemi's own installation: {remedy}"
    );
    assert!(
        remedy.contains("meltemi-vendor-acp"),
        "and names the layer that is missing: {remedy}"
    );
    assert!(
        command.is_none(),
        "a bundled layer offers no install command at all, third-party or not: {command:?}"
    );
    for forbidden in ["npm i", "npx", "cargo install", "pip install"] {
        assert!(
            !remedy.contains(forbidden),
            "no third-party install route may appear here: {remedy}"
        );
    }

    // The launch refusal says the same thing, in its own remedy field.
    let error = meltemid::levels::resolve_id_launch(&catalog, "vendor", &path_var, Some(&empty))
        .expect_err("a missing pilot layer refuses to launch");
    assert_eq!(error.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);
    let data = error.data.as_ref().expect("the refusal carries error data");
    let detail = data
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let hint = data
        .get("remedy")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        detail.contains("meltemi-vendor-acp"),
        "the refusal names the absent layer: {detail}"
    );
    assert!(
        hint.to_lowercase().contains("reinstall") && hint.to_lowercase().contains("meltemi"),
        "and its remedy is the one for a layer of that type: {hint}"
    );

    for d in [&dir, &empty] {
        let _ = std::fs::remove_dir_all(d);
    }
}

// Scenario: Remedio con el comando exacto por capa
#[test]
fn every_surface_prints_the_remedy_it_is_given() {
    // The remedy is composed once, in the daemon, and each surface renders that
    // same sentence: none of them rewrites it, and none of them owns a second
    // copy of the rule about what a bundled layer's remedy says.
    let cli = read("tui/src/run.rs");
    assert!(
        cli.contains("remedy: {remedy}"),
        "the scriptable surface prints the remedy in its human mode"
    );
    let tui = read("tui/src/shell/render.rs");
    assert!(
        tui.contains("{remedy}"),
        "the terminal view prints it beside the entry"
    );
    let gui = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        gui.contains("{selected.remedy}"),
        "the desktop detail prints it too"
    );
    // And none of them composes one of its own.
    for (name, source) in [("tui/run.rs", &cli), ("tui/render.rs", &tui)] {
        assert!(
            !source.contains("reinstall or repair"),
            "{name} must render the daemon's remedy, not write its own"
        );
    }
}

/// Core parity (constitution §4) for the provenance of a bundled find: a field
/// the daemon reports must reach every surface, not just the one whose author
/// happened to be looking. No spec scenario of its own — it is the §4 rule
/// applied to the field `fleet/list` gained here.
#[test]
fn the_three_surfaces_show_where_a_bundled_binary_came_from() {
    let cli = read("tui/src/run.rs");
    assert!(
        cli.contains("(bundled with Meltemi)") && cli.contains("FleetLayerSource::Bundled"),
        "the scriptable surface says a pilot binary came with Meltemi"
    );
    let tui = read("tui/src/shell/render.rs");
    assert!(
        tui.contains("[empaquetado con Meltemi]") && tui.contains("[bundled with Meltemi]"),
        "the Fleet view marks it, in both languages"
    );
    assert!(
        tui.contains("FleetLayerSource::Bundled"),
        "reading the provenance the daemon reported, not guessing from a name"
    );
    let gui = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        gui.contains("layer.source === \"bundled\""),
        "the desktop layer detail marks it too"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    let keys = catalog
        .lines()
        .filter(|line| line.contains("\"fleet.layer.bundled\""))
        .count();
    assert_eq!(keys, 2, "the label exists in both message catalogs");
    // `--json` needs no work: it is the daemon's response verbatim, which is
    // exactly why the field had to be part of the contract.
    let contract = read("proto/schemas/v1/fleet.schema.json");
    assert!(
        contract.contains("fleetLayerSource"),
        "the machine-readable surface carries it because the schema does"
    );
    // And the living matrix says so, so a future field cannot land in one
    // surface and be discovered missing from the others by a user.
    let matrix = read("docs/paridad-nucleo.md");
    assert!(
        matrix.contains("provenance"),
        "the parity matrix records that the per-layer detail is rendered alike"
    );
}
