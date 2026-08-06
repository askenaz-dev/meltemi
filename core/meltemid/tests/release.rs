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
    // Scenario: Ruptura exige minor
    // the policy defines what breaks and bumps.
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
    // Single workspace version (source of truth in the root manifest). The
    // literal `0.1.0` used to be asserted here, which made every bump of the
    // number turn red the test that exists to keep the number singular — and it
    // did, at 0.1.1. What the policy actually requires is that the root manifest
    // declares one version and every member inherits it, so that is what is
    // checked.
    let manifest = read(&root, "Cargo.toml");
    let (_, workspace_package) = manifest
        .split_once("[workspace.package]")
        .expect("the root manifest declares [workspace.package]");
    let version = workspace_package
        .lines()
        .take_while(|line| !line.trim_start().starts_with('[')) // that section only
        .find_map(|line| line.trim().strip_prefix("version = "))
        .expect("[workspace.package] carries the version")
        .trim()
        .trim_matches('"');
    assert!(
        version.split('.').count() == 3 && version.split('.').all(|p| p.parse::<u32>().is_ok()),
        "the workspace version is a semver triple, found {version:?}"
    );
    for member in [
        "core/meltemid/Cargo.toml",
        "tui/Cargo.toml",
        "desktop/Cargo.toml",
        "proto/meltemi-proto/Cargo.toml",
    ] {
        assert!(
            read(&root, member).contains("version.workspace = true"),
            "{member} inherits the single workspace version instead of carrying its own"
        );
    }
}

/// The value of an `env:` declaration in a workflow, by key. Only the
/// declaration matches: a shell line that merely expands `${KEY}` does not
/// begin with `KEY:`.
fn workflow_env(text: &str, key: &str) -> String {
    text.lines()
        .find(|line| line.trim_start().starts_with(&format!("{key}:")))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| panic!("no `{key}` is declared"))
}

#[test]
fn a_push_to_main_packages_every_platform_and_leaves_it_downloadable() {
    // Scenario: Push a main deja el build descargable
    // the integration path builds the release path's per-platform artifacts and
    // uploads them as artifacts of the run.
    let wf = read(&repo_root(), ".github/workflows/build.yml");

    // Triggered by main, and by hand — the second is also the first cadence
    // dial: dropping the push trigger leaves a working manual path (design D5).
    assert!(
        wf.contains("branches: [\"main\"]"),
        "the integration build runs on a push to main"
    );
    assert!(
        wf.contains("workflow_dispatch:"),
        "and can be run by hand, which is the cadence dial"
    );
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(wf.contains(os), "the integration build runs on {os}");
    }

    // The same six files the release path produces: three core archives and
    // three normalized installers, all built here.
    for artifact in [
        "meltemi-Windows.zip",
        "meltemi-macOS.tar.gz",
        "meltemi-Linux.tar.gz",
        "meltemi-desktop-Windows.msi",
        "meltemi-desktop-macOS.dmg",
        "meltemi-desktop-Linux.deb",
    ] {
        assert!(
            wf.contains(artifact),
            "the integration build produces {artifact}"
        );
    }
    assert!(
        wf.contains("tauri build"),
        "the desktop installers are bundled, not assumed"
    );
    assert!(
        wf.contains("SHA256SUMS"),
        "the run's own manifest of what it built travels with it"
    );

    // And it ends downloadable from the run page.
    assert!(
        wf.contains("actions/upload-artifact"),
        "the output is uploaded as an artifact of the run"
    );
}

#[test]
fn the_integration_artifact_names_its_commit_and_bounds_its_retention() {
    // Scenario: Nombre del artefacto declara commit y build
    // Scenario: Retención acotada y declarada
    // the name says unsigned build and which commit; the retention is a number
    // in the pipeline, not an open-ended default.
    let wf = read(&repo_root(), ".github/workflows/build.yml");

    let name = wf
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("name: meltemi-"))
        .expect("the uploaded artifact is named");
    assert!(
        name.contains("unsigned"),
        "the artifact name declares it is an unsigned build: {name}"
    );
    assert!(
        name.contains("short_sha"),
        "and identifies the commit that produced it: {name}"
    );
    assert!(
        wf.contains("GITHUB_SHA:0:8"),
        "the short SHA is derived from the commit being built"
    );

    // The files INSIDE keep the stable, version-free names of the release path,
    // so this rehearses the normalization step too (design D3). A per-run name
    // there would stop rehearsing exactly the step that breaks unnoticed.
    assert!(
        wf.contains("normalized $(basename \"$src\") -> $dest"),
        "the installers are renamed to the stable scheme, as a release does"
    );

    let retention = wf
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("retention-days:"))
        .expect("the pipeline declares how long these artifacts live")
        .trim()
        .parse::<u32>()
        .expect("the retention is a number of days");
    assert!(
        retention > 0 && retention <= 90,
        "the retention is bounded, not indefinite: found {retention} days"
    );
}

