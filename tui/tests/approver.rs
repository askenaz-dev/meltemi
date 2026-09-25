// SPDX-License-Identifier: Apache-2.0

//! The terminal approver end to end (aprobador-en-el-cli): the shipped grammar
//! and dispatch, an in-process daemon, the scripted `mock-agent`, and a
//! scripted human at the keys — never this repository, never a real agent,
//! never the network, never a real terminal.
//!
//! Written as a single test run in phases, because it clears a process-global
//! environment variable: being the only reader keeps that mutation free of
//! concurrent threads.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use meltemi::approver::keys::{Arming, RawCounts, Scripted, ScriptedKeys};
use meltemi::approver::{Approver, Key};
use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

use common::{mock_agent_bin, spawn_daemon};

/// What the approver wrote: the stand-in for stderr.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<u8>>>);

impl Write for Said {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Said {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// A scripted human: the approver, what it wrote, and how often the keys
/// entered and left raw mode.
struct Human {
    approver: Approver,
    said: Said,
    counts: RawCounts,
}

fn human(script: impl IntoIterator<Item = Scripted>) -> Human {
    let keys = ScriptedKeys::new(script);
    let counts = keys.counts();
    let said = Said::default();
    let approver = Approver::asking(Box::new(keys), Box::new(said.clone()), Arming::SCRIPTED);
    Human {
        approver,
        said,
        counts,
    }
}

/// A fixture project whose agent is the mock with `mock_args`, with no rule of
/// any kind: every request reaches the human. `extra` is appended to its
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
        std::env::temp_dir().join(format!("meltemi-cli-approver-{}-{tag}", std::process::id()));
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

/// Runs one command line through the shipped grammar and dispatch with this
/// approver, returning the exit code, stdout and stderr.
async fn run(endpoint: &str, args: &[String], approver: Approver) -> (i32, String, String) {
    let plan = meltemi::cli::plan(args, false);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = tokio::time::timeout(
        Duration::from_secs(60),
        meltemi::dispatch_with(plan, endpoint, false, &mut out, &mut err, approver),
    )
    .await
    .expect("the command answered");
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// `meltemi --json session <instruction> <root>`.
fn session(instruction: &str, root: &Path) -> Vec<String> {
    vec![
        "--json".into(),
        "session".into(),
        instruction.into(),
        root_str(root),
    ]
}

/// The one JSON object a `--json` command printed.
fn answer_of(stdout: &str) -> Value {
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is exactly one JSON value ({e}): {stdout}"))
}

/// A second client of the daemon, like the tray of another surface.
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
                name: "approver-e2e".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// The permission decisions a session's log recorded.
async fn decisions(endpoint: &str, root: &Path, session_id: &str) -> Vec<Value> {
    let (peer, _incoming) = client(endpoint).await;
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str(root), "sessionId": session_id }),
        )
        .await
        .expect("the session's log");
    peer.close();
    log["lines"]
        .as_array()
        .expect("log lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
        .filter(|event| event["type"] == "permission_decided")
        .map(|event| event["payload"].clone())
        .collect()
}

/// Waits until the scripted human has been asked something.
async fn asked(said: &Said) {
    tokio::time::timeout(Duration::from_secs(30), async {
        while !said.text().contains("permission requested by session") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the question was asked");
}

// Scenario: Una tecla elige una opción del agente
// Scenario: Denegar elige el rechazo del agente
// Scenario: Sin terminal se deniega con el rechazo del agente
// Scenario: La salida de máquina queda intacta con el aprobador activo
// Scenario: Ctrl+C deniega y cancela el turno
// Scenario: Una pregunta que vence se retira del terminal
// Scenario: Una pregunta resuelta por otra vía se retira del terminal
// Scenario: El terminal vuelve a su estado al terminar
// Scenario: La regla del terminal gobierna la siguiente sesión
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_terminal_approver_end_to_end() {
    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("approver").await;

    a_key_chooses_and_stdout_keeps_one_object(&endpoint).await;
    deny_chooses_the_agents_rejection(&endpoint).await;
    without_a_terminal_the_agents_rejection(&endpoint).await;
    ctrl_c_cancels_and_exits_thirteen(&endpoint).await;
    an_expired_question_is_withdrawn(&endpoint).await;
    a_question_decided_elsewhere_is_withdrawn(&endpoint).await;
    the_remembered_rule_governs_the_next_session(&endpoint).await;

    daemon.abort();
}

async fn a_key_chooses_and_stdout_keeps_one_object(endpoint: &str) {
    let root = fixture("key", &[], "");
    let human = human([Scripted::Key(Key::Char('1'))]);
    let (code, stdout, _) = run(endpoint, &session("look around", &root), human.approver).await;
    assert_eq!(code, 0, "{stdout}");

    // stdout carries the one answer, and nothing of the question.
    let answer = answer_of(&stdout);
    assert!(!stdout.contains("permission requested"), "{stdout}");
    let said = human.said.text();
    assert!(said.contains("permission requested by session"), "{said}");
    assert!(said.contains("  1 Allow  (allow once)"), "{said}");
    assert_eq!(answer["deniedPermissions"], 0, "{answer}");

    // The agent got the option the key chose, decided by the client.
    let session_id = answer["sessionId"].as_str().expect("a session id");
    let decided = decisions(endpoint, &root, session_id).await;
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert_eq!(decided[0]["decidedBy"], "client");
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "allow" })
    );
    assert_eq!(human.counts.entered(), human.counts.left());
    let _ = std::fs::remove_dir_all(&root);
}

