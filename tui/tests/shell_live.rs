// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of the shell's live wiring (task 6.3): the connection actor
//! drives an ephemeral in-process daemon over the real transport and reports a
//! live connection snapshot and the session list. Runs against a temporary
//! endpoint, never the real user daemon.

mod common;

use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use meltemi::shell::conn::{Command, connection_actor};
use meltemi::shell::live::Update;
use meltemi::shell::messages::Lang;
use meltemi::shell::render::ConnState;

use common::spawn_daemon;

#[tokio::test]
async fn connection_actor_reports_connected_and_sessions() {
    let (endpoint, daemon) = spawn_daemon("shell-live").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    let mut connected = false;
    let mut got_sessions = false;
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            match update {
                Update::Conn(ConnState::Connected { .. }) => connected = true,
                Update::Sessions(_) => got_sessions = true,
                _ => {}
            }
            if connected && got_sessions {
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("actor reported Connected + Sessions within 10s");
    assert!(connected, "the actor must report a live Connected snapshot");
    assert!(got_sessions, "the actor must report the session list");

    // Dropping the command sender ends the actor cleanly.
    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
}

// Scenario: Sesión no dirigible lo dice con remedio
#[tokio::test]
async fn a_refused_direction_comes_back_with_the_daemons_diagnosis_and_remedy() {
    // The instruction reaches the daemon and the refusal reaches the user: no
    // session was found, and the answer says what to do about it instead of
    // dropping the text the user typed.
    let (endpoint, daemon) = spawn_daemon("shell-direct").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    cmd_tx
        .send(Command::Direct {
            session_id: "11111111-2222-3333-4444-555555555555".into(),
            project_root: std::env::temp_dir().display().to_string(),
            instruction: "Arregla el build".into(),
        })
        .expect("the actor is listening");

    let mut refusal = None;
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            if let Update::Notice(text) = update
                && text.contains("no admite")
            {
                refusal = Some(text);
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("the refusal reached the surface within 10s");
    let refusal = refusal.expect("a refusal notice");
    assert!(
        refusal.contains("meltemi sessions"),
        "the daemon's remedy travels with the diagnosis: {refusal}"
    );

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
}
