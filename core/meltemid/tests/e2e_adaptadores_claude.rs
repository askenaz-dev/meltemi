// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of the headless-session adapter (adaptadores-propios-acp
//! task 3.6), with every layer of the real chain in place except the provider:
//!
//! ```text
//! test client ─JSON-RPC→ meltemid ─ACP→ meltemi-claude-acp ─events→ mock-claude-wire
//!                                            ↑                            │
//!                                            └── hook / MCP prompt tool ───┘
//! ```
//!
//! The adapter is the **real binary**, piloted by the real daemon exactly as it
//! pilots any other ACP agent — same permission proxy, same session log, no
//! private channel to the daemon anywhere. What stands in for the provider is
//! the scripted wire, because CI never runs a real agent and never touches the
//! network (constitution §5, design D10).
//!
//! What this proves that the in-memory tests cannot: that the two permission
//! channels the adapter configures are actually usable by the process it
//! configures them for. The scripted wire runs the hook through a real shell
//! and speaks real MCP over stdio to the permission server, both launched from
//! the files the adapter wrote — so the adapter's own quoting, argument order
//! and protocol are exercised here rather than assumed. Both loops close at a
//! human's tray and come back through the daemon's proxy, untouched.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// A session that streams, then asks about one tool through each of the two
/// permission channels the adapter installs.
///
/// The hook decides first and finally, which is the provider's own order; the
/// prompt tool is asked separately here so that the channel a CLI without hooks
/// would use is exercised too.
///
/// The session is announced *after* the first input, where the real CLI
/// announces it (task 5.3): a wire that spoke before being spoken to would let
/// an adapter that waits for the announcement first pass this test, which is
/// exactly what happened until the real binary was run.
const TWO_CHANNELS: &str = r#"
{"mock":"await-input"}
{"mock":"once"}
{"type":"system","subtype":"init","cwd":"/mock/project","session_id":"mock-claude-session","tools":["Read","Write","Bash"],"mcp_servers":[],"model":"mock-sonnet","permissionMode":"default","slash_commands":[],"apiKeySource":"none","claude_code_version":"2.0.0-mock"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Working"}},"session_id":"mock-claude-session"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" on it."}},"session_id":"mock-claude-session"}
{"mock":"ask-hook","tool":"Write","toolUseId":"toolu_gate","input":{"file_path":"NOTES.md","content":"scripted"}}
{"mock":"ask-prompt-tool","tool":"Bash","toolUseId":"toolu_prompt","input":{"command":"echo hola"}}
{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"mock-claude-session","result":"Done.","usage":{"input_tokens":120,"output_tokens":34}}
"#;

/// A workspace binary, next to this test's own executable.
fn workspace_bin(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let path = dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    });
    assert!(
        path.exists(),
        "{name} was not built at {}; run `cargo test` at the workspace root",
        path.display()
    );
    path
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A project whose agent is the real adapter, with the scripted wire standing
/// in for the official CLI.
fn fixture(tag: &str, permissions: Option<&str>) -> PathBuf {
    let adapter = workspace_bin("meltemi-claude-acp");
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-claude-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let script = root.join("two-channels.ndjson");
    std::fs::write(&script, TWO_CHANNELS.trim_start()).unwrap();
    // SAFETY: every test in this binary sets the same values, before any
    // adapter is launched. The adapter inherits them through the daemon, which
    // is the same inheritance a real launch uses.
    unsafe {
        std::env::set_var("MELTEMI_CLAUDE_BIN", workspace_bin("mock-claude-wire"));
        std::env::set_var("MELTEMI_MOCK_CLAUDE_SCRIPT", &script);
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}']\n",
            adapter.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    if let Some(rules) = permissions {
        std::fs::write(root.join(".meltemi").join("permissions.toml"), rules).unwrap();
    }
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> (Peer, mpsc::UnboundedReceiver<Incoming>) {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-claude".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// Everything the session log recorded, as raw events.
async fn session_events(peer: &Peer, root: &str) -> Vec<Value> {
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().expect("sessions array");
    assert!(!sessions.is_empty(), "the session was recorded: {list:#}");
    let id = sessions[0]["sessionId"].as_str().expect("a session id");

    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root, "sessionId": id, "limit": 1000 }),
        )
        .await
        .expect("session/log ok");
    log["lines"]
        .as_array()
        .expect("the log's raw lines")
        .iter()
        .map(|line| {
            serde_json::from_str(line.as_str().expect("a JSONL line"))
                .expect("every logged line is JSON")
        })
        .collect()
}