async fn deny_chooses_the_agents_rejection(endpoint: &str) {
    let root = fixture("deny", &[], "");
    let human = human([Scripted::Key(Key::Char('d'))]);
    let (code, stdout, _) = run(endpoint, &session("look around", &root), human.approver).await;
    assert_eq!(code, 0, "{stdout}");
    let answer = answer_of(&stdout);
    assert_eq!(answer["deniedPermissions"], 1, "{answer}");
    let decided = decisions(endpoint, &root, answer["sessionId"].as_str().unwrap()).await;
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "reject" }),
        "the agent's own rejection, not a cancellation"
    );
    assert!(human.said.text().contains("→ denied: Reject (reject once)"));
    let _ = std::fs::remove_dir_all(&root);
}

async fn without_a_terminal_the_agents_rejection(endpoint: &str) {
    let root = fixture("no-terminal", &[], "");
    let said = Said::default();
    let approver = Approver::denying(Box::new(said.clone()));
    let (code, stdout, _) = run(endpoint, &session("look around", &root), approver).await;
    assert_eq!(code, 0, "{stdout}");
    let answer = answer_of(&stdout);
    let decided = decisions(endpoint, &root, answer["sessionId"].as_str().unwrap()).await;
    assert_eq!(
        decided[0]["outcome"],
        json!({ "outcome": "selected", "optionId": "reject" })
    );
    let notice = said.text();
    assert!(notice.contains("no terminal to ask in"), "{notice}");
    assert!(notice.contains(".meltemi/permissions.toml"), "{notice}");
    assert!(!notice.contains("--mode"), "{notice}");
    let _ = std::fs::remove_dir_all(&root);
}

async fn ctrl_c_cancels_and_exits_thirteen(endpoint: &str) {
    let root = fixture("ctrl-c", &["--honor-cancel"], "");
    let human = human([Scripted::Key(Key::CtrlC)]);
    let (code, stdout, _) = run(endpoint, &session("look around", &root), human.approver).await;
    assert_eq!(code, 13, "Ctrl+C ends the command as cancelled: {stdout}");
    // stdout still carries exactly one object: what the daemon answered.
    let answer = answer_of(&stdout);
    let decided = decisions(endpoint, &root, answer["sessionId"].as_str().unwrap()).await;
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert_eq!(
        decided[0]["decidedBy"], "interrupted",
        "the cancellation took the request out before any answer: {decided:?}"
    );
    assert!(
        human
            .said
            .text()
            .contains("stopping: the session is being cancelled")
    );
    assert_eq!(human.counts.entered(), human.counts.left());
    let _ = std::fs::remove_dir_all(&root);
}