#[test]
fn the_size_budgets_are_the_same_numbers_in_every_packaging_path() {
    // Scenario: Presupuesto excedido falla el build de integración
    // both packaging paths measure against the same limits and exit non-zero.
    // This is the test that answers the one real cost of keeping the integration
    // build in a file of its own (design D1): two files that must agree
    // eventually do not, unless something fails when they stop agreeing.
    let root = repo_root();
    let release = read(&root, ".github/workflows/release.yml");
    let build = read(&root, ".github/workflows/build.yml");

    for budget in [
        "MELTEMI_TUI_BUDGET_BYTES",
        "MELTEMI_GUI_INSTALLER_BUDGET_BYTES",
        "MELTEMI_ADAPTER_BUDGET_BYTES",
    ] {
        let on_release = workflow_env(&release, budget);
        let on_build = workflow_env(&build, budget);
        assert_eq!(
            on_release, on_build,
            "`{budget}` is {on_release} in .github/workflows/release.yml and \
             {on_build} in .github/workflows/build.yml; a budget measured \
             differently depending on which file runs it is not a budget"
        );
        assert!(
            build.contains(&format!("${{{budget}}}")),
            "the integration build measures against `{budget}`, not just declares it"
        );
    }
    // Measuring without failing is a report, not a gate.
    assert!(
        build.matches("exit 1").count() >= 3,
        "each budget aborts the integration build when exceeded"
    );
}

#[test]
fn the_release_pipeline_gates_and_budget_abort() {
    // Scenario: Presupuesto excedido aborta
    // the budget gate exits non-zero.
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    assert!(wf.contains("tags:"), "release originates from a tag");
    for gate in ["cargo fmt", "clippy", "cargo test", "cargo-deny"] {
        assert!(wf.contains(gate), "the pipeline runs the `{gate}` gate");
    }
    assert!(
        wf.contains("MELTEMI_TUI_BUDGET_BYTES") && wf.contains("exit 1"),
        "the TUI size budget gate aborts the release when exceeded"
    );
    // The bundled ACP adapters travel in every archive, so their size is part
    // of what a user downloads and is gated like everything else that ships.
    assert!(
        wf.contains("MELTEMI_ADAPTER_BUDGET_BYTES"),
        "the bundled adapters have a size budget of their own"
    );
    // The three platforms.
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(wf.contains(os), "the pipeline builds on {os}");
    }
}

#[test]
fn every_workflow_script_stays_inside_its_block() {
    // A `run: |` body is a YAML block scalar: one line at column 0 ends the
    // block and the whole workflow stops parsing, which GitHub reports as a run
    // with no jobs and no log — an expensive way to learn about a shell heredoc.
    // Cheap structural check, no YAML parser and so no new dependency.
    let root = repo_root();
    let dir = root.join(".github/workflows");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("the workflows directory exists") {
        let path = entry.expect("readable entry").path();
        if path.extension().is_none_or(|ext| ext != "yml") {
            continue;
        }
        checked += 1;
        let name = path.file_name().expect("named file").to_string_lossy();
        let text = std::fs::read_to_string(&path).expect("readable workflow");
        let mut block: Option<usize> = None;
        for (number, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue; // blank lines belong to whatever block is open
            }
            let indent = line.len() - line.trim_start().len();
            if let Some(opened) = block {
                if indent > opened {
                    continue; // body of the block, correctly indented
                }
                assert!(
                    indent > 0,
                    "{name}:{} leaves its block scalar at column 0: {line:?}",
                    number + 1
                );
                block = None; // the block ended on a sibling key
            }
            if line.trim_end().ends_with(": |") {
                block = Some(indent);
            }
        }
    }
    assert!(checked >= 2, "both workflows are linted, found {checked}");
}

