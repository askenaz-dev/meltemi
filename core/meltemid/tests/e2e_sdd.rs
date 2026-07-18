// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of the SDD authoring cycle (ciclo-sdd-autoria task 5.2):
//! a full spec-full cycle with gates driven by a scripted authoring mock; an
//! invalid delta returns to the agent without opening the gate; and `explore`
//! leaves the project tree untouched. Runs against temporary fixtures.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn mock_agent_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    })
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture pointing the agent at the mock (with args) and allowing writes.
fn fixture(root: &Path, mock_args: &str) {
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}'{mock_args}]\n",
            mock_agent_bin().display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
}

async fn client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-sdd".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn spawn(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn gate(peer: &Peer, root: &str, change: &str, decision: &str) -> Value {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::SDD_GATE,
            &json!({ "projectRoot": root, "changeName": change, "decision": decision }),
        ),
    )
    .await
    .expect("gate returned")
    .expect("gate ok")
}

#[tokio::test]
async fn spec_full_cycle_advances_through_gates_to_completion() {
    // Scenario: Gate aprueba y avanza (through all four artifacts).
    let root = std::env::temp_dir().join(format!("meltemi-e2e-sdd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, ", '--sdd-author'");
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn("sdd-full").await;
    let peer = client(&endpoint).await;
    let root_str = root.display().to_string();

    // Start the cycle: authors the proposal, validates, opens the first gate.
    let started = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::SDD_PROPOSE,
            &json!({ "idea": "add sdd thing", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(started["phase"], "gate_pending", "{started:#}");
    assert_eq!(started["artifact"], "proposal");
    let change = started["changeName"].as_str().unwrap().to_string();

    // Approve each artifact in turn: proposal → specs → design → tasks → done.
    let after_proposal = gate(&peer, &root_str, &change, "approve").await;
    assert_eq!(after_proposal["artifact"], "specs", "{after_proposal:#}");
    let after_specs = gate(&peer, &root_str, &change, "approve").await;
    assert_eq!(after_specs["artifact"], "design");
    let after_design = gate(&peer, &root_str, &change, "approve").await;
    assert_eq!(after_design["artifact"], "tasks");
    let after_tasks = gate(&peer, &root_str, &change, "approve").await;
    assert_eq!(after_tasks["phase"], "completed", "{after_tasks:#}");

    // The artifacts and the persisted cycle state exist.
    let change_dir = root.join(".meltemi").join("changes").join(&change);
    assert!(change_dir.join("proposal.md").is_file());
    assert!(
        change_dir
            .join("specs")
            .join("example-capability")
            .join("spec.md")
            .is_file()
    );
    assert!(
        change_dir.join(".cycle-state.json").is_file(),
        "state persisted"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn an_invalid_delta_returns_without_opening_the_gate() {
    // Scenario: Inválido no llega al humano.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-sddbad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, ", '--sdd-author', '--sdd-invalid'");
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn("sdd-bad").await;
    let peer = client(&endpoint).await;
    let root_str = root.display().to_string();

    let started = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::SDD_PROPOSE,
            &json!({ "idea": "invalid specs change", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    let change = started["changeName"].as_str().unwrap().to_string();
    assert_eq!(started["phase"], "gate_pending", "proposal is prose, valid");

    // Approving the proposal advances to specs, which the mock writes invalid:
    // the engine returns diagnostics and the gate does NOT open.
    let after = gate(&peer, &root_str, &change, "approve").await;
    assert_eq!(
        after["phase"], "invalid",
        "invalid specs do not reach a gate: {after:#}"
    );
    assert!(
        !after["diagnostics"].as_array().unwrap().is_empty(),
        "diagnostics are returned to the agent"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn explore_leaves_the_tree_untouched() {
    // Scenario: Exploración inocua.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-sddexp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, "");
    std::fs::create_dir_all(root.join(".meltemi").join("rumbo")).unwrap();
    std::fs::write(root.join(".meltemi").join("constitution.md"), "# C\n").unwrap();
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let before = std::fs::read_to_string(root.join(".meltemi").join("constitution.md")).unwrap();

    let (endpoint, daemon) = spawn("sdd-exp").await;
    let peer = client(&endpoint).await;
    let root_str = root.display().to_string();

    let explored = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::SDD_EXPLORE,
            &json!({ "projectRoot": root_str, "topic": "how should we structure this?" }),
        ),
    )
    .await
    .expect("explore returned")
    .expect("explore ok");
    assert_eq!(explored["phase"], "explored");

    // The project tree is unchanged.
    let after = std::fs::read_to_string(root.join(".meltemi").join("constitution.md")).unwrap();
    assert_eq!(before, after, "explore must not modify the tree");
    assert!(
        !root.join(".meltemi").join("changes").exists() || {
            std::fs::read_dir(root.join(".meltemi").join("changes"))
                .map(|mut d| d.next().is_none())
                .unwrap_or(true)
        }
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
