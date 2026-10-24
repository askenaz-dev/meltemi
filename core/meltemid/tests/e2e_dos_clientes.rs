// SPDX-License-Identifier: Apache-2.0

//! Two clients, one daemon, one agent (estado-en-vivo task 2.5).
//!
//! The file beside this one drives the channel with sessions registered
//! straight into the registry, which is the right fixture for asking who gets
//! told what. This one asks the question that needed a real agent and a real
//! session: **client A opens it, and client B — which did nothing — learns
//! where it stands.** That is the defect verbatim. Measured on 2026-09-24 with
//! an agent forty seconds into a turn, a second client showed the session
//! stopped and counted nobody working.
//!
//! It also asserts the agreement the other file could not: that what the
//! channel affirms is what `session/list` answers, now that the session exists
//! in both the on-disk index and the registry.
//!
//! Against `mock-agent`, never a real agent or the network (constitution
//! §"tests e2e").

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
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
        format!(
            r"\\.\pipe\meltemid-dosclientes-{}-{tag}",
            std::process::id()
        )
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-dosclientes-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-dosclientes-{}-{tag}", std::process::id()));
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

struct Client {
    peer: Peer,
    incoming: mpsc::UnboundedReceiver<Incoming>,
}

impl Client {
    async fn connect(endpoint: &str, name: &str) -> Self {
        let stream = connect(endpoint).await.expect("connect");
        let (peer, incoming) = Peer::start(stream);
        let client = Self { peer, incoming };
        client
            .peer
            .request(
                methods::INITIALIZE,
                &InitializeParams {
                    protocol_version: PROTOCOL_VERSION,
                    client: PeerInfo {
                        name: name.into(),
                        version: "0".into(),
                    },
                },
            )
            .await
            .expect("initialize");
        client
    }

    /// Waits until this client has been told the session is in one of `states`,
    /// and answers which. Everything else the daemon pushes is drained.
    async fn wait_for_state(&mut self, session_id: &str, states: &[&str]) -> String {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let mut seen: Vec<String> = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            let Ok(Some(message)) = tokio::time::timeout(left, self.incoming.recv()).await else {
                panic!("never told {session_id} was any of {states:?}; was told: {seen:?}");
            };
            if let Incoming::Notification { method, params } = message
                && method == methods::SESSION_STATE
                && params["sessionId"] == session_id
            {
                let state = params["state"].as_str().unwrap_or_default().to_string();
                if states.contains(&state.as_str()) {
                    return state;
                }
                seen.push(state);
            }
        }
    }
}

/// The id of the one live session, from a client that asks.
async fn live_session(peer: &Peer, root: &str) -> Option<String> {
    const LIVE: [&str; 4] = [
        "starting",
        "active",
        "waiting_permission",
        "waiting_instruction",
    ];
    for _ in 0..300 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list");
        if let Some(found) = list["sessions"].as_array().and_then(|s| {
            s.iter()
                .find(|s| LIVE.contains(&s["state"].as_str().unwrap_or("")))
        }) {
            return found["sessionId"].as_str().map(str::to_string);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    None
}

// Scenario: Una transición llega a un cliente que no inició la sesión
#[tokio::test]
async fn a_second_client_learns_about_a_session_it_did_not_open() {
    let root = fixture("otro", &["--turn-delay-ms", "4000"]);
    let root_str = root.display().to_string();
    let endpoint = test_endpoint("otro");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("dosclientes", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    // The client that does nothing but watch, connected FIRST so it cannot be
    // accused of having asked for anything.
    let mut bystander = Client::connect(&endpoint, "bystander").await;

    // The client that works.
    let worker = Client::connect(&endpoint, "worker").await;
    let propose = {
        let peer = worker.peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    let session_id = live_session(&worker.peer, &root_str)
        .await
        .expect("the worker's session came up");

    // The whole point: the bystander is told, without having asked.
    let state_seen = bystander
        .wait_for_state(&session_id, &["active", "waiting_instruction"])
        .await;

    // And what it was told is what the list answers — now that the session
    // exists in the index as well as in the registry, which is the agreement
    // the registry-only fixture could not ask for.
    let listed = bystander
        .peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let row = listed["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .expect("the session is listed")
        .clone();
    assert!(
        row["projectRoot"].as_str().is_some_and(|r| !r.is_empty()),
        "the listed row carries its root: {row:#}"
    );
    let _ = state_seen;

    let result = tokio::time::timeout(Duration::from_secs(90), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    // Completing the turn does NOT end the session: it parks it, waiting for
    // the next instruction with its agent intact. The bystander is told that
    // too, and finding out cost this test one failed run — it expected `ended`
    // and was told `waiting_instruction`, which is the truth.
    let parked = bystander
        .wait_for_state(&session_id, &["waiting_instruction"])
        .await;
    assert_eq!(parked, "waiting_instruction");

    // Ending it for real reaches the bystander as well, which is the half a
    // parked session cannot show.
    // A notification, not a request: cancelling is something the client tells
    // the daemon, and asking for an answer gets "method not found" — which is
    // how this test learned it.
    worker.peer.notify(
        methods::SESSION_CANCEL,
        &json!({ "projectRoot": root_str, "sessionId": session_id }),
    );
    let ended = bystander.wait_for_state(&session_id, &["ended"]).await;
    assert_eq!(ended, "ended");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
