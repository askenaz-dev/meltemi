// SPDX-License-Identifier: Apache-2.0

//! Starting the daemon on demand, with the real `meltemid` binary
//! (acceso-remoto-afinado): a client starts it at an explicit endpoint only when
//! asked, reuses the running instance, and — unasked — refuses an explicit
//! endpoint that does not answer even with the binary at hand.
//!
//! One test run in phases, because it sets process-global environment
//! variables that the daemon it starts inherits: being the only test here
//! keeps those mutations free of concurrent readers. Every location is a
//! temporary one; the user's own endpoint and data are never touched.

use std::time::{Duration, Instant};

use meltemi_client::bootstrap::{ENV_DAEMON_BIN, ENV_START_HERE, connect_or_start};
use meltemi_client::paths::{ENV_CONFIG_DIR, ENV_DATA_DIR, ENV_ENDPOINT};
use meltemi_client::rpc::Peer;
use meltemi_client::transport;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

fn test_endpoint() -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-arranque-{}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-arranque-{}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// Sets a variable for this process and the daemons it starts.
fn set(name: &str, value: &str) {
    // SAFETY: this test binary runs a single test, one phase after another:
    // nothing reads the environment concurrently.
    unsafe { std::env::set_var(name, value) }
}

fn unset(name: &str) {
    // SAFETY: as in `set`.
    unsafe { std::env::remove_var(name) }
}

async fn initialized(stream: meltemi_client::transport::Stream) -> Peer {
    let (peer, incoming) = Peer::start(stream);
    // The receiver is held by a reader so the connection stays up.
    tokio::spawn(async move {
        let mut incoming = incoming;
        while incoming.recv().await.is_some() {}
    });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "arranque-e2e".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// What each phase observed, asserted only after the daemon is gone: a
/// failing assertion must never leave a detached daemon holding the binary.
#[derive(Debug, Default)]
struct Seen {
    refused_unasked: Option<String>,
    refused_at_once: bool,
    nothing_appeared: bool,
    started_when_asked: bool,
    answered_status: bool,
    reused: bool,
    stopped: bool,
}

// Scenario: Un endpoint ajeno que no responde no arranca nada
// Scenario: Arrancar en un endpoint explícito se pide
// Scenario: Primer cliente arranca el daemon
// Scenario: Segunda invocación reutiliza la instancia
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_daemon_starts_where_it_is_its_own_or_asked_and_is_reused() {
    let endpoint = test_endpoint();
    let base = std::env::temp_dir().join(format!("meltemid-arranque-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // The daemon started below inherits all of this: its endpoint, its data and
    // its configuration are the test's, never the user's.
    set(ENV_ENDPOINT, &endpoint);
    set(ENV_DATA_DIR, &base.join("data").display().to_string());
    set(ENV_CONFIG_DIR, &base.join("config").display().to_string());
    set(ENV_DAEMON_BIN, env!("CARGO_BIN_EXE_meltemid"));
    unset(ENV_START_HERE);
    let mut seen = Seen::default();

    // Unasked, at an endpoint that is not the user's own: refused at once,
    // although the real binary is right there to be started.
    let started = Instant::now();
    match connect_or_start(&endpoint).await {
        Ok(_) => {}
        Err(error) => seen.refused_unasked = Some(error.to_string()),
    }
    seen.refused_at_once = started.elapsed() < Duration::from_secs(2);
    tokio::time::sleep(Duration::from_millis(500)).await;
    seen.nothing_appeared = transport::connect(&endpoint).await.is_err();

    // Asked: the client starts the daemon there and connects to it.
    set(ENV_START_HERE, "1");
    if let Ok(stream) = connect_or_start(&endpoint).await {
        seen.started_when_asked = true;
        let peer = initialized(stream).await;
        seen.answered_status = peer
            .request(methods::STATUS, &serde_json::json!({}))
            .await
            .is_ok();

        // A second client reuses the running instance. With the launcher
        // pointed at a binary that does not exist, a second start would fail:
        // connecting proves none was attempted.
        set(ENV_DAEMON_BIN, "meltemid-no-such-binary");
        if let Ok(again) = connect_or_start(&endpoint).await {
            let second = initialized(again).await;
            seen.reused = second
                .request(methods::STATUS, &serde_json::json!({}))
                .await
                .is_ok();
            second.close();
        }

        // Orderly shutdown, then wait for the endpoint to go quiet.
        let _ = peer
            .request(methods::SHUTDOWN, &serde_json::json!({}))
            .await;
        peer.close();
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if transport::connect(&endpoint).await.is_err() {
                seen.stopped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    unset(ENV_START_HERE);

    let refused = seen
        .refused_unasked
        .as_deref()
        .unwrap_or_else(|| panic!("an explicit endpoint must be refused unasked: {seen:?}"));
    assert!(refused.contains("check the tunnel"), "{refused}");
    assert!(refused.contains("MELTEMI_START_HERE=1"), "{refused}");
    assert!(
        seen.refused_at_once,
        "the refusal waits for nothing: {seen:?}"
    );
    assert!(
        seen.nothing_appeared,
        "no daemon was started unasked: {seen:?}"
    );
    assert!(
        seen.started_when_asked,
        "asked, the client starts it: {seen:?}"
    );
    assert!(seen.answered_status, "and it answers: {seen:?}");
    assert!(seen.reused, "a second client reuses the instance: {seen:?}");
    assert!(seen.stopped, "the daemon stopped when asked to: {seen:?}");
    let _ = std::fs::remove_dir_all(&base);
}
