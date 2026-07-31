// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the free session (lanzador-conversacional), driving an
//! ephemeral daemon and the scripted `mock-agent` against temporary fixtures —
//! never this repo, never a real agent, never the network (constitution
//! §"tests e2e").
//!
//! The free session is the door the method's verbs never had: an instruction on
//! a project root, with no change, no task and no gate. What these tests hold it
//! to is that nothing about the government was traded away for that.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
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
        format!(r"\\.\pipe\meltemid-e2e-libre-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-libre-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo whose config points the agent at the mock with `mock_args`,
/// and whose permissions allow writes so a turn runs without escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-libre-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let mock = mock_agent_bin();
    assert!(mock.exists(), "run `cargo test` at the workspace root");
    let mut command = format!("'{}'", mock.display().to_string().replace('\\', "/"));
    for arg in mock_args {
        command.push_str(&format!(", '{arg}'"));
    }
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = [{command}]\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("libre-{tag}"), shutdown_tx);
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
                name: "e2e-libre-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

// Scenario: Sesión libre completada no lista como interrumpida
#[tokio::test]
async fn a_completed_free_session_is_ended_not_interrupted() {
    // The finalizer is the whole point of this test. Skipping it is the defect
    // `gui-acabado-y-cierre-sdd` D3 fixed: the index never received `ended_at`,
    // so a session that finished perfectly listed as interrupted forever.
    let root = fixture("finalize", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("finalize").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "find out why the build is slow" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");
    let session_id = started["sessionId"].as_str().expect("a session id");

    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().expect("sessions");
    let session = sessions
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .unwrap_or_else(|| panic!("the free session is missing from {list:#}"));
    assert_eq!(
        session["state"], "ended",
        "a completed free session ended; it did not crash: {list:#}"
    );
    assert!(
        sessions.iter().all(|s| s["state"] != "interrupted"),
        "nothing here was interrupted: {list:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
