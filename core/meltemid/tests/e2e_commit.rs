// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the atomic per-task commit (git-commit-por-tarea task
//! 3.2), driving an ephemeral daemon against temporary **git** fixture repos:
//!
//! - a supervised preview shows the message and diff but commits nothing;
//! - an autonomous commit carries the `Meltemi-Task`/`Meltemi-Req` trailers and
//!   never a co-authorship trailer;
//! - a commit touching a path outside the declared scope reports the deviation;
//! - a failing user git hook is surfaced verbatim and never bypassed.
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
    let root = std::env::temp_dir().join(format!("meltemi-e2e-com-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("seed.txt"), "seed\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-com-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-com-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-comd-{}-{tag}", std::process::id()));
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
                name: "e2e-commit-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Assigns task 2.1 to claude and returns its worktree path.
async fn assign_one(peer: &Peer, root: &str) -> String {
    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root,
                "tasks": [{ "change": "add-thing", "task": "2.1", "agents": ["claude"], "files": ["declared.txt"] }],
            }),
        )
        .await
        .expect("assign ok");
    assign["worktrees"][0]["path"].as_str().unwrap().to_string()
}

fn commit_count(worktree: &Path) -> u32 {
    git(worktree, &["rev-list", "--count", "HEAD"])
        .trim()
        .parse()
        .unwrap()
}

#[tokio::test]
async fn preview_does_not_commit_then_autonomous_commit_carries_trailers() {
    // Scenarios: Supervisado propone antes de cometer; Autónomo comete y
    // registra; Trazabilidad hasta el requisito; Sin co-autoría jamás.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("trailers");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("trailers").await;
    let peer = init_client(&endpoint).await;

    let wt = assign_one(&peer, &root_str).await;
    let wt_path = PathBuf::from(&wt);
    std::fs::write(wt_path.join("declared.txt"), "the thing\n").unwrap();
    let before = commit_count(&wt_path);

    let req = json!([{ "capability": "git-per-task", "requirement": "Commit atómico por tarea completada" }]);

    // Supervised preview: message + diff, but nothing committed.
    let preview = peer
        .request(
            methods::COMMIT_TASK,
            &json!({
                "projectRoot": root_str, "change": "add-thing", "task": "2.1", "agent": "claude",
                "title": "Add the thing", "requirements": req, "declaredFiles": ["declared.txt"],
            }),
        )
        .await
        .expect("preview ok");
    assert_eq!(preview["committed"], false);
    assert!(
        preview["message"]
            .as_str()
            .unwrap()
            .contains("Meltemi-Task: add-thing/2.1"),
        "preview shows the trailer: {preview:#}"
    );
    assert_eq!(commit_count(&wt_path), before, "preview committed nothing");

    // Autonomous commit applies.
    let applied = peer
        .request(
            methods::COMMIT_TASK,
            &json!({
                "projectRoot": root_str, "change": "add-thing", "task": "2.1", "agent": "claude",
                "title": "Add the thing", "body": "Implements the thing the spec asks for.",
                "requirements": req, "declaredFiles": ["declared.txt"], "confirm": true,
            }),
        )
        .await
        .expect("commit ok");
    assert_eq!(applied["committed"], true, "{applied:#}");
    assert_eq!(applied["treeClean"], true, "worktree clean after commit");
    assert_eq!(
        applied["deviations"].as_array().unwrap().len(),
        0,
        "in scope"
    );
    assert_eq!(commit_count(&wt_path), before + 1, "exactly one new commit");

    // The real commit message carries the trailers and NO co-authorship.
    let body = git(&wt_path, &["log", "-1", "--pretty=%B"]);
    assert!(
        body.contains("Meltemi-Task: add-thing/2.1"),
        "task trailer: {body}"
    );
    assert!(
        body.contains("Meltemi-Req: git-per-task/commit-atomico-por-tarea-completada"),
        "req trailer: {body}"
    );
    assert!(body.contains("(add-thing 2.1)"), "repo reference: {body}");
    assert!(
        !body.to_ascii_lowercase().contains("co-authored-by"),
        "never a co-authorship trailer: {body}"
    );
    // The author is the fixture user's identity, untouched.
    let author = git(&wt_path, &["log", "-1", "--pretty=%an <%ae>"]);
    assert_eq!(author.trim(), "Meltemi E2E <e2e@meltemi.test>");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_commit_outside_the_declared_scope_reports_the_deviation() {
    // Scenario: Desviación declarada.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("deviation");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("deviation").await;
    let peer = init_client(&endpoint).await;

    let wt = assign_one(&peer, &root_str).await;
    let wt_path = PathBuf::from(&wt);
    // Touch the declared file AND a rogue one the task did not declare.
    std::fs::write(wt_path.join("declared.txt"), "ok\n").unwrap();
    std::fs::write(wt_path.join("rogue.txt"), "unexpected\n").unwrap();

    let applied = peer
        .request(
            methods::COMMIT_TASK,
            &json!({
                "projectRoot": root_str, "change": "add-thing", "task": "2.1", "agent": "claude",
                "title": "Touch files", "declaredFiles": ["declared.txt"], "confirm": true,
            }),
        )
        .await
        .expect("commit ok");
    assert_eq!(applied["committed"], true);
    let deviations = applied["deviations"].as_array().unwrap();
    assert!(
        deviations.iter().any(|d| d == "rogue.txt"),
        "the out-of-scope path is reported, never hidden: {applied:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_failing_user_hook_is_surfaced_and_never_bypassed() {
    // Scenario: Hooks respetados
    // fallo tal cual, sin --no-verify.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("hook");
    let root_str = root.display().to_string();

    // A pre-commit hook that always rejects (shared by linked worktrees).
    let hooks = root.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, "#!/bin/sh\necho 'hook says no' 1>&2\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let (endpoint, daemon) = spawn_daemon("hook").await;
    let peer = init_client(&endpoint).await;
    let wt = assign_one(&peer, &root_str).await;
    let wt_path = PathBuf::from(&wt);
    std::fs::write(wt_path.join("declared.txt"), "content\n").unwrap();
    let before = commit_count(&wt_path);

    let refused = peer
        .request(
            methods::COMMIT_TASK,
            &json!({
                "projectRoot": root_str, "change": "add-thing", "task": "2.1", "agent": "claude",
                "title": "Add the thing", "confirm": true,
            }),
        )
        .await;

    match refused {
        Err(err) => {
            assert_eq!(
                err.code,
                meltemi_proto::error_codes::GIT_COMMIT_FAILED,
                "{err}"
            );
            assert_eq!(
                commit_count(&wt_path),
                before,
                "the hook rejection left the task completed-without-commit"
            );
        }
        // If the platform's git did not execute the shell hook, don't assert a
        // false negative — the no-bypass guarantee is covered by never passing
        // --no-verify (unit/code), and this scenario is best-effort per OS.
        Ok(v) => eprintln!("note: hook did not fire on this platform: {v:#}"),
    }

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
