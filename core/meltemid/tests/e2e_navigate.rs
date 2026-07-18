// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the method-navigation verbs (navegacion-del-metodo),
//! driving an ephemeral daemon against a temporary `.meltemi/` fixture:
//!
//! - change/list aggregates read-only state (artifacts, tasks, review, verify)
//!   for active changes and lists archived history dated;
//! - change/show returns artifacts and per-capability deltas verbatim;
//! - spec/list and spec/show navigate the living truth;
//! - sdd/validate reports clean vs findings without mutating anything.
//!
//! Runs against temporary fixtures, never this repo (constitution §"tests e2e").

use std::path::PathBuf;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, error_codes, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

const LIVING: &str = "# demo-cap Specification\n\n## Purpose\nDemo.\n\n## Requirements\n### Requirement: Existing thing\nThe system SHALL keep existing.\n\n#### Scenario: Existing scenario\n- **WHEN** x\n- **THEN** y\n";

const DELTA: &str = "## ADDED Requirements\n\n### Requirement: Thing one\nThe system SHALL do thing one.\n\n#### Scenario: Alpha\n- **WHEN** a\n- **THEN** b\n\n### Requirement: Thing two\nThe system SHALL do thing two.\n\n#### Scenario: Beta\n- **WHEN** c\n- **THEN** d\n";

