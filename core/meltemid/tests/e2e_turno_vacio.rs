// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the empty-turn verdict (turno-vacio-es-error), driving an
//! ephemeral daemon and the scripted `mock-agent` against temporary fixtures —
//! never this repo, never a real agent, never the network (constitution §5).
//!
//! The case these hold the daemon to was measured on 2026-09-24: an agent
//! whose provider failed closed its turn with `end_turn` having said and done
//! nothing, and Meltemi called it success. The mock's `--silent-turn` is that
//! agent. What the daemon owes is a verdict in the log for every such turn, the
//! same verdict in every response that reports the turn, and never that verdict
//! for a turn a human stopped.

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

fn mock_path() -> String {
    let mock = mock_agent_bin();
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
        format!(r"\\.\pipe\meltemid-e2e-vacio-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-vacio-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// The agent command a fixture configures: the mock with `mock_args`.
fn agent_command(mock_args: &[&str]) -> String {
    let mut command = format!("'{}'", mock_path());
    for arg in mock_args {
        command.push_str(&format!(", '{arg}'"));
    }
    command
}

/// Points an existing fixture's agent at the mock with other arguments. Config
/// is read per session, so the next session runs the new command.
fn configure(root: &Path, mock_args: &[&str]) {
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = [{}]\n", agent_command(mock_args)),
    )
    .unwrap();
}

/// A fixture repo whose agent is the mock with `mock_args`, and whose
/// permissions allow everything so a turn runs without escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-vacio-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    configure(&root, mock_args);
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    resolved(&root)
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

fn git(cwd: &Path, args: &[&str]) {
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
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("vacio-{tag}"), shutdown_tx);
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
                name: "e2e-vacio-client".into(),
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

/// The session log's events, decoded.
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

fn types(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| e["type"].as_str().map(str::to_string))
        .collect()
}

fn count(types: &[String], kind: &str) -> usize {
    types.iter().filter(|t| *t == kind).count()
}

async fn sessions(peer: &Peer, root: &str) -> Vec<Value> {
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list ok");
    list["sessions"].as_array().cloned().unwrap_or_default()
}

