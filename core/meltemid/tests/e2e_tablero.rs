// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the race board's daemon half (tablero-de-carrera),
//! driving an ephemeral daemon and `mock-agent` launch profiles against a
//! temporary **git** fixture:
//!
//! - a competitor dispatch writes a first-class session record (its real level
//!   and its provenance), so the historical listing shows it whole without
//!   walking the logs;
//! - the reconstruction from logs stays a safety net and still recovers the
//!   agent and the subscription from the resolution event;
//! - every dispatched lane of a race declares who ran it, in which session and
//!   how it ended, while a lane nobody dispatched declares nothing at all;
//! - lanes cut from different bases each keep their own.
//!
//! Runs against temporary fixtures, never this repo. Requires `git`; skips if
//! absent. Uses a per-fixture `[fleet] registry` config key (no process env).

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

/// The resolved form of a freshly created directory. The daemon canonicalizes
/// every root it keys by, so a fixture that hands over the raw temp path is
/// comparing two spellings of one directory — which agrees on a developer
/// machine and stops agreeing on a CI runner (8.3 short names on Windows, the
/// `/private/var` symlink on macOS).
fn resolved(path: &Path) -> PathBuf {
    let Ok(canonical) = path.canonicalize() else {
        return path.to_path_buf();
    };
    let shown = canonical.to_string_lossy();
    match shown.strip_prefix(r"\\?\") {
        Some(plain) if !plain.starts_with("UNC\\") => PathBuf::from(plain),
        _ => canonical.clone(),
    }
}

fn mock_agent_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir
}

fn mock_agent_bin() -> PathBuf {
    mock_agent_dir().join(if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    })
}

