// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of what the session stream carries (lanzador-conversacional
//! D3), driving an ephemeral daemon and the scripted `mock-agent` against
//! temporary fixtures — never this repo, never a real agent, never the network.
//!
//! The subscription and the notification are old; what changed is the content.
//! A live client used to see the agent's prose and nothing else: the prompt that
//! opens a turn and the completion that closes it went only to the log, so no
//! client could delimit a turn without re-reading the file it was already
//! streaming.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::{Incoming, Peer};
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
        format!(r"\\.\pipe\meltemid-e2e-stream-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-stream-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo pointing the agent at the mock, with `permissions` when the
/// test wants a posture (absent means every request escalates).
fn fixture(tag: &str, mock_args: &[&str], permissions: Option<&str>) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-stream-{}-{tag}", std::process::id()));
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
    if let Some(rules) = permissions {
        std::fs::write(root.join(".meltemi").join("permissions.toml"), rules).unwrap();
    }
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("stream-{tag}"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> (Peer, mpsc::UnboundedReceiver<Incoming>) {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-stream-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// Drains a connection's inbound traffic into two channels: session events, and
/// the permission requests the daemon pushes for a decision.
fn collect(
    mut incoming: mpsc::UnboundedReceiver<Incoming>,
) -> (
    mpsc::UnboundedReceiver<Value>,
    mpsc::UnboundedReceiver<Value>,
    tokio::task::JoinHandle<()>,
) {
    let (events_tx, events_rx) = mpsc::unbounded_channel();
    let (asks_tx, asks_rx) = mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        while let Some(message) = incoming.recv().await {
            match message {
                Incoming::Notification { method, params } if method == methods::SESSION_EVENT => {
                    let _ = events_tx.send(params);
                }
                Incoming::Request { method, params, .. }
                    if method == methods::PERMISSION_REQUEST =>
                {
                    let _ = asks_tx.send(params);
                }
                _ => {}
            }
        }
    });
    (events_rx, asks_rx, handle)
}

/// The session's log as decoded events, which is the truth the stream reads.
async fn log_events(peer: &Peer, root: &str, session_id: &str) -> Vec<Value> {
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
        .collect()
}

// Scenario: El cliente en vivo ve abrir y cerrar el turno
// Scenario: Cada evento una sola vez por conexión
#[tokio::test]
async fn the_stream_carries_the_whole_log_once_even_to_a_watcher_of_its_own_session() {
    // A slow turn so the connection can start watching a session it already
    // owns while that session is still running — the overlap where a second
    // delivery path would show up as a duplicate.
    let root = fixture(
        "whole",
        &["--turn-delay-ms", "1200"],
        Some("[[rule]]\neffect = \"allow\"\n"),
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("whole").await;
    let (peer, incoming) = init_client(&endpoint).await;
    let (mut events, _asks, drain) = collect(incoming);

    let start = tokio::spawn({
        let peer = peer.clone();
        let root = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "look around and report" }),
            )
            .await
        }
    });

    // The identity arrives on the stream before the agent's first token: this is
    // what lets a surface navigate into the conversation instead of waiting out
    // the turn, and the connection declared no interest to get it.
    let first = tokio::time::timeout(Duration::from_secs(10), events.recv())
        .await
        .expect("an event arrives before the turn ends")
        .expect("the stream is open");
    assert_eq!(
        first["event"]["type"], "session_started",
        "the first thing a client hears is which session it started: {first:#}"
    );
    let session_id = first["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();

    // Now watch the very session this connection started. Watching one's own
    // session must not double it: the hub has one delivery path, not two.
    peer.request(
        methods::SESSION_WATCH,
        &json!({ "sessionId": session_id, "watch": true }),
    )
    .await
    .expect("session/watch ok");

    let started = tokio::time::timeout(Duration::from_secs(30), start)
        .await
        .expect("session/start returned")
        .expect("join")
        .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");

    // The log is the truth; the stream is a reading of it. Wait for the tail —
    // the response and the last notifications race on their way out — then
    // compare the two, event for event.
    let persisted = log_events(&peer, &root_str, &session_id).await;
    let mut received = vec![first];
    for _ in 0..200 {
        while let Ok(event) = events.try_recv() {
            received.push(event);
        }
        if received.len() >= persisted.len() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let streamed: Vec<Value> = received
        .iter()
        .filter(|e| e["sessionId"] == session_id.as_str())
        .map(|e| e["event"].clone())
        .collect();
    assert_eq!(
        streamed, persisted,
        "the stream delivered exactly the log, in order and once each"
    );

    let types: Vec<&str> = streamed.iter().filter_map(|e| e["type"].as_str()).collect();
    assert!(
        types.contains(&"prompt_sent") && types.contains(&"turn_completed"),
        "a live client can delimit the turn without re-reading the log: {types:?}"
    );

    drain.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: El permiso sigue decidiéndose por su propio camino
#[tokio::test]
async fn the_permission_still_arrives_decidable_and_the_event_is_only_its_trace() {
    // No rules: the request escalates to the human, which is the only case where
    // the difference between a decidable push and an audit trail matters.
    let root = fixture("permission", &[], None);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("permission").await;
    let (peer, incoming) = init_client(&endpoint).await;
    let (mut events, mut asks, drain) = collect(incoming);

    let start = tokio::spawn({
        let peer = peer.clone();
        let root = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "try to write something" }),
            )
            .await
        }
    });

    // The decidable request comes by its own path, with its options — a stream
    // event carries none and could never be answered.
    let ask = tokio::time::timeout(Duration::from_secs(20), asks.recv())
        .await
        .expect("the permission is pushed for a decision")
        .expect("the connection is open");
    assert!(
        ask["options"]
            .as_array()
            .is_some_and(|options| options.iter().any(|o| o["kind"] == "allow_once")),
        "the push carries the options that make it decidable: {ask:#}"
    );

    // Decided through the queue the proxy owns, by id — the same path as before
    // this change, untouched by anything the stream now carries.
    let entry = peer
        .request(methods::PERMISSION_PENDING, &json!({}))
        .await
        .expect("pending ok")["pending"]
        .as_array()
        .and_then(|entries| entries.first().cloned())
        .expect("the escalated request is in the tray");
    let request_id = entry["requestId"]
        .as_str()
        .expect("a request id")
        .to_string();
    let allow = entry["options"]
        .as_array()
        .expect("options")
        .iter()
        .find(|option| option["kind"] == "allow_once")
        .expect("an allow option")["optionId"]
        .as_str()
        .expect("an option id")
        .to_string();

    peer.request(
        methods::PERMISSION_DECIDE,
        &json!({ "requestId": request_id, "optionId": allow }),
    )
    .await
    .expect("decide ok");

    let started = tokio::time::timeout(Duration::from_secs(30), start)
        .await
        .expect("session/start returned")
        .expect("join")
        .expect("session/start ok");
    assert_eq!(started["deniedPermissions"], 0, "{started:#}");

    // And the same request is on the stream as an audit trace, request and
    // decision both — the conversation is another view of the queue, never
    // another queue.
    let mut types = Vec::new();
    for _ in 0..200 {
        while let Ok(event) = events.try_recv() {
            if let Some(kind) = event["event"]["type"].as_str() {
                types.push(kind.to_string());
            }
        }
        if types.iter().any(|t| t == "permission_decided") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        types.iter().any(|t| t == "permission_requested"),
        "the request is on the stream: {types:?}"
    );
    assert!(
        types.iter().any(|t| t == "permission_decided"),
        "so is how it was resolved: {types:?}"
    );

    drain.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
