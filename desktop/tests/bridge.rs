// SPDX-License-Identifier: Apache-2.0

//! Bridge actor tests against an in-process daemon over the real transport
//! (same harness as the TUI e2e suite): the desktop surface must give the
//! same connection guarantees as the terminal one.

use serde_json::json;
use tokio::sync::{mpsc, oneshot};

use meltemi_client::transport::Listener;
use meltemi_desktop::bridge::{BridgeCommand, BridgeEvent, ConnState, bridge_actor};
use meltemid::server::{DaemonState, serve_until_shutdown};

/// A fresh, process-unique endpoint for an ephemeral test daemon.
fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemi-desktop-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemi-desktop-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// Spins an in-process daemon at a fresh endpoint.
async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind endpoint");
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn request(
    commands: &mpsc::UnboundedSender<BridgeCommand>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, meltemi_desktop::bridge::BridgeError> {
    let (reply, response) = oneshot::channel();
    commands
        .send(BridgeCommand::Request {
            method: method.into(),
            params,
            reply,
        })
        .expect("actor alive");
    response.await.expect("reply delivered")
}

// Scenario: Shell inmediato con conexión asíncrona
#[tokio::test]
async fn bridge_connects_initializes_and_serves_a_status_passthrough() {
    let (endpoint, _daemon) = spawn_daemon("bridge-status").await;
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut events) = mpsc::unbounded_channel();
    let actor = tokio::spawn(bridge_actor(endpoint, command_rx, event_tx));

    // The passthrough works end to end (connect → initialize → request).
    let status = request(&command_tx, "status", json!({}))
        .await
        .expect("status result");
    assert!(
        status.get("daemonVersion").is_some(),
        "status carries the daemon version: {status}"
    );

    // The state feed reported Connecting and then Connected with the version.
    let mut saw_connecting = false;
    let mut saw_connected = false;
    while let Ok(event) = events.try_recv() {
        if let BridgeEvent::State(state) = event {
            match state {
                ConnState::Connecting => saw_connecting = true,
                ConnState::Connected { version, .. } => {
                    assert!(!version.is_empty());
                    saw_connected = true;
                }
                ConnState::Unreachable { detail, .. } => {
                    panic!("unexpected unreachable: {detail}")
                }
            }
        }
    }
    assert!(saw_connecting && saw_connected);

    drop(command_tx);
    actor.await.expect("actor stops when the host drops");
}

#[tokio::test]
async fn a_contract_error_surfaces_kind_detail_and_remedy() {
    let (endpoint, _daemon) = spawn_daemon("bridge-error").await;
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, _events) = mpsc::unbounded_channel();
    let _actor = tokio::spawn(bridge_actor(endpoint, command_rx, event_tx));

    // `session/log` on a project without sessions refuses with structured
    // data; the bridge must not flatten it (the CLI's known gap).
    let error = request(&command_tx, "no/such-method", json!({}))
        .await
        .expect_err("unknown method refuses");
    assert_eq!(error.code, Some(-32601));
    assert_eq!(error.kind, "contract");
    assert!(error.message.contains("method not found"));
}

// Scenario: Fallo de arranque diagnosticado
#[tokio::test]
async fn requests_while_unreachable_fail_fast_with_the_endpoint_diagnosed() {
    // No daemon here, and none is started: the endpoint is a test's, not the
    // user's own (acceso-remoto-afinado D1) — and were a start asked for, the
    // launcher points at a binary that does not exist. The actor stays in
    // backoff either way.
    unsafe {
        std::env::set_var("MELTEMI_DAEMON_BIN", "nonexistent-meltemid-binary");
    }
    let endpoint = test_endpoint("bridge-unreachable");
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut events) = mpsc::unbounded_channel();
    let _actor = tokio::spawn(bridge_actor(endpoint.clone(), command_rx, event_tx));

    let error = request(&command_tx, "status", json!({}))
        .await
        .expect_err("unreachable fails fast");
    assert_eq!(error.kind, "unreachable");
    assert!(error.code.is_none());

    // The state feed carries the endpoint so the surface can show the socket
    // path with the diagnosis.
    let state = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(BridgeEvent::State(ConnState::Unreachable {
                endpoint: reported, ..
            })) = events.recv().await
            {
                return reported;
            }
        }
    })
    .await
    .expect("unreachable state reported");
    assert_eq!(state, endpoint);
}

// Scenario: Cambiar de máquina reconecta y lo dice
#[tokio::test]
async fn switching_machines_reconnects_against_the_new_one_without_restarting() {
    // Two daemons standing in for two machines. What is under test is that one
    // client process, without restarting, stops talking to the first and starts
    // talking to the second — and that the state feed says which endpoint it is
    // on, because a surface that did not say would make an action on the wrong
    // machine look like an action on your own.
    let (first, _one) = spawn_daemon("switch-first").await;
    let (second, _two) = spawn_daemon("switch-second").await;
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut events) = mpsc::unbounded_channel();
    let _actor = tokio::spawn(bridge_actor(first.clone(), command_rx, event_tx));

    // Talking to the first.
    let status = request(&command_tx, "status", json!({}))
        .await
        .expect("the first machine answers");
    assert!(status.get("daemonVersion").is_some(), "{status}");

    command_tx
        .send(BridgeCommand::Switch {
            endpoint: second.clone(),
        })
        .expect("actor alive");

    // And now the second, through the same process and the same channel.
    let status = request(&command_tx, "status", json!({}))
        .await
        .expect("the second machine answers");
    assert!(status.get("daemonVersion").is_some(), "{status}");

    // The feed named the endpoint it ended up on. Collected with a deadline
    // because the reconnection is asynchronous by nature.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut connected_to_second = false;
    while tokio::time::Instant::now() < deadline && !connected_to_second {
        match tokio::time::timeout(std::time::Duration::from_millis(250), events.recv()).await {
            Ok(Some(BridgeEvent::State(ConnState::Connected { endpoint, .. }))) => {
                connected_to_second = endpoint == second;
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => {}
        }
    }
    assert!(
        connected_to_second,
        "the state feed says which machine it is on"
    );
}

#[tokio::test]
async fn a_machine_chosen_while_unreachable_is_dialed_without_waiting_out_the_backoff() {
    // The case a user actually hits: the current machine is down, the client is
    // in its retry backoff, and they pick another one. Sitting through the
    // remains of the previous machine's bad luck would be the client punishing
    // them for its own schedule.
    #[cfg(windows)]
    let dead = format!(r"\\.\pipe\meltemi-desktop-dead-{}", std::process::id());
    #[cfg(unix)]
    let dead = std::env::temp_dir()
        .join(format!("meltemi-desktop-dead-{}.sock", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let (alive, _daemon) = spawn_daemon("switch-from-dead").await;

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, _events) = mpsc::unbounded_channel();
    let _actor = tokio::spawn(bridge_actor(dead, command_rx, event_tx));

    // Let it fail at least once and settle into the wait.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    command_tx
        .send(BridgeCommand::Switch {
            endpoint: alive.clone(),
        })
        .expect("actor alive");

    let answered = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        request(&command_tx, "status", json!({})),
    )
    .await
    .expect("the switch is not held behind the old machine's backoff");
    assert!(answered.is_ok(), "{answered:?}");
}
