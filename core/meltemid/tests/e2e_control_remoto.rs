// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of directing an existing session (control-remoto-asistido),
//! driving an ephemeral daemon and the scripted `mock-agent` against temporary
//! fixtures:
//!
//! - directing an ACTIVE session queues the instruction and dispatches it as the
//!   next turn of the same agent session, without interrupting the turn in
//!   progress, auditing both the queueing and the dispatch in the JSONL;
//! - directing a terminated-but-RESUMABLE session resumes it, linked to the
//!   original;
//! - a non-existent or non-resumable session refuses with 2004;
//! - cancelling a session with a non-empty queue leaves consistent state: the
//!   queued instruction is recorded but never dispatched.
//!
//! Runs against temporary fixtures, never this repo, and the CI `mock-agent`,
//! never a real agent or the network (constitution §"tests e2e").

use std::path::PathBuf;
use std::time::Duration;

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
        format!(r"\\.\pipe\meltemid-e2e-cr-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-cr-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo whose config points the agent at the mock with `mock_args`,
/// and whose permissions allow writes so a turn runs without client escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-cr-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let mock = mock_agent_bin().display().to_string().replace('\\', "/");
    let mut command = format!("'{mock}'");
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
    let state = DaemonState::for_test(&format!("cr-{tag}"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    // Permissions are resolved by the allow rule, so no permission request ever
    // reaches the client; drain any daemon-initiated traffic anyway.
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-cr-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Polls `session/list` until an active session for `root` appears, returning
/// its id. Fails the test if none appears within a few seconds.
async fn wait_for_active_session(peer: &Peer, root: &str) -> String {
    for _ in 0..200 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list");
        if let Some(sessions) = list["sessions"].as_array()
            && let Some(active) = sessions.iter().find(|s| s["state"] == "active")
        {
            return active["sessionId"].as_str().unwrap().to_string();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no active session appeared for {root}");
}

/// The event `type`s of a session's JSONL log, in order.
async fn log_event_types(peer: &Peer, root: &str, session_id: &str) -> Vec<String> {
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root, "sessionId": session_id }),
        )
        .await
        .expect("session/log");
    log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l.as_str().unwrap()).ok())
        .map(|e| e["type"].as_str().unwrap_or("").to_string())
        .collect()
}

#[tokio::test]
async fn directing_an_active_session_dispatches_as_the_next_turn() {
    // Scenario: Instrucción a una sesión activa se despacha como siguiente turno
    let root = fixture("active", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("active").await;
    let peer = init_client(&endpoint).await;

    // Start a proposal in the background; its first turn holds open ~3s.
    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    // While it runs, direct an instruction: it queues as the next turn.
    let session_id = wait_for_active_session(&peer, &root_str).await;
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "also add a light theme",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued", "{directed:#}");
    assert_eq!(directed["queuePosition"], 1);
    assert_eq!(directed["sessionId"], session_id);

    // The proposal turn ran, then the directed instruction ran as a second turn.
    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    let types = log_event_types(&peer, &root_str, &session_id).await;
    let prompts = types.iter().filter(|t| *t == "prompt_sent").count();
    assert_eq!(
        prompts, 2,
        "initial prompt + the directed instruction: {types:?}"
    );
    let queued_at = types.iter().position(|t| t == "instruction_queued");
    let second_prompt_at = types
        .iter()
        .enumerate()
        .filter(|(_, t)| *t == "prompt_sent")
        .nth(1)
        .map(|(i, _)| i);
    assert!(
        queued_at.is_some(),
        "the instruction was recorded: {types:?}"
    );
    assert!(
        queued_at < second_prompt_at,
        "queued before it was dispatched (log-before-enqueue): {types:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn directing_a_resumable_session_resumes_it_linked() {
    // Scenario: Instrucción a una sesión reanudable la reanuda
    let root = fixture("resume", &["--load-session"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("resume").await;
    let peer = init_client(&endpoint).await;

    // Run a proposal to completion; the load-capable mock leaves it resumable.
    peer.request(
        methods::PROPOSE,
        &json!({ "idea": "add dark mode", "projectRoot": root_str }),
    )
    .await
    .expect("propose ok");

    // Find the ended, resumable session.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let original = list["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["resumable"] == true)
        .expect("a resumable session");
    let original_id = original["sessionId"].as_str().unwrap().to_string();

    // Directing it resumes as a NEW session linked to the original.
    let resumed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": original_id,
                "instruction": "now add a light theme too",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(resumed["disposition"], "resumed", "{resumed:#}");
    assert_eq!(resumed["resumedFrom"], original_id);
    assert_eq!(resumed["status"], "completed");
    assert_ne!(
        resumed["sessionId"], original_id,
        "resume mints a new session"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_non_directable_session_refuses_with_a_remedy() {
    // Scenario: Sesión no dirigible rehúsa con remedio
    let root = fixture("refuse", &[]); // no --load-session: not resumable
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("refuse").await;
    let peer = init_client(&endpoint).await;

    // An unknown session id refuses with SESSION_NOT_FOUND (2004).
    let unknown = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": "no-such-session",
                "instruction": "do something",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect_err("unknown session refuses");
    assert_eq!(unknown.code, 2004, "{unknown}");

    // A finished, NON-resumable session (the mock did not announce load) also
    // refuses — it cannot be resumed.
    peer.request(
        methods::PROPOSE,
        &json!({ "idea": "add dark mode", "projectRoot": root_str }),
    )
    .await
    .expect("propose ok");
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let ended = list["sessions"].as_array().unwrap()[0]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let refused = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": ended,
                "instruction": "resume me",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect_err("a non-resumable ended session refuses");
    assert_eq!(refused.code, 2004, "{refused}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn cancelling_with_a_queued_instruction_leaves_it_undispatched() {
    // Scenario: Dirigir no interrumpe ni cancela
    let root = fixture("cancel", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cancel").await;
    let peer = init_client(&endpoint).await;

    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    // Queue an instruction during the active turn, then cancel the session.
    let session_id = wait_for_active_session(&peer, &root_str).await;
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "this must never run",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued");
    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));

    // The proposal returns; the queued instruction was recorded but never
    // dispatched — a cancel stops further turns (D2).
    let _ = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join");

    let types = log_event_types(&peer, &root_str, &session_id).await;
    assert!(
        types.iter().any(|t| t == "instruction_queued"),
        "the instruction is recorded (visible loss, not a silent one): {types:?}"
    );
    let prompts = types.iter().filter(|t| *t == "prompt_sent").count();
    assert_eq!(
        prompts, 1,
        "only the initial prompt ran; the queued instruction was not dispatched: {types:?}"
    );
    // The session is finalized (deregistered): no longer active.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let still_active = list["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["sessionId"] == session_id && s["state"] == "active");
    assert!(!still_active, "the session is no longer active");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
