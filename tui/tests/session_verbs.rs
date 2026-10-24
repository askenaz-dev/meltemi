// SPDX-License-Identifier: Apache-2.0

//! Following, reading and cancelling a session end to end
//! (verbos-del-agent-boss): a detached session another surface opened is
//! followed by the shipped binary with no terminal while the CLI decides its
//! request and cancels it; and, with the test playing the daemon — the
//! bridge's pattern —, the endings no real daemon can be made to stage on cue.
//! An in-process daemon, the scripted `mock-agent`, temporary fixtures: never
//! this repository, never a real agent, never the network.
//!
//! Written as a single test run in phases, because it clears a process-global
//! environment variable and spawns processes that read the environment: being
//! the only thread keeps that mutation free of concurrent readers.

mod common;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use meltemi_client::rpc::Peer;
use meltemi_client::transport::Listener;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

use common::{FakeDaemon, mock_agent_bin, spawn_daemon, test_endpoint};

fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
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
        std::env::temp_dir().join(format!("meltemi-cli-follow-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = [{command}]\n"),
    )
    .unwrap();
    root
}

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
                name: "follow-e2e".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// Another surface holding a detached session open: the session exists as soon
/// as it answers, and its permission pushes are never answered.
struct Surface {
    peer: Peer,
    session: String,
    _reading: JoinHandle<()>,
}

async fn detached_session(endpoint: &str, root: &Path) -> Surface {
    let (peer, mut incoming) = client(endpoint).await;
    let reading = tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    let started = peer
        .request(
            methods::SESSION_START,
            &json!({
                "projectRoot": root.display().to_string(),
                "instruction": "look around",
                "detach": true,
            }),
        )
        .await
        .expect("a detached session starts");
    let session = started["sessionId"]
        .as_str()
        .expect("the session's id")
        .to_string();
    Surface {
        peer,
        session,
        _reading: reading,
    }
}

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

/// The session's state while the daemon lists it as running; `None` once it
/// does not.
async fn live_state(endpoint: &str, session: &str) -> Option<String> {
    let (peer, _incoming) = client(endpoint).await;
    let status = peer
        .request(methods::STATUS, &json!({}))
        .await
        .expect("status");
    peer.close();
    status["sessions"]
        .as_array()?
        .iter()
        .find(|s| s["sessionId"] == session)
        .map(|s| s["state"].as_str().unwrap_or_default().to_string())
}

async fn running(endpoint: &str, session: &str) -> bool {
    live_state(endpoint, session).await.is_some()
}