/// A `.meltemi/` fixture: a living demo-cap, an active change in MIXED state
/// (tasks 1/2, review 1/2, verify 1/2), and an archived change.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-nav-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let w = |rel: &str, content: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    };

    w(".meltemi/specs/demo-cap/spec.md", LIVING);
    w(".meltemi/changes/add-demo/proposal.md", "## Why\nDemo.\n");
    w(".meltemi/changes/add-demo/design.md", "## Context\nDemo.\n");
    w(".meltemi/changes/add-demo/specs/demo-cap/spec.md", DELTA);
    w(
        ".meltemi/changes/add-demo/tasks.md",
        "## 1. Work\n\n- [x] 1.1 First\n- [ ] 1.2 Second\n",
    );
    // Review state: one of the two requirements decided -> review 1/2.
    w(
        ".meltemi/changes/add-demo/.review-state.json",
        "{\"states\":{\"demo-cap\\u001fThing one\":\"approved\"}}",
    );
    // A test names scenario Alpha (linked) but not Beta -> verify 1/2.
    w("tests/nav_link.rs", "// Scenario: Alpha\nfn t() {}\n");
    // Archived history.
    w(
        ".meltemi/changes/archive/2026-07-01-old-change/proposal.md",
        "## Why\nOld.\n",
    );
    w(
        ".meltemi/changes/archive/2026-07-01-old-change/tasks.md",
        "- [x] 1.1 done\n",
    );
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-nav-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-nav-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-navd-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(base.join("data"), base.join("config"), shutdown_tx);
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
                name: "e2e-nav-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn change_list_aggregates_state_and_lists_archived() {
    // Scenario: Listado con estado por change
    // Scenario: Archivadas consultables
    // Scenario: Estado parcial honesto
    let root = fixture("list");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("list").await;
    let peer = init_client(&endpoint).await;

    let list = peer
        .request(methods::CHANGE_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("change/list ok");
    let changes = list["changes"].as_array().unwrap();
    let active = changes.iter().find(|c| c["name"] == "add-demo").unwrap();
    assert_eq!(active["archived"], false);
    assert_eq!(active["artifacts"]["proposal"], true);
    assert_eq!(active["artifacts"]["design"], true);
    assert_eq!(active["tasksDone"], 1);
    assert_eq!(active["tasksTotal"], 2);
    assert_eq!(
        active["reviewDecided"], 1,
        "one requirement approved: {active:#}"
    );
    assert_eq!(active["reviewTotal"], 2);
    assert_eq!(active["verified"], 1, "scenario Alpha linked: {active:#}");
    assert_eq!(active["verifyTotal"], 2);

    let archived = changes.iter().find(|c| c["name"] == "old-change").unwrap();
    assert_eq!(archived["archived"], true);
    assert_eq!(archived["archivedAt"], "2026-07-01");
    // Honest partial state: old-change has no design nor specs; the listing
    // reflects the absence rather than inventing it.
    assert_eq!(archived["artifacts"]["design"], false);
    assert_eq!(archived["artifacts"]["specs"], false);
    assert_eq!(archived["artifacts"]["proposal"], true);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn change_and_spec_show_and_list() {
    // Scenario: Mostrar una change con sus artefactos
    // Scenario: Mostrar una spec viva
    // Scenario: Nombre inexistente rehúsa con remedio
    let root = fixture("show");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("show").await;
    let peer = init_client(&endpoint).await;

    let show = peer
        .request(
            methods::CHANGE_SHOW,
            &json!({ "projectRoot": root_str, "change": "add-demo" }),
        )
        .await
        .expect("change/show ok");
    let arts: Vec<&str> = show["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert!(arts.contains(&"proposal") && arts.contains(&"design"));
    assert_eq!(show["deltas"][0]["capability"], "demo-cap");
    assert!(
        show["deltas"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Thing one")
    );

    // A nonexistent change is refused.
    let missing = peer
        .request(
            methods::CHANGE_SHOW,
            &json!({ "projectRoot": root_str, "change": "ghost" }),
        )
        .await
        .expect_err("unknown change refused");
    assert_eq!(missing.code, error_codes::ARTIFACT_NOT_FOUND);

    let specs = peer
        .request(methods::SPEC_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("spec/list ok");
    let cap = specs["specs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["capability"] == "demo-cap")
        .unwrap();
    assert_eq!(cap["requirements"], 1);
    assert_eq!(cap["scenarios"], 1);

    let shown = peer
        .request(
            methods::SPEC_SHOW,
            &json!({ "projectRoot": root_str, "capability": "demo-cap" }),
        )
        .await
        .expect("spec/show ok");
    assert_eq!(shown["requirements"][0]["name"], "Existing thing");
    assert_eq!(
        shown["requirements"][0]["scenarios"][0]["name"],
        "Existing scenario"
    );
    assert_eq!(
        shown["requirements"][0]["scenarios"][0]["steps"][0]["marker"],
        "when"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn validate_reports_clean_and_findings_without_mutating() {
    // Scenario: Validar una change sin tocarla
    // Scenario: Hallazgos reportados sin archivar
    // Scenario: Verdad viva validada sin argumento
    // Scenario: Validación con hallazgos distinguible
    let root = fixture("validate");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("validate").await;
    let peer = init_client(&endpoint).await;

    // A well-formed additive change validates clean.
    let clean = peer
        .request(
            methods::SDD_VALIDATE,
            &json!({ "projectRoot": root_str, "change": "add-demo" }),
        )
        .await
        .expect("validate ok");
    assert_eq!(clean["scope"], "change");
    assert_eq!(clean["clean"], true, "additive change is clean: {clean:#}");

    // A delta that MODIFIES a requirement absent from the living truth: findings.
    let bad = root.join(".meltemi/changes/bad-demo/specs/demo-cap");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(
        bad.join("spec.md"),
        "## MODIFIED Requirements\n\n### Requirement: Ghost\nNope.\n\n#### Scenario: G\n- **WHEN** a\n- **THEN** b\n",
    )
    .unwrap();
    let living_before =
        std::fs::read_to_string(root.join(".meltemi/specs/demo-cap/spec.md")).unwrap();

    let findings = peer
        .request(
            methods::SDD_VALIDATE,
            &json!({ "projectRoot": root_str, "change": "bad-demo" }),
        )
        .await
        .expect("validate ok");
    assert_eq!(findings["clean"], false, "conflict reported: {findings:#}");
    assert!(
        !findings["diagnostics"].as_array().unwrap().is_empty(),
        "diagnostics present"
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".meltemi/specs/demo-cap/spec.md")).unwrap(),
        living_before,
        "validation mutated nothing"
    );

    // The whole living truth validates without a change argument.
    let living = peer
        .request(methods::SDD_VALIDATE, &json!({ "projectRoot": root_str }))
        .await
        .expect("validate living ok");
    assert_eq!(living["scope"], "living-truth");
    assert_eq!(
        living["clean"], true,
        "the demo living truth is valid: {living:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
