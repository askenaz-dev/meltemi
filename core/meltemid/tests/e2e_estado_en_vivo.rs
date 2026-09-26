// SPDX-License-Identifier: Apache-2.0

//! What a connection is told about where sessions stand (estado-en-vivo).
//!
//! These drive a real daemon over a real transport and register sessions
//! directly in the registry, without an agent. That is deliberate: what is under
//! test is the **channel and the snapshot**, not a session's lifecycle, and an
//! agent would add a subprocess, a handshake and a turn to a test whose subject
//! is which connections get told what. The end-to-end case with a real agent —
//! one client opening a session and another client learning about it — lives
//! beside this file with `mock-agent`.

use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, SessionState, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-estado-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-estado-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A connected client, plus the stream of everything the daemon pushed to it.
struct Client {
    peer: Peer,
    incoming: mpsc::UnboundedReceiver<Incoming>,
}

impl Client {
    async fn connect(endpoint: &str) -> Self {
        let stream = connect(endpoint).await.expect("connect");
        let (peer, incoming) = Peer::start(stream);
        Self { peer, incoming }
    }

    async fn initialize(&self) {
        self.peer
            .request(
                methods::INITIALIZE,
                &InitializeParams {
                    protocol_version: PROTOCOL_VERSION,
                    client: PeerInfo {
                        name: "e2e-estado".into(),
                        version: "0".into(),
                    },
                },
            )
            .await
            .expect("initialize");
    }

    /// Collects `session/state` notifications until `want` of them have arrived,
    /// ignoring anything else the daemon pushes.
    async fn affirmations(&mut self, want: usize) -> Vec<Value> {
        let mut seen = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while seen.len() < want {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            let Ok(Some(message)) = tokio::time::timeout(left, self.incoming.recv()).await else {
                panic!(
                    "expected {want} affirmations, saw {}: {seen:#?}",
                    seen.len()
                );
            };
            if let Incoming::Notification { method, params } = message
                && method == methods::SESSION_STATE
            {
                seen.push(params);
            }
        }
        seen
    }
}

fn state_of(affirmations: &[Value], session_id: &str) -> Option<String> {
    affirmations
        .iter()
        .rfind(|a| a["sessionId"] == session_id)
        .and_then(|a| a["state"].as_str().map(str::to_string))
}

// Scenario: Una conexión nueva recibe el estado actual antes de cualquier transición
#[tokio::test]
async fn a_new_connection_learns_where_every_session_stands_before_any_transition() {
    let endpoint = test_endpoint("snapshot");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("estado-snapshot", shutdown_tx);

    // Two sessions already under way, in different states and different
    // projects, BEFORE anyone connects. This is the case the defect was about:
    // a client that arrives late and shows nothing.
    state
        .sessions
        .register("s-1", vec!["mock".into()], "/repo-a".into())
        .await;
    state.sessions.set_state("s-1", SessionState::Active).await;
    state
        .sessions
        .register("s-2", vec!["mock".into()], "/repo-b".into())
        .await;
    state.sessions.begin_waiting("s-2").await;

    let daemon = tokio::spawn(serve_until_shutdown(listener, state.clone(), shutdown_rx));
    let mut client = Client::connect(&endpoint).await;
    client.initialize().await;

    let snapshot = client.affirmations(2).await;
    assert_eq!(
        state_of(&snapshot, "s-1").as_deref(),
        Some("active"),
        "{snapshot:#?}"
    );
    assert_eq!(
        state_of(&snapshot, "s-2").as_deref(),
        Some("waiting_permission"),
        "{snapshot:#?}"
    );

    // The root travels with it, because a surface narrows by project without
    // asking again.
    let first = snapshot
        .iter()
        .find(|a| a["sessionId"] == "s-1")
        .expect("s-1 was affirmed");
    assert_eq!(first["projectRoot"], "/repo-a", "{first:#}");
    assert!(
        first["since"].as_str().is_some_and(|s| s.ends_with('Z')),
        "the instant is a UTC timestamp: {first:#}"
    );
    assert_eq!(first["pendingPermissions"], 0, "{first:#}");

    // And the transitions that follow the snapshot arrive as they happen.
    state.sessions.set_state("s-1", SessionState::Ended).await;
    let after = client.affirmations(1).await;
    assert_eq!(state_of(&after, "s-1").as_deref(), Some("ended"));

    daemon.abort();
}

#[tokio::test]
async fn a_connection_that_never_initialized_is_told_nothing() {
    // Deny-by-default reaches this too: a connection that has not completed the
    // handshake is not a client, and the daemon owes it no state.
    let endpoint = test_endpoint("uninitialized");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("estado-uninit", shutdown_tx);
    state
        .sessions
        .register("s-1", vec!["mock".into()], "/repo".into())
        .await;

    let daemon = tokio::spawn(serve_until_shutdown(listener, state.clone(), shutdown_rx));
    let mut client = Client::connect(&endpoint).await;
    // No `initialize`. Move a session and give the daemon time to push.
    state.sessions.set_state("s-1", SessionState::Active).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        client.incoming.try_recv().is_err(),
        "nothing is pushed to a connection that has not initialized"
    );

    daemon.abort();
}

// Scenario: La notificación no lleva contenido del proyecto
#[tokio::test]
async fn an_affirmation_carries_five_fields_and_no_sixth() {
    // The privacy promise, read off the wire rather than trusted: whatever the
    // daemon knows about a session, this notification says only where it
    // stands.
    let endpoint = test_endpoint("fields");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("estado-fields", shutdown_tx);
    state
        .sessions
        .register(
            "s-1",
            vec!["mock".into(), "--secret-flag".into()],
            "/repo".into(),
        )
        .await;

    let daemon = tokio::spawn(serve_until_shutdown(listener, state.clone(), shutdown_rx));
    let mut client = Client::connect(&endpoint).await;
    client.initialize().await;
    let snapshot = client.affirmations(1).await;
    let affirmation = snapshot[0].as_object().expect("an object");

    let mut keys: Vec<&str> = affirmation.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "pendingPermissions",
            "projectRoot",
            "sessionId",
            "since",
            "state"
        ],
        "five fields, and the agent's command line is not one of them: {affirmation:#?}"
    );
    assert!(
        !serde_json::to_string(affirmation)
            .expect("serializes")
            .contains("--secret-flag"),
        "nothing of how the agent was launched travels here"
    );

    daemon.abort();
}

// Why the agreement between the channel and `session/list` is NOT asserted
// here: `session/list` builds its rows from the session index on disk and
// borrows only the live state from the registry, so a fixture that registers
// straight into the registry — as every test above does, on purpose — has no
// row to be found in. Measured, not assumed: the assertion failed on "the
// session is listed", not on a disagreement. The agreement is asserted where
// the two can actually diverge — the registry's own state against the
// affirmation it publishes, in `session.rs` — and end to end against a real
// agent, where both the index and the registry hold the session.