#[tokio::test]
async fn both_permission_channels_reach_the_human_and_the_turn_streams() {
    // Scenarios: Eventos de sesión mapeados en streaming; Permiso decidido por
    // el proxy vigente; Versión efectiva registrada en el log
    //
    // Not the hard gate over a permissive mode: this wire announces the default
    // mode, so what it shows is the gate deciding, never the gate surviving a
    // mode that would otherwise skip the asking. That is
    // `e2e_adaptadores_claude_bypass`.
    //
    // One turn, all the way down and twice back up. The daemon launches the
    // real adapter; the adapter launches the scripted wire with a hook and an
    // MCP permission server it wrote itself; the wire runs both, exactly as the
    // official CLI runs them — a real shell for the hook, real MCP over stdio
    // for the tool — and each question arrives at this client as a
    // `permission/request` because no rule decides it.
    let root = fixture("both", None);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("claude-both").await;
    let (peer, incoming) = init_client(&endpoint).await;

    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let decider = tokio::spawn({
        let peer = peer.clone();
        let asked = asked.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { id, method, params } = message
                    && method == methods::PERMISSION_REQUEST
                {
                    asked.lock().unwrap().push(
                        params["toolCall"]["toolCallId"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    );
                    let allow = params["options"]
                        .as_array()
                        .expect("options")
                        .iter()
                        .find(|option| option["kind"] == "allow_once")
                        .expect("an allow option")["optionId"]
                        .clone();
                    peer.respond(
                        id,
                        Ok(json!({ "outcome": { "outcome": "selected", "optionId": allow } })),
                    );
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(120),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "what the scripted wire says", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    let events = session_events(&peer, &root_str).await;
    let questions = asked.lock().unwrap().clone();
    assert!(
        questions.contains(&"toolu_gate".to_string()),
        "the hard gate's question reached the human: {questions:?} — {events:#?}"
    );
    assert!(
        questions.contains(&"toolu_prompt".to_string()),
        "and so did the prompt tool's: {questions:?} — {events:#?}"
    );

    let updates: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "agent_update")
        .map(|event| &event["payload"]["update"])
        .collect();

    // The provenance of what was actually launched, and which credential it came
    // up on — the fact nobody can reconstruct afterwards. It arrives as two
    // updates because it is known at two moments: which binary was launched is
    // a fact at `session/new`, and which credential it came up on is a fact only
    // once the CLI has spoken, which this CLI does not do until it has a turn.
    let provenance: Vec<&Value> = updates
        .iter()
        .map(|update| &update["_meta"]["meltemi"])
        .filter(|meta| meta.is_object())
        .collect();
    let launched = provenance
        .iter()
        .find(|meta| meta["providerBin"].is_string())
        .unwrap_or_else(|| panic!("the effective binary is in the log: {updates:#?}"));
    assert!(
        launched["providerBin"]
            .as_str()
            .expect("a binary")
            .contains("mock-claude-wire"),
        "the log names what was launched: {launched:#}"
    );
    assert!(
        launched["providerVersion"]
            .as_str()
            .is_some_and(|version| version.starts_with("2.0.0")),
        "and the version it turned out to be: {launched:#}"
    );
    assert!(
        launched["providerSession"]
            .as_str()
            .is_some_and(|session| session.len() == 36 && session.matches('-').count() == 4),
        "and the id both sides call it, in the shape the CLI's session flag takes \
         — a UUID, dictated rather than learned: {launched:#}"
    );
    let announced = provenance
        .iter()
        .find(|meta| meta["providerKeySource"].is_string())
        .unwrap_or_else(|| panic!("what the CLI announced is in the log too: {updates:#?}"));
    assert_eq!(
        announced["providerKeySource"], "none",
        "under the session the user signed into: {announced:#}"
    );
    assert_eq!(
        announced["providerModel"], "mock-sonnet",
        "with the model it said it would run: {announced:#}"
    );

    // The turn streamed: what the wire sent token by token is in the log.
    let said: String = updates
        .iter()
        .filter(|update| update["sessionUpdate"] == "agent_message_chunk")
        .filter_map(|update| update["content"]["text"].as_str())
        .collect();
    assert!(
        said.contains("Working on it."),
        "the streamed message reached the session log: {updates:#?}"
    );

    // And both approved calls are in the log as tool calls that succeeded,
    // which is what the wire emits when the decision comes back allow.
    for call in ["toolu_gate", "toolu_prompt"] {
        assert!(
            updates
                .iter()
                .any(|update| update["toolCallId"] == call && update["status"] == "completed"),
            "`{call}` ran after being allowed: {updates:#?}"
        );
    }

    decider.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_rule_decides_the_gates_question_without_ever_asking_a_human() {
    // Scenario: Permiso decidido por el proxy vigente
    //
    // The same chain with a rule in place: the proxy resolves it, the client is
    // never asked, and neither the gate nor the adapter decides anything on its
    // own — each relays what came back.
    let root = fixture("rule", Some("[[rule]]\neffect = \"allow\"\n"));
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("claude-rule").await;
    let (peer, incoming) = init_client(&endpoint).await;

    let escalated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watcher = tokio::spawn({
        let escalated = escalated.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { method, .. } = &message
                    && method == methods::PERMISSION_REQUEST
                {
                    escalated.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(120),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "decided by a rule", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");
    assert_eq!(
        result["deniedPermissions"], 0,
        "the rule allowed it: {result:#}"
    );
    assert!(
        !escalated.load(std::sync::atomic::Ordering::SeqCst),
        "a rule decided it; no human was troubled"
    );

    watcher.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
