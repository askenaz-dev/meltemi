// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of checkpoints and reversion (checkpoints-rollback task
//! 4.2), driving an ephemeral daemon against temporary **git** fixture repos:
//!
//! - a pre-task checkpoint snapshots the worktree and logs the event;
//! - reverting a task restores its worktree exactly and leaves other worktrees
//!   untouched;
//! - the reversion declares its honest scope: complete when clean, and never
//!   total when an approved out-of-tree operation was recorded.
//!
//! Runs against temporary fixtures, never this repo (constitution
//! §"tests e2e"). Requires `git` on PATH; skips with a note if absent.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn git_fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-cp-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("file.txt"), "base\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-cp-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-cp-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-cpd-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data_dir = base.join("data");
    let config_dir = base.join("config");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(data_dir, config_dir, shutdown_tx);
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
                name: "e2e-checkpoints-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Assigns one task to two agents (two isolated worktrees) and returns their
/// paths keyed by agent, plus the endpoint's peer.
async fn assign_two(peer: &Peer, root: &str) -> (String, String) {
    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root,
                "tasks": [{
                    "change": "add-thing", "task": "1.1",
                    "agents": ["claude", "gemini"], "files": ["file.txt"],
                }],
            }),
        )
        .await
        .expect("assign ok");
    let wts = assign["worktrees"].as_array().unwrap();
    let path_of = |agent: &str| {
        wts.iter().find(|w| w["agent"] == agent).unwrap()["path"]
            .as_str()
            .unwrap()
            .to_string()
    };
    (path_of("claude"), path_of("gemini"))
}

#[tokio::test]
async fn checkpoint_then_revert_restores_exactly_and_isolates() {
    // Scenarios: Checkpoint pre-tarea; Revertir una tarea no toca a las demás;
    // Reversión limpia (complete).
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("revert");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("revert").await;
    let peer = init_client(&endpoint).await;

    let (claude, gemini) = assign_two(&peer, &root_str).await;

    // Checkpoint claude's worktree before it works.
    let created = peer
        .request(
            methods::CHECKPOINT_CREATE,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude" }),
        )
        .await
        .expect("checkpoint create ok");
    assert!(
        created["checkpoint"]["gitRef"]
            .as_str()
            .unwrap()
            .starts_with("refs/meltemi/checkpoints/"),
        "technical ref: {created:#}"
    );
    // The lifecycle event was logged to the checkpoints JSONL.
    let events = std::fs::read_to_string(root.join(".meltemi/checkpoints/events.jsonl")).unwrap();
    assert!(
        events.contains("checkpoint_created"),
        "checkpoint_created event recorded: {events}"
    );

    // Agent work in both worktrees: claude edits its file; gemini edits its own.
    std::fs::write(Path::new(&claude).join("file.txt"), "claude edited\n").unwrap();
    std::fs::write(Path::new(&claude).join("scratch.txt"), "untracked\n").unwrap();
    std::fs::write(Path::new(&gemini).join("file.txt"), "gemini edited\n").unwrap();

    // Revert claude to its checkpoint. Confirmation required.
    let refused = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude" }),
        )
        .await
        .expect_err("revert without confirm must be refused");
    assert_eq!(refused.code, meltemi_proto::error_codes::WORKTREE_REFUSED);

    let reverted = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude", "confirm": true }),
        )
        .await
        .expect("revert ok");
    assert_eq!(reverted["reverted"], true);
    assert_eq!(
        reverted["scope"]["complete"], true,
        "clean reversion: {reverted:#}"
    );
    assert_eq!(
        reverted["scope"]["irreversible"].as_array().unwrap().len(),
        0
    );

    // Claude's worktree is back to base; its untracked file is gone.
    assert_eq!(
        std::fs::read_to_string(Path::new(&claude).join("file.txt")).unwrap(),
        "base\n",
        "tracked file restored"
    );
    assert!(
        !Path::new(&claude).join("scratch.txt").exists(),
        "later untracked file cleaned"
    );
    // Gemini's worktree is untouched.
    assert_eq!(
        std::fs::read_to_string(Path::new(&gemini).join("file.txt")).unwrap(),
        "gemini edited\n",
        "other worktree intact"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn reversion_declares_irreversible_out_of_tree_operations() {
    // Scenario: Irreversibles declaradas
    // no anuncia reversión total.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("scope");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("scope").await;
    let peer = init_client(&endpoint).await;

    let (_claude, _gemini) = assign_two(&peer, &root_str).await;
    peer.request(
        methods::CHECKPOINT_CREATE,
        &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude" }),
    )
    .await
    .expect("checkpoint ok");

    // During the task, an out-of-tree command was approved (the proxy would
    // classify and record it; here we record it via the ledger method).
    peer.request(
        methods::CHECKPOINT_RECORD_OP,
        &json!({
            "projectRoot": root_str, "change": "add-thing", "task": "1.1",
            "agent": "claude", "operation": "ran command: npm publish",
        }),
    )
    .await
    .expect("record-op ok");

    // The list surfaces the irreversible op against the checkpoint.
    let list = peer
        .request(
            methods::CHECKPOINT_LIST,
            &json!({ "projectRoot": root_str }),
        )
        .await
        .expect("list ok");
    let cp = &list["checkpoints"][0];
    assert!(
        cp["irreversible"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o == "ran command: npm publish"),
        "irreversible listed: {list:#}"
    );

    // The preview (no confirm) refuses and its detail names the irreversible.
    let preview = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude" }),
        )
        .await
        .expect_err("preview refuses without confirm");
    let detail = preview
        .data
        .as_ref()
        .and_then(|d| d["detail"].as_str())
        .unwrap_or("");
    assert!(
        detail.contains("npm publish"),
        "preview names the irreversible: {preview}"
    );

    // The confirmed reversion restores the tree but is NOT total.
    let reverted = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1", "agent": "claude", "confirm": true }),
        )
        .await
        .expect("revert ok");
    assert_eq!(reverted["scope"]["worktreeRestored"], true);
    assert_eq!(
        reverted["scope"]["complete"], false,
        "an out-of-tree op means the reversion is not total: {reverted:#}"
    );
    assert!(
        reverted["scope"]["irreversible"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o == "ran command: npm publish"),
        "the remaining irreversible op is declared"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
