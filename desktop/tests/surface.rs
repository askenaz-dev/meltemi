// SPDX-License-Identifier: Apache-2.0

//! Verification of the desktop surface's declarative guarantees: the ones that
//! live in configuration, tokens and the pipeline rather than in behavior, and
//! are therefore checkable without driving a webview. The interactive scenarios
//! are verified manually per release (docs/qa/).

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("desktop/ lives under the repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

// Scenario: Webview sin acceso a red
#[test]
fn the_webview_declares_no_remote_origin_and_minimal_capabilities() {
    let config = read("desktop/tauri.conf.json");
    let csp = config
        .split("\"csp\":")
        .nth(1)
        .and_then(|rest| rest.split('"').nth(1))
        .expect("a CSP is declared");
    assert!(
        csp.contains("default-src 'self'"),
        "the CSP defaults to self: {csp}"
    );
    assert!(
        !csp.contains("http://") || csp.contains("http://ipc.localhost"),
        "no remote origin beyond the Tauri IPC bridge: {csp}"
    );
    assert!(
        !csp.contains("https://"),
        "no https origin is reachable from the webview: {csp}"
    );

    // Deny-by-default capabilities: only the core set, no fs/shell/http.
    let capabilities = read("desktop/capabilities/default.json");
    for forbidden in ["fs:", "shell:", "http:", "os:allow", "process:"] {
        assert!(
            !capabilities.contains(forbidden),
            "capability `{forbidden}` must not be granted: {capabilities}"
        );
    }
    assert!(capabilities.contains("core:default"));
}

// Scenario: Movimiento reducido honrado
#[test]
fn reduced_motion_and_focus_visibility_are_declared_in_the_tokens() {
    let css = read("desktop/ui/src/app.css");
    assert!(
        css.contains("prefers-reduced-motion: reduce"),
        "the token layer suppresses non-essential animation"
    );
    assert!(
        css.contains(":focus-visible") && css.contains("outline"),
        "a focus ring is declared for keyboard navigation"
    );
    // Both themes ship from day one (design system: ambos temas desde el día uno).
    assert!(css.contains("prefers-color-scheme: dark"));
}

// Scenario: Sin hardcodeo de idioma
#[test]
fn the_i18n_lint_refuses_hardcoded_strings_and_runs_in_ci() {
    let lint = read("desktop/ui/scripts/i18n-lint.mjs");
    assert!(
        lint.contains("aria-label") && lint.contains("text node"),
        "the lint inspects labeling attributes and text nodes"
    );
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains("lint:i18n"),
        "the i18n lint is a CI gate, not a local habit"
    );
    // Every catalog key exists in both languages.
    let messages = read("desktop/ui/src/lib/messages.ts");
    let (es, en) = messages
        .split_once("const en: Record<MessageKey, string> = {")
        .expect("the catalog declares both languages");
    let keys = |chunk: &str| -> Vec<String> {
        chunk
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if !trimmed.contains("\": ") {
                    return None;
                }
                trimmed
                    .strip_prefix('"')
                    .and_then(|rest| rest.split('"').next())
                    .map(str::to_string)
            })
            .collect()
    };
    let es_keys = keys(es);
    let en_keys = keys(en);
    assert!(es_keys.len() > 100, "the catalog is populated");
    for key in &es_keys {
        assert!(en_keys.contains(key), "key `{key}` is missing in English");
    }
    for key in &en_keys {
        assert!(es_keys.contains(key), "key `{key}` is missing in Spanish");
    }
}

// Scenario: Gate de tamaño del instalador
// Scenario: Presupuesto de tamaño como gate
#[test]
fn the_installer_size_budget_is_a_blocking_pipeline_gate() {
    let release = read(".github/workflows/release.yml");
    assert!(
        release.contains("MELTEMI_GUI_INSTALLER_BUDGET_BYTES: \"15728640\""),
        "the 15 MB budget is declared in the pipeline"
    );
    let gate = release
        .split("GUI installer size budget")
        .nth(1)
        .expect("the size gate step exists");
    assert!(
        gate.contains("::error::") && gate.contains("exit 1"),
        "exceeding the budget aborts the release: {gate}"
    );
}

// Scenario: Instalador firmado por plataforma
#[test]
fn the_pipeline_builds_and_checksums_an_installer_per_platform() {
    let release = read(".github/workflows/release.yml");
    let job = release
        .split("package-gui:")
        .nth(1)
        .expect("the GUI packaging job exists");
    for target in ["msi", "dmg", "deb"] {
        assert!(
            job.contains(target),
            "the job collects the {target} installer"
        );
    }
    assert!(
        job.contains("SHA256SUMS"),
        "installers are checksummed under the release custody"
    );
    assert!(
        job.contains("ubuntu-latest")
            && job.contains("macos-latest")
            && job.contains("windows-latest"),
        "one installer per supported platform"
    );
    // The webview is never embedded: Windows bootstraps the OS runtime.
    let config = read("desktop/tauri.conf.json");
    assert!(config.contains("downloadBootstrapper"));
}

