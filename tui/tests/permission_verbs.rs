// SPDX-License-Identifier: Apache-2.0

//! The permission verbs end to end (verbos-del-agent-boss): a session another
//! surface opened waits in the daemon's queue — a raw client that never answers
//! the permission push, like a tray left open — and the shipped grammar lists
//! it and decides it. An in-process daemon, the scripted `mock-agent`, and
//! temporary fixtures: never this repository, never a real agent, never the
//! network.
//!
//! Written as a single test run in phases, because it clears a process-global
//! environment variable: being the only reader keeps that mutation free of
//! concurrent threads.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::task::JoinHandle;

use meltemi_client::rpc::{Peer, RpcError};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

use common::{FakeDaemon, mock_agent_bin, spawn_daemon, test_endpoint};

/// A fixture project whose agent is the mock with `mock_args`, with no rule of
/// any kind: every request waits for a human. `extra` is appended to its
/// configuration.
fn fixture(tag: &str, mock_args: &[&str], extra: &str) -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo build --workspace` first",
        mock.display()
    );
    let mut command = format!("'{}'", mock.display().to_string().replace('\\', "/"));
    for arg in mock_args {
        command.push_str(&format!(", '{arg}'"));
    }
    let root =
        std::env::temp_dir().join(format!("meltemi-cli-decide-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = [{command}]\n{extra}"),
    )
    .unwrap();
    root
}

fn root_str(root: &Path) -> String {
    root.display().to_string()
}

/// A client of the daemon, like another surface.
async fn client(
    endpoint: &str,
) -> (
    Peer,
    tokio::sync::mpsc::UnboundedReceiver<meltemi_client::rpc::Incoming>,
) {
    let stream = meltemi_client::transport::connect(endpoint)
        .await
        .expect("the test daemon accepts");
    let (peer, incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "decide-e2e".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// Another surface with a session open: it started the session and keeps its
/// connection, and it never answers the permission push — its tray is open and
/// nobody is at it. The session's requests wait in the daemon's queue.
struct Surface {
    peer: Peer,
    turn: JoinHandle<Result<Value, RpcError>>,
    _reading: JoinHandle<()>,
}

async fn open_session(endpoint: &str, root: &Path, instruction: &str) -> Surface {
    let (peer, mut incoming) = client(endpoint).await;
    // Reading keeps the connection alive; nothing read is ever answered.
    let reading = tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    let turn = tokio::spawn({
        let peer = peer.clone();
        let params = json!({ "projectRoot": root_str(root), "instruction": instruction });
        async move { peer.request(methods::SESSION_START, &params).await }
    });
    Surface {
        peer,
        turn,
        _reading: reading,
    }
}

impl Surface {
    /// The session's turn, once the decisions let it end.
    async fn finished(self) -> Value {
        let result = tokio::time::timeout(Duration::from_secs(30), self.turn)
            .await
            .expect("the turn ended")
            .expect("the turn task")
            .expect("session/start answered");
        self.peer.close();
        result
    }
}

/// The queue, as a second client reads it.
async fn queue(endpoint: &str) -> Vec<Value> {
    let (peer, _incoming) = client(endpoint).await;
    let pending = peer
        .request(methods::PERMISSION_PENDING, &json!({}))
        .await
        .expect("the queue");
    peer.close();
    pending["pending"].as_array().cloned().unwrap_or_default()
}

/// Waits for a request other than `known` to wait in the queue, and returns it.
async fn next_request(endpoint: &str, known: &[String]) -> Value {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(entry) = queue(endpoint).await.into_iter().find(|entry| {
                entry["expired"] == false
                    && !known.contains(&entry["requestId"].as_str().unwrap_or("").to_string())
            }) {
                return entry;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("a request came to wait in the queue")
}

/// Whether the queue still lists `request` as waiting.
async fn still_waiting(endpoint: &str, request: &str) -> bool {
    queue(endpoint)
        .await
        .iter()
        .any(|entry| entry["requestId"] == request && entry["expired"] == false)
}

/// Runs one command line through the shipped grammar and dispatch.
async fn run(endpoint: &str, line: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = line.iter().map(|arg| (*arg).to_string()).collect();
    let plan = meltemi::cli::plan(&args, false);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = tokio::time::timeout(
        Duration::from_secs(60),
        meltemi::dispatch(plan, endpoint, false, &mut out, &mut err),
    )
    .await
    .expect("the command answered");
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// The records of a session's log.
async fn records(endpoint: &str, root: &Path, session_id: &str) -> Vec<Value> {
    let (peer, _incoming) = client(endpoint).await;
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str(root), "sessionId": session_id, "limit": 1000 }),
        )
        .await
        .expect("the session's log");
    peer.close();
    log["lines"]
        .as_array()
        .expect("log lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
        .collect()
}

/// The permission decisions a session's log recorded.
async fn decisions(endpoint: &str, root: &Path, session_id: &str) -> Vec<Value> {
    records(endpoint, root, session_id)
        .await
        .into_iter()
        .filter(|event| event["type"] == "permission_decided")
        .map(|event| event["payload"].clone())
        .collect()
}

fn id_of(entry: &Value) -> String {
    entry["requestId"]
        .as_str()
        .expect("a request id")
        .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_permission_verbs_end_to_end() {
    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("decide").await;

    the_listing_and_the_simple_approval(&endpoint).await;
    the_agents_rejection(&endpoint).await;
    several_approvals_and_options_held_to_the_agents_list(&endpoint).await;
    denying_without_a_rejection_cancels(&endpoint).await;
    what_no_longer_waits_is_not_decided(&endpoint).await;
    from_a_shell_with_no_terminal(&endpoint).await;

    daemon.abort();
    a_decision_that_loses_the_race(&endpoint).await;
}

// Scenario: permissions muestra lo que espera y desde cuándo
// Scenario: approve elige la aprobación simple del agente
async fn the_listing_and_the_simple_approval(endpoint: &str) {
    let root = fixture("approve", &["--titled-permission"], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let waiting = next_request(endpoint, &[]).await;
    let request = id_of(&waiting);
    let session = waiting["sessionId"].as_str().unwrap().to_string();

    // The listing opens with its totals and shows what the queue carries.
    let (code, stdout, stderr) = run(endpoint, &["permissions"]).await;
    assert_eq!(code, 0, "{stderr}");
    let first = stdout.lines().next().unwrap_or_default();
    assert_eq!(first, "1 waiting for a decision", "{stdout}");
    let row = stdout
        .lines()
        .find(|line| line.trim_start().starts_with(&request))
        .unwrap_or_else(|| panic!("a row for {request}: {stdout}"));
    for shown in [session.as_str(), "edit", "Write the proposal", "waited "] {
        assert!(row.contains(shown), "`{shown}` in the row: {row}");
    }
    assert!(
        stdout.contains("options: `allow` Allow (allow once) · `reject` Reject (reject once)"),
        "{stdout}"
    );

    // Under --json, exactly the daemon's queue, as one object.
    let (code, stdout, _) = run(endpoint, &["--json", "permissions"]).await;
    assert_eq!(code, 0);
    let listed: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(listed["pending"][0]["requestId"], request.as_str());

    // `approve` takes the agent's single one-time approval, and says which.
    let (code, stdout, stderr) = run(endpoint, &["approve", &request]).await;
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains(&format!(
            "approved {request} with `allow` — Allow (allow once)"
        )),
        "{stdout}"
    );
    let result = surface.finished().await;
    assert_eq!(result["deniedPermissions"], 0, "{result}");
    let decided = decisions(endpoint, &root, &session).await;
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert_eq!(decided[0]["decidedBy"], "client", "{decided:?}");
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "allow" })
    );

    // Scenario: Una petición que ya no espera no se decide
    // Decided once, it waits no more: a second decision is refused with 12.
    let (code, _, stderr) = run(endpoint, &["deny", &request]).await;
    assert_eq!(code, 12, "{stderr}");
    assert!(stderr.contains("is not waiting for a decision"), "{stderr}");
    assert_eq!(decisions(endpoint, &root, &session).await.len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: deny elige el rechazo del agente o cancela
async fn the_agents_rejection(endpoint: &str) {
    let root = fixture("deny", &[], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let waiting = next_request(endpoint, &[]).await;
    let request = id_of(&waiting);
    let session = waiting["sessionId"].as_str().unwrap().to_string();

    let (code, stdout, stderr) = run(endpoint, &["--json", "deny", &request]).await;
    assert_eq!(code, 0, "{stderr}");
    let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(said["decision"], "deny", "{said}");
    assert_eq!(said["optionId"], "reject", "{said}");
    assert_eq!(said["optionKind"], "reject_once", "{said}");
    assert_eq!(said["status"], "applied", "{said}");

    let result = surface.finished().await;
    assert_eq!(result["deniedPermissions"], 1, "{result}");
    let decided = decisions(endpoint, &root, &session).await;
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "reject" }),
        "the agent's own rejection, not a cancellation"
    );
    assert_eq!(decided[0]["decidedBy"], "client");
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Varias aprobaciones exigen nombrar una
// Scenario: approve rehúsa una opción que el agente no ofreció
async fn several_approvals_and_options_held_to_the_agents_list(endpoint: &str) {
    // `--ask` opens with the agent's own question: two one-time approvals and
    // no rejection.
    let root = fixture("several", &["--ask"], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let question = next_request(endpoint, &[]).await;
    let request = id_of(&question);
    let session = question["sessionId"].as_str().unwrap().to_string();

    for (line, words, lists) in [
        (
            vec!["approve", request.as_str()],
            "2 one-time approvals",
            true,
        ),
        (
            vec!["approve", request.as_str(), "--option", "nope"],
            "offered no option `nope`",
            true,
        ),
        (
            vec!["deny", request.as_str(), "--option", "option-0"],
            "is an approval (allow once), not a rejection",
            false,
        ),
    ] {
        let (code, _, stderr) = run(endpoint, &line).await;
        assert_eq!(code, 2, "{line:?}: {stderr}");
        assert!(stderr.contains(words), "{line:?}: {stderr}");
        if lists {
            assert!(
                stderr.contains("`option-0` Rewrite it (recommended) (allow once)"),
                "the agent's options are listed: {stderr}"
            );
        }
        assert!(stderr.contains("nothing was decided"), "{stderr}");
        assert!(
            still_waiting(endpoint, &request).await,
            "{line:?} decided nothing"
        );
    }

    // Named, the option decides — and the agent answers with it.
    let (code, stdout, stderr) =
        run(endpoint, &["approve", &request, "--option", "option-1"]).await;
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("with `option-1` — Patch it"), "{stdout}");

    // The agent goes on to ask for the write: denied, and the turn ends.
    let write = next_request(endpoint, std::slice::from_ref(&request)).await;
    let (code, _, stderr) = run(endpoint, &["deny", &id_of(&write)]).await;
    assert_eq!(code, 0, "{stderr}");
    surface.finished().await;
    let said = records(endpoint, &root, &session)
        .await
        .iter()
        .any(|event| event.to_string().contains("answering with option-1"));
    assert!(said, "the agent continued with the option that was named");
    let _ = std::fs::remove_dir_all(&root);
}

async fn denying_without_a_rejection_cancels(endpoint: &str) {
    let root = fixture("cancel", &["--ask"], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let question = next_request(endpoint, &[]).await;
    let request = id_of(&question);
    let session = question["sessionId"].as_str().unwrap().to_string();

    let (code, stdout, stderr) = run(endpoint, &["deny", &request]).await;
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("the agent offered no rejection, so it was cancelled"),
        "{stdout}"
    );
    let write = next_request(endpoint, std::slice::from_ref(&request)).await;
    let (code, _, _) = run(endpoint, &["deny", &id_of(&write)]).await;
    assert_eq!(code, 0);
    surface.finished().await;
    let decided = decisions(endpoint, &root, &session).await;
    assert_eq!(decided[0]["outcome"], json!({ "outcome": "cancelled" }));
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un comodín no es un id
async fn what_no_longer_waits_is_not_decided(endpoint: &str) {
    // A request that waits, to prove the refusals below leave it alone.
    let root = fixture("waits", &[], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let waiting = next_request(endpoint, &[]).await;
    let request = id_of(&waiting);

    for line in [
        vec!["approve", "all"],
        vec!["approve", "*"],
        vec!["deny", "all"],
        vec!["approve", "perm-999999"],
    ] {
        let (code, _, stderr) = run(endpoint, &line).await;
        assert_eq!(code, 12, "{line:?}: {stderr}");
        assert!(stderr.contains("nothing was decided"), "{line:?}: {stderr}");
        assert!(
            still_waiting(endpoint, &request).await,
            "{line:?} decided nothing"
        );
    }
    // Under --json the refusal is one object carrying the code.
    let (code, stdout, stderr) = run(endpoint, &["--json", "approve", "all"]).await;
    assert_eq!(code, 12);
    let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(said["error"]["code"], 12, "{said}");
    assert_eq!(said["error"]["kind"], "not_waiting", "{said}");
    assert!(stderr.is_empty(), "{stderr}");

    let (code, _, _) = run(endpoint, &["deny", &request]).await;
    assert_eq!(code, 0);
    surface.finished().await;
    let _ = std::fs::remove_dir_all(&root);

    // A request that expired under the daemon's wait policy is listed for a
    // grace, and not decided.
    let root = fixture("expired", &[], "\n[permissions]\nwait = 1\n");
    let surface = open_session(endpoint, &root, "look around").await;
    let waiting = next_request(endpoint, &[]).await;
    let request = id_of(&waiting);
    tokio::time::timeout(Duration::from_secs(30), async {
        while !queue(endpoint)
            .await
            .iter()
            .any(|entry| entry["requestId"] == request.as_str() && entry["expired"] == true)
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the request expired");
    let (code, _, stderr) = run(endpoint, &["approve", &request]).await;
    assert_eq!(code, 12, "{stderr}");
    assert!(
        stderr.contains("expired under the daemon's wait policy"),
        "{stderr}"
    );
    surface.finished().await;
    let _ = std::fs::remove_dir_all(&root);
}

/// The shipped binary as `ssh <pc> meltemi …` runs it: stdin closed, stdout
/// and stderr pipes — no terminal anywhere.
async fn binary(endpoint: &str, args: &[&str]) -> std::process::Output {
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_meltemi"))
        .args(args)
        .env("MELTEMI_ENDPOINT", endpoint)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // A failing assertion must not leave the binary running: an orphan
        // holds the built executable open and the next build fails on a lock.
        .kill_on_drop(true)
        .spawn()
        .expect("spawn meltemi");
    tokio::time::timeout(Duration::from_secs(60), child.wait_with_output())
        .await
        .expect("the command answered without a terminal")
        .expect("its output")
}

// Scenario: Aprobar sin terminal desde otra conexión
async fn from_a_shell_with_no_terminal(endpoint: &str) {
    let root = fixture("remote", &[], "");
    let surface = open_session(endpoint, &root, "look around").await;
    let waiting = next_request(endpoint, &[]).await;
    let request = id_of(&waiting);
    let session = waiting["sessionId"].as_str().unwrap().to_string();

    let listed = binary(endpoint, &["permissions"]).await;
    let stdout = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed.status.success(),
        "{stdout} / {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(stdout.starts_with("1 waiting for a decision"), "{stdout}");
    assert!(stdout.contains(&request), "{stdout}");

    let approved = binary(endpoint, &["approve", &request]).await;
    let stdout = String::from_utf8_lossy(&approved.stdout);
    assert!(
        approved.status.success(),
        "{stdout} / {}",
        String::from_utf8_lossy(&approved.stderr)
    );
    assert!(stdout.contains(&format!("approved {request}")), "{stdout}");

    // The session goes on with its turn, decided by this client.
    let result = surface.finished().await;
    assert_eq!(result["deniedPermissions"], 0, "{result}");
    let decided = decisions(endpoint, &root, &session).await;
    assert_eq!(decided[0]["decidedBy"], "client", "{decided:?}");
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "allow" })
    );
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Una petición que ya no espera no se decide
async fn a_decision_that_loses_the_race(_real: &str) {
    // Waiting when read, resolved by another surface before this decision
    // arrives: the daemon answers `already_resolved`. No real daemon stages
    // that on cue, so the test plays it.
    let endpoint = test_endpoint("decide-lost-race");
    let mut listener = meltemi_client::transport::Listener::bind(&endpoint)
        .await
        .expect("bind");
    let decide = tokio::spawn({
        let endpoint = endpoint.clone();
        async move { run(&endpoint, &["--json", "approve", "perm-7"]).await }
    });
    let mut daemon = FakeDaemon::accept(&mut listener).await;
    daemon.initialize().await;
    daemon
        .answer(
            methods::PERMISSION_PENDING,
            json!({ "pending": [{
                "requestId": "perm-7",
                "sessionId": "s-7",
                "tool": "edit",
                "summary": "Write the proposal",
                "options": [
                    { "optionId": "allow", "name": "Allow", "kind": "allow_once" },
                    { "optionId": "reject", "name": "Reject", "kind": "reject_once" },
                ],
                "waitingSeconds": 3,
                "expired": false,
            }] }),
        )
        .await;
    daemon
        .answer(
            methods::PERMISSION_DECIDE,
            json!({ "status": "already_resolved" }),
        )
        .await;
    let (code, stdout, stderr) = tokio::time::timeout(Duration::from_secs(10), decide)
        .await
        .expect("the decision answered")
        .expect("the decision task");
    assert_eq!(code, 12, "{stdout} / {stderr}");
    let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(said["error"]["code"], 12, "{said}");
    assert!(
        said["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("the first decision wins")),
        "{said}"
    );
    drop(daemon);
}