async fn until(what: &str, mut check: impl AsyncFnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(30), async {
        while !check().await {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what}"));
}

async fn waiting_request(endpoint: &str, session: &str) -> String {
    let mut found = String::new();
    until("the session's request waits in the queue", async || {
        let (peer, _incoming) = client(endpoint).await;
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("the queue");
        peer.close();
        if let Some(entry) = pending["pending"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["sessionId"] == session && entry["expired"] == false)
        {
            found = entry["requestId"].as_str().unwrap().to_string();
            true
        } else {
            false
        }
    })
    .await;
    found
}

/// The shipped binary following a session as `ssh <pc> meltemi watch …` runs
/// it: stdin closed, stdout and stderr pipes. Each line it prints arrives on
/// the channel the moment it is written.
struct Watching {
    child: tokio::process::Child,
    lines: mpsc::UnboundedReceiver<String>,
    seen: Vec<String>,
}

fn watch_binary(endpoint: &str, session: &str) -> Watching {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_meltemi"))
        .args(["watch", session])
        .env("MELTEMI_ENDPOINT", endpoint)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn meltemi watch");
    let stdout = child.stdout.take().expect("its stdout");
    let (tx, lines) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    Watching {
        child,
        lines,
        seen: Vec::new(),
    }
}

impl Watching {
    /// Waits for a printed record of this type, keeping every line seen.
    async fn until_record(&mut self, kind: &str) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = tokio::time::timeout(left, self.lines.recv())
                .await
                .unwrap_or_else(|_| panic!("a `{kind}` record while following: {:?}", self.seen))
                .unwrap_or_else(|| panic!("the stream ended before `{kind}`: {:?}", self.seen));
            let record: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("each line is one JSON object ({e}): {line}"));
            self.seen.push(line);
            if record["type"] == kind {
                return;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn following_reading_and_cancelling_end_to_end() {
    // SAFETY: this test binary runs this single test, and its phases run one
    // after another: nothing reads the environment concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("follow").await;

    let (session, root) = the_whole_session_followed_without_a_terminal(&endpoint).await;
    what_no_longer_runs_is_not_an_error(&endpoint, &session).await;
    an_unknown_session_is_a_contract_error(&endpoint).await;
    stopping_the_watch_leaves_the_session_running(&endpoint).await;
    let _ = std::fs::remove_dir_all(&root);
    daemon.abort();

    the_watch_ends_with_its_daemon().await;
    the_watch_ends_with_its_session_even_without_the_close().await;
}

// Scenario: Seguir sin terminal
// Scenario: watch termina con la sesión
// Scenario: cancel detiene la sesión como la TUI
// Scenario: Lo que watch emite es lo que log guarda
// Scenario: Un flujo emite un objeto por línea
async fn the_whole_session_followed_without_a_terminal(endpoint: &str) -> (String, PathBuf) {
    let root = fixture("whole", &["--titled-permission"]);
    let surface = detached_session(endpoint, &root).await;
    let session = surface.session.clone();
    let mut watching = watch_binary(endpoint, &session);

    // The request is on the stream while the session is blocked on it: the
    // records arrive as they happen, not when the session ends.
    watching.until_record("permission_requested").await;
    let request = waiting_request(endpoint, &session).await;
    let (code, _, stderr) = run(endpoint, &["approve", &request]).await;
    assert_eq!(code, 0, "{stderr}");
    watching.until_record("permission_decided").await;
    // The turn ends and the session parks between turns, alive.
    until("the session parks between turns", async || {
        live_state(endpoint, &session).await.as_deref() == Some("waiting_instruction")
    })
    .await;

    // `cancel` ends it, and says whether it was still closing when it answered.
    let (code, stdout, stderr) = run(endpoint, &["--json", "cancel", &session]).await;
    assert_eq!(code, 0, "{stderr}");
    let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(said["cancel"], "requested", "{said}");
    assert_eq!(said["sessionId"], session.as_str(), "{said}");

    // The watch prints the close last, and ends with the session.
    watching.until_record("session_ended").await;
    let status = tokio::time::timeout(Duration::from_secs(30), watching.child.wait())
        .await
        .expect("the watch ends with its session")
        .expect("wait");
    assert!(status.success(), "{status:?}");
    let mut stderr = String::new();
    let mut pipe = watching.child.stderr.take().expect("its stderr");
    tokio::io::AsyncReadExt::read_to_string(&mut pipe, &mut stderr)
        .await
        .unwrap();
    assert!(stderr.contains("Ctrl+C stops following"), "{stderr}");
    while let Ok(line) = watching.lines.try_recv() {
        watching.seen.push(line);
    }
    let watched: Vec<Value> = watching
        .seen
        .iter()
        .map(|line| serde_json::from_str(line).expect("a JSON record"))
        .collect();
    assert_eq!(
        watched.last().map(|record| record["type"].clone()),
        Some(json!("session_ended"))
    );
    // Parked, it would have waited for an instruction; it ended because the
    // cancel reached it — the close came after it, and nothing else asked.
    // The close is published before the session leaves the registry, a window
    // `follow.rs` names as expected: its leaving is waited for, bounded, rather
    // than asked once (seguir-y-cancelar-sin-carrera, design D1).
    until("the cancelled session is gone", async || {
        !running(endpoint, &session).await
    })
    .await;

    // `log` holds what the watch printed: the same records, in the same
    // order, at the end of the record — one JSON object per line.
    let (code, stdout, stderr) = run(endpoint, &["--json", "log", &session]).await;
    assert_eq!(code, 0, "{stderr}");
    let logged: Vec<Value> = stdout
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("each line is one JSON object ({e}): {line}"))
        })
        .collect();
    assert!(logged.len() >= watched.len(), "{logged:?}");
    assert_eq!(
        &logged[logged.len() - watched.len()..],
        &watched[..],
        "the watch printed the tail of the record"
    );
    assert!(
        logged
            .iter()
            .any(|record| record["type"] == "session_started"),
        "the record from its start"
    );

    surface.peer.close();
    (session, root)
}

// Scenario: Seguir o cancelar una sesión que ya terminó no es un error
async fn what_no_longer_runs_is_not_an_error(endpoint: &str, session: &str) {
    let started = Instant::now();
    let (code, stdout, stderr) = run(endpoint, &["watch", session]).await;
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty(), "nothing to follow: {stdout}");
    assert!(stderr.contains("is not running (ended)"), "{stderr}");
    assert!(
        stderr.contains(&format!("meltemi log {session}")),
        "{stderr}"
    );
    assert!(started.elapsed() < Duration::from_secs(10), "no wait");

    let (code, stdout, stderr) = run(endpoint, &["--json", "cancel", session]).await;
    assert_eq!(code, 0, "{stderr}");
    let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
    assert_eq!(said["cancel"], "not_running", "{said}");
    assert_eq!(said["state"], "ended", "{said}");
}

// Scenario: Una sesión desconocida es error de contrato
async fn an_unknown_session_is_a_contract_error(endpoint: &str) {
    for verb in ["watch", "log", "cancel"] {
        let (code, stdout, stderr) = run(endpoint, &[verb, "no-such-session"]).await;
        assert_eq!(code, 11, "{verb}: {stderr}");
        assert!(stdout.is_empty(), "{verb}: {stdout}");
        assert!(
            stderr.contains("no session `no-such-session` is known to this daemon"),
            "{verb}: {stderr}"
        );
        let (code, stdout, _) = run(endpoint, &["--json", verb, "no-such-session"]).await;
        assert_eq!(code, 11);
        let said: Value = serde_json::from_str(stdout.trim()).expect("one JSON object");
        assert_eq!(said["error"]["code"], 11, "{said}");
        assert_eq!(said["error"]["kind"], "session_not_found", "{said}");
    }
}