async fn an_expired_question_is_withdrawn(endpoint: &str) {
    let root = fixture("expired", &[], "\n[permissions]\nwait = 1\n");
    let human = human([Scripted::Hold]);
    let (code, stdout, _) = run(endpoint, &session("look around", &root), human.approver).await;
    assert_eq!(code, 0, "{stdout}");
    let said = human.said.text();
    assert!(
        said.contains("withdrawn: it expired under the daemon's wait policy"),
        "{said}"
    );
    let answer = answer_of(&stdout);
    let decided = decisions(endpoint, &root, answer["sessionId"].as_str().unwrap()).await;
    assert_eq!(decided[0]["decidedBy"], "timeout", "{decided:?}");
    // The terminal was left as often as it was entered.
    assert_eq!(human.counts.entered(), human.counts.left());
    let _ = std::fs::remove_dir_all(&root);
}

async fn a_question_decided_elsewhere_is_withdrawn(endpoint: &str) {
    let root = fixture("elsewhere", &[], "");
    let human = human([Scripted::Hold]);
    let command = tokio::spawn({
        let endpoint = endpoint.to_string();
        let args = session("look around", &root);
        let approver = human.approver.clone();
        async move { run(&endpoint, &args, approver).await }
    });
    // The question is open on the terminal; another surface decides it.
    asked(&human.said).await;
    let (tray, _incoming) = client(endpoint).await;
    let pending = tray
        .request(methods::PERMISSION_PENDING, &json!({}))
        .await
        .expect("the queue");
    // Expired requests of earlier phases stay listed; the open one does not
    // carry that mark.
    let request_id = pending["pending"]
        .as_array()
        .expect("the queue")
        .iter()
        .find(|entry| entry["expired"] == false)
        .and_then(|entry| entry["requestId"].as_str())
        .expect("the open request is queued")
        .to_string();
    let decided = tray
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": "allow" }),
        )
        .await
        .expect("decide");
    assert_eq!(decided["status"], "applied", "{decided}");
    tray.close();

    let (code, stdout, _) = command.await.expect("the command task");
    assert_eq!(code, 0, "{stdout}");
    let said = human.said.text();
    assert!(
        said.contains("withdrawn: it was resolved elsewhere"),
        "{said}"
    );
    assert_eq!(human.counts.entered(), human.counts.left());
    let _ = std::fs::remove_dir_all(&root);
}

async fn the_remembered_rule_governs_the_next_session(endpoint: &str) {
    let root = fixture("remember", &["--titled-permission"], "");
    std::fs::create_dir_all(root.join("notes")).unwrap();
    let path = root.join("notes").join("plan.md");
    let instruction = format!("Write the plan.\nPROPOSAL_PATH: {}", path.display());

    // First session: `r`, then `y`.
    let human = human([Scripted::Key(Key::Char('r')), Scripted::Key(Key::Char('y'))]);
    let (code, stdout, _) = run(endpoint, &session(&instruction, &root), human.approver).await;
    assert_eq!(code, 0, "{stdout}");
    let said = human.said.text();
    assert!(said.contains("r allow and remember"), "{said}");
    assert!(
        said.contains("remember: allow tool edit, paths starting with"),
        "{said}"
    );
    assert!(said.contains("and remembered"), "{said}");
    let rules = std::fs::read_to_string(root.join(".meltemi/permissions.toml"))
        .expect("the rule was written in the project");
    assert!(rules.contains("edit"), "{rules}");
    assert!(path.exists(), "the approval reached the agent");

    // Next session, without a terminal: the rule decides, nobody is asked.
    let said = Said::default();
    let approver = Approver::denying(Box::new(said.clone()));
    let (code, stdout, _) = run(endpoint, &session(&instruction, &root), approver).await;
    assert_eq!(code, 0, "{stdout}");
    assert!(
        said.text().is_empty(),
        "nothing reached the approver: {}",
        said.text()
    );
    let answer = answer_of(&stdout);
    let decided = decisions(endpoint, &root, answer["sessionId"].as_str().unwrap()).await;
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert_eq!(decided[0]["decidedBy"], "rule", "{decided:?}");
    let _ = std::fs::remove_dir_all(&root);
}
