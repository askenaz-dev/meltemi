// SPDX-License-Identifier: Apache-2.0

//! E2E of the last-metre bridge (`meltemi bridge`): the test binds a REAL
//! endpoint with the client crate's own listener — a named pipe on Windows, a
//! Unix socket elsewhere — and drives the REAL `meltemi` binary, speaking
//! JSON-RPC lines through its piped stdio. No fake politeness: what travels is
//! bytes, and what is asserted is the round trip.

mod common;

use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use meltemi_client::transport::Listener;

/// The built `meltemi` binary under test.
fn meltemi_bin() -> &'static str {
    env!("CARGO_BIN_EXE_meltemi")
}

/// Spawns the bridge against `endpoint` with piped stdio.
fn spawn_bridge(endpoint: &str) -> tokio::process::Child {
    Command::new(meltemi_bin())
        .arg("bridge")
        .env("MELTEMI_ENDPOINT", endpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn meltemi bridge")
}

// Scenario: Un canal remoto completo sobre el puente
// Scenario: El puente en la plataforma sin reenvío estándar
#[tokio::test]
async fn the_bridge_carries_jsonrpc_lines_both_ways() {
    // The test IS the daemon: the same listener type the daemon binds, on this
    // platform's real endpoint kind — which on Windows exercises the named
    // pipe the change exists for.
    let endpoint = common::test_endpoint("bridge-channel");
    let mut listener = Listener::bind(&endpoint).await.expect("bind endpoint");

    let mut child = spawn_bridge(&endpoint);
    let mut child_in = child.stdin.take().expect("child stdin");
    let child_out = child.stdout.take().expect("child stdout");

    let stream = timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("the bridge dials the endpoint promptly")
        .expect("accept");
    let (daemon_read, mut daemon_write) = tokio::io::split(stream);
    let mut daemon_lines = BufReader::new(daemon_read);

    // Client → daemon: a request line written to the bridge's stdin arrives
    // verbatim at the endpoint.
    let request = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"status\"}\n";
    child_in
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    child_in.flush().await.expect("flush request");
    let mut seen = String::new();
    timeout(Duration::from_secs(10), daemon_lines.read_line(&mut seen))
        .await
        .expect("the request crosses the bridge")
        .expect("read request");
    assert_eq!(seen.trim_end(), request.trim_end(), "bytes travel verbatim");

    // Daemon → client: the response line comes back through the bridge's
    // stdout, equally verbatim.
    let response = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\n";
    daemon_write
        .write_all(response.as_bytes())
        .await
        .expect("write response");
    let mut child_lines = BufReader::new(child_out);
    let mut echoed = String::new();
    timeout(Duration::from_secs(10), child_lines.read_line(&mut echoed))
        .await
        .expect("the response crosses back")
        .expect("read response");
    assert_eq!(echoed.trim_end(), response.trim_end());

    // Scenario: El cierre de un extremo cierra el puente — the daemon side
    // hangs up; the bridge must end orderly, not linger.
    drop(daemon_write);
    drop(daemon_lines);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("the bridge ends when the endpoint closes")
        .expect("wait");
    assert!(
        status.success(),
        "an orderly close is a success: {status:?}"
    );
}

// Scenario: El cierre de un extremo cierra el puente
#[tokio::test]
async fn closing_stdin_closes_the_bridge() {
    let endpoint = common::test_endpoint("bridge-stdin-close");
    let mut listener = Listener::bind(&endpoint).await.expect("bind endpoint");

    let mut child = spawn_bridge(&endpoint);
    let child_in = child.stdin.take().expect("child stdin");
    let _stream = timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("accepted")
        .expect("accept");

    // The remote side is done: its stdin closes. Nothing may linger waiting
    // for a daemon that still has the connection open.
    drop(child_in);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("stdin closing ends the bridge")
        .expect("wait");
    assert!(
        status.success(),
        "an orderly close is a success: {status:?}"
    );
}

// Scenario: Sin daemon, el puente rehúsa sin colgarse
#[tokio::test]
async fn without_a_daemon_the_bridge_refuses_at_once() {
    // An endpoint nobody is listening on — the shape is real, the daemon is
    // absent.
    let endpoint = common::test_endpoint("bridge-no-daemon");

    let started = Instant::now();
    let mut child = spawn_bridge(&endpoint);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("the refusal is immediate, not a hang")
        .expect("wait");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "refusing must not wait for a daemon to appear"
    );
    assert!(!status.success(), "an unreachable endpoint is an error");

    // Diagnosis AND remedy, on stderr — stdout belongs to the channel and must
    // stay silent.
    let mut err = String::new();
    child
        .stderr
        .take()
        .expect("child stderr")
        .read_to_string(&mut err)
        .await
        .expect("read stderr");
    assert!(
        err.contains("not accepting connections") && err.contains("meltemi status"),
        "the refusal carries its remedy: {err}"
    );
}
