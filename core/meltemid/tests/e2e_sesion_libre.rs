// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the free session (lanzador-conversacional), driving an
//! ephemeral daemon and the scripted `mock-agent` against temporary fixtures —
//! never this repo, never a real agent, never the network (constitution
//! §"tests e2e").
//!
//! The free session is the door the method's verbs never had: an instruction on
//! a project root, with no change, no task and no gate. What these tests hold it
//! to is that nothing about the government was traded away for that.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
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
        format!(r"\\.\pipe\meltemid-e2e-libre-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-libre-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo whose config points the agent at the mock with `mock_args`,
/// and whose permissions allow writes so a turn runs without escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-libre-{}-{tag}", std::process::id()));
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
    let state = DaemonState::for_test(&format!("libre-{tag}"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-libre-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Runs `git <args>` in `cwd`, returning trimmed stdout.
fn git(cwd: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Turns a fixture into a git repository with one commit, so there is history
/// to snapshot.
fn with_history(root: &std::path::Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "e2e@meltemi.test"]);
    git(root, &["config", "user.name", "Meltemi E2E"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-m", "fixture"]);
}

/// The session log's events, decoded.
async fn log_events(peer: &Peer, root: &str, session_id: &str) -> Vec<serde_json::Value> {
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
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l.as_str().unwrap()).ok())
        .collect()
}

// Scenario: Sesión libre completada no lista como interrumpida
#[tokio::test]
async fn a_completed_free_session_is_ended_not_interrupted() {
    // The finalizer is the whole point of this test. Skipping it is the defect
    // `gui-acabado-y-cierre-sdd` D3 fixed: the index never received `ended_at`,
    // so a session that finished perfectly listed as interrupted forever.
    let root = fixture("finalize", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("finalize").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "find out why the build is slow" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");
    let session_id = started["sessionId"].as_str().expect("a session id");

    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().expect("sessions");
    let session = sessions
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .unwrap_or_else(|| panic!("the free session is missing from {list:#}"));
    assert_eq!(
        session["state"], "ended",
        "a completed free session ended; it did not crash: {list:#}"
    );
    assert!(
        sessions.iter().all(|s| s["state"] != "interrupted"),
        "nothing here was interrupted: {list:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Punto de restauración creado al arrancar
// Scenario: La sesión libre no crea worktrees ni competidores
#[tokio::test]
async fn the_restore_point_is_taken_without_moving_anything_of_the_users() {
    let root = fixture("checkpoint", &[]);
    with_history(&root);
    // Something staged and not committed: the sharpest way to see whether the
    // user's own index survived the snapshot.
    std::fs::write(root.join("staged.txt"), "work in progress\n").unwrap();
    git(&root, &["add", "staged.txt"]);
    let head_before = git(&root, &["rev-parse", "HEAD"]);
    let branch_before = git(&root, &["symbolic-ref", "HEAD"]);
    let staged_before = git(&root, &["diff", "--cached", "--name-only"]);

    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("checkpoint").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "look around" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("session/start ok");

    let git_ref = started["checkpointRef"]
        .as_str()
        .unwrap_or_else(|| panic!("a repository with history gets a restore point: {started:#}"));
    let session_id = started["sessionId"].as_str().expect("a session id");
    assert_eq!(
        git_ref,
        format!("refs/meltemi/checkpoints/free/{session_id}-mock-agent"),
        "the reserved triple, slugged: {started:#}"
    );
    assert!(
        started["checkpointUnavailable"].is_null() && started["checkpointRemedy"].is_null(),
        "a restore point that exists needs no excuse: {started:#}"
    );
    // The ref really resolves to a commit.
    assert!(!git(&root, &["rev-parse", git_ref]).is_empty());

    // Nothing of the user's moved: not HEAD, not the branch it points at, not
    // the index they had staged.
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(git(&root, &["symbolic-ref", "HEAD"]), branch_before);
    assert_eq!(
        git(&root, &["diff", "--cached", "--name-only"]),
        staged_before,
        "the snapshot used a scratch index; the user's staging area is theirs"
    );

    // The conversation carries it: the restore point is part of the session's
    // own transcript, not a fact hidden in a side file.
    let events = log_events(&peer, &root_str, session_id).await;
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "checkpoint_created" && e["payload"]["gitRef"] == git_ref),
        "the restore point is recorded in the session log: {events:#?}"
    );

    // And no isolation was invented behind the user's back: a free session runs
    // on the root, so it creates no worktree and nobody competes with it.
    let worktrees = peer
        .request(methods::WORKTREE_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("worktree/list ok");
    assert_eq!(
        worktrees["worktrees"].as_array().map(Vec::len),
        Some(0),
        "the free session made no worktree: {worktrees:#}"
    );
    assert!(
        !root.join(".meltemi").join("worktrees").exists(),
        "no worktree tree was created for the pseudo-change"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