#[test]
fn signed_artifacts_and_verification_are_documented() {
    // Scenario: Verificación por el usuario
    // checksum and signature verifiable.
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
    // One name for the signature asset, everywhere. The pipeline's release notes
    // said `SHA256SUMS.sig` while the documented procedure and the signing script
    // produced `SHA256SUMS.minisig`, so the notes every downloader reads pointed
    // at a file that was never published — and nothing failed, because nothing
    // was checking.
    let root = repo_root();
    let signature = "SHA256SUMS.minisig";
    for file in [
        "docs/release.md",
        "scripts/sign-release.ps1",
        ".github/workflows/release.yml",
    ] {
        let text = read(&root, file);
        assert!(
            text.contains(signature),
            "{file} names the signature asset `{signature}`"
        );
        // `.sig` alone would be a second, wrong name for the same thing.
        assert!(
            !text.contains("SHA256SUMS.sig"),
            "{file} still uses the old `SHA256SUMS.sig` name"
        );
    }
    // The trust anchor lives in the repository, not on the release page: whoever
    // can publish a release can edit the text beside it, so a key printed only
    // there proves nothing.
    assert!(
        !doc.contains("public key printed on every release page"),
        "the public key must not be sourced from the release page it authenticates"
    );
}

#[test]
fn the_pipeline_attests_its_checksums_and_the_command_is_published() {
    // Scenario: Procedencia publicada con la release
    // the mint over the merged checksums, and the documented verify command.
    let root = repo_root();
    let wf = read(&root, ".github/workflows/release.yml");
    let attest_line = wf
        .lines()
        .find(|line| line.contains("uses: actions/attest@"))
        .expect("the release job carries the actions/attest step");
    // Pinned by full commit SHA (§10): a step that mints with the job's OIDC
    // identity must not be movable by a floating tag.
    let pin = attest_line
        .split('@')
        .nth(1)
        .expect("a ref after the @")
        .split_whitespace()
        .next()
        .expect("the ref itself");
    assert!(
        pin.len() == 40 && pin.chars().all(|c| c.is_ascii_hexdigit()),
        "actions/attest must be pinned to a full commit SHA, found `{pin}`"
    );
    // The subjects are the entries of the merged SHA256SUMS — the same single
    // file the maintainer signs — and the mint sits between the merge and the
    // draft, so a failed attestation aborts before anything is published.
    assert!(
        wf.contains("subject-checksums: release-assets/SHA256SUMS"),
        "the attestation subjects come from the merged SHA256SUMS"
    );
    let merge = wf
        .find("Merge artifacts under one checksum file")
        .expect("the merge step exists");
    let attest = wf.find("uses: actions/attest@").expect("the attest step");
    let draft = wf
        .find("Create the draft release")
        .expect("the draft step exists");
    assert!(
        merge < attest && attest < draft,
        "the mint must sit between the checksum merge and the draft creation"
    );
    // The three permissions of the mint, granted where the mint happens.
    for permission in [
        "id-token: write",
        "attestations: write",
        "artifact-metadata: write",
    ] {
        assert!(
            wf.contains(permission),
            "the release job grants `{permission}`"
        );
    }
    // The published verification command pins the workflow identity.
    let doc = read(&root, "docs/release.md");
    assert!(
        doc.contains("gh attestation verify") && doc.contains("--signer-workflow"),
        "docs/release.md publishes the provenance verification command"
    );
}

#[test]
fn the_attestation_scope_is_declared_without_overclaiming() {
    // Scenario: Alcance de la atestación declarado sin exagerar
    // which job mints it, what it covers, and no build-coverage claim.
    // The doc wraps its prose at will, so the phrases are asserted over
    // collapsed whitespace.
    let doc = read(&repo_root(), "docs/release.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        doc.contains("`release` job"),
        "docs/release.md names the job that mints the attestation"
    );
    assert!(
        doc.contains("attests the aggregation, not each build step"),
        "docs/release.md states the honest scope: the aggregation"
    );
    assert!(
        doc.contains("does not attest the compilation"),
        "docs/release.md denies the per-artifact build claim in so many words"
    );
    assert!(
        doc.contains("not a substitute for the maintainer's signature"),
        "the attestation must never be presented as replacing the signature"
    );
}

#[test]
fn the_transparency_log_is_declared_and_is_not_telemetry() {
    // Scenario: Registro público de la atestación declarado
    // what gets recorded, said by the project before a reader discovers it.
    // Collapsed whitespace, for the same reason as the scope test above.
    let doc = read(&repo_root(), "docs/release.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        doc.contains("transparency log"),
        "docs/release.md declares the public transparency log"
    );
    assert!(
        doc.contains("names and digests") && doc.contains("identity of the workflow"),
        "docs/release.md says what the log records"
    );
    assert!(
        doc.contains("never user data"),
        "the log entry is distinguished from telemetry: build metadata, never user data"
    );
}

