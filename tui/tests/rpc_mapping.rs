// SPDX-License-Identifier: Apache-2.0

//! End-to-end mapping tests (task 5.3): the CLI's command layer drives an
//! ephemeral in-process daemon over the real transport. These two tests do not
//! mutate process-global environment, so they are safe to run concurrently.
//! The environment-mutating cases live in their own test binaries
//! (`propose_mapping`, `unreachable`).

mod common;

use std::time::Duration;

use meltemi::cli::Command;
use meltemi::run::execute;

use common::spawn_daemon;

#[tokio::test]
async fn status_maps_to_the_status_method() {
    // Scenario: status consulta el estado del daemon.
    let (endpoint, handle) = spawn_daemon("status").await;

    let outcome = execute(Command::Status, &endpoint)
        .await
        .expect("status succeeds");
    assert!(
        outcome.json["daemonVersion"].as_str().is_some(),
        "status must report the daemon version, got: {}",
        outcome.json
    );
    assert!(outcome.json["sessions"].is_array());
    assert!(outcome.human.contains("daemon "));

    handle.abort();
}

#[tokio::test]
async fn stop_maps_to_shutdown_and_stops_the_daemon() {
    // Scenario: stop -> shutdown; the daemon then stops.
    let (endpoint, handle) = spawn_daemon("stop").await;

    let outcome = execute(Command::Stop, &endpoint)
        .await
        .expect("stop succeeds");
    assert_eq!(outcome.json["shutdown"], "requested");

    // The serve loop must return after the shutdown request.
    let stopped = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(stopped.is_ok(), "daemon must stop after `stop`");
}
