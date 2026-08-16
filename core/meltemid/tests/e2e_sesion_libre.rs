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
    // Spelled the way the registry will spell it: every root it records is
    // canonicalized, and a temporary directory is where that diverges from what
    // we typed — an 8.3 short name on a Windows runner, `/var` reached through a
    // symlink into `/private/var` on macOS. A developer's temp directory
    // resolves to itself, which is why only CI would ever see the difference.
    let resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
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

// Scenario: El punto de restauración de una sesión libre no es revertible
#[tokio::test]
async fn reverting_a_free_session_checkpoint_refuses_and_leaves_the_tree_alone() {
    let root = fixture("guard", &[]);
    with_history(&root);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("guard").await;
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
    let session_id = started["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();
    assert!(started["checkpointRef"].is_string(), "{started:#}");

    // The user works after the restore point was taken: an edit to a tracked
    // file, and an untracked file `git clean -fd` would delete without asking.
    std::fs::write(root.join("tracked.txt"), "human work\n").unwrap();
    git(&root, &["add", "tracked.txt"]);
    git(&root, &["commit", "-m", "human"]);
    std::fs::write(root.join("tracked.txt"), "human work, edited\n").unwrap();
    std::fs::write(root.join("untracked.txt"), "not committed anywhere\n").unwrap();

    // Asking what a reversion would do is already refused: a surface has no
    // scope report to render a control from.
    let refused = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({
                "projectRoot": root_str,
                "change": "free",
                "task": session_id,
                "agent": "mock-agent",
            }),
        )
        .await
        .expect_err("a free session's restore point is not revertible");
    assert_eq!(refused.code, meltemi_proto::error_codes::WORKTREE_REFUSED);
    let data = refused.data.clone().expect("a refusal carries data");
    assert!(
        data["remedy"]
            .as_str()
            .unwrap_or_default()
            .contains("git restore"),
        "the remedy points at git, which is what the ref is for: {refused}"
    );

    // Confirming does not buy it either — confirmation is for a worktree, and
    // this is the user's own tree.
    let confirmed = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &json!({
                "projectRoot": root_str,
                "change": "free",
                "task": session_id,
                "agent": "mock-agent",
                "confirm": true,
            }),
        )
        .await
        .expect_err("confirmation does not unlock the user's tree");
    assert_eq!(confirmed.code, meltemi_proto::error_codes::WORKTREE_REFUSED);

    // And the tree is exactly as the human left it.
    assert_eq!(
        std::fs::read_to_string(root.join("tracked.txt")).unwrap(),
        "human work, edited\n",
        "no reset --hard touched the user's edit"
    );
    assert!(
        root.join("untracked.txt").exists(),
        "no clean -fd deleted the user's untracked file"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Sesión libre corre gobernada sin change
// Scenario: El proyecto de una sesión libre queda registrado
// Scenario: Sin parámetro se usa el agente configurado
#[tokio::test]
async fn a_free_session_runs_governed_without_a_change() {
    let root = fixture("governed", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("governed").await;
    let peer = init_client(&endpoint).await;

    // No change, no task, no gate, and no `.meltemi/changes/` anywhere.
    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "read the code and tell me what it does" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");
    let session_id = started["sessionId"].as_str().expect("a session id");
    assert!(
        !root.join(".meltemi").join("changes").exists(),
        "a free session scaffolds no change"
    );

    let events = log_events(&peer, &root_str, session_id).await;

    // Which binary ran, and why that one, from the log alone.
    let resolved = events
        .iter()
        .find(|e| e["type"] == "agent_resolved")
        .unwrap_or_else(|| panic!("the resolution is recorded: {events:#?}"));
    assert!(
        resolved["payload"]["binary"]
            .as_str()
            .unwrap_or_default()
            .contains("mock-agent"),
        "the effective binary is named: {resolved:#}"
    );
    assert_eq!(
        resolved["payload"]["source"], "configured",
        "no agent was named, so the project's configured one ran: {resolved:#}"
    );

    // The permission the agent asked for went through the proxy, and what
    // resolved it is on the record.
    let decided = events
        .iter()
        .find(|e| e["type"] == "permission_decided")
        .unwrap_or_else(|| panic!("the request passed through the proxy: {events:#?}"));
    assert_eq!(decided["payload"]["decidedBy"], "rule", "{decided:#}");

    // And the project is in the registry without the user doing anything else.
    let projects = peer
        .request(methods::PROJECT_LIST, &json!({}))
        .await
        .expect("project/list ok");
    assert!(
        projects["projects"]
            .as_array()
            .expect("projects")
            .iter()
            .any(|p| p["root"] == root_str.as_str()),
        "starting work on a root is pointing Meltemi at it: {projects:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Instrucción de seguimiento se despacha como siguiente turno
#[tokio::test]
async fn a_follow_up_instruction_becomes_the_next_turn_of_the_same_session() {
    // A slow first turn, so the follow-up lands while the session is running:
    // this is a conversation, and the second thing said must not need a second
    // session to say it in.
    let root = fixture("followup", &["--turn-delay-ms", "2500"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("followup").await;
    let peer = init_client(&endpoint).await;

    let start = tokio::spawn({
        let peer = peer.clone();
        let root = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "start looking at the build" }),
            )
            .await
        }
    });

    // Find the live session and direct the follow-up into it.
    let mut session_id = None;
    for _ in 0..300 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
            .await
            .expect("session/list");
        if let Some(active) = list["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|s| s["state"] == "active"))
        {
            session_id = Some(active["sessionId"].as_str().unwrap().to_string());
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let session_id = session_id.expect("the free session is live and listed");

    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "now check the test suite too",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct ok");
    assert_eq!(
        directed["disposition"], "queued",
        "the turn in flight is not interrupted: {directed:#}"
    );
    assert_eq!(directed["queuePosition"], 1);

    let started = tokio::time::timeout(Duration::from_secs(60), start)
        .await
        .expect("session/start returned")
        .expect("join")
        .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");

    let types: Vec<String> = log_events(&peer, &root_str, &session_id)
        .await
        .iter()
        .map(|e| e["type"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        types.iter().filter(|t| *t == "prompt_sent").count(),
        2,
        "the instruction ran as a second turn of the same session: {types:?}"
    );
    let queued_at = types.iter().position(|t| t == "instruction_queued");
    let second_prompt = types
        .iter()
        .enumerate()
        .filter(|(_, t)| *t == "prompt_sent")
        .nth(1)
        .map(|(i, _)| i);
    assert!(
        queued_at.is_some() && queued_at < second_prompt,
        "queued before dispatched, so the steering is auditable: {types:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: El iniciador recibe el arranque sin pedirlo
// Scenario: Resultado final honesto para el cliente scriptable
#[tokio::test]
async fn the_initiator_hears_the_session_start_before_the_agent_speaks() {
    let root = fixture("identity", &["--turn-delay-ms", "1200"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("identity").await;

    // This connection declares interest in nothing at all.
    let stream = connect(&endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
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
    let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel();
    let drain = tokio::spawn(async move {
        while let Some(message) = incoming.recv().await {
            if let meltemid::rpc::Incoming::Notification { method, params } = message
                && method == methods::SESSION_EVENT
            {
                let _ = events_tx.send(params);
            }
        }
    });

    let start = tokio::spawn({
        let peer = peer.clone();
        let root = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "take a look" }),
            )
            .await
        }
    });

    let first = tokio::time::timeout(Duration::from_secs(10), events_rx.recv())
        .await
        .expect("the start arrives while the turn is still running")
        .expect("the stream is open");
    assert_eq!(first["event"]["type"], "session_started", "{first:#}");
    let announced = first["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();
    assert!(
        !announced.is_empty(),
        "a surface can navigate into the conversation with this: {first:#}"
    );

    // The result stays final for the client that listens to nothing: the same
    // id, how the turn ended, and how many permissions were denied.
    let started = tokio::time::timeout(Duration::from_secs(60), start)
        .await
        .expect("session/start returned")
        .expect("join")
        .expect("session/start ok");
    assert_eq!(started["sessionId"], announced.as_str());
    assert_eq!(started["status"], "completed", "{started:#}");
    assert_eq!(started["deniedPermissions"], 0, "{started:#}");

    drain.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Sin cliente conectado la sesión libre no gana privilegios
#[tokio::test]
async fn with_nobody_attending_the_free_session_is_denied_like_any_other() {
    // No permission rules, so the request escalates to a human; zero grace, so
    // the constitutional deny fires as soon as the last client is gone.
    let root = fixture("noclient", &[]);
    std::fs::remove_file(root.join(".meltemi").join("permissions.toml")).unwrap();
    let mock = mock_agent_bin().display().to_string().replace('\\', "/");
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{mock}']\n\n[permissions]\nno-client-grace = 0\n"),
    )
    .unwrap();
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("noclient").await;

    let peer = init_client(&endpoint).await;
    let _start = tokio::spawn({
        let peer = peer.clone();
        let root = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "write something for me" }),
            )
            .await
        }
    });

    // Wait until the request is really escalated and waiting for a human.
    let mut escalated = false;
    for _ in 0..300 {
        let probe = init_client(&endpoint).await;
        let pending = probe
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("pending ok");
        probe.close();
        if pending["pending"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
        {
            escalated = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        escalated,
        "a free session escalates like every other session"
    );

    // The last client leaves. Nobody is attending, so nothing is granted.
    peer.close();

    // An UNINTERRUPTED stretch with nobody connected, before any polling starts.
    //
    // This is the part the test used to get wrong, and it fought itself: every
    // poll opens a connection, and a reader held open IS a client. The deny
    // fires only while the registry is empty, so a loop that connects every
    // 50 ms leaves only the gaps between connections for it to fire in — and on
    // a slow, contended runner a connection's teardown can outlast the gap, so
    // the count is never observed at zero and the deny never comes. It passed
    // here and failed on the Windows runner, which is the signature of a test
    // racing itself rather than a daemon misbehaving.
    //
    // Giving the daemon one clear stretch removes the race instead of widening
    // the budget around it: once the deny has fired, later connections cannot
    // un-fire it.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Every poll from here still opens a connection and closes it again, which
    // no longer matters for the reason above.
    let mut events = Vec::new();
    for _ in 0..300 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let reader = init_client(&endpoint).await;
        let list = reader
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
            .await
            .expect("session/list ok");
        let ended = list["sessions"]
            .as_array()
            .and_then(|sessions| sessions.iter().find(|s| s["state"] == "ended"))
            .map(|session| session["sessionId"].as_str().unwrap().to_string());
        if let Some(id) = ended {
            events = log_events(&reader, &root_str, &id).await;
            reader.close();
            break;
        }
        reader.close();
    }
    assert!(
        !events.is_empty(),
        "the session ended and its log was read; an empty list means it never ended"
    );
    assert!(
        events.iter().any(|e| e["type"] == "permission_decided"
            && e["payload"]["decidedBy"] == "default_deny"
            && e["payload"]["denied"] == true),
        "the constitutional deny is explicit and on the record: {events:#?}"
    );

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Raíz sin git arranca y lo declara
#[tokio::test]
async fn a_root_that_is_not_a_repository_starts_and_declares_it() {
    let root = fixture("nogit", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("nogit").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "have a look around" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("a folder without git is still a place to work");
    assert_eq!(
        started["status"], "completed",
        "the session ran: no restore point is not a refusal: {started:#}"
    );
    assert!(started["checkpointRef"].is_null(), "{started:#}");
    assert_eq!(started["checkpointUnavailable"], "not_a_git_repo");
    assert!(
        started["checkpointRemedy"]
            .as_str()
            .unwrap_or_default()
            .contains("git init"),
        "the remedy for a folder that is not a repository: {started:#}"
    );

    // Nothing was left behind pretending to be a restore point.
    assert!(
        !root.join(".meltemi").join("checkpoints").exists(),
        "no checkpoint machinery is created where it cannot work"
    );
    assert!(!root.join(".meltemi").join("worktrees").exists());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Repositorio sin historia arranca y da el remedio que corresponde
#[tokio::test]
async fn a_repository_with_no_history_starts_and_gets_the_other_remedy() {
    let root = fixture("nohistory", &[]);
    git(&root, &["init"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("nohistory").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "have a look around" }),
        ),
    )
    .await
    .expect("session/start returned")
    .expect("a repository with nothing in it is still a place to work");
    assert_eq!(started["status"], "completed", "{started:#}");
    assert!(started["checkpointRef"].is_null(), "{started:#}");
    assert_eq!(started["checkpointUnavailable"], "no_history");

    let remedy = started["checkpointRemedy"].as_str().unwrap_or_default();
    assert!(
        remedy.contains("first commit"),
        "the remedy is the first commit: {started:#}"
    );
    assert!(
        !remedy.contains("git init"),
        "a repository that already exists must never be told to initialize \
         itself: {started:#}"
    );

    assert!(!root.join(".meltemi").join("worktrees").exists());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// Waits until a session reports the given state, or gives up saying what it
/// found instead.
async fn wait_for_state(
    peer: &Peer,
    root: &str,
    session_id: &str,
    want: &str,
) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..400 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list ok");
        if let Some(found) = list["sessions"]
            .as_array()
            .and_then(|all| all.iter().find(|s| s["sessionId"] == session_id))
        {
            if found["state"] == want {
                return found.clone();
            }
            last = found.clone();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("session never reached `{want}`; last seen: {last:#}");
}

// Scenario: La sesión sobrevive al turno y espera
// Scenario: La instrucción despierta la espera sin reanudar
// Scenario: Arranque desacoplado responde con el identificador
#[tokio::test]
async fn a_detached_session_waits_after_its_turn_and_the_next_instruction_wakes_it() {
    let root = fixture("waits", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("waits").await;
    let peer = init_client(&endpoint).await;

    // Detached: the answer arrives as soon as the session exists, carrying its
    // id and NO status — the turn has not happened yet.
    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({
                "projectRoot": root_str,
                "instruction": "find out why the build is slow",
                "detach": true,
            }),
        ),
    )
    .await
    .expect("a detached start answers without waiting for the turn")
    .expect("session/start ok");
    assert!(
        started["status"].is_null(),
        "a turn that has not happened has no status: {started:#}"
    );
    let session_id = started["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();

    // The turn runs, and the session does NOT end: it waits.
    let waiting = wait_for_state(&peer, &root_str, &session_id, "waiting_instruction").await;
    assert!(
        waiting["endedAt"].is_null(),
        "a waiting session has not ended: {waiting:#}"
    );
    let events = log_events(&peer, &root_str, &session_id).await;
    assert!(
        !events.iter().any(|e| e["type"] == "session_ended"),
        "nothing was finalized: the session is between turns, not over: {events:#?}"
    );

    // The next instruction runs as the NEXT TURN of the same session — queued,
    // never resumed. A resume would answer `resumed` and mint a new session id.
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "now make it faster",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct ok");
    assert_eq!(
        directed["disposition"], "queued",
        "the waiting session took the instruction directly, without resuming: {directed:#}"
    );
    assert_eq!(directed["sessionId"], session_id, "{directed:#}");

    // Two prompts in one session log is the proof that the agent subprocess
    // survived the first turn: a resume would have written its own log.
    for _ in 0..400 {
        let events = log_events(&peer, &root_str, &session_id).await;
        if events.iter().filter(|e| e["type"] == "prompt_sent").count() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let events = log_events(&peer, &root_str, &session_id).await;
    assert_eq!(
        events.iter().filter(|e| e["type"] == "prompt_sent").count(),
        2,
        "the relayed instruction ran in the SAME agent session: {events:#?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La sesión en espera se cancela como cualquier otra
#[tokio::test]
async fn cancelling_a_waiting_session_ends_it() {
    let root = fixture("cancel-waiting", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cancel-waiting").await;
    let peer = init_client(&endpoint).await;

    let started = peer
        .request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "look around", "detach": true }),
        )
        .await
        .expect("session/start ok");
    let session_id = started["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();
    wait_for_state(&peer, &root_str, &session_id, "waiting_instruction").await;

    // A session parked at the boundary waits on the queue's signal and nothing
    // else. If cancelling did not reach it, `session/cancel` would stop meaning
    // anything for a session that is not running a turn — which, after this
    // change, is most of the time.
    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));
    let ended = wait_for_state(&peer, &root_str, &session_id, "ended").await;
    assert!(
        !ended["endedAt"].is_null(),
        "and it was finalized properly, not merely forgotten: {ended:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// A fixture whose sessions stop waiting almost immediately, so the bound can be
/// observed without the test sleeping for the real default.
fn fixture_with_idle(tag: &str, idle_seconds: u64) -> PathBuf {
    let root = fixture(tag, &[]);
    let config = root.join(".meltemi").join("config.toml");
    let existing = std::fs::read_to_string(&config).expect("the fixture wrote a config");
    std::fs::write(
        &config,
        format!(
            "{existing}
[sessions]
idle-timeout = {idle_seconds}
"
        ),
    )
    .expect("config written");
    root
}

// Scenario: Sin pedirlo, el arranque responde como siempre
#[tokio::test]
async fn an_attached_start_answers_at_the_end_with_its_status_as_it_always_did() {
    // The additive half. Everything above changes only for a caller that asked
    // to stay; a caller that did not gets the same answer, at the same moment,
    // with the same fields — including a `status`, which is what makes the
    // scriptable surface usable at all.
    let root = fixture("attached", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("attached").await;
    let peer = init_client(&endpoint).await;

    let started = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "have a look" }),
        ),
    )
    .await
    .expect("an attached start answers when its turn ends, not fifteen minutes later")
    .expect("session/start ok");
    assert_eq!(
        started["status"], "completed",
        "the outcome is in the answer: {started:#}"
    );
    let session_id = started["sessionId"].as_str().expect("a session id");

    // And it ENDED. Not asking to stay is not a request to be kept alive.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let session = list["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .unwrap_or_else(|| panic!("the session is missing from {list:#}"));
    assert_eq!(
        session["state"], "ended",
        "a caller who waited for the outcome did not ask for a session to keep: {list:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Instrucción a una sesión que espera se despacha de inmediato
#[tokio::test]
async fn an_instruction_to_a_waiting_session_runs_at_once_rather_than_resuming() {
    let root = fixture("wakes-now", &[]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("wakes-now").await;
    let peer = init_client(&endpoint).await;

    let started = peer
        .request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "look around", "detach": true }),
        )
        .await
        .expect("session/start ok");
    let session_id = started["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();
    wait_for_state(&peer, &root_str, &session_id, "waiting_instruction").await;

    // The distinguishing fact: `queued`, not `resumed`. A resume would mint a
    // NEW session id and link it — which is what used to happen, and what this
    // change exists to stop happening for a conversation that never ended.
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "and now the tests",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct ok");
    assert_eq!(directed["disposition"], "queued", "{directed:#}");
    assert_eq!(
        directed["sessionId"], session_id,
        "the same session, not a linked successor: {directed:#}"
    );
    assert!(
        directed["resumedFrom"].is_null(),
        "nothing was resumed: {directed:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La espera vencida termina con su motivo
#[tokio::test]
async fn an_expired_wait_ends_with_its_own_reason_and_stays_resumable() {
    // One second, so the bound is observable. The default is fifteen minutes,
    // which is the point: this is measuring the mechanism, not the number.
    let root = fixture_with_idle("idle-expires", 1);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("idle-expires").await;
    let peer = init_client(&endpoint).await;

    let started = peer
        .request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "look around", "detach": true }),
        )
        .await
        .expect("session/start ok");
    let session_id = started["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();

    let ended = wait_for_state(&peer, &root_str, &session_id, "ended").await;
    assert!(!ended["endedAt"].is_null(), "properly finalized: {ended:#}");

    // And it says what actually happened. A session whose wait expired did NOT
    // complete, and recording it as completed would be the log agreeing with a
    // story nobody told it.
    let events = log_events(&peer, &root_str, &session_id).await;
    let end = events
        .iter()
        .find(|e| e["type"] == "session_ended")
        .unwrap_or_else(|| panic!("the session recorded its end: {events:#?}"));
    assert_eq!(
        end["payload"]["reason"], "idle_timeout",
        "the reason is the one that happened, not the flattering one: {end:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
