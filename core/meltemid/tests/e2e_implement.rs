// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the `implement` verb (comando-implement task 4.1),
//! driving an ephemeral daemon and the scripted `mock-agent` against a
//! temporary **git** fixture:
//!
//! - a deployment composes the full cycle per task (checkpoint → turn in the
//!   worktree → per-task commit with traceability → tick), and a re-run resumes
//!   past the already-done tasks;
//! - plan mode previews the sequence without touching anything;
//! - autonomous without permission rules degrades to supervised with a notice.
//!
//! Runs against temporary fixtures, never this repo (constitution §"tests e2e").
//! Requires `git` on PATH; skips with a note if absent.

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

/// A git fixture with a configured mock agent, an allow rule (unless `rules` is
/// false), and a change with a two-task `tasks.md`.
fn fixture(tag: &str, rules: bool) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-impl-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/add-demo")).unwrap();

    let mock = mock_agent_bin();
    assert!(mock.exists(), "run `cargo test` at the workspace root");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = ['{}']\n", mock.display()),
    )
    .unwrap();
    if rules {
        std::fs::write(
            root.join(".meltemi/permissions.toml"),
            "[[rule]]\neffect = \"allow\"\n",
        )
        .unwrap();
    }
    std::fs::write(
        root.join(".meltemi/changes/add-demo/tasks.md"),
        "## 1. Work\n\n- [ ] 1.1 First task\n- [ ] 1.2 Second task\n",
    )
    .unwrap();

    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-impl-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-impl-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-impld-{}-{tag}", std::process::id()));
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
                name: "e2e-impl-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn deploys_the_cycle_and_resumes_on_rerun() {
    // Scenarios: Tarea completa el ciclo compuesto; Progreso sobrevive reinicio.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // SAFETY: single test process; clear any agent override.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let root = fixture("deploy", true);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("deploy").await;
    let peer = init_client(&endpoint).await;

    let res = peer
        .request(
            methods::SDD_IMPLEMENT,
            &json!({ "projectRoot": root_str, "change": "add-demo", "agent": "claude" }),
        )
        .await
        .expect("implement ok");
    assert_eq!(res["mode"], "act", "{res:#}");
    let committed = res["committed"].as_array().unwrap();
    assert!(
        committed.iter().any(|c| c == "1.1") && committed.iter().any(|c| c == "1.2"),
        "both tasks committed this run: {res:#}"
    );

    // tasks.md is ticked (resumable progress).
    let tasks = std::fs::read_to_string(root.join(".meltemi/changes/add-demo/tasks.md")).unwrap();
    assert!(tasks.contains("- [x] 1.1"), "1.1 ticked: {tasks}");
    assert!(tasks.contains("- [x] 1.2"), "1.2 ticked");

    // Each task's worktree has a commit carrying the traceability trailer.
    let wt = root.join(".meltemi/worktrees/add-demo/1-1-claude");
    assert!(wt.is_dir(), "task worktree created");
    let body = git(&wt, &["log", "-1", "--pretty=%B"]);
    assert!(
        body.contains("Meltemi-Task: add-demo/1.1"),
        "trailer: {body}"
    );
    assert!(
        !body.to_ascii_lowercase().contains("co-authored-by"),
        "no co-authorship"
    );

    // Re-running resumes: every task is already done, nothing new committed.
    let again = peer
        .request(
            methods::SDD_IMPLEMENT,
            &json!({ "projectRoot": root_str, "change": "add-demo", "agent": "claude" }),
        )
        .await
        .expect("re-run ok");
    assert_eq!(
        again["committed"].as_array().unwrap().len(),
        0,
        "nothing to redo: {again:#}"
    );
    assert!(
        again["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"] == "already-done"),
        "all already-done on resume: {again:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn plan_mode_previews_and_autonomy_degrades_without_rules() {
    // Scenarios: Plan aprobado antes de actuar (preview, tree intact); Sin
    // reglas no hay autonomía.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    // No permission rules → autonomy must degrade.
    let root = fixture("plan", false);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("plan").await;
    let peer = init_client(&endpoint).await;

    let res = peer
        .request(
            methods::SDD_IMPLEMENT,
            &json!({
                "projectRoot": root_str, "change": "add-demo", "agent": "claude",
                "planOnly": true, "autonomous": true,
            }),
        )
        .await
        .expect("plan ok");
    assert_eq!(res["mode"], "plan");
    assert_eq!(res["autonomous"], false, "autonomy degraded: {res:#}");
    assert!(
        res["degraded"]
            .as_str()
            .is_some_and(|s| s.contains("rules")),
        "the degradation reason is visible: {res:#}"
    );
    assert!(
        res["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"] == "planned"),
        "tasks are planned, not committed"
    );
    // Plan touched nothing: tasks.md is untouched, no worktrees created.
    let tasks = std::fs::read_to_string(root.join(".meltemi/changes/add-demo/tasks.md")).unwrap();
    assert!(tasks.contains("- [ ] 1.1"), "plan committed nothing");
    assert!(
        !root.join(".meltemi/worktrees").exists(),
        "no worktrees in plan mode"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
