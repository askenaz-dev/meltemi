// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the verify/archive verbs (comandos-verify-archive task
//! 4.2), driving an ephemeral daemon against a temporary `.meltemi/` fixture:
//!
//! - verify links scenarios named by a test and records manual marks;
//! - archive is blocked until every requirement is verified or excepted, then
//!   folds the change's deltas into the living truth and preserves it dated;
//! - a merge conflict blocks the fold and leaves the living truth intact.
//!
//! Runs against temporary fixtures, never this repo (constitution §"tests e2e").

use std::path::{Path, PathBuf};

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, error_codes, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

const LIVING: &str = "# demo-cap Specification\n\n## Purpose\nDemo.\n\n## Requirements\n### Requirement: Existing thing\nIt exists.\n\n#### Scenario: Existing scenario\n- **WHEN** x\n- **THEN** y\n";

const DELTA: &str = "## ADDED Requirements\n\n### Requirement: New thing\nIt is new.\n\n#### Scenario: Linked one\n- **WHEN** a\n- **THEN** b\n\n#### Scenario: Unlinked one\n- **WHEN** c\n- **THEN** d\n";

/// Builds a `.meltemi/` fixture: a living `demo-cap` spec and a change whose
/// delta ADDS a requirement with a linked and an unlinked scenario. A test
/// source names the linked scenario.
fn fixture(tag: &str, change: &str, delta: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-va-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let living = root.join(".meltemi/specs/demo-cap");
    std::fs::create_dir_all(&living).unwrap();
    std::fs::write(living.join("spec.md"), LIVING).unwrap();

    let cap = root
        .join(".meltemi/changes")
        .join(change)
        .join("specs/demo-cap");
    std::fs::create_dir_all(&cap).unwrap();
    std::fs::write(cap.join("spec.md"), delta).unwrap();
    std::fs::write(
        root.join(".meltemi/changes")
            .join(change)
            .join("proposal.md"),
        "## Why\nDemo change.\n",
    )
    .unwrap();

    // A test source that names the "Linked one" scenario (the link convention).
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("tests/link.rs"),
        "// Scenario: Linked one\nfn t() {}\n",
    )
    .unwrap();
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-va-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-va-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-vad-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data_dir = base.join("data");
    let config_dir = base.join("config");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(data_dir, config_dir, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-va-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn verify_links_tests_and_records_manual_marks() {
    // Scenarios: Escenario vinculado a su test; Verificación manual con nota.
    let root = fixture("verify", "add-demo", DELTA);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("verify").await;
    let peer = init_client(&endpoint).await;

    let v = peer
        .request(
            methods::SDD_VERIFY,
            &json!({ "projectRoot": root_str, "change": "add-demo" }),
        )
        .await
        .expect("verify ok");
    let status_of = |name: &str| {
        v["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["scenario"] == name)
            .unwrap_or_else(|| panic!("scenario `{name}` missing: {v:#}"))["status"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(status_of("Linked one"), "linked", "named by a test");
    assert_eq!(status_of("Unlinked one"), "unverified");
    assert_eq!(v["complete"], false);

    // Manually verify the unlinked scenario with a note.
    peer.request(
        methods::SDD_VERIFY_MARK,
        &json!({ "projectRoot": root_str, "change": "add-demo", "scenario": "Unlinked one", "note": "checked by hand" }),
    )
    .await
    .expect("mark ok");

    let v2 = peer
        .request(
            methods::SDD_VERIFY,
            &json!({ "projectRoot": root_str, "change": "add-demo" }),
        )
        .await
        .expect("verify ok");
    let unlinked = v2["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["scenario"] == "Unlinked one")
        .unwrap();
    assert_eq!(unlinked["status"], "manual", "now manual: {v2:#}");
    assert_eq!(unlinked["note"], "checked by hand");
    assert_eq!(v2["complete"], true, "all scenarios verified now");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn archive_is_gated_then_folds_and_preserves() {
    // Scenarios: Bloqueo sin verificación; Excepción justificada registrada;
    // Histórico y proyección; Fusión total o nada.
    let root = fixture("arch", "add-demo", DELTA);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("arch").await;
    let peer = init_client(&endpoint).await;

    // Link "Linked one" but leave "Unlinked one" unverified → archive blocked.
    let blocked = peer
        .request(
            methods::SDD_ARCHIVE,
            &json!({ "projectRoot": root_str, "change": "add-demo" }),
        )
        .await
        .expect_err("archive must block on an unverified requirement");
    assert_eq!(blocked.code, error_codes::VERIFY_INCOMPLETE, "{blocked}");
    // Nothing folded yet: the living spec still lacks the new requirement.
    let living_path = root.join(".meltemi/specs/demo-cap/spec.md");
    assert!(
        !std::fs::read_to_string(&living_path)
            .unwrap()
            .contains("New thing"),
        "living truth intact while blocked"
    );

    // Except the unverified scenario with a justification → archive proceeds.
    let archived = peer
        .request(
            methods::SDD_ARCHIVE,
            &json!({
                "projectRoot": root_str, "change": "add-demo",
                "exceptions": [{ "scenario": "Unlinked one", "justification": "manual QA, test pending" }],
            }),
        )
        .await
        .expect("archive ok");
    assert!(
        archived["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "demo-cap"),
        "folded demo-cap: {archived:#}"
    );
    assert!(
        archived["excepted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "Unlinked one"),
        "the exception is reported"
    );

    // The living truth now contains the added requirement AND kept the old one.
    let living = std::fs::read_to_string(&living_path).unwrap();
    assert!(
        living.contains("### Requirement: New thing"),
        "added: {living}"
    );
    assert!(living.contains("### Requirement: Existing thing"), "kept");

    // The change was preserved in the dated history and removed from active.
    assert!(
        !root.join(".meltemi/changes/add-demo").exists(),
        "change moved out of active"
    );
    let archived_to = archived["archivedTo"].as_str().unwrap();
    assert!(
        Path::new(archived_to).exists(),
        "preserved at {archived_to}"
    );
    assert!(archived_to.contains("add-demo"));

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_merge_conflict_blocks_and_leaves_the_truth_intact() {
    // Scenario: Fusión total o nada — un conflicto no toca la verdad viva.
    // A delta that MODIFIES a requirement absent from the living truth.
    let bad_delta = "## MODIFIED Requirements\n\n### Requirement: Ghost thing\nDoes not exist.\n\n#### Scenario: Linked one\n- **WHEN** a\n- **THEN** b\n";
    let root = fixture("conflict", "bad-demo", bad_delta);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("conflict").await;
    let peer = init_client(&endpoint).await;

    let living_before =
        std::fs::read_to_string(root.join(".meltemi/specs/demo-cap/spec.md")).unwrap();

    let conflict = peer
        .request(
            methods::SDD_ARCHIVE,
            &json!({
                "projectRoot": root_str, "change": "bad-demo",
                "exceptions": [{ "scenario": "Linked one", "justification": "n/a" }],
            }),
        )
        .await
        .expect_err("a merge conflict must block the archive");
    assert_eq!(
        conflict.code,
        error_codes::SPEC_MERGE_CONFLICT,
        "{conflict}"
    );

    let living_after =
        std::fs::read_to_string(root.join(".meltemi/specs/demo-cap/spec.md")).unwrap();
    assert_eq!(living_before, living_after, "the living truth is untouched");
    assert!(
        root.join(".meltemi/changes/bad-demo").exists(),
        "the change stays active after a blocked archive"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
