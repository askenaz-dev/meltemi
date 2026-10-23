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

/// A workflow with its comment lines removed. The negative assertions below are
/// about what the pipeline *does*, and both workflows explain in prose the very
/// things they must never execute — a naive substring search would trip on the
/// explanation instead of on the deed.
fn executable_lines(workflow: &str) -> String {
    workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One job of a workflow: from its `  <name>:` header to the next job header at
/// the same indentation.
fn job_block(workflow: &str, job: &str) -> String {
    let header = format!("  {job}:");
    let mut inside = false;
    let mut block: Vec<&str> = Vec::new();
    for line in workflow.lines() {
        let is_job_header = line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_start().starts_with('#')
            && line.trim_end().ends_with(':');
        if inside && is_job_header {
            break;
        }
        if line.trim_end() == header {
            inside = true;
        }
        if inside {
            block.push(line);
        }
    }
    assert!(inside, "the workflow declares a `{job}` job");
    block.join("\n")
}

#[test]
fn the_integration_build_cannot_create_or_sign_a_release() {
    // Scenario: El build de integración no crea release
    // Scenario: La ruta de integración no puede publicar
    // nothing on this path creates, signs or attests anything — and the token it
    // runs with could not, even if a step tried.
    let wf = executable_lines(&read(&repo_root(), ".github/workflows/build.yml"));

    // The deeds, absent.
    for forbidden in [
        "gh release",
        "release create",
        "softprops/action-gh-release",
        "upload-release-asset",
        "actions/attest",
        "minisign",
        ".minisig",
    ] {
        assert!(
            !wf.contains(forbidden),
            "the integration build must not contain `{forbidden}`: this path \
             packages, it does not publish, sign or attest"
        );
    }

    // And the permission that would be needed, absent too — which is what makes
    // the guarantee structural instead of a promise (design D6). Every write
    // scope at once: contents, id-token, attestations.
    assert!(
        !wf.contains(": write"),
        "the integration build grants no write permission of any kind"
    );
    let permissions: Vec<&str> = wf
        .lines()
        .skip_while(|line| line.trim_end() != "permissions:")
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .map(str::trim)
        .collect();
    assert_eq!(
        permissions,
        ["contents: read"],
        "the integration build declares read-only permissions and nothing else"
    );
}

#[test]
fn the_release_path_opens_only_where_the_gate_says_so() {
    // Scenario: Tag empujado a mano no publica
    // Nothing listens to tags: the workflow starts on a push to main or on a
    // dispatch from main, its first job asks the gate, and every job that
    // builds, packages or publishes runs only when the gate opens the path
    // (release-con-el-push D1).
    let root = repo_root();
    let release_yml = read(&root, ".github/workflows/release.yml");
    let on: Vec<&str> = release_yml
        .lines()
        .skip_while(|line| line.trim_end() != "on:")
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .collect();
    let on = on.join(" ");
    assert!(
        !on.contains("tags"),
        "no tag starts the release workflow: {on}"
    );
    assert!(
        on.contains("branches: [\"main\"]") && on.contains("workflow_dispatch"),
        "a push to main and a dispatch start it: {on}"
    );

    let version = job_block(&release_yml, "version");
    assert!(
        version.contains("meltemi-release -- gate --commit \"$GITHUB_SHA\" --main origin/main"),
        "the first job asks the gate about this very commit"
    );
    assert!(
        version.contains("if: github.ref == 'refs/heads/main'"),
        "a dispatch is honoured from main only"
    );
    assert!(
        version.contains("fetch-depth: 0"),
        "the gate sees every tag and origin/main"
    );

    let opened = "if: needs.version.outputs.release == 'true'";
    for job in ["gates", "deny", "package", "package-gui", "publish"] {
        let block = job_block(&release_yml, job);
        assert!(
            block.contains(opened),
            "the `{job}` job runs only when the gate opens the path"
        );
        assert!(
            block
                .lines()
                .any(|line| line.trim_start().starts_with("needs:") && line.contains("version")),
            "the `{job}` job waits for the gate's answer"
        );
    }

    // The tag is the pipeline's output, on the commit that passed the gates.
    let draft = job_block(&release_yml, "publish");
    assert!(
        draft.contains("--target \"$GITHUB_SHA\""),
        "the release creates its tag on the commit that passed the gates"
    );
    assert!(
        !draft.contains("--verify-tag"),
        "no tag exists before the pipeline creates it"
    );

    // No second publisher anywhere in .github/workflows.
    let dir = root.join(".github/workflows");
    for entry in std::fs::read_dir(&dir).expect("the workflows directory exists") {
        let path = entry.expect("readable entry").path();
        if path.extension().is_none_or(|ext| ext != "yml") {
            continue;
        }
        let name = path.file_name().expect("named file").to_string_lossy();
        if name == "release.yml" {
            continue;
        }
        let text = executable_lines(&std::fs::read_to_string(&path).expect("readable workflow"));
        assert!(
            !text.contains("gh release create"),
            "{name} creates a release; the release workflow is the only one that may"
        );
    }
}

#[test]
fn the_version_free_download_urls_still_answer_from_the_published_release() {
    // Scenario: La última release publicada sigue resolviendo
    // the published surfaces link `releases/latest/download/<stable-name>`, and
    // the only path that can move what that URL resolves to is the release
    // workflow: a workflow artifact has no stable URL and never enters a release.
    let root = repo_root();
    let build = executable_lines(&read(&root, ".github/workflows/build.yml"));
    assert!(
        !build.contains("releases/latest"),
        "the integration build neither writes nor claims the latest-release URL"
    );

    for surface in ["README.md", "LEEME.md"] {
        let text = read(&root, surface);
        for asset in [
            "meltemi-desktop-Windows.msi",
            "meltemi-desktop-macOS.dmg",
            "meltemi-desktop-Linux.deb",
            "meltemi-Windows.zip",
            "meltemi-macOS.tar.gz",
            "meltemi-Linux.tar.gz",
        ] {
            assert!(
                text.contains(&format!("releases/latest/download/{asset}")),
                "{surface} links `{asset}` at the version-free latest-release URL"
            );
            // The same stable name the integration build produces — which is why
            // the two paths must never share a destination.
            assert!(
                build.contains(asset),
                "the integration build rehearses `{asset}` under its stable name"
            );
        }
    }
}

#[test]
fn an_unsigned_build_says_so_where_it_is_downloaded_and_inside_itself() {
    // Scenario: Aviso de build sin firmar donde se descarga
    // Scenario: El artefacto lleva su propio aviso
    // one text, written once in the tree, reaching the run page and the artifact.
    let root = repo_root();
    let notice = read(&root, "scripts/unsigned-build-notice.txt");
    let build = read(&root, ".github/workflows/build.yml");

    // What it must say: which guarantees are absent, and where to go instead.
    assert!(
        notice.contains("UNSIGNED BUILD") && notice.contains("NOT A RELEASE"),
        "the notice names what this is before anything else"
    );
    for absence in [
        "No signature",
        "No provenance attestation",
        "No publication",
    ] {
        assert!(
            notice.contains(absence),
            "the notice declares `{absence}`: absence is the whole message"
        );
    }
    assert!(
        notice.contains("releases/latest"),
        "and points at the published release as the way to install"
    );
    // The one file most likely to be misread: a SHA256SUMS with no signature
    // beside it looks like verification and is only a manifest.
    let flat = notice.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("It is not signed")
            && flat.contains("do not read it as verification of origin"),
        "the notice disarms its own checksums file"
    );

    // Both destinations, from that same file — never retyped into the YAML.
    assert!(
        build.contains("cat scripts/unsigned-build-notice.txt >> \"$GITHUB_STEP_SUMMARY\""),
        "the notice reaches the run page, which is where the download button is"
    );
    assert!(
        build.contains("cp scripts/unsigned-build-notice.txt dist/UNSIGNED-BUILD.txt"),
        "and travels inside the artifact, which is what survives the download"
    );

    // And the documentation tells the two paths apart for whoever arrives here
    // instead of at a run page.
    let doc = read(&root, "docs/release.md");
    let doc_flat = doc.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        doc.contains(".github/workflows/build.yml"),
        "docs/release.md names the integration workflow"
    );
    assert!(
        doc_flat.contains("not distribution"),
        "and says a run artifact is not distribution"
    );
    assert!(
        doc.contains("UNSIGNED-BUILD.txt") && doc.contains("permissions: contents: read"),
        "and states both the notice and the permission that makes publishing impossible"
    );
    assert!(
        doc.contains("es la ruta") || doc_flat.contains("dos rutas de build y solo una publica"),
        "the Spanish summary carries the same distinction"
    );
}