/// A git fixture whose registry declares a **level 2** provider (the adapter
/// resolves to the mock) and a launch profile over it. Level 2 is the point:
/// a record reconstructed from logs cannot know the level and would list the
/// session at 1, so the level is what tells the index apart from the net.
fn fixture(tag: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-tablero-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();
    let root = resolved(&root);

    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "run `cargo test` at the workspace root ({} is missing)",
        mock.display()
    );
    let mock_path = mock.display().to_string().replace('\\', "/");

    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"tablero-e2e\"\n\
             [[agents]]\nid = \"provider-l2\"\nname = \"Provider L2\"\nlevel = 2\n\
             adapter = \"mock-agent\"\nadapter-args = []\ncli-bin = \"provider-cli\"\n\
             candidate-paths = ['{mock_path}']\n"
        ),
    )
    .unwrap();

    let registry_toml = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock_path}']\n\n\
             [fleet]\nregistry = '{registry_toml}'\n\n\
             [[fleet.profile]]\nname = \"work\"\nagent = \"provider-l2\"\n\
             env = {{ MELTEMI_MOCK_MARKER = \"work-ctx\" }}\n"
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
        format!(
            r"\\.\pipe\meltemid-e2e-tablero-{}-{tag}",
            std::process::id()
        )
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-tablero-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// The endpoint, the daemon task and the daemon's DATA directory — the tests
/// reach into the last one to damage the index on purpose.
async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>, PathBuf) {
    let base =
        std::env::temp_dir().join(format!("meltemi-e2e-tablerod-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    let base = resolved(&base);
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(base.join("data"), base.join("config"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle, base.join("data"))
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
                name: "e2e-tablero-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn dispatch(peer: &Peer, root: &str, task: &str, agent: &str) -> Value {
    peer.request(
        methods::WORKTREE_DISPATCH,
        &json!({ "projectRoot": root, "change": "dark-mode", "task": task, "agent": agent }),
    )
    .await
    .unwrap_or_else(|e| panic!("dispatch {agent}: {e}"))
}

async fn sessions_of(peer: &Peer, root: &str) -> Vec<Value> {
    let listed = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list ok");
    listed["sessions"].as_array().cloned().unwrap_or_default()
}

async fn race(peer: &Peer, root: &str, task: &str) -> Value {
    peer.request(
        methods::WORKTREE_DIFF,
        &json!({ "projectRoot": root, "change": "dark-mode", "task": task }),
    )
    .await
    .expect("worktree/diff ok")
}

/// The lane of one agent in a diff result.
fn lane<'a>(race: &'a Value, agent: &str) -> &'a Value {
    race["competitors"]
        .as_array()
        .expect("competitors")
        .iter()
        .find(|c| c["agent"] == json!(agent))
        .unwrap_or_else(|| panic!("no lane for `{agent}`: {race:#}"))
}

/// Every environment knob a fixture-scoped resolution must not inherit.
fn clear_process_agent_env() {
    // SAFETY: single test process.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }
}

// Scenario: Sesión de despacho listada completa
#[tokio::test]
async fn a_dispatch_session_is_listed_whole_without_reconstruction() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    clear_process_agent_env();
    let root = fixture("record");
    let root_str = root.display().to_string();
    let (endpoint, daemon, _data) = spawn_daemon("record").await;
    let peer = init_client(&endpoint).await;

    let result = dispatch(&peer, &root_str, "1.1", "work").await;
    assert_eq!(result["resolution"]["level"], 2, "{result:#}");
    let session_id = result["sessionId"]
        .as_str()
        .expect("the dispatch names the session it opened")
        .to_string();

    let listed = sessions_of(&peer, &root_str).await;
    let session = listed
        .iter()
        .find(|s| s["sessionId"] == json!(session_id))
        .unwrap_or_else(|| panic!("the dispatch session is listed: {listed:#?}"));

    // The real level, which is the assertion that tells the index apart from
    // the safety net: a record rebuilt from a log defaults to 1.
    assert_eq!(
        session["level"], 2,
        "the real level, not the default: {session:#}"
    );
    assert_eq!(session["agentId"], "provider-l2", "{session:#}");
    assert_eq!(session["profile"], "work", "the subscription that ran it");
    assert_eq!(session["state"], "ended", "the end record completed it");
    assert_eq!(session["finalStatus"], "completed");
    // The lane, not the repository: the session names the tree it ran in, which
    // is what lets a board join a lane to the session that ran it.
    assert_eq!(
        session["projectRoot"],
        json!(result["worktree"].as_str().unwrap()),
        "the record names the worktree the turn ran in: {session:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La red de seguridad recupera la procedencia
#[tokio::test]
async fn the_safety_net_recovers_the_provenance_from_the_log() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    clear_process_agent_env();
    let root = fixture("net");
    let root_str = root.display().to_string();
    let (endpoint, daemon, data) = spawn_daemon("net").await;
    let peer = init_client(&endpoint).await;

    let result = dispatch(&peer, &root_str, "1.1", "work").await;
    let session_id = result["sessionId"]
        .as_str()
        .expect("named session")
        .to_string();

    // Damage the index the way a crash or a wiped file would: the logs are the
    // source of truth, so the listing must survive it.
    let key = meltemid::paths::project_key(&root);
    let index = meltemid::session_index::sessions_dir(&data, &key).join("index.jsonl");
    assert!(
        index.is_file(),
        "the dispatch wrote an index: {}",
        index.display()
    );
    std::fs::remove_file(&index).expect("remove the index");

    let listed = sessions_of(&peer, &root_str).await;
    let session = listed
        .iter()
        .find(|s| s["sessionId"] == json!(session_id))
        .unwrap_or_else(|| panic!("the session is still listed from its log: {listed:#?}"));

    // The resolution event is what carries the agent and the subscription into
    // a reconstruction; both come back.
    assert_eq!(session["agentId"], "provider-l2", "{session:#}");
    assert_eq!(session["profile"], "work", "{session:#}");
    // And the net is a net: the log does not carry the level, so a rebuilt
    // record says 1. That is precisely why the index record exists.
    assert_eq!(
        session["level"], 1,
        "a rebuilt record cannot know the level: {session:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La calle declara procedencia, sesión y estado
#[tokio::test]
async fn every_dispatched_lane_declares_who_ran_it_and_how_it_ended() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    clear_process_agent_env();
    let root = fixture("provenance");
    let root_str = root.display().to_string();
    let (endpoint, daemon, _data) = spawn_daemon("provenance").await;
    let peer = init_client(&endpoint).await;

    // Two lanes resolved two different ways: one through a launch profile, one
    // by naming the catalog id — so "the profile when there was one" is
    // exercised in both directions rather than asserted once.
    let by_profile = dispatch(&peer, &root_str, "1.1", "work").await;
    let by_catalog = dispatch(&peer, &root_str, "1.1", "provider-l2").await;

    let race = race(&peer, &root_str, "1.1").await;
    let profiled = lane(&race, "work");
    assert_eq!(profiled["source"], "profile", "{profiled:#}");
    assert_eq!(profiled["profile"], "work");
    assert_eq!(profiled["level"], 2, "the lane's real level");
    assert_eq!(profiled["committed"], true);
    assert!(
        profiled["sha"].as_str().is_some_and(|s| s.len() >= 7),
        "a committed lane names its head: {profiled:#}"
    );
    assert_eq!(
        profiled["sessionId"], by_profile["sessionId"],
        "the lane and the dispatch name the SAME session"
    );

    let catalogued = lane(&race, "provider-l2");
    assert_eq!(catalogued["source"], "catalog", "{catalogued:#}");
    assert!(
        catalogued["profile"].is_null(),
        "no profile ran it, and none is invented: {catalogued:#}"
    );
    assert_eq!(catalogued["sessionId"], by_catalog["sessionId"]);

    // The two lanes are told apart by their sessions, not merged.
    assert_ne!(profiled["sessionId"], catalogued["sessionId"]);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Los campos aditivos no rompen al cliente anterior
#[tokio::test]
async fn a_lane_without_a_dispatch_shows_no_provenance_at_all() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    clear_process_agent_env();
    let root = fixture("silent");
    let root_str = root.display().to_string();
    let (endpoint, daemon, _data) = spawn_daemon("silent").await;
    let peer = init_client(&endpoint).await;

    // Two lanes assigned; only one of them ever ran a turn.
    peer.request(
        methods::WORKTREE_ASSIGN,
        &json!({
            "projectRoot": root_str,
            "tasks": [{ "change": "dark-mode", "task": "1.1", "agents": ["work", "ghost"] }]
        }),
    )
    .await
    .expect("assign ok");
    dispatch(&peer, &root_str, "1.1", "work").await;

    let race = race(&peer, &root_str, "1.1").await;
    let ghost = lane(&race, "ghost");
    for field in ["source", "profile", "level", "sessionId"] {
        assert!(
            ghost.get(field).is_none(),
            "`{field}` is absent on a lane nobody dispatched, not null and not \
             borrowed from the neighbour: {ghost:#}"
        );
    }
    // The lane still carries what IS known about it: its base and the fact
    // that nothing was committed on it.
    assert_eq!(ghost["committed"], false, "{ghost:#}");
    assert!(ghost.get("sha").is_none(), "nothing committed, no sha");
    assert!(ghost["baseRev"].as_str().is_some_and(|s| !s.is_empty()));
    // And the neighbour's provenance is intact — absence is per lane.
    assert_eq!(lane(&race, "work")["source"], "profile");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Bases divergentes visibles por calle
#[tokio::test]
async fn lanes_cut_from_different_bases_each_keep_their_own() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    clear_process_agent_env();
    let root = fixture("bases");
    let root_str = root.display().to_string();
    let (endpoint, daemon, _data) = spawn_daemon("bases").await;
    let peer = init_client(&endpoint).await;

    // The first lane is cut from the repository's HEAD...
    let first = dispatch(&peer, &root_str, "1.1", "work").await;
    // ...then the repository moves on, and the second lane is cut from there.
    std::fs::write(root.join("moved.txt"), "the base moved\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "move the base"]);
    let second = dispatch(&peer, &root_str, "1.1", "provider-l2").await;
    assert_ne!(first["worktree"], second["worktree"]);

    let race = race(&peer, &root_str, "1.1").await;
    let a = lane(&race, "work")["baseRev"].as_str().expect("own base");
    let b = lane(&race, "provider-l2")["baseRev"]
        .as_str()
        .expect("own base");
    assert_ne!(a, b, "each lane keeps the base it was actually cut from");
    // The result's single base is one of them, and the per-lane fields are
    // what a board must read: fusing them would misname what a diff is against.
    let common = race["baseRev"].as_str().expect("result base");
    assert!(
        common == a || common == b,
        "the result's base is a real one, not a synthesis: {race:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
