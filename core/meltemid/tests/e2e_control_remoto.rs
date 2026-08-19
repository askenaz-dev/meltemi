// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of directing an existing session (control-remoto-asistido),
//! driving an ephemeral daemon and the scripted `mock-agent` against temporary
//! fixtures:
//!
//! - directing an ACTIVE session queues the instruction and dispatches it as the
//!   next turn of the same agent session, without interrupting the turn in
//!   progress, auditing both the queueing and the dispatch in the JSONL;
//! - directing a terminated-but-RESUMABLE session resumes it, linked to the
//!   original;
//! - a non-existent or non-resumable session refuses with 2004;
//! - cancelling a session with a non-empty queue leaves consistent state: the
//!   queued instruction is recorded but never dispatched.
//!
//! Runs against temporary fixtures, never this repo, and the CI `mock-agent`,
//! never a real agent or the network (constitution §"tests e2e").

use std::path::PathBuf;
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
        format!(r"\\.\pipe\meltemid-e2e-cr-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-cr-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo whose config points the agent at the mock with `mock_args`,
/// and whose permissions allow writes so a turn runs without client escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-cr-{}-{tag}", std::process::id()));
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
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("cr-{tag}"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    // Permissions are resolved by the allow rule, so no permission request ever
    // reaches the client; drain any daemon-initiated traffic anyway.
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-cr-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Polls `session/list` until a LIVE session for `root` appears, returning its
/// id. Fails the test if none appears within a few seconds.
///
/// Live, not `active` specifically, and the difference is a race this helper
/// used to lose. A session whose first permission escalates goes to
/// `waiting_permission` within microseconds of reaching `active`; a poll that
/// looked only for `active` caught that window on a developer's machine and
/// missed it on every CI runner, where it failed on all three platforms at
/// once. What the callers actually mean is "a session exists and has not
/// ended", and that is what is asked for now.
async fn wait_for_live_session(peer: &Peer, root: &str) -> String {
    // Every state the contract calls alive. Spelled out here rather than
    // imported so the test states its own expectation.
    const LIVE: [&str; 4] = [
        "starting",
        "active",
        "waiting_permission",
        "waiting_instruction",
    ];
    let mut seen = String::new();
    for _ in 0..200 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list");
        if let Some(sessions) = list["sessions"].as_array() {
            if let Some(live) = sessions
                .iter()
                .find(|s| LIVE.iter().any(|state| s["state"] == *state))
            {
                return live["sessionId"].as_str().unwrap().to_string();
            }
            seen = sessions
                .iter()
                .map(|s| s["state"].as_str().unwrap_or("?").to_string())
                .collect::<Vec<_>>()
                .join(", ");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no live session appeared for {root}; the states seen were: [{seen}]");
}

/// The event `type`s of a session's JSONL log, in order.
async fn log_event_types(peer: &Peer, root: &str, session_id: &str) -> Vec<String> {
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
        .map(|e| e["type"].as_str().unwrap_or("").to_string())
        .collect()
}

/// Waits until a turn is actually IN FLIGHT — the prompt reached the agent —
/// rather than merely until the session exists. An interruption asked for
/// before the turn starts has no turn to stop, and the daemon now answers
/// `queued` for it, which is the truth and not what this test is about.
async fn wait_for_turn_in_flight(peer: &Peer, root: &str, session_id: &str) {
    for _ in 0..400 {
        if log_event_types(peer, root, session_id)
            .await
            .iter()
            .any(|t| t == "prompt_sent")
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no turn reached the agent for {session_id}");
}

#[tokio::test]
async fn directing_an_active_session_dispatches_as_the_next_turn() {
    // Scenario: Instrucción a una sesión activa se despacha como siguiente turno
    let root = fixture("active", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("active").await;
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

    // While it runs, direct an instruction: it queues as the next turn.
    let session_id = wait_for_live_session(&peer, &root_str).await;
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "also add a light theme",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued", "{directed:#}");
    assert_eq!(directed["queuePosition"], 1);
    assert_eq!(directed["sessionId"], session_id);

    // The proposal turn ran, then the directed instruction ran as a second turn.
    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    let types = log_event_types(&peer, &root_str, &session_id).await;
    let prompts = types.iter().filter(|t| *t == "prompt_sent").count();
    assert_eq!(
        prompts, 2,
        "initial prompt + the directed instruction: {types:?}"
    );
    let queued_at = types.iter().position(|t| t == "instruction_queued");
    let second_prompt_at = types
        .iter()
        .enumerate()
        .filter(|(_, t)| *t == "prompt_sent")
        .nth(1)
        .map(|(i, _)| i);
    assert!(
        queued_at.is_some(),
        "the instruction was recorded: {types:?}"
    );
    assert!(
        queued_at < second_prompt_at,
        "queued before it was dispatched (log-before-enqueue): {types:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn directing_a_resumable_session_resumes_it_linked() {
    // Scenario: Instrucción a una sesión reanudable la reanuda
    let root = fixture("resume", &["--load-session"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("resume").await;
    let peer = init_client(&endpoint).await;

    // Run a proposal to completion; the load-capable mock leaves it resumable.
    peer.request(
        methods::PROPOSE,
        &json!({ "idea": "add dark mode", "projectRoot": root_str }),
    )
    .await
    .expect("propose ok");

    // Find the ended, resumable session.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let original = list["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["resumable"] == true)
        .expect("a resumable session");
    let original_id = original["sessionId"].as_str().unwrap().to_string();

    // Directing it resumes as a NEW session linked to the original.
    let resumed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": original_id,
                "instruction": "now add a light theme too",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(resumed["disposition"], "resumed", "{resumed:#}");
    assert_eq!(resumed["resumedFrom"], original_id);
    assert_eq!(resumed["status"], "completed");
    assert_ne!(
        resumed["sessionId"], original_id,
        "resume mints a new session"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_non_directable_session_refuses_with_a_remedy() {
    // Scenario: Sesión no dirigible rehúsa con remedio
    let root = fixture("refuse", &[]); // no --load-session: not resumable
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("refuse").await;
    let peer = init_client(&endpoint).await;

    // An unknown session id refuses with SESSION_NOT_FOUND (2004).
    let unknown = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": "no-such-session",
                "instruction": "do something",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect_err("unknown session refuses");
    assert_eq!(unknown.code, 2004, "{unknown}");

    // A finished, NON-resumable session (the mock did not announce load) also
    // refuses — it cannot be resumed.
    peer.request(
        methods::PROPOSE,
        &json!({ "idea": "add dark mode", "projectRoot": root_str }),
    )
    .await
    .expect("propose ok");
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    let ended = list["sessions"].as_array().unwrap()[0]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let refused = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": ended,
                "instruction": "resume me",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect_err("a non-resumable ended session refuses");
    assert_eq!(refused.code, 2004, "{refused}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn directing_a_live_non_directable_session_says_it_is_active() {
    // Scenario: Sesión no dirigible rehúsa con remedio
    // A running session that drives its own turn loop (here, `explore`) is not
    // directable — but it is LIVE, so the refusal must say so, never misreport a
    // running session as ended-and-not-resumable (frontera honesta).
    let root = fixture("live-nondirect", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("live-nondirect").await;
    let peer = init_client(&endpoint).await;

    // `explore` opens a session that does not accept direction; hold it open.
    let explore = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::SDD_EXPLORE,
                &json!({ "projectRoot": root, "topic": "shape the idea" }),
            )
            .await
        })
    };

    let session_id = wait_for_live_session(&peer, &root_str).await;
    let refused = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "steer me",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect_err("a live non-directable session refuses");
    assert_eq!(refused.code, 2004, "{refused}");
    let message = refused.to_string().to_lowercase();
    assert!(
        message.contains("does not accept direction") && !message.contains("ended"),
        "the refusal is honest about the session being active, not ended: {refused}"
    );

    let _ = tokio::time::timeout(Duration::from_secs(30), explore).await;
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn cancelling_with_a_queued_instruction_leaves_it_undispatched() {
    // Scenario: Dirigir no interrumpe ni cancela
    // Scenario: Una cancelación sigue terminando la sesión
    //
    // The prudence interrupting was added ALONGSIDE, never in place of:
    // cancelling still ends the session, and a queue that is not empty is not a
    // reason to keep going (redirigir-turno design D5).
    let root = fixture("cancel", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cancel").await;
    let peer = init_client(&endpoint).await;

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

    // Queue an instruction during the active turn, then cancel the session.
    let session_id = wait_for_live_session(&peer, &root_str).await;
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "this must never run",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued");
    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));

    // The proposal returns; the queued instruction was recorded but never
    // dispatched — a cancel stops further turns (D2).
    let _ = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join");

    let types = log_event_types(&peer, &root_str, &session_id).await;
    assert!(
        types.iter().any(|t| t == "instruction_queued"),
        "the instruction is recorded (visible loss, not a silent one): {types:?}"
    );
    let prompts = types.iter().filter(|t| *t == "prompt_sent").count();
    assert_eq!(
        prompts, 1,
        "only the initial prompt ran; the queued instruction was not dispatched: {types:?}"
    );
    // The session is finalized (deregistered): no longer active.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    // Not "no longer active" — no longer ALIVE. Since `sesion-que-espera` a
    // session that stops running a turn parks in `waiting_instruction` instead
    // of ending, so asking only about `active` would let a cancellation that
    // failed to end the session pass this guard.
    let still_live = list["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["sessionId"] == session_id)
        .map(|s| {
            [
                "starting",
                "active",
                "waiting_permission",
                "waiting_instruction",
            ]
            .iter()
            .any(|state| s["state"] == *state)
        })
        .unwrap_or(false);
    assert!(!still_live, "cancelling ended the session: {list:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn interrupting_relays_the_turn_and_the_session_keeps_going() {
    // Scenario: La instrucción releva al turno interrumpido
    // Scenario: El registro dice quién detuvo el turno
    //
    // `--honor-cancel` is what makes this a real interruption rather than a
    // race the mock would have won anyway: without it the mock ignores
    // `session/cancel` and the turn ends on its own schedule, which proves
    // nothing about who stopped it.
    let root = fixture("relay", &["--turn-delay-ms", "5000", "--honor-cancel"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("relay").await;
    let peer = init_client(&endpoint).await;

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

    let session_id = wait_for_live_session(&peer, &root_str).await;
    wait_for_turn_in_flight(&peer, &root_str, &session_id).await;
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "stop, do the light theme first",
                "projectRoot": root_str,
                "interrupt": true,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "relayed", "{directed:#}");

    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");

    let types = log_event_types(&peer, &root_str, &session_id).await;

    // The session did NOT end at the interruption: a second prompt ran, and it
    // ran the instruction that relayed it.
    let prompts = types.iter().filter(|t| *t == "prompt_sent").count();
    assert_eq!(
        prompts, 2,
        "the relay ran as the next turn instead of ending the session: {types:?}"
    );

    // And the history says who stopped it. This is the whole point of the event:
    // `turn_completed { cancelled }` would read the same whether the agent gave
    // up or a human redirected it.
    assert!(
        types.iter().any(|t| t == "turn_interrupted"),
        "the interruption is distinguishable from an agent that stopped itself: {types:?}"
    );
    assert!(
        types.iter().position(|t| t == "turn_interrupted")
            < types
                .iter()
                .enumerate()
                .filter(|(_, t)| *t == "prompt_sent")
                .nth(1)
                .map(|(i, _)| i),
        "recorded before the relay was dispatched: {types:?}"
    );
    assert_eq!(result["status"], "completed", "{result:#}");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn interrupting_resolves_the_permission_left_hanging() {
    // Scenario: El permiso en vuelo se resuelve al interrumpir
    //
    // The fixture deliberately carries NO allow rule: the mock's permission
    // escalates to the client, the client never answers, and the turn sits
    // there. That wait is the one an interruption has to be able to end —
    // the agent is blocked on OUR answer, so it cannot honour a cancel it
    // never gets to read.
    let root = fixture("perm-relay", &["--honor-cancel"]);
    let _ = std::fs::remove_file(root.join(".meltemi").join("permissions.toml"));
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("perm-relay").await;
    let peer = init_client(&endpoint).await;

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

    let session_id = wait_for_live_session(&peer, &root_str).await;
    wait_for_turn_in_flight(&peer, &root_str, &session_id).await;

    // Wait until the request is actually pending: interrupting before it exists
    // would pass for the wrong reason.
    let mut waited = false;
    for _ in 0..200 {
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("permission/pending");
        if pending["pending"].as_array().is_some_and(|p| !p.is_empty()) {
            waited = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(waited, "the permission reached the tray");

    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "never mind, do the light theme",
                "projectRoot": root_str,
                "interrupt": true,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "relayed", "{directed:#}");

    // The wait ended, and it is recorded with its ending. Deliberately NOT by
    // awaiting the propose: the relay is a whole new turn that asks its own
    // permission, and a test that waited for the flow to finish would be
    // measuring the SECOND wait, not the one the interruption was meant to end.
    let mut interrupted_decision = None;
    for _ in 0..400 {
        let log = peer
            .request(
                methods::SESSION_LOG,
                &json!({ "sessionId": session_id, "projectRoot": root_str }),
            )
            .await
            .expect("session/log");
        interrupted_decision = log["lines"]
            .as_array()
            .expect("lines")
            .iter()
            .filter_map(|l| serde_json::from_str::<Value>(l.as_str().unwrap()).ok())
            .find(|e| {
                e["type"] == "permission_decided" && e["payload"]["decidedBy"] == "interrupted"
            });
        if interrupted_decision.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let decision = interrupted_decision
        .expect("the hanging permission was resolved by the interruption, not left to expire");
    // `Cancelled` is what went back to the agent, and it still counts as a
    // denial: an unanswered request that stopped counting would let a turn a
    // human cut short report "no denials".
    assert_eq!(
        decision["payload"]["outcome"]["outcome"], "cancelled",
        "{decision:#}"
    );
    assert_eq!(decision["payload"]["denied"], true, "{decision:#}");

    propose.abort();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_turn_the_agent_cancelled_by_itself_still_ends_the_session() {
    // Scenario: Un turno cancelado por el agente no continúa
    //
    // `--cancel-turn` makes the mock report `Cancelled` with nobody having
    // asked for it. That is NOT an invitation to keep sending it work: an agent
    // that gave up on a turn is a different fact from a human redirecting one,
    // and the older prudence stays exactly as it was.
    // The delay is only so the instruction can be queued WHILE the turn runs:
    // without it the session ends before there is anything to queue into.
    let root = fixture("self-cancel", &["--turn-delay-ms", "3000", "--cancel-turn"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("self-cancel").await;
    let peer = init_client(&endpoint).await;

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

    let session_id = wait_for_live_session(&peer, &root_str).await;
    wait_for_turn_in_flight(&peer, &root_str, &session_id).await;

    // Queue an instruction WITHOUT asking to interrupt. The agent cancels its
    // own turn; the queued instruction must not be dispatched into a session
    // that is ending.
    let directed = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session_id,
                "instruction": "and a light theme",
                "projectRoot": root_str,
            }),
        )
        .await
        .expect("session/direct");
    assert_eq!(directed["disposition"], "queued", "{directed:#}");

    let _ = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("the session ended rather than continuing");

    let types = log_event_types(&peer, &root_str, &session_id).await;
    assert_eq!(
        types.iter().filter(|t| *t == "prompt_sent").count(),
        1,
        "the queued instruction was never dispatched into a cancelled turn: {types:?}"
    );
    assert!(
        !types.iter().any(|t| t == "turn_interrupted"),
        "nobody interrupted anything: the agent stopped itself: {types:?}"
    );
    assert!(
        types.iter().any(|t| t == "session_ended"),
        "and the session ended, as it always did: {types:?}"
    );

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// The full log, parsed, for the tests that need payloads and not just types.
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

// Scenario: Se cambia por la vía estándar cuando el agente la anuncia
#[tokio::test]
async fn an_announced_option_is_set_over_the_open_connection_without_relaunching() {
    // The mock announces its options and honours ACP's own
    // `session/set_config_option` — behind a flag, so every other e2e keeps
    // reading the log it was written against. The turn is held open so the
    // change happens while the session is genuinely live.
    let root = fixture(
        "live-config",
        &["--turn-delay-ms", "3000", "--config-options"],
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("live-config").await;
    let peer = init_client(&endpoint).await;

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

    let session_id = wait_for_live_session(&peer, &root_str).await;
    // Wait for the turn to actually reach the agent, not merely for the session
    // to exist: the announcement arrives with the handshake, and a session in
    // `starting` has not had one yet. Reading earlier was a race this test lost
    // on the first run.
    wait_for_turn_in_flight(&peer, &root_str, &session_id).await;

    // The announcement is in the log, which is the only place a surface reads
    // it: the handshake's facts land where the session's history is read.
    let announced = log_events(&peer, &root_str, &session_id)
        .await
        .into_iter()
        .find(|e| e["type"] == "config_options_announced")
        .expect("the agent's announcement is recorded");
    let options = announced["payload"]["options"]
        .as_array()
        .expect("options")
        .clone();
    let model = options
        .iter()
        .find(|o| o["id"] == "model")
        .expect("the mock announces a model selector");
    assert_eq!(model["type"], "select", "{model:#}");
    assert_eq!(model["currentValue"], "fast", "{model:#}");

    // The change, over the connection that is already open.
    let changed = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "model", "value": "slow" }),
        )
        .await
        .expect("session/set-config-option");
    let now = changed["options"]
        .as_array()
        .expect("options")
        .iter()
        .find(|o| o["id"] == "model")
        .expect("the model option is still announced")
        .clone();
    assert_eq!(
        now["currentValue"], "slow",
        "what the AGENT reports is the record: {changed:#}"
    );

    // The toggle takes the other branch of the same verb, so both ACP kinds
    // are exercised without a provider.
    let toggled = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "thinking", "value": "true" }),
        )
        .await
        .expect("session/set-config-option");
    let thinking = toggled["options"]
        .as_array()
        .expect("options")
        .iter()
        .find(|o| o["id"] == "thinking")
        .expect("the toggle is still announced")
        .clone();
    assert_eq!(thinking["currentValue"], true, "{toggled:#}");

    // A value the agent never announced is refused rather than sent: Meltemi
    // does not speak in the agent's name.
    let refused = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "model", "value": "invented" }),
        )
        .await
        .expect_err("an unannounced value is refused");
    assert_eq!(refused.code, 2007, "{refused}");

    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    // NOTHING was relaunched: one session, started once, and the same id
    // throughout. A change that relaunched would show as a second start.
    let types = log_event_types(&peer, &root_str, &session_id).await;
    assert_eq!(
        types.iter().filter(|t| *t == "session_started").count(),
        1,
        "the session was never relaunched: {types:?}"
    );
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list");
    assert_eq!(
        list["sessions"].as_array().expect("sessions").len(),
        1,
        "and no second session was created: {list:#}"
    );
    // Each accepted change is recorded where the session's history is read —
    // the handshake's announcement plus one per change.
    assert_eq!(
        types
            .iter()
            .filter(|t| *t == "config_options_announced")
            .count(),
        3,
        "the announcement and both changes are in the log: {types:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Sin opción anunciada no se ofrece el cambio en vivo
#[tokio::test]
async fn an_agent_that_announced_nothing_gets_no_live_change_offered_for_it() {
    // The same mock WITHOUT the flag: it announces nothing, which is the
    // ordinary case and the one every real adapter is in today.
    let root = fixture("no-config", &["--turn-delay-ms", "3000"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("no-config").await;
    let peer = init_client(&endpoint).await;

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

    let session_id = wait_for_live_session(&peer, &root_str).await;
    // Same wait as its twin, and for the same reason: before the handshake the
    // refusal is a different one — there is no open connection yet — and that
    // is not the refusal this scenario is about.
    wait_for_turn_in_flight(&peer, &root_str, &session_id).await;

    // No announcement in the log. This absence IS the answer a surface reads:
    // the control exists only where the event does, so an agent that announced
    // nothing gets no selector invented in its name.
    let types = log_event_types(&peer, &root_str, &session_id).await;
    assert!(
        !types.iter().any(|t| t == "config_options_announced"),
        "nothing was announced, so nothing is recorded: {types:?}"
    );

    // And the daemon refuses rather than attempting it: a surface that offered
    // it anyway would still not get a value sent in the agent's name.
    let refused = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "model", "value": "slow" }),
        )
        .await
        .expect_err("an agent that announced nothing has nothing to set");
    assert_eq!(refused.code, 2007, "{refused}");
    let said = format!("{refused}");
    assert!(
        said.contains("announced no configuration options"),
        "the refusal says WHY, so a surface can tell it from a wrong id: {said}"
    );

    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
