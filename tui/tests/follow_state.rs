// SPDX-License-Identifier: Apache-2.0

//! `sessions --follow` end to end (estado-en-vivo design D8).
//!
//! The test plays the daemon — the bridge's pattern — because what has to be
//! staged here is the daemon's side of a stream: an opening snapshot, a
//! transition after it, and then a daemon that goes away. No real daemon can be
//! made to do those three things on cue, and the point of the verb is precisely
//! what the terminal prints for each of them.
//!
//! Never this repository, never a real agent, never the network.

mod common;

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};

use meltemi_client::transport::Listener;

use common::{FakeDaemon, test_endpoint};

fn cli() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_meltemi"))
}

fn affirmation(session: &str, state: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "session/state",
        "params": {
            "sessionId": session,
            "projectRoot": "/repo",
            "state": state,
            "since": "2026-09-26T12:00:00Z",
            "pendingPermissions": 0
        }
    })
}

// Scenario: Cada transición es una línea
// Scenario: El flujo termina con su daemon
#[tokio::test]
async fn following_writes_one_record_per_affirmation_and_ends_with_its_daemon() {
    let endpoint = test_endpoint("follow-state");
    let mut listener = Listener::bind(&endpoint).await.expect("bind");

    let mut child = tokio::process::Command::new(cli())
        .args(["--json", "sessions", "--follow"])
        .env("MELTEMI_ENDPOINT", &endpoint)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("the shipped binary runs");
    let mut out = BufReader::new(child.stdout.take().expect("piped stdout")).lines();

    let mut daemon = FakeDaemon::accept(&mut listener).await;
    daemon.initialize().await;

    // The snapshot a connection gets on initialize: two sessions, one record
    // each.
    daemon.send(affirmation("s-1", "active")).await;
    daemon.send(affirmation("s-2", "waiting_permission")).await;
    // And a transition after it.
    daemon.send(affirmation("s-1", "ended")).await;

    let mut records = Vec::new();
    for _ in 0..3 {
        let line = tokio::time::timeout(Duration::from_secs(10), out.next_line())
            .await
            .expect("a record arrived")
            .expect("read")
            .expect("a line");
        records.push(serde_json::from_str::<Value>(&line).expect("each record is one JSON line"));
    }

    assert_eq!(records[0]["sessionId"], "s-1");
    assert_eq!(records[0]["state"], "active");
    assert_eq!(records[1]["sessionId"], "s-2");
    assert_eq!(records[1]["state"], "waiting_permission");
    assert_eq!(
        records[2]["state"], "ended",
        "the transition is its own record, not a rewrite of the first"
    );
    // The instant travels, because a reader subtracts it.
    assert!(
        records[0]["since"]
            .as_str()
            .is_some_and(|s| s.ends_with('Z')),
        "{:#}",
        records[0]
    );

    // The daemon goes. The stream ends with the code the CLI reserves for a
    // daemon that is not there, rather than hanging on a pipe nobody will write
    // to again.
    drop(daemon);
    drop(listener);
    let status = tokio::time::timeout(Duration::from_secs(20), child.wait())
        .await
        .expect("the stream noticed the daemon go")
        .expect("wait");
    assert_eq!(
        status.code(),
        Some(10),
        "a daemon that went away is exit 10, not a success and not a hang"
    );
}

// Scenario: Sin la bandera nada cambia
#[tokio::test]
async fn without_the_flag_it_answers_the_list_once_and_exits() {
    let endpoint = test_endpoint("no-follow");
    let mut listener = Listener::bind(&endpoint).await.expect("bind");

    let mut child = tokio::process::Command::new(cli())
        .args(["--json", "sessions"])
        .env("MELTEMI_ENDPOINT", &endpoint)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("the shipped binary runs");
    let mut out = BufReader::new(child.stdout.take().expect("piped stdout")).lines();

    let mut daemon = FakeDaemon::accept(&mut listener).await;
    daemon.initialize().await;
    daemon
        .answer("session/list", json!({ "sessions": [] }))
        .await;

    // One answer, and the process is done: following is what the flag is for.
    let status = tokio::time::timeout(Duration::from_secs(20), child.wait())
        .await
        .expect("the verb answered and exited")
        .expect("wait");
    assert!(status.success(), "{status:?}");
    let first = out.next_line().await.expect("read").expect("a line");
    assert!(
        serde_json::from_str::<Value>(&first).is_ok(),
        "the list is one JSON answer: {first}"
    );
}
