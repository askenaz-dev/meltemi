// SPDX-License-Identifier: Apache-2.0

//! Documentation lint (documentacion-inicial): the root README and the `docs/`
//! skeleton must be present with their sections, internal links must resolve,
//! the generated CLI reference must be fresh, and the quickstart's scriptable
//! steps must run against the real built binary.
//!
//! Reads the repository's own static docs and runs the built `meltemi` binary;
//! it is a docs/pipeline lint, not an agent e2e.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root (this crate lives at `<root>/tui`).
fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // <root>
    dir
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("missing doc: {rel}"))
}

fn norm(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn readme_and_docs_are_present_with_their_sections() {
    // Scenario: Primer contacto completo
    // README explains what/status/how.
    let root = repo_root();
    let readme = read(&root, "README.md");
    for section in ["## What it is", "## Status", "## Install", "## First step"] {
        assert!(readme.contains(section), "README missing `{section}`");
    }
    assert!(
        readme.contains("LEEME.md"),
        "README links its Spanish mirror"
    );
    // No third-party product names in the public README (only open standards).
    let lower = readme.to_ascii_lowercase();
    assert!(
        !lower.contains("claude") && !lower.contains("gemini") && !lower.contains("copilot"),
        "the public README must not name third-party products"
    );

    // The docs skeleton is present.
    for doc in [
        "LEEME.md",
        "docs/quickstart.md",
        "docs/arquitectura.md",
        "docs/metodo-sdd.md",
        "docs/accesibilidad.md",
        "docs/plataformas.md",
        "docs/agentes.md",
        "docs/referencia-cli.md",
        "docs/acceso-remoto.md",
    ] {
        assert!(root.join(doc).is_file(), "missing doc: {doc}");
    }
}

#[test]
fn the_remote_access_frontier_is_documented() {
    // Scenario: La frontera está documentada como postura
    // Scenario: Propuesta de push sin túnel se rechaza
    let doc = read(&repo_root(), "docs/acceso-remoto.md").to_ascii_lowercase();
    // The frontier: remote control requires a live tunnel.
    assert!(
        doc.contains("túnel vivo"),
        "the live-tunnel requirement is declared"
    );
    // The three remote verbs are named.
    for verb in ["monitorear", "aprobar", "dirigir"] {
        assert!(doc.contains(verb), "the `{verb}` remote verb is documented");
    }
    // The absence of push without a connection, explained as a posture with its
    // why (§3 / no cloud relay), not as a missing feature.
    assert!(
        doc.contains("postura de privacidad"),
        "the no-push stance is framed as a posture"
    );
    assert!(
        doc.contains("§3") && doc.contains("relé"),
        "the why is grounded in §3 and the no-relay rule"
    );
    // A push-without-tunnel proposal is rejected save a foundational amendment.
    assert!(
        doc.contains("se rechaza") && doc.contains("enmienda"),
        "the rejection rule (save amendment) is stated"
    );
}

#[test]
fn the_git_bash_gotcha_is_documented() {
    // Scenario: La trampa de git-bash documentada
    // the H6 finding with remedy.
    let notes = read(&repo_root(), "docs/plataformas.md");
    assert!(
        notes.contains("MELTEMI_ENDPOINT"),
        "the mangled var is named"
    );
    assert!(
        notes.contains("MSYS_NO_PATHCONV"),
        "the remedy for the git-bash mangling is documented"
    );
}

#[test]
fn the_cli_reference_is_generated_and_fresh() {
    // Scenario: Referencia nunca desincronizada
    // the committed reference must
    // equal what the generator produces from the grammar.
    // Scenario: La referencia es la enumeración autoritativa
    let committed = read(&repo_root(), "docs/referencia-cli.md");
    let generated = meltemi::cli::reference();
    assert_eq!(
        norm(&committed),
        norm(&generated),
        "docs/referencia-cli.md is stale; regenerate with \
         `cargo run --example gen_cli_ref > docs/referencia-cli.md`"
    );
}

#[test]
fn internal_doc_links_resolve() {
    // Docs lint: internal markdown links point at files that exist.
    let root = repo_root();
    let docs = [
        "README.md",
        "LEEME.md",
        "docs/quickstart.md",
        "docs/arquitectura.md",
        "docs/metodo-sdd.md",
        "docs/accesibilidad.md",
        "docs/plataformas.md",
        "docs/acceso-remoto.md",
    ];
    let mut problems = Vec::new();
    for doc in docs {
        let content = read(&root, doc);
        let base = root.join(doc);
        let base_dir = base.parent().unwrap();
        for target in markdown_link_targets(&content) {
            // Skip external links and pure anchors.
            if target.starts_with("http") || target.starts_with('#') {
                continue;
            }
            let path = target.split('#').next().unwrap_or(&target);
            if path.is_empty() {
                continue;
            }
            if !base_dir.join(path).exists() {
                problems.push(format!("{doc} -> `{target}` does not resolve"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "broken internal links:\n{}",
        problems.join("\n")
    );
}

/// Extracts the targets of `[text](target)` markdown links.
fn markdown_link_targets(content: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b']'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'('
            && let Some(end) = content[i + 2..].find(')')
        {
            targets.push(content[i + 2..i + 2 + end].trim().to_string());
            i += 2 + end;
        }
        i += 1;
    }
    targets
}

/// The built `meltemi` binary next to the test executable.
fn meltemi_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "meltemi.exe"
    } else {
        "meltemi"
    })
}

#[test]
fn quickstart_local_steps_run_against_the_binary() {
    // Scenario: Quickstart en CI
    // the scriptable steps run against the real
    // binary and a divergent output fails.
    let bin = meltemi_bin();
    assert!(
        bin.exists(),
        "build the `meltemi` binary first (cargo test builds it)"
    );

    // `meltemi version` — local, prints a semver-shaped version, exit 0.
    let out = Command::new(&bin)
        .arg("version")
        .output()
        .expect("run version");
    assert!(out.status.success(), "`meltemi version` exits 0");
    let version = String::from_utf8_lossy(&out.stdout);
    assert!(
        version.trim().split('.').count() >= 2,
        "version looks like a version: {version:?}"
    );

    // `meltemi help` — local, lists the documented subcommands.
    let out = Command::new(&bin).arg("help").output().expect("run help");
    assert!(out.status.success(), "`meltemi help` exits 0");
    let help = String::from_utf8_lossy(&out.stdout);
    for verb in ["propose", "review", "implement", "archive"] {
        assert!(
            help.contains(verb),
            "help documents `{verb}`: divergence would fail here"
        );
    }
}