// Scenario: Dejar de seguir no cancela la sesión
async fn stopping_the_watch_leaves_the_session_running(endpoint: &str) {
    let root = fixture("stop-following", &["--titled-permission"]);
    let surface = detached_session(endpoint, &root).await;
    let session = surface.session.clone();
    let mut watching = watch_binary(endpoint, &session);
    watching.until_record("permission_requested").await;

    // The watch goes away the way Ctrl+C takes it: the process ends.
    watching.child.kill().await.expect("stop following");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        running(endpoint, &session).await,
        "the session keeps running when nobody follows it"
    );

    // Tidy: the session is cancelled on purpose, and leaves.
    let (code, _, stderr) = run(endpoint, &["cancel", &session]).await;
    assert_eq!(code, 0, "{stderr}");
    until("the cancelled session leaves", async || {
        !running(endpoint, &session).await
    })
    .await;
    surface.peer.close();
    let _ = std::fs::remove_dir_all(&root);
}

/// What a watch asks of the daemon, answered by the test.
trait Watched {
    async fn open(&mut self, session: &str);
    async fn record(&mut self, session: &str, kind: &str);
}

impl Watched for FakeDaemon {
    /// The opening every watch makes: initialize, the interest, and the look
    /// that finds the session running.
    async fn open(&mut self, session: &str) {
        self.initialize().await;
        self.answer(
            methods::SESSION_WATCH,
            json!({ "sessionId": session, "watching": true }),
        )
        .await;
        self.answer(methods::STATUS, status_with(&[session])).await;
    }

    async fn record(&mut self, session: &str, kind: &str) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": methods::SESSION_EVENT,
            "params": {
                "sessionId": session,
                "event": { "v": 1, "ts": "2026-09-25T12:00:00Z", "type": kind, "payload": { "text": "go" } },
            },
        }))
        .await;
    }
}

fn status_with(sessions: &[&str]) -> Value {
    json!({
        "daemonVersion": "0",
        "uptimeSeconds": 1,
        "sessions": sessions
            .iter()
            .map(|id| json!({ "sessionId": id, "agentCommand": ["mock-agent"], "state": "active" }))
            .collect::<Vec<_>>(),
    })
}

/// Runs `line` in process against `endpoint`, in its own task.
fn spawn_run(endpoint: &str, line: &[&str]) -> JoinHandle<(i32, String, String)> {
    let endpoint = endpoint.to_string();
    let line: Vec<String> = line.iter().map(|arg| (*arg).to_string()).collect();
    tokio::spawn(async move {
        let refs: Vec<&str> = line.iter().map(String::as_str).collect();
        run(&endpoint, &refs).await
    })
}

// Scenario: watch no sobrevive a su daemon
// Scenario: Un error a mitad del flujo cierra con un objeto
async fn the_watch_ends_with_its_daemon() {
    let endpoint = test_endpoint("follow-daemon-dies");
    let mut listener = Listener::bind(&endpoint).await.expect("bind");
    let watch = spawn_run(&endpoint, &["--json", "watch", "s-1"]);
    let mut daemon = FakeDaemon::accept(&mut listener).await;
    daemon.open("s-1").await;
    daemon.record("s-1", "prompt_sent").await;
    // The daemon dies: its end of the connection closes.
    let died = Instant::now();
    drop(daemon);

    let (code, stdout, stderr) = tokio::time::timeout(Duration::from_secs(10), watch)
        .await
        .expect("the watch does not outlive its daemon")
        .expect("the watch task");
    assert!(died.elapsed() < Duration::from_secs(5), "at once");
    assert_eq!(code, 10, "{stdout} / {stderr}");
    let lines: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON object per line"))
        .collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert_eq!(lines[0]["type"], "prompt_sent", "the record first");
    assert_eq!(lines[1]["error"]["code"], 10, "then the error closes it");
    assert_eq!(lines[1]["error"]["kind"], "daemon_unreachable");
    assert!(stderr.is_empty(), "{stderr}");
}

// Scenario: watch no sobrevive a la sesión aunque el cierre no llegue
async fn the_watch_ends_with_its_session_even_without_the_close() {
    let endpoint = test_endpoint("follow-session-gone");
    let mut listener = Listener::bind(&endpoint).await.expect("bind");
    let watch = spawn_run(&endpoint, &["watch", "s-2"]);
    let mut daemon = FakeDaemon::accept(&mut listener).await;
    daemon.open("s-2").await;
    daemon.record("s-2", "turn_completed").await;
    // The next look finds the session gone, and no close ever came.
    daemon.answer(methods::STATUS, status_with(&[])).await;

    let (code, stdout, stderr) = tokio::time::timeout(Duration::from_secs(15), watch)
        .await
        .expect("the watch does not outlive its session")
        .expect("the watch task");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stderr.contains("is no longer running"), "{stderr}");
    assert!(stderr.contains("meltemi log s-2"), "{stderr}");
    drop(daemon);
}
