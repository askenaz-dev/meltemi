// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of session history and resume (sesiones-reanudables task
//! 5.2), driving an ephemeral daemon (with a controlled data directory) and
//! the scripted `mock-agent`:
//!
//! - a session log with no recorded end lists as `interrupted`, never active;
//! - a completed session lists as historical and its log pages by line range;
//! - a session from a load-announcing agent is `resumable`; one from an agent
//!   without the capability is honestly not resumable.
//!
//! Runs against temporary fixtures, never this repo (constitution
//! §"tests e2e").

use std::path::{Path, PathBuf};

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

fn init_params() -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: "e2e-sessions-client".into(),
            version: "0.0.0".into(),
        },
    }
}

/// A fixture project pointing the agent at the mock, with a bare allow rule so
/// the mock's write proceeds without a client approver. `load` announces the
/// session-load capability.
fn fixture(root: &Path, load: bool) {
    let mock = mock_agent_bin();
    assert!(mock.exists(), "run `cargo test` at the workspace root");
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    let command = if load {
        format!("['{}', '--load-session']", mock.display())
    } else {
        format!("['{}']", mock.display())
    };
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = {command}\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
}

/// Spins a daemon with a controlled data directory (so a "crashed" log can be
/// pre-written) and returns the endpoint, data dir, and serve handle.
async fn spawn_daemon(tag: &str) -> (String, PathBuf, tokio::task::JoinHandle<()>) {
    let base =
        std::env::temp_dir().join(format!("meltemi-e2e-sessions-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data_dir = base.join("data");
    let config_dir = base.join("config");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();

    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(data_dir.clone(), config_dir, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, data_dir, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    // Drain incoming so the connection stays alive (dropping the receiver would
    // tear it down). A bare allow rule means no permission request is pushed.
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(methods::INITIALIZE, &init_params())
        .await
        .expect("initialize");
    peer
}

fn find<'a>(result: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    result["sessions"]
        .as_array()
        .expect("sessions array")
        .iter()
        .find(|s| s["sessionId"] == id)
        .unwrap_or_else(|| panic!("session `{id}` missing in {result:#}"))
}

#[tokio::test]
async fn a_crashed_session_lists_as_interrupted() {
    // Scenario: La caída no deja fantasmas. A session log with a start but no
    // end resolves to interrupted, never active.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let (endpoint, data_dir, daemon) = spawn_daemon("crash").await;

    // Pre-write a started-only log into the daemon's data dir (a crash left it
    // without an end), no index — the daemon rebuilds from the log.
    let key = meltemid::paths::project_key(&root);
    let sessions = data_dir.join("projects").join(&key).join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let started = format!(
        r#"{{"v":1,"ts":"2026-07-11T10:00:00Z","type":"session_started","payload":{{"sessionId":"crashed-1","agentCommand":["mock-agent"],"projectRoot":"{}"}}}}"#,
        root.display().to_string().replace('\\', "\\\\")
    );
    std::fs::write(sessions.join("crashed-1.jsonl"), format!("{started}\n")).unwrap();

    let peer = init_client(&endpoint).await;
    let result = peer
        .request(
            methods::SESSION_LIST,
            &json!({ "projectRoot": root.display().to_string() }),
        )
        .await
        .expect("session/list ok");

    let crashed = find(&result, "crashed-1");
    assert_eq!(crashed["state"], "interrupted", "got: {result:#}");
    assert_eq!(crashed["resumable"], false);
    assert!(
        result["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["state"] != "active"),
        "no crashed session appears active"
    );

    // State filter narrows to interrupted only.
    let filtered = peer
        .request(
            methods::SESSION_LIST,
            &json!({ "projectRoot": root.display().to_string(), "state": "interrupted" }),
        )
        .await
        .expect("filtered ok");
    assert_eq!(filtered["sessions"].as_array().unwrap().len(), 1);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_completed_session_is_historical_resumable_and_pageable() {
    // Scenario: Históricas listadas
    // Scenario: Transcript paginado
    // Scenario: Reanudar con capacidad anunciada
    let root = std::env::temp_dir().join(format!("meltemi-e2e-hist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, true); // the mock announces session load
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let (endpoint, _data_dir, daemon) = spawn_daemon("hist").await;
    let peer = init_client(&endpoint).await;
    let root_str = root.display().to_string();

    // Run a propose so a real session log and index record exist.
    let proposed = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "history session", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(proposed["status"], "completed", "{proposed:#}");
    let change = proposed["changeName"].as_str().unwrap().to_string();

    // The session is listed as historical (ended) and resumable (the agent
    // announced load and its session id was persisted).
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1, "one historical session: {list:#}");
    let session = &sessions[0];
    assert_eq!(session["state"], "ended");
    assert_eq!(session["resumable"], true, "load-capable → resumable");
    let session_id = session["sessionId"].as_str().unwrap().to_string();

    // session/log pages by line range and reports the total.
    let full = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str, "sessionId": session_id }),
        )
        .await
        .expect("session/log ok");
    let total = full["total"].as_u64().unwrap();
    assert!(total >= 3, "a real turn logs several events: {full:#}");
    assert_eq!(full["lines"].as_array().unwrap().len() as u64, total);

    // The last page (offset near the end) returns just the tail.
    let tail = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str, "sessionId": session_id, "offset": total - 1, "limit": 10 }),
        )
        .await
        .expect("tail ok");
    assert_eq!(tail["offset"], total - 1);
    assert_eq!(tail["lines"].as_array().unwrap().len(), 1, "just the tail");
    assert_eq!(tail["total"], total);

    // The proposal really was written by the (rule-allowed) mock turn.
    let proposal = root
        .join(".meltemi")
        .join("changes")
        .join(&change)
        .join("proposal.md");
    assert!(
        std::fs::read_to_string(&proposal)
            .map(|c| c.contains("mock-agent"))
            .unwrap_or(false),
        "the agent wrote the proposal"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_session_without_load_capability_is_not_resumable() {
    // Scenario: Sin capacidad, honestidad. The mock without --load-session does
    // not announce load, so its session is inspectable but not resumable.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-noload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, false);
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let (endpoint, _data_dir, daemon) = spawn_daemon("noload").await;
    let peer = init_client(&endpoint).await;
    let root_str = root.display().to_string();

    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "no load session", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");

    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let session = &list["sessions"].as_array().unwrap()[0];
    assert_eq!(session["state"], "ended");
    assert_eq!(
        session["resumable"], false,
        "an agent without load support is not resumable: {list:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
