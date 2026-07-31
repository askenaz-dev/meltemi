// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of multi-project, multi-subscription work
//! (multiproyecto-suscripciones tasks 8.4 and 8.7), driving one ephemeral
//! daemon against **two** temporary git fixtures, each dispatching under its
//! own launch profile (two "subscriptions" of the same mock agent):
//!
//! - `project/list` reports both projects with their session counters;
//! - one unfiltered `session/list` brings every session of both projects, each
//!   with its own root, its resolved agent and the subscription that ran it —
//!   which is what lets a client aggregate the Project → Sessions tree;
//! - a root that no longer exists on disk stays listed and is marked absent.
//!
//! Runs against temporary fixtures, never this repo (constitution
//! §"tests e2e"). Requires `git`; skips if absent.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn git(dir: &Path, args: &[&str]) {
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
}

fn mock_agent_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir
}

fn mock_bin() -> String {
    mock_agent_dir()
        .join(if cfg!(windows) {
            "mock-agent.exe"
        } else {
            "mock-agent"
        })
        .display()
        .to_string()
        .replace('\\', "/")
}

/// A git fixture project whose config declares one launch profile — its
/// "subscription" — over a registry agent that resolves to the mock binary.
fn fixture(tag: &str, subscription: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-multi-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();

    // A candidate path is the binary FILE, not its directory (see e2e_flota).
    let mock_candidate = mock_bin();
    assert!(
        std::path::Path::new(&mock_bin()).exists(),
        "run `cargo test` at the workspace root"
    );
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"multi-e2e\"\n\
             [[agents]]\nid = \"provider-a\"\nname = \"Provider A\"\nlevel = 1\n\
             bin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock_candidate}']\n"
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
            "[agent]\ncommand = ['{}']\n\n\
             [fleet]\nregistry = '{registry}'\n\n\
             [[fleet.profile]]\nname = \"{subscription}\"\nagent = \"provider-a\"\n\
             env = {{ MELTEMI_MOCK_MARKER = \"{subscription}-ctx\" }}\n",
            mock_bin()
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
        format!(r"\\.\pipe\meltemid-e2e-multi-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-multi-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base =
        std::env::temp_dir().join(format!("meltemi-e2e-multid-{}-{tag}", std::process::id()));
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
                name: "e2e-multi-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Dispatches the fixture's only task under a named profile.
async fn dispatch(peer: &Peer, root: &Path, subscription: &str) {
    peer.request(
        methods::WORKTREE_DISPATCH,
        &json!({
            "projectRoot": root.display().to_string(),
            "change": "dark-mode",
            "task": "1.1",
            "agent": subscription,
        }),
    )
    .await
    .unwrap_or_else(|e| panic!("dispatch under {subscription}: {e}"));
}

/// The spelling the registry keeps, which is not always the one we asked with:
/// `project/register` canonicalizes before recording, and a temporary directory
/// is exactly where that shows. A Windows runner hands out the 8.3 short form
/// (`C:\Users\RUNNER~1\…`) and macOS reaches `/var` through a symlink into
/// `/private/var`, so the raw path and the recorded one differ on both — while
/// a developer's own temp directory has neither and agrees by accident.
fn registry_spelling(root: &Path) -> String {
    let Ok(resolved) = root.canonicalize() else {
        return root.display().to_string();
    };
    #[cfg(windows)]
    {
        let shown = resolved.to_string_lossy();
        if let Some(plain) = shown.strip_prefix(r"\\?\")
            && !plain.starts_with("UNC\\")
        {
            return plain.to_string();
        }
    }
    resolved.display().to_string()
}

fn project_of<'a>(list: &'a Value, root: &Path) -> &'a Value {
    let wanted = registry_spelling(root);
    list["projects"]
        .as_array()
        .expect("projects array")
        .iter()
        .find(|p| p["root"] == wanted)
        .unwrap_or_else(|| panic!("project `{wanted}` missing in {list:#}"))
}

#[tokio::test]
async fn two_projects_two_subscriptions_are_listed_and_aggregatable() {
    // Scenario: Dos proyectos listados por recencia
    // Scenario: Proyecto sin sesiones vivas sigue presente
    // Scenario: Listado global agregable por proyecto
    // Scenario: Sesión de un perfil declara su suscripción
    // Scenario: El listado nunca lleva el contexto de autenticación
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // SAFETY: single-threaded test setup before any session runs.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let alpha = fixture("alpha", "work");
    let beta = fixture("beta", "personal");
    let (endpoint, daemon) = spawn_daemon("pair").await;
    let peer = init_client(&endpoint).await;

    // Real use in both projects, each under its own subscription.
    dispatch(&peer, &alpha, "work").await;
    dispatch(&peer, &beta, "personal").await;

    // The registry knows both, with their counters and their roots.
    let projects = peer
        .request(methods::PROJECT_LIST, &json!({}))
        .await
        .expect("project/list ok");
    let a = project_of(&projects, &alpha);
    let b = project_of(&projects, &beta);
    assert_eq!(a["exists"], true, "got: {projects:#}");
    assert_eq!(b["exists"], true);
    assert!(
        a["sessionsTotal"].as_u64().unwrap() >= 1,
        "alpha counts its session: {projects:#}"
    );
    assert!(b["sessionsTotal"].as_u64().unwrap() >= 1);
    assert!(
        !a["firstSeenAt"].as_str().unwrap().is_empty(),
        "the registry records when the project was first seen"
    );

    // ONE unfiltered query brings both projects' sessions, each with its root,
    // its resolved agent and the subscription that ran it: everything a client
    // needs to aggregate the tree without a new method (design D7).
    let all = peer
        .request(methods::SESSION_LIST, &json!({}))
        .await
        .expect("session/list ok");
    let sessions = all["sessions"].as_array().expect("sessions array");
    assert!(sessions.len() >= 2, "both sessions are listed: {all:#}");

    let under = |needle: &Path| -> Vec<&Value> {
        let prefix = needle.display().to_string();
        sessions
            .iter()
            .filter(|s| {
                s["projectRoot"]
                    .as_str()
                    .is_some_and(|r| r.starts_with(&prefix))
            })
            .collect()
    };
    let in_alpha = under(&alpha);
    let in_beta = under(&beta);
    assert_eq!(in_alpha.len(), 1, "alpha's session is scoped by its root");
    assert_eq!(in_beta.len(), 1);
    // The same underlying agent, two distinguishable subscriptions.
    assert_eq!(in_alpha[0]["agentId"], "provider-a", "got: {all:#}");
    assert_eq!(in_beta[0]["agentId"], "provider-a");
    assert_eq!(in_alpha[0]["profile"], "work");
    assert_eq!(in_beta[0]["profile"], "personal");
    // Fair play §2: the profile's env overlay is never transported.
    let raw = serde_json::to_string(&all).unwrap();
    assert!(
        !raw.contains("MELTEMI_MOCK_MARKER") && !raw.contains("work-ctx"),
        "no session field carries the profile's environment: {raw}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&alpha);
    let _ = std::fs::remove_dir_all(&beta);
}

#[tokio::test]
async fn a_project_whose_root_moved_stays_listed_as_absent() {
    // Scenario: Raíz desaparecida se conserva marcada
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // SAFETY: single-threaded test setup before any session runs.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let root = fixture("moved", "work");
    let (endpoint, daemon) = spawn_daemon("moved").await;
    let peer = init_client(&endpoint).await;
    dispatch(&peer, &root, "work").await;

    // The folder goes away (moved elsewhere, or deleted).
    let _ = std::fs::remove_dir_all(&root);

    let projects = peer
        .request(methods::PROJECT_LIST, &json!({}))
        .await
        .expect("project/list ok");
    let entry = project_of(&projects, &root);
    assert_eq!(
        entry["exists"], false,
        "the vanished root is reported absent, not dropped: {projects:#}"
    );
    assert!(
        entry["sessionsTotal"].as_u64().unwrap() >= 1,
        "its history is still counted"
    );

    // The caller can ask for only what is still on disk.
    let existing = peer
        .request(methods::PROJECT_LIST, &json!({ "existingOnly": true }))
        .await
        .expect("project/list filtered ok");
    let wanted = root.display().to_string();
    assert!(
        existing["projects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["root"] != wanted),
        "existingOnly hides it, the unfiltered list keeps it: {existing:#}"
    );

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn a_directory_is_registered_and_forgotten_over_the_wire() {
    // The two verbs, reachable as verbs: a folder that never ran anything enters
    // the registry by contract and leaves the listing the same way.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-multi-plain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root_str = root.display().to_string();

    let (endpoint, daemon) = spawn_daemon("registry").await;
    let peer = init_client(&endpoint).await;

    let registered = peer
        .request(methods::PROJECT_REGISTER, &json!({ "root": root_str }))
        .await
        .expect("project/register ok");
    assert_eq!(registered["project"]["sessionsTotal"], 0, "{registered:#}");
    let listed = peer
        .request(methods::PROJECT_LIST, &json!({}))
        .await
        .expect("project/list ok");
    assert_eq!(project_of(&listed, &root)["exists"], true);

    let forgotten = peer
        .request(methods::PROJECT_FORGET, &json!({ "root": root_str }))
        .await
        .expect("project/forget ok");
    assert_eq!(forgotten["forgotten"], true, "{forgotten:#}");
    let after = peer
        .request(methods::PROJECT_LIST, &json!({}))
        .await
        .expect("project/list ok");
    let recorded = registry_spelling(&root);
    assert!(
        !after["projects"]
            .as_array()
            .expect("projects")
            .iter()
            .any(|project| project["root"] == recorded.as_str()),
        "the listing dropped it: {after:#}"
    );
    // Nothing on disk went with it.
    assert!(root.is_dir());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