#[test]
fn the_trust_anchor_is_never_sourced_from_the_release_page() {
    // Scenario: Ancla de confianza fuera de la página que autentica
    // the key's documented home is the tree; no instruction says otherwise.
    let root = repo_root();
    let doc = read(&root, "docs/release.md");
    assert!(
        doc.contains("here, in the repository"),
        "docs/release.md declares the repository as the key's home"
    );
    // The signing walkthrough must not send the key to the page a publisher
    // controls: notes and site point at the anchor, never carry it.
    assert!(
        !doc.contains("paste the public key into the release notes")
            && !doc.contains("goes in the release notes"),
        "docs/release.md must not direct the key to the release notes"
    );
    // Both readmes source the key from the repository file.
    let readme = read(&root, "README.md");
    assert!(
        readme.contains("the public key in docs/release.md"),
        "README.md sources the public key from docs/release.md"
    );
    let leeme = read(&root, "LEEME.md");
    assert!(
        leeme.contains("la clave pública de docs/release.md"),
        "LEEME.md sources the public key from docs/release.md"
    );
}

#[test]
fn the_signing_tools_limits_are_declared_not_promised_away() {
    // Scenario: Límites de la herramienta declarados
    // offline, not hardware-backed; repudiation defined without revocation.
    let doc = read(&repo_root(), "docs/release.md");
    assert!(
        doc.contains("offline, not hardware-backed"),
        "custody promises offline storage, not a hardware token minisign lacks"
    );
    assert!(
        doc.contains("no revocation mechanism"),
        "custody states that minisign cannot revoke a signature"
    );
    assert!(
        doc.contains("repudiated from a given date"),
        "repudiation is defined as a new key in the tree plus a dated statement"
    );
    assert!(
        !doc.contains("hardware-backed storage"),
        "the doc must not promise hardware-backed custody"
    );
}

#[test]
fn installers_are_auditable_and_create_the_mel_alias() {
    // Scenario: Instalación con alias
    // meltemi, meltemid, and `mel`.
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
fn the_bundled_adapters_ship_beside_the_daemon_on_all_three_platforms() {
    // Detection probes the daemon's own directory for a bundled layer
    // (adaptadores-propios-acp D8), so an installer that puts them anywhere
    // else — or nowhere — leaves every level-2 entry reporting a missing
    // adapter with a remedy the user cannot act on.
    let root = repo_root();
    let adapters = ["meltemi-claude-acp", "meltemi-codex-acp"];

    // Both packaging paths: the release one and the integration one
    // (artefactos-de-cada-push). An archive that rehearses the release without
    // its adapters rehearses the wrong thing.
    for workflow in [
        ".github/workflows/release.yml",
        ".github/workflows/build.yml",
    ] {
        let wf = read(&root, workflow);
        for platform in [
            "meltemi-Windows.zip",
            "meltemi-macOS.tar.gz",
            "meltemi-Linux.tar.gz",
        ] {
            let line = wf
                .lines()
                .find(|line| line.contains(platform) && !line.trim_start().starts_with('#'))
                .unwrap_or_else(|| panic!("{workflow} packages {platform}"));
            for adapter in adapters {
                assert!(
                    line.contains(adapter),
                    "{workflow}: {platform} must carry `{adapter}`: {line}"
                );
            }
        }
    }

    let sh = read(&root, "scripts/install.sh");
    let ps = read(&root, "scripts/install.ps1");
    for adapter in adapters {
        assert!(
            sh.contains(adapter),
            "the unix installer places `{adapter}` beside the daemon"
        );
        assert!(
            ps.contains(&format!("{adapter}.exe")),
            "the windows installer places `{adapter}.exe` beside the daemon"
        );
    }
    // Both installers put everything in ONE directory: the daemon's own.
    assert!(
        sh.contains("-C \"$INSTALL_DIR\""),
        "the unix installer extracts into the single install directory"
    );
    assert!(
        ps.contains("-DestinationPath $InstallDir"),
        "the windows installer expands into the single install directory"
    );

    // And the published description says so, so a manual installation lands
    // the same way as a scripted one.
    let doc = read(&root, "docs/release.md");
    for adapter in adapters {
        assert!(
            doc.contains(adapter),
            "docs/release.md names `{adapter}` among the published binaries"
        );
    }
    // Prose wraps; the fact does not.
    let flat = doc.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("same directory as the daemon"),
        "and states where they must land"
    );
}

#[test]
fn the_reserved_crates_carry_namespace_metadata() {
    // Scenario: Crates apuntan al proyecto
    // repository + description present.
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
