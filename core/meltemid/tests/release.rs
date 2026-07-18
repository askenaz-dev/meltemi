// SPDX-License-Identifier: Apache-2.0

//! Release-distribution lint (distribucion-releases): the versioning policy,
//! the release pipeline with its hard gates, the signing/verification docs, the
//! auditable installers, and the crate-namespace metadata must all be present
//! and coherent. A docs/pipeline lint over the repository's own files.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("missing file: {rel}"))
}

#[test]
fn versioning_policy_defines_breaking_changes_and_single_version() {
    // Scenario: Ruptura exige minor — the policy defines what breaks and bumps.
    let root = repo_root();
    let policy = read(&root, "docs/versionado.md");
    assert!(policy.contains("SemVer"), "policy names SemVer");
    assert!(policy.contains("minor"), "breaking → minor pre-1.0");
    for contract in ["proto/", "CLI grammar", "artifact format"] {
        assert!(
            policy.contains(contract),
            "the breaking-change definition covers `{contract}`"
        );
    }
    // Single workspace version (source of truth in the root manifest).
    let manifest = read(&root, "Cargo.toml");
    assert!(
        manifest.contains("[workspace.package]") && manifest.contains("version = \"0.1.0\""),
        "the workspace carries a single version"
    );
}

#[test]
fn the_release_pipeline_gates_and_budget_abort() {
    // Scenario: Presupuesto excedido aborta — the budget gate exits non-zero.
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    assert!(wf.contains("tags:"), "release originates from a tag");
    for gate in ["cargo fmt", "clippy", "cargo test", "cargo-deny"] {
        assert!(wf.contains(gate), "the pipeline runs the `{gate}` gate");
    }
    assert!(
        wf.contains("MELTEMI_TUI_BUDGET_BYTES") && wf.contains("exit 1"),
        "the TUI size budget gate aborts the release when exceeded"
    );
    // The three platforms.
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(wf.contains(os), "the pipeline builds on {os}");
    }
}

#[test]
fn signed_artifacts_and_verification_are_documented() {
    // Scenario: Verificación por el usuario — checksum and signature verifiable.
    let doc = read(&repo_root(), "docs/release.md");
    assert!(doc.contains("SHA256SUMS"), "checksums are published");
    assert!(
        doc.to_ascii_lowercase().contains("signature"),
        "signature verification is documented"
    );
    assert!(
        doc.to_ascii_lowercase().contains("custody"),
        "key custody procedure is documented (maintainer's)"
    );
}

#[test]
fn installers_are_auditable_and_create_the_mel_alias() {
    // Scenario: Instalación con alias — meltemi, meltemid, and `mel`.
    let root = repo_root();
    let sh = read(&root, "scripts/install.sh");
    assert!(
        sh.contains("SHA256SUMS") || sh.contains("sha256"),
        "unix installer verifies"
    );
    assert!(
        sh.contains("mel\""),
        "unix installer creates the `mel` alias"
    );
    assert!(sh.contains("meltemid"), "unix installer places the daemon");

    let ps = read(&root, "scripts/install.ps1");
    assert!(
        ps.contains("Get-FileHash"),
        "windows installer verifies the checksum"
    );
    assert!(
        ps.contains("mel.cmd"),
        "windows installer creates the `mel` shim"
    );
    assert!(
        ps.contains("meltemid.exe"),
        "windows installer places the daemon"
    );
}

#[test]
fn the_reserved_crates_carry_namespace_metadata() {
    // Scenario: Crates apuntan al proyecto — repository + description present.
    let root = repo_root();
    for crate_manifest in [
        "proto/meltemi-proto/Cargo.toml",
        "core/meltemid/Cargo.toml",
        "tui/Cargo.toml",
    ] {
        let m = read(&root, crate_manifest);
        assert!(
            m.contains("description"),
            "{crate_manifest} has a description (publishable)"
        );
        assert!(
            m.contains("repository.workspace = true") || m.contains("repository ="),
            "{crate_manifest} points at the project repository"
        );
    }
}