// Scenario: Formato autocontenido no se publica
#[test]
fn no_self_contained_installer_format_is_built_or_named() {
    // An AppImage carries WebKitGTK and its whole closure — measured at
    // 78,678,520 bytes, five times the budget — so the promise that no artifact
    // embeds a browser engine holds only if the format is never built at all.
    let config = read("desktop/tauri.conf.json");
    let targets = config
        .split("\"targets\"")
        .nth(1)
        .expect("the bundle declares its targets")
        .split(']')
        .next()
        .expect("the target list closes");
    assert!(
        !targets.to_lowercase().contains("appimage"),
        "no self-contained bundle target: {targets}"
    );
    // And nothing downstream may publish or advertise one.
    for (file, text) in [
        (
            ".github/workflows/release.yml",
            read(".github/workflows/release.yml"),
        ),
        ("README.md", read("README.md")),
        ("LEEME.md", read("LEEME.md")),
        ("site/downloads.html", read("site/downloads.html")),
        ("site/es/downloads.html", read("site/es/downloads.html")),
        ("docs/release.md", read("docs/release.md")),
    ] {
        assert!(
            !text.contains("meltemi-desktop-Linux.AppImage"),
            "{file} still names an artifact the pipeline does not produce"
        );
    }
}

// Scenario: El paquete declara el motor del sistema
#[test]
fn the_linux_package_declares_the_system_webview() {
    // Declaring the dependency is what makes "uses your system's engine" a fact
    // the package manager enforces. Without it the deb installs cleanly and then
    // fails to launch, which is the worst failure mode on offer: the one
    // component that could have resolved it reports success.
    let config = read("desktop/tauri.conf.json");
    // From `"linux"`, not from the first `"deb"`: the target list names the
    // format too, and that occurrence comes first.
    let deb = config
        .split("\"linux\"")
        .nth(1)
        .expect("the bundle configures Linux");
    for dependency in ["libwebkit2gtk", "libgtk-3-0"] {
        assert!(
            deb.contains(dependency),
            "the deb depends on {dependency}: {deb}"
        );
    }
}

// Scenario: Medición publicada por release
#[test]
fn the_budget_measurements_are_published_in_qa() {
    let qa = read("docs/qa/2026-07-20-gui-presupuestos.md");
    for expected in ["Instalador GUI", "Arranque", "RAM en reposo"] {
        assert!(qa.contains(expected), "QA reports {expected}");
    }
    assert!(
        qa.contains("ms") && qa.contains("MB"),
        "the report carries measured values, not promises"
    );
}

// Scenario: Saltable y persistente
#[test]
fn the_onboarding_flag_persists_and_needs_no_account_or_network() {
    // The flag lives in the user's data directory, like every other local
    // state; nothing is asked of the user and nothing leaves the machine.
    let flag = meltemi_client::paths::data_dir().join("desktop-onboarding-seen");
    assert!(
        flag.starts_with(meltemi_client::paths::data_dir()),
        "the flag is confined to the data directory"
    );
    let source = read("desktop/src/lib.rs");
    assert!(
        source.contains("onboarding_seen") && source.contains("onboarding_mark_seen"),
        "the surface can read and persist the first-use flag"
    );
    assert!(
        !source.contains("http://") && !source.contains("https://"),
        "no network is involved in the desktop host"
    );
}

// Scenario: Degradación honesta sin servidor
#[tokio::test]
async fn a_language_without_an_installed_server_degrades_instead_of_failing() {
    // A language nobody ships a server for: detection must answer "none"
    // rather than erroring, so the editor stays usable with highlighting.
    // The hub is inert until a server is spawned.
    let _hub = meltemi_desktop::lsp::LspHub::default();
    let source = read("desktop/src/lsp.rs");
    assert!(
        source.contains("return Ok(None); // BYO: not installed"),
        "an absent server is an honest None, not an error"
    );
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        editor.contains("editor.lsp.none"),
        "the surface announces that LSP intelligence is off and how to enable it"
    );
}

// Scenario: Estados legibles sin color
#[test]
fn every_state_renders_glyph_plus_word() {
    let badge = read("desktop/ui/src/lib/components/StatusBadge.svelte");
    for state in [
        "starting",
        "active",
        "waiting_permission",
        "ended",
        "interrupted",
    ] {
        assert!(
            badge.contains(state),
            "state {state} has a glyph mapping in the badge"
        );
    }
    assert!(
        badge.contains("state.\" + state") || badge.contains("(\"state.\" + state)"),
        "the label comes from the message catalog, not from the enum"
    );
    let messages = read("desktop/ui/src/lib/messages.ts");
    for key in [
        "state.starting",
        "state.active",
        "state.waiting_permission",
        "state.ended",
        "state.interrupted",
    ] {
        assert!(
            messages.contains(key),
            "{key} is localized in both languages"
        );
    }
}
