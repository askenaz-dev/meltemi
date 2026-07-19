// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of multi-provider dispatch (flota-multiproveedor), driving
//! an ephemeral daemon and two `mock-agent` launch profiles against a temporary
//! **git** fixture:
//!
//! - two distinct-provider profiles race the same task in separate worktrees,
//!   each launching under its OWN auth context (proven by a per-profile env
//!   marker the mock echoes), committing with traceability, NEVER ticking
//!   tasks.md;
//! - resolution honors profile > catalog id > configured, and refuses an
//!   undetected id (2001) without degrading.
//!
//! Runs against temporary fixtures, never this repo. Requires `git`; skips if
//! absent. Uses a per-fixture `[fleet] registry` config key (no process env).

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

fn mock_agent_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir
}

/// A git fixture whose registry declares two providers (both resolving to the
/// mock via candidate-paths) and two launch profiles with distinct auth
/// contexts, plus a configured agent for the free-label fallback.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-flota-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();

    let mock_dir = mock_agent_dir();
    assert!(
        mock_dir
            .join(if cfg!(windows) {
                "mock-agent.exe"
            } else {
                "mock-agent"
            })
            .exists(),
        "run `cargo test` at the workspace root"
    );
    let mock_dir_toml = mock_dir.display().to_string().replace('\\', "/");

    // A local registry: two providers, both resolving to the mock binary.
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"flota-e2e\"\n\
             [[agents]]\nid = \"provider-a\"\nname = \"Provider A\"\nlevel = 1\nbin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock_dir_toml}']\n\n\
             [[agents]]\nid = \"provider-b\"\nname = \"Provider B\"\nlevel = 1\nbin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock_dir_toml}']\n"
        ),
    )
    .unwrap();

    let registry_toml = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    let mock_bin = mock_dir
        .join(if cfg!(windows) {
            "mock-agent.exe"
        } else {
            "mock-agent"
        })
        .display()
        .to_string()
        .replace('\\', "/");
    // Config: registry + a configured agent (free-label fallback) + two profiles
    // pointing at the two providers, each with a distinct auth-context marker.
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock_bin}']\n\n\
             [fleet]\nregistry = '{registry_toml}'\n\n\
             [[fleet.profile]]\nname = \"work\"\nagent = \"provider-a\"\nenv = {{ MELTEMI_MOCK_MARKER = \"work-ctx\" }}\n\n\
             [[fleet.profile]]\nname = \"thorough\"\nagent = \"provider-b\"\nenv = {{ MELTEMI_MOCK_MARKER = \"thorough-ctx\" }}\n"
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
        "## 1. Build\n\n- [ ] 1.1 Add the toggle\n",
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
        format!(r"\\.\pipe\meltemid-e2e-flota-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-flota-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base =
        std::env::temp_dir().join(format!("meltemi-e2e-flotad-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(base.join("data"), base.join("config"), shutdown_tx);
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
                name: "e2e-flota-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn dispatch(peer: &Peer, root: &str, agent: &str) -> serde_json::Value {
    peer.request(
        methods::WORKTREE_DISPATCH,
        &json!({ "projectRoot": root, "change": "dark-mode", "task": "1.1", "agent": agent }),
    )
    .await
    .unwrap_or_else(|e| panic!("dispatch {agent}: {e}"))
}

#[tokio::test]
async fn two_profiles_race_with_distinct_auth_contexts() {
    // Scenario: Carrera de dos proveedores distintos
    // Scenario: El despacho no marca la tarea
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // SAFETY: single test process.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }
    let root = fixture("race");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("race").await;
    let peer = init_client(&endpoint).await;

    let work = dispatch(&peer, &root_str, "work").await;
    let thorough = dispatch(&peer, &root_str, "thorough").await;

    // Both resolved as profiles, to their own underlying provider, committed,
    // and NEVER ticked the task.
    for (res, name) in [(&work, "work"), (&thorough, "thorough")] {
        assert_eq!(res["resolution"]["source"], "profile", "{name}: {res:#}");
        assert_eq!(res["resolution"]["profile"], name);
        assert_eq!(res["committed"], true, "{name} committed");
        assert_eq!(res["taskTicked"], false, "a dispatch never ticks the task");
    }
    // Distinct worktrees/branches.
    assert_ne!(work["worktree"], thorough["worktree"]);

    // The auth-context env overlay reached each subprocess: each worktree's
    // artifact echoes its OWN marker (proving the two ran under distinct
    // contexts, not one binary twice).
    let marker_of = |res: &serde_json::Value| {
        let wt = PathBuf::from(res["worktree"].as_str().unwrap());
        std::fs::read_to_string(wt.join("task-1-1.md")).unwrap()
    };
    assert!(
        marker_of(&work).contains("marker=work-ctx"),
        "work ran under its context"
    );
    assert!(
        marker_of(&thorough).contains("marker=thorough-ctx"),
        "thorough ran under its context"
    );

    // tasks.md is untouched — a competitor does not own the task.
    let tasks = std::fs::read_to_string(root.join(".meltemi/changes/dark-mode/tasks.md")).unwrap();
    assert!(
        tasks.contains("- [ ] 1.1"),
        "tasks.md never ticked: {tasks}"
    );

    // Each commit carries per-task traceability, no co-authorship.
    let wt = PathBuf::from(work["worktree"].as_str().unwrap());
    let body = git(&wt, &["log", "-1", "--pretty=%B"]);
    assert!(
        body.contains("Meltemi-Task: dark-mode/1.1"),
        "trailer: {body}"
    );
    assert!(!body.to_ascii_lowercase().contains("co-authored-by"));

    // Comparable side by side against the common base.
    let diff = peer
        .request(
            methods::WORKTREE_DIFF,
            &json!({ "projectRoot": root_str, "change": "dark-mode", "task": "1.1" }),
        )
        .await
        .expect("diff ok");
    assert_eq!(diff["competitors"].as_array().unwrap().len(), 2);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn resolution_honors_catalog_id_and_free_label_fallback() {
    // Scenario: Sesión lanza el binario de su id de catálogo
    // Scenario: Etiqueta libre cae al agente configurado con registro
    // (resolución perfil > catálogo > configurado, D1)
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }
    let root = fixture("sources");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("sources").await;
    let peer = init_client(&endpoint).await;

    // A bare catalog id resolves as `catalog`.
    let cat = dispatch(&peer, &root_str, "provider-a").await;
    assert_eq!(cat["resolution"]["source"], "catalog", "{cat:#}");
    assert!(cat["resolution"]["profile"].is_null());

    // A free label (neither profile nor id) falls back to the configured agent.
    let free = dispatch(&peer, &root_str, "some-free-label").await;
    assert_eq!(free["resolution"]["source"], "configured", "{free:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
