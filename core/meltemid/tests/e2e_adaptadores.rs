// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of Meltemi's own ACP adapters (adaptadores-propios-acp task
//! 2.5), with every layer of the real chain in place except the provider:
//!
//! ```text
//! test client ─JSON-RPC→ meltemid ─ACP→ meltemi-codex-acp ─JSON-RPC→ mock-codex-wire
//! ```
//!
//! The adapter is the **real binary**, piloted by the real daemon exactly as it
//! pilots any other ACP agent — same permission proxy, same session log, no
//! private channel anywhere. What stands in for the provider is the scripted
//! wire, because CI never runs a real agent and never touches the network
//! (constitution §5, design D10). The scripted wire speaks the contract the
//! official CLI dumps for itself, and the conformance suite is what keeps it
//! honest about that.
//!
//! What this proves that the in-memory tests cannot: that the adapter's own
//! stdio, argument handling and process supervision work on this platform, and
//! that a permission the provider asks for reaches a human's tray and comes
//! back decided through the daemon's proxy, untouched by the adapter.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

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
///
/// The stand-in is declared by environment variable, which is also the escape
/// hatch a user has when the CLI lives somewhere the PATH does not reach. It is
/// set on this process because the daemon runs inside it here and the adapter
/// inherits from the daemon — the same inheritance a real launch uses.
fn fixture(tag: &str, permissions: Option<&str>) -> PathBuf {
    let adapter = workspace_bin("meltemi-codex-acp");
    let wire = workspace_bin("mock-codex-wire");
    // SAFETY: every test in this binary sets the same value, before any adapter
    // is launched.
    unsafe {
        std::env::set_var("MELTEMI_CODEX_BIN", &wire);
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-adapters-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
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
                name: "e2e-adapters".into(),
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
async fn the_daemon_pilots_the_real_adapter_and_the_provider_reaches_the_permission_tray() {
    // Scenarios: Conversación del servidor mapeada en streaming; Aprobación del
    // servidor decidida por el proxy; Versión efectiva registrada en el log
    //
    // One turn, all the way down: the daemon launches the real adapter, the
    // adapter launches the scripted wire and shakes hands with it, the turn
    // streams, the wire asks to change a file, and that question arrives at this
    // client as a `permission/request` because no rule decides it. The answer
    // travels back down and the turn closes.
    let root = fixture("full", None);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("adapters-full").await;
    let (peer, incoming) = init_client(&endpoint).await;

    let asked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let decider = tokio::spawn({
        let peer = peer.clone();
        let asked = asked.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { id, method, params } = message
                    && method == methods::PERMISSION_REQUEST
                {
                    asked.store(true, std::sync::atomic::Ordering::SeqCst);
                    // The question is about the very item the wire streamed, so
                    // a human sees what they are deciding about.
                    assert_eq!(
                        params["toolCall"]["toolCallId"], "item-2",
                        "the request names the item it is about: {params:#}"
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
        Duration::from_secs(60),
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
    assert!(
        asked.load(std::sync::atomic::Ordering::SeqCst),
        "the provider's approval reached the human's tray through the proxy: {events:#?}"
    );

    let updates: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "agent_update")
        .map(|event| &event["payload"]["update"])
        .collect();

    // The provenance of what was actually launched: the effective binary and the
    // version it turned out to be, recorded like any other agent update.
    let provenance = updates
        .iter()
        .find_map(|update| {
            let meta = &update["_meta"]["meltemi"];
            meta.is_object().then(|| meta.clone())
        })
        .unwrap_or_else(|| panic!("the effective binary is in the log: {updates:#?}"));
    assert!(
        provenance["providerBin"]
            .as_str()
            .expect("a binary")
            .contains("mock-codex-wire"),
        "the log names what was launched: {provenance:#}"
    );
    assert_eq!(provenance["providerVersion"], "0.77.0");

    // And the turn streamed: the message the wire sent in chunks is in the log.
    let said: String = updates
        .iter()
        .filter(|update| update["sessionUpdate"] == "agent_message_chunk")
        .filter_map(|update| update["content"]["text"].as_str())
        .collect();
    assert!(
        said.contains("Working on it."),
        "the streamed message reached the session log: {updates:#?}"
    );

    // The file change the approval was about is in the log too, as a tool call.
    assert!(
        updates.iter().any(|update| {
            update["sessionUpdate"] == "tool_call" && update["toolCallId"] == "item-2"
        }),
        "the item the human decided about was shown before the question: {updates:#?}"
    );

    decider.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_rule_decides_the_providers_approval_without_ever_asking_a_human() {
    // Scenario: Aprobación del servidor decidida por el proxy
    //
    // The same chain with a rule in place: the proxy resolves it, the client is
    // never asked, and the adapter still never decides anything itself — it
    // relays whatever came back.
    let root = fixture("rule", Some("[[rule]]\neffect = \"allow\"\n"));
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("adapters-rule").await;
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
        Duration::from_secs(60),
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
