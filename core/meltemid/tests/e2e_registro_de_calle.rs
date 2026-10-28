// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of a lane's session record (registro-de-la-sesion-de-calle),
//! driving an ephemeral daemon and the scripted `mock-agent` against temporary
//! fixtures — never this repo, never a real agent, never the network
//! (constitution §5).
//!
//! What the integrated run of 2026-09-26 measured, and these hold the daemon
//! against: a `worktree/dispatch` session is listed with its lane as its root,
//! and `session/log` asked with that root answered an empty page; and a project
//! that never opened a workshop saw its lanes as embedded repositories.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn mock_path() -> String {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let mock = dir.join(if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    });
    assert!(
        mock.exists(),
        "run `cargo build --workspace` first ({} is missing)",
        mock.display()
    );
    mock.display().to_string().replace('\\', "/")
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-calle-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-calle-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// The path spelled the way the registry spells it (see `e2e_sesion_libre`).
fn resolved(root: &Path) -> PathBuf {
    let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    #[cfg(windows)]
    {
        let shown = resolved.to_string_lossy();
        if let Some(plain) = shown.strip_prefix(r"\\?\")
            && !plain.starts_with("UNC\\")
        {
            return PathBuf::from(plain);
        }
    }
    resolved
}

fn git(cwd: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A committed fixture with one change of two tasks and one catalog entry,
/// `lane`, whose agent is the mock with session load — so a lane's session is
/// resumable. No workshop is ever opened in it.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-calle-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();
    let root = resolved(&root);
    let mock = mock_path();
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"calle-e2e\"\n\
             [[agents]]\nid = \"lane\"\nname = \"Lane\"\nlevel = 1\n\
             bin = \"mock-agent\"\nacp-args = ['--load-session']\ncandidate-paths = ['{mock}']\n"
        ),
    )
    .unwrap();
    let registry = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock}', '--load-session']\n\n[fleet]\nregistry = '{registry}'\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/tasks.md"),
        "## 1. Build\n\n- [ ] 1.1 Add the toggle\n- [ ] 1.2 Remember the choice\n",
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

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("calle-{tag}"), shutdown_tx);
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
                name: "e2e-calle-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn request(peer: &Peer, method: &str, params: Value) -> Value {
    tokio::time::timeout(Duration::from_secs(60), peer.request(method, &params))
        .await
        .unwrap_or_else(|_| panic!("{method} answered"))
        .unwrap_or_else(|e| panic!("{method} ok: {e}"))
}

async fn dispatch(peer: &Peer, root: &str, task: &str) -> String {
    let dispatched = request(
        peer,
        methods::WORKTREE_DISPATCH,
        json!({ "projectRoot": root, "change": "dark-mode", "task": task, "agent": "lane" }),
    )
    .await;
    assert_eq!(dispatched["status"], "completed", "{dispatched:#}");
    dispatched["sessionId"].as_str().unwrap().to_string()
}

/// The root the listing gives for a session — what any client then asks with.
async fn listed_root(peer: &Peer, root: &str, session_id: &str) -> String {
    let list = request(peer, methods::SESSION_LIST, json!({ "projectRoot": root })).await;
    list["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .unwrap_or_else(|| panic!("the project lists {session_id}: {list:#}"))["projectRoot"]
        .as_str()
        .expect("a root")
        .to_string()
}

async fn log_lines(peer: &Peer, root: &str, session_id: &str) -> Vec<String> {
    let log = request(
        peer,
        methods::SESSION_LOG,
        json!({ "projectRoot": root, "sessionId": session_id }),
    )
    .await;
    log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .map(|l| l.as_str().unwrap().to_string())
        .collect()
}

fn is_lane(root: &str) -> bool {
    root.replace('\\', "/")
        .contains("/.meltemi/worktrees/dark-mode/")
}

// Scenario: El registro de un despacho se lee por su calle
#[tokio::test]
async fn a_dispatched_lane_is_read_by_its_lane() {
    let root = fixture("log");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("log").await;
    let peer = init_client(&endpoint).await;

    let session_id = dispatch(&peer, &root_str, "1.1").await;
    let lane = listed_root(&peer, &root_str, &session_id).await;
    assert!(is_lane(&lane), "a lane is listed by its lane: {lane}");

    let by_lane = log_lines(&peer, &lane, &session_id).await;
    assert!(
        by_lane.iter().any(|l| l.contains("\"session_started\"")),
        "the lane's own root finds its log: {by_lane:#?}"
    );
    let by_project = log_lines(&peer, &root_str, &session_id).await;
    assert_eq!(by_lane, by_project, "either root reads the same log");

    peer.close();
    daemon.abort();
    let _ = git(&root, &["worktree", "prune"]);
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Reanudar una calle deja el registro con su proyecto
#[tokio::test]
async fn a_resumed_lane_is_logged_with_its_project() {
    let root = fixture("resume");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("resume").await;
    let peer = init_client(&endpoint).await;

    let original = dispatch(&peer, &root_str, "1.1").await;
    let lane = listed_root(&peer, &root_str, &original).await;
    // The answer to the dispatch can arrive before the session is released;
    // what resumes is a session the listing calls resumable.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let list = request(
            &peer,
            methods::SESSION_LIST,
            json!({ "projectRoot": root_str }),
        )
        .await;
        let listed = list["sessions"]
            .as_array()
            .and_then(|all| {
                all.iter()
                    .find(|s| s["sessionId"] == original.as_str())
                    .cloned()
            })
            .expect("listed");
        if listed["resumable"] == true {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the lane never became resumable: {listed:#}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // A client resumes it with the root the listing gave it.
    let resumed = request(
        &peer,
        methods::SESSION_DIRECT,
        json!({
            "sessionId": original,
            "instruction": "also remember the choice",
            "projectRoot": lane,
        }),
    )
    .await;
    assert_eq!(resumed["disposition"], "resumed", "{resumed:#}");
    assert_eq!(resumed["resumedFrom"], original.as_str());
    let resumed_id = resumed["sessionId"].as_str().unwrap().to_string();

    // Listed with its project, still with its lane as its root, and its log
    // read by that lane.
    assert_eq!(listed_root(&peer, &root_str, &resumed_id).await, lane);
    let lines = log_lines(&peer, &lane, &resumed_id).await;
    assert!(
        lines.iter().any(|l| l.contains("\"session_started\"")),
        "the resumed lane's log is read by its lane: {lines:#?}"
    );

    peer.close();
    daemon.abort();
    let _ = git(&root, &["worktree", "prune"]);
    let _ = std::fs::remove_dir_all(&root);
}