async fn wait_for_state(peer: &Peer, root: &str, session_id: &str, want: &str) {
    let mut last = Value::Null;
    for _ in 0..400 {
        if let Some(found) = sessions(peer, root)
            .await
            .into_iter()
            .find(|s| s["sessionId"] == session_id)
        {
            if found["state"] == want {
                return;
            }
            last = found;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("session never reached `{want}`; last seen: {last:#}");
}

/// Polls the log until `kind` appears `n` times.
async fn wait_for_events(peer: &Peer, root: &str, session_id: &str, kind: &str, n: usize) {
    for _ in 0..400 {
        if count(&types(&log_events(peer, root, session_id).await), kind) >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let seen = types(&log_events(peer, root, session_id).await);
    panic!("`{kind}` never appeared {n} time(s): {seen:?}");
}

/// The first session that is live, once one is.
async fn wait_for_live_session(peer: &Peer, root: &str) -> String {
    for _ in 0..400 {
        if let Some(live) = sessions(peer, root).await.into_iter().find(|s| {
            [
                "starting",
                "active",
                "waiting_permission",
                "waiting_instruction",
            ]
            .iter()
            .any(|state| s["state"] == *state)
        }) {
            return live["sessionId"].as_str().unwrap().to_string();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no session came alive");
}

// Scenario: Turno sin palabra ni acción declarado vacío
// Scenario: Un arranque cuyo turno fue vacío lo declara
#[tokio::test]
async fn a_silent_turn_is_declared_empty_in_the_log_and_the_response() {
    let root = fixture("silent", &["--silent-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("silent").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SESSION_START,
        json!({ "projectRoot": root_str, "instruction": "fix the build" }),
    )
    .await;
    assert_eq!(
        started["status"], "completed",
        "the agent's own word stays: {started:#}"
    );
    assert_eq!(started["emptyTurn"], true, "{started:#}");

    let session_id = started["sessionId"].as_str().unwrap();
    let events = log_events(&peer, &root_str, session_id).await;
    let empty: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "turn_empty")
        .collect();
    assert_eq!(empty.len(), 1, "{events:#?}");
    assert_eq!(empty[0]["payload"]["stopReason"], "completed");
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "turn_completed" && e["payload"]["stopReason"] == "completed"),
        "the close keeps the reason the agent reported: {events:#?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un turno con trabajo no deja clave
#[tokio::test]
async fn a_turn_that_carried_work_leaves_no_verdict() {
    let root = fixture("worked", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("worked").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SESSION_START,
        json!({ "projectRoot": root_str, "instruction": "fix the build" }),
    )
    .await;
    assert_eq!(started["status"], "completed", "{started:#}");
    assert!(
        started.get("emptyTurn").is_none(),
        "a turn that carried work leaves no key: {started:#}"
    );
    let events = log_events(&peer, &root_str, started["sessionId"].as_str().unwrap()).await;
    assert_eq!(count(&types(&events), "turn_empty"), 0, "{events:#?}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Sin estado no hay veredicto en la respuesta
#[tokio::test]
async fn a_detached_start_carries_no_verdict_and_the_log_does() {
    let root = fixture("detached", &["--silent-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("detached").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SESSION_START,
        json!({ "projectRoot": root_str, "instruction": "fix the build", "detach": true }),
    )
    .await;
    assert!(
        started.get("status").is_none() && started.get("emptyTurn").is_none(),
        "a turn that has not happened has neither a status nor a verdict: {started:#}"
    );
    let session_id = started["sessionId"].as_str().unwrap().to_string();
    wait_for_events(&peer, &root_str, &session_id, "turn_empty", 1).await;

    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Cada turno de una sesión de varios turnos se mide
#[tokio::test]
async fn every_turn_of_a_session_of_several_turns_is_measured() {
    // Work, then silence, then work: exactly one verdict, between the second
    // prompt and the third. A counter that was never reset between turns would
    // miss it; one that was never read per turn would put it anywhere.
    let root = fixture("several", &["--silent-turn-at", "1"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("several").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SESSION_START,
        json!({ "projectRoot": root_str, "instruction": "look around", "detach": true }),
    )
    .await;
    let session_id = started["sessionId"].as_str().unwrap().to_string();
    wait_for_state(&peer, &root_str, &session_id, "waiting_instruction").await;

    for (n, instruction) in [(2, "now say nothing"), (3, "now say something")] {
        let directed = request(
            &peer,
            methods::SESSION_DIRECT,
            json!({ "sessionId": session_id, "instruction": instruction, "projectRoot": root_str }),
        )
        .await;
        assert_eq!(directed["disposition"], "queued", "{directed:#}");
        wait_for_events(&peer, &root_str, &session_id, "prompt_sent", n).await;
        wait_for_state(&peer, &root_str, &session_id, "waiting_instruction").await;
    }

    let kinds = types(&log_events(&peer, &root_str, &session_id).await);
    assert_eq!(count(&kinds, "turn_empty"), 1, "{kinds:?}");
    let prompts: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| *k == "prompt_sent")
        .map(|(i, _)| i)
        .collect();
    let verdict = kinds.iter().position(|k| k == "turn_empty").unwrap();
    assert!(
        prompts[1] < verdict && verdict < prompts[2],
        "the verdict belongs to the second turn: {kinds:?}"
    );

    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un turno que no terminó no se declara vacío
#[tokio::test]
async fn a_turn_that_did_not_finish_is_not_declared_empty() {
    // Silent, and cancelled by the agent itself: it did not finish, so it says
    // so already, and saying "empty" too would be repeating it.
    let root = fixture("unfinished", &["--silent-turn", "--cancel-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("unfinished").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SESSION_START,
        json!({ "projectRoot": root_str, "instruction": "fix the build" }),
    )
    .await;
    assert_eq!(started["status"], "cancelled", "{started:#}");
    assert!(started.get("emptyTurn").is_none(), "{started:#}");
    let events = log_events(&peer, &root_str, started["sessionId"].as_str().unwrap()).await;
    assert_eq!(count(&types(&events), "turn_empty"), 0, "{events:#?}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un turno que el humano detuvo no se declara vacío
#[tokio::test]
async fn a_turn_a_human_stopped_is_never_declared_empty() {
    // The mock ignores the cancellation, as an agent may, and answers
    // `end_turn` having said nothing. A human stopped it: that is not the
    // provider failing, whatever the agent answered.
    let root = fixture("cancelled", &["--silent-turn", "--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cancelled").await;
    let peer = init_client(&endpoint).await;

    let starting = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "fix the build" }),
            )
            .await
        })
    };
    let session_id = wait_for_live_session(&peer, &root_str).await;
    wait_for_events(&peer, &root_str, &session_id, "prompt_sent", 1).await;
    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));

    let started = tokio::time::timeout(Duration::from_secs(60), starting)
        .await
        .expect("session/start answered")
        .expect("join")
        .expect("session/start ok");
    assert!(
        started.get("emptyTurn").is_none(),
        "a stopped turn is not an empty one: {started:#}"
    );
    let events = log_events(&peer, &root_str, &session_id).await;
    assert_eq!(count(&types(&events), "turn_empty"), 0, "{events:#?}");

    let _ = std::fs::remove_dir_all(&root);

    // And interrupted with a relay: the same answer.
    let root = fixture(
        "interrupted",
        &["--silent-turn-at", "0", "--turn-delay-ms", "3000"],
    );
    let root_str = root.display().to_string();
    let proposing = {
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
    let session_id = wait_for_live_session(&peer, &root_str).await;
    wait_for_events(&peer, &root_str, &session_id, "prompt_sent", 1).await;
    let directed = request(
        &peer,
        methods::SESSION_DIRECT,
        json!({
            "sessionId": session_id,
            "instruction": "stop, do the light theme first",
            "projectRoot": root_str,
            "interrupt": true,
        }),
    )
    .await;
    assert_eq!(directed["disposition"], "relayed", "{directed:#}");
    let proposed = tokio::time::timeout(Duration::from_secs(60), proposing)
        .await
        .expect("propose answered")
        .expect("join")
        .expect("propose ok");
    assert!(proposed.get("emptyTurn").is_none(), "{proposed:#}");
    let kinds = types(&log_events(&peer, &root_str, &session_id).await);
    assert_eq!(count(&kinds, "prompt_sent"), 2, "the relay ran: {kinds:?}");
    assert_eq!(
        count(&kinds, "turn_empty"),
        0,
        "the interrupted turn was stopped, not empty: {kinds:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Propuesta con un turno vacío
#[tokio::test]
async fn a_proposal_whose_turn_was_empty_says_so() {
    let root = fixture("propose", &["--silent-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("propose").await;
    let peer = init_client(&endpoint).await;

    let proposed = request(
        &peer,
        methods::PROPOSE,
        json!({ "idea": "add dark mode", "projectRoot": root_str }),
    )
    .await;
    assert_eq!(proposed["status"], "completed", "{proposed:#}");
    assert_eq!(proposed["emptyTurn"], true, "{proposed:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Despacho con un turno vacío
#[tokio::test]
async fn a_dispatch_whose_turn_was_empty_says_so() {
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-vacio-{}-dispatch", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();
    let root = resolved(&root);
    let mock = mock_path();
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"vacio-e2e\"\n\
             [[agents]]\nid = \"quiet\"\nname = \"Quiet\"\nlevel = 1\n\
             bin = \"mock-agent\"\nacp-args = ['--silent-turn']\ncandidate-paths = ['{mock}']\n"
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
        format!("[agent]\ncommand = ['{mock}']\n\n[fleet]\nregistry = '{registry}'\n"),
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

    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("dispatch").await;
    let peer = init_client(&endpoint).await;

    let dispatched = request(
        &peer,
        methods::WORKTREE_DISPATCH,
        json!({ "projectRoot": root_str, "change": "dark-mode", "task": "1.1", "agent": "quiet" }),
    )
    .await;
    assert_eq!(dispatched["status"], "completed", "{dispatched:#}");
    assert_eq!(dispatched["emptyTurn"], true, "{dispatched:#}");
    let session_id = dispatched["sessionId"].as_str().unwrap();
    let events = log_events(&peer, &root_str, session_id).await;
    assert_eq!(count(&types(&events), "turn_empty"), 1, "{events:#?}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un gate cuyo turno de autoría fue vacío lo declara
#[tokio::test]
async fn a_gate_whose_authoring_turn_was_empty_says_so() {
    // The proposal is written by a mock that works; the next artifact by one
    // that falls silent. Config is read per session, so switching it between
    // the two calls is switching agents.
    let root = fixture("gate", &["--sdd-author"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("gate").await;
    let peer = init_client(&endpoint).await;

    let started = request(
        &peer,
        methods::SDD_PROPOSE,
        json!({ "idea": "add sdd thing", "projectRoot": root_str }),
    )
    .await;
    assert_eq!(started["phase"], "gate_pending", "{started:#}");
    assert!(
        started.get("emptyTurn").is_none(),
        "the proposal turn worked: {started:#}"
    );
    let change = started["changeName"].as_str().unwrap().to_string();

    configure(&root, &["--sdd-author", "--silent-turn"]);
    let gated = request(
        &peer,
        methods::SDD_GATE,
        json!({ "projectRoot": root_str, "changeName": change, "decision": "approve" }),
    )
    .await;
    assert_eq!(gated["emptyTurn"], true, "{gated:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La vía rápida declara el vacío de cualquiera de sus turnos
#[tokio::test]
async fn a_fast_forward_proposal_declares_any_empty_turn() {
    let root = fixture("fast", &["--sdd-author", "--silent-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("fast").await;
    let peer = init_client(&endpoint).await;

    let proposed = request(
        &peer,
        methods::SDD_PROPOSE,
        json!({ "idea": "a trivial thing", "projectRoot": root_str, "forceMode": "fast_forward" }),
    )
    .await;
    assert_eq!(proposed["mode"], "fast_forward", "{proposed:#}");
    assert_eq!(proposed["emptyTurn"], true, "{proposed:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
