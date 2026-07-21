// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of `worktree/apply-edit` (edit-surface; gui-tauri-paridad
//! design D4/D5), driving an ephemeral daemon and the scripted `mock-agent`
//! against temporary git fixtures:
//!
//! - a free tree takes the edit without friction and logs `human_edit` to the
//!   project-scoped edits JSONL;
//! - a path escape refuses with 4001 and touches nothing;
//! - with a turn in flight, the save demands the reinforced acknowledgement
//!   (`confirm: true`), lands in the session's JSONL, and the agent's next
//!   turn opens with the note listing the edited files.
//!
//! Runs against temporary fixtures, never this repo, and the CI `mock-agent`,
//! never a real agent or the network (constitution §"tests e2e").

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
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
        format!(r"\\.\pipe\meltemid-e2e-edit-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-edit-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// A temporary git repo whose `.meltemi` config points the agent at the mock.
fn git_fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-edit-{}-{tag}", std::process::id()));
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

    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("README.md"), "fixture\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("edit-{tag}"), shutdown_tx);
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
                name: "e2e-edit-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn wait_for_active_session(peer: &Peer, root: &str) -> String {
    for _ in 0..200 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list");
        if let Some(sessions) = list["sessions"].as_array()
            && let Some(active) = sessions.iter().find(|s| s["state"] == "active")
        {
            return active["sessionId"].as_str().unwrap().to_string();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no active session appeared for {root}");
}

/// The parsed events of a session's JSONL log, in order.
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

#[tokio::test]
async fn an_edit_on_a_free_tree_applies_without_friction_and_logs_to_the_project() {
    // Scenario: Worktree libre sin fricción
    let root = git_fixture("free", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("free").await;
    let peer = init_client(&endpoint).await;

    let result = peer
        .request(
            methods::WORKTREE_APPLY_EDIT,
            &json!({
                "projectRoot": root_str,
                "file": "notes/todo.md",
                "content": "- ship the GUI\n",
            }),
        )
        .await
        .expect("apply-edit on a free tree needs no confirm");
    assert_eq!(result["treeState"], "free", "{result:#}");
    assert_eq!(result["loggedTo"], "project");
    assert_eq!(result["bytesWritten"], 15);

    let written = std::fs::read_to_string(root.join("notes/todo.md")).expect("file written");
    assert_eq!(written, "- ship the GUI\n");

    // The human_edit event landed in the project-scoped edits JSONL.
    let events = std::fs::read_to_string(root.join(".meltemi/edits/events.jsonl"))
        .expect("project edits log exists");
    let last: Value = serde_json::from_str(events.lines().last().unwrap()).unwrap();
    assert_eq!(last["type"], "human_edit");
    assert_eq!(last["payload"]["file"], "notes/todo.md");
    assert!(last["payload"]["sessionId"].is_null());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_path_escape_is_refused_and_touches_nothing() {
    let root = git_fixture("escape", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("escape").await;
    let peer = init_client(&endpoint).await;

    let error = peer
        .request(
            methods::WORKTREE_APPLY_EDIT,
            &json!({
                "projectRoot": root_str,
                "file": "../outside.txt",
                "content": "nope",
            }),
        )
        .await
        .expect_err("escape refused");
    assert_eq!(error.code, meltemi_proto::error_codes::WORKTREE_REFUSED);
    let data = error.data.expect("structured refusal");
    assert_eq!(data["kind"], "path_escape");
    assert!(!root.parent().unwrap().join("outside.txt").exists());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn an_in_flight_turn_demands_reinforcement_and_notes_the_next_turn() {
    let root = git_fixture("inflight", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("inflight").await;
    let peer = init_client(&endpoint).await;

    // Start a proposal in the background; its first turn holds open ~3s.
    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };
    let session_id = wait_for_active_session(&peer, &root_str).await;

    // Scenario: Edición con turno en vuelo
    // Unconfirmed saves refuse while the session runs; once the prompt is in
    // flight the refusal carries the reinforced kind.
    let mut kind = String::new();
    for _ in 0..80 {
        let error = peer
            .request(
                methods::WORKTREE_APPLY_EDIT,
                &json!({
                    "projectRoot": root_str,
                    "file": "src/patch.rs",
                    "content": "// human draft\n",
                }),
            )
            .await
            .expect_err("a running session demands acknowledgement");
        kind = error.data.expect("structured refusal")["kind"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            kind == "session_active" || kind == "turn_in_flight",
            "soft-lock kinds only, got {kind}"
        );
        if kind == "turn_in_flight" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(kind, "turn_in_flight", "the reinforced state was observed");

    // Acknowledged, the save applies mid-turn and lands in the session's log.
    let result = peer
        .request(
            methods::WORKTREE_APPLY_EDIT,
            &json!({
                "projectRoot": root_str,
                "file": "src/patch.rs",
                "content": "// human draft\n",
                "confirm": true,
            }),
        )
        .await
        .expect("confirmed save applies");
    assert_eq!(result["treeState"], "turn_in_flight");
    assert_eq!(result["loggedTo"], "session");

    // A directed instruction guarantees a second turn on the same session.
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "continue with the light theme",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued");

    let outcome = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(outcome["status"], "completed", "{outcome:#}");

    // Scenario: Nota al siguiente turno del agente
    let events = log_events(&peer, &root_str, &session_id).await;
    let human_edits: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "human_edit")
        .collect();
    assert_eq!(human_edits.len(), 1, "the confirmed save was traced");
    assert_eq!(human_edits[0]["payload"]["file"], "src/patch.rs");
    assert_eq!(human_edits[0]["payload"]["sessionId"], session_id);

    let prompts: Vec<String> = events
        .iter()
        .filter(|e| e["type"] == "prompt_sent")
        .map(|e| e["payload"]["text"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(prompts.len(), 2, "initial + directed turn");
    assert!(
        prompts[1].starts_with("Note: since your last turn"),
        "the next turn opens with the human-edit note: {}",
        prompts[1]
    );
    assert!(
        prompts[1].contains("src/patch.rs"),
        "the note names the edited file"
    );
    assert!(
        prompts[1].contains("continue with the light theme"),
        "the directed instruction follows the note"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