#[test]
fn the_release_pipeline_gates_and_budget_abort() {
    // Scenario: Presupuesto excedido aborta
    // the budget gate exits non-zero.
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    assert!(
        wf.contains("meltemi-release -- gate"),
        "a release originates from the gate's answer, not from a tag"
    );
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
    // The workflow no longer names it: the release is published by the
    // pipeline and signed by its attestation (release-con-el-push 3.3); this
    // whole test is rewritten with the documentation in task 4.3.
    for file in ["docs/release.md", "scripts/sign-release.ps1"] {
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
fn the_release_is_attested_and_published_with_nothing_attached_afterwards() {
    // Scenario: Procedencia publicada con la release
    // Scenario: Nada se adjunta tras publicar
    // One attestation over the merged SHA256SUMS, its bundle kept as an asset,
    // everything uploaded to a draft, and the draft published last: with
    // immutable releases nothing can be attached afterwards (release-con-el-push
    // D3, D4).
    let root = repo_root();
    let wf = read(&root, ".github/workflows/release.yml");
    let publish = job_block(&wf, "publish");
    let attest_line = publish
        .lines()
        .find(|line| line.contains("uses: actions/attest@"))
        .expect("the publish job carries the actions/attest step");
    let pin = attest_line
        .split('@')
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("a ref after the @");
    assert!(
        pin.len() == 40 && pin.chars().all(|c| c.is_ascii_hexdigit()),
        "actions/attest is pinned to a full commit, found `{pin}`"
    );
    assert!(
        publish.contains("subject-checksums: release-assets/SHA256SUMS"),
        "the attestation subjects are the entries of the merged SHA256SUMS"
    );
    assert!(
        publish.contains("cp \"$BUNDLE\" release-assets/attestation.sigstore.json")
            && publish.contains("${{ steps.attest.outputs.bundle-path }}"),
        "the attestation bundle is kept as an asset of the release"
    );
    let at = |needle: &str| {
        publish
            .find(needle)
            .unwrap_or_else(|| panic!("the publish job has `{needle}`"))
    };
    let order = [
        at("Merge artifacts under one checksum file"),
        at("uses: actions/attest@"),
        at("Keep the attestation bundle as an asset"),
        at("gh release create \"$TAG\" --draft --target \"$GITHUB_SHA\""),
        at("gh release upload \"$TAG\" release-assets/* --clobber"),
        at("gh release edit \"$TAG\" --draft=false"),
    ];
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "merge, attest, keep the bundle, create the draft, upload, publish: in that order {order:?}"
    );
    let after_publish = &publish[at("gh release edit \"$TAG\" --draft=false")..];
    assert!(
        !after_publish.contains("gh release upload"),
        "nothing is uploaded after the release is published"
    );
    for permission in [
        "contents: write",
        "id-token: write",
        "attestations: write",
        "artifact-metadata: write",
    ] {
        assert!(
            publish.contains(permission),
            "the publish job grants `{permission}`"
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
fn a_rerun_finishes_a_half_published_release_instead_of_starting_another() {
    // Scenario: Re-ejecución tras fallo parcial termina lo que falta
    // A draft does not create its tag, so after a failure half-way the gate
    // opens the path again and the publish job finds the draft: it completes
    // it, and a published release is left alone (release-con-el-push D4).
    let publish = job_block(
        &read(&repo_root(), ".github/workflows/release.yml"),
        "publish",
    );
    assert!(
        publish.contains("gh release view \"$TAG\" --json isDraft"),
        "the publish job asks what state the release is in before acting"
    );
    for (state, deed) in [
        (
            "            false)",
            "is already published: nothing to upload",
        ),
        (
            "            true)",
            "a draft of $TAG exists: it is completed",
        ),
        ("            missing)", "gh release create \"$TAG\" --draft"),
    ] {
        let branch = publish
            .split(state)
            .nth(1)
            .unwrap_or_else(|| panic!("the publish job handles the state {state}"));
        assert!(
            branch
                .split(";;")
                .next()
                .is_some_and(|body| body.contains(deed)),
            "state {state} leads to: {deed}"
        );
    }
    assert_eq!(
        publish.matches("gh release create").count(),
        1,
        "one place creates a release, and only when none exists"
    );
    for step in ["Upload every asset to the draft", "Publish the draft"] {
        let header = publish
            .split(step)
            .nth(1)
            .expect("the step exists")
            .split("run:")
            .next()
            .expect("its header");
        assert!(
            header.contains("if: steps.draft.outputs.published != 'true'"),
            "`{step}` is skipped for a release already published"
        );
    }
}

#[test]
fn the_notes_come_from_the_changelog_when_it_has_the_version() {
    // Scenario: Notas desde el changelog
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    let version = job_block(&wf, "version");
    assert!(
        version.contains("meltemi-release -- notes \"$VERSION\" > release-notes.md"),
        "the version job takes the version's section of CHANGELOG.md"
    );
    assert!(
        version.contains("notes=changelog") && version.contains("notes=generated"),
        "and says whether the notes were found or will be generated"
    );
    let publish = job_block(&wf, "publish");
    assert!(
        publish.contains("--notes-file notes/release-notes.md")
            && publish.contains("--generate-notes"),
        "the release carries the changelog's section, or generated notes when there is none"
    );
}

#[test]
fn publication_credentials_live_only_in_the_release_environment() {
    // Scenario: Credenciales solo en el entorno de publicación
    // A job that reads a secret runs in the `release` environment, which admits
    // main only; no other job of any workflow names a secret
    // (release-con-el-push D5).
    for (name, text) in workflows(&repo_root()) {
        let jobs: Vec<String> = text
            .lines()
            .skip_while(|line| line.trim_end() != "jobs:")
            .skip(1)
            .filter(|line| {
                line.starts_with("  ")
                    && !line.starts_with("   ")
                    && !line.trim_start().starts_with('#')
                    && line.trim_end().ends_with(':')
            })
            .map(|line| line.trim().trim_end_matches(':').to_string())
            .collect();
        for job in jobs {
            let block = executable_lines(&job_block(&text, &job));
            if block.contains("secrets.") {
                assert!(
                    block.contains("environment: release"),
                    "{name}: the `{job}` job reads a secret outside the release environment"
                );
            }
        }
    }
    let publish = job_block(
        &read(&repo_root(), ".github/workflows/release.yml"),
        "publish",
    );
    assert!(
        publish.contains("environment: release"),
        "the publish job runs in the release environment"
    );
}

#[test]
fn installers_and_scripts_are_inside_what_the_attestation_covers() {
    // Scenario: Instalador atestiguado por plataforma
    // Scenario: Script instalador publicado y atestiguado
    // The packaging jobs upload the installers and the scripts as meltemi-*
    // artifacts, the publish job takes every meltemi-* artifact into the merged
    // SHA256SUMS, and the attestation's subjects are that file's entries.
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    let core = job_block(&wf, "package");
    assert!(
        core.contains("cp scripts/install.sh scripts/install.ps1 dist/")
            && core.contains("name: meltemi-${{ runner.os }}"),
        "the core packaging job ships both installer scripts as a meltemi-* artifact"
    );
    let gui = job_block(&wf, "package-gui");
    assert!(
        gui.contains("name: meltemi-gui-${{ runner.os }}"),
        "the GUI job ships each platform's installer as a meltemi-* artifact"
    );
    let publish = job_block(&wf, "publish");
    assert!(
        publish.contains("pattern: meltemi-*")
            && publish.contains("find collected -type f ! -name 'SHA256SUMS'")
            && publish.contains("sha256sum * > SHA256SUMS")
            && publish.contains("subject-checksums: release-assets/SHA256SUMS"),
        "every meltemi-* artifact enters the SHA256SUMS whose entries the attestation covers"
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

/// Every workflow of the repository, as `(file name, text)`, sorted by name.
fn workflows(root: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root.join(".github").join("workflows"))
        .expect("the workflows directory is readable")
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(entry.path()).expect("a workflow is readable");
            (name, text)
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "the repository has workflows to check");
    out
}

#[test]
fn every_action_is_pinned_by_a_full_commit_with_its_version_named() {
    // Scenario: Acción sin pinear rechazada
    // With no person between a version commit and a publication, an action
    // referenced by a tag that somebody moves would publish whatever it now
    // points at. A commit cannot be moved (release-con-el-push D5). The version
    // rides along as a comment so a reader knows what was pinned.
    let mut offenders = Vec::new();
    for (name, text) in workflows(&repo_root()) {
        for line in executable_lines(&text).lines() {
            let Some(rest) = line
                .trim_start()
                .trim_start_matches("- ")
                .strip_prefix("uses:")
            else {
                continue;
            };
            let reference = rest.split('#').next().unwrap_or("").trim();
            if reference.starts_with("./") {
                continue;
            }
            let pinned = reference.rsplit_once('@').is_some_and(|(_, at)| {
                at.len() == 40
                    && at
                        .chars()
                        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
            });
            let named = rest.contains("# v");
            if !pinned || !named {
                offenders.push(format!("{name}: {}", rest.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "every action is pinned by a full commit with its version as a comment; not so: {}",
        offenders.join("; ")
    );
}

#[test]
fn every_workflow_declares_read_only_permissions_at_the_top() {
    // Scenario: Workflow sin permisos declarados rechazado
    // A workflow that declares nothing inherits the repository default, which
    // was write; the jobs that publish raise their own (release-con-el-push D5).
    for (name, text) in workflows(&repo_root()) {
        assert!(
            text.lines().any(|line| line.trim_end() == "permissions:"),
            "{name} declares its permissions at the top"
        );
        let top: Vec<&str> = text
            .lines()
            .skip_while(|line| line.trim_end() != "permissions:")
            .skip(1)
            .take_while(|line| line.starts_with("  "))
            .map(str::trim)
            .collect();
        assert_eq!(
            top,
            ["contents: read"],
            "{name}'s top-level permissions are read-only and nothing else"
        );
    }
}

#[test]
fn the_web_is_told_after_the_release_and_only_then() {
    // Scenario: La web se redespliega tras publicar
    // Scenario: Sin publicación no hay aviso
    let notify = job_block(
        &read(&repo_root(), ".github/workflows/release.yml"),
        "notify-web",
    );
    assert!(
        notify.contains("gh workflow run deploy.yml --repo askenaz-dev/meltemi-web --ref main"),
        "the notice starts meltemi-web's deploy workflow"
    );
    let needs = notify
        .lines()
        .find(|line| line.trim_start().starts_with("needs:"))
        .expect("the notice waits for something");
    assert!(
        needs.contains("publish"),
        "the notice waits for the publication: {needs}"
    );
    for override_status in ["always()", "failure()", "cancelled()", "!cancelled()"] {
        assert!(
            !notify.contains(override_status),
            "the notice runs only after a successful publication, never with `{override_status}`"
        );
    }
    assert!(
        notify.contains("environment: release") && notify.contains("secrets.WEB_DISPATCH_TOKEN"),
        "its credential lives in the release environment"
    );
}

#[test]
fn a_failed_notice_cannot_touch_the_release_and_carries_no_data() {
    // Scenario: Aviso fallido no deshace la release
    // Scenario: El aviso no lleva datos
    let wf = read(&repo_root(), ".github/workflows/release.yml");
    let notify = executable_lines(&job_block(&wf, "notify-web"));
    assert!(
        !notify.contains("gh release"),
        "the notice never touches the release it announces"
    );
    let publish = job_block(&wf, "publish");
    assert!(
        !publish.contains("notify-web"),
        "the publication does not wait for the notice, so its failure undoes nothing"
    );
    let dispatch = notify
        .lines()
        .find(|line| line.contains("gh workflow run"))
        .expect("the dispatch line");
    for field in [" -f ", " -F ", "--field", "--raw-field", "--json"] {
        assert!(
            !dispatch.contains(field),
            "the dispatch carries no data (`{field}`): {dispatch}"
        );
    }
}
