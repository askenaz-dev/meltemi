// SPDX-License-Identifier: Apache-2.0

//! The hard gate against a CLI that announces its most permissive mode
//! (adaptadores-propios-acp task 3.4), with the real daemon piloting the real
//! adapter binary.
//!
//! This lives in its own test binary because it needs its own scripted wire,
//! and a wire is selected by environment variable — which is per process.
//!
//! **What this can and cannot settle**, said before the assertions rather than
//! after, because the scenario has two halves and only one of them is ours:
//!
//! - Ours, and settled here: the adapter installs the gate and relays the
//!   proxy's answer *whatever mode the CLI announces*. Nothing in the launch,
//!   in the gate or in the session reads `permissionMode` and softens. The
//!   session also records the mode, so a human can see the CLI is running wide
//!   open even while every call is still being decided.
//! - The provider's, and not settled here: that a real CLI in
//!   `bypassPermissions` really does run its `PreToolUse` hook and really does
//!   obey a denial. That is documented by the provider and observable only
//!   against the official binary; a scripted wire consults the hook because it
//!   was written to, which proves the fixture honest and nothing more. It is
//!   recorded as such in the change's `.verify.jsonl`, with what the real CLI
//!   was and was not asked.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// A CLI announcing that it has been told to ask nobody, and then asking.
///
/// `bypassPermissions` is the mode in which the CLI's own permission tool is
/// never consulted — which is exactly why the gate exists and why it is
/// installed with a matcher of everything. The session is announced after the
/// first input, where the real CLI announces it (task 5.3).
const WIDE_OPEN: &str = r#"
{"mock":"await-input"}
{"mock":"once"}
{"type":"system","subtype":"init","cwd":"/mock/project","session_id":"mock-claude-session","tools":["Bash"],"mcp_servers":[],"model":"mock-sonnet","permissionMode":"bypassPermissions","slash_commands":[],"apiKeySource":"none","claude_code_version":"2.0.0-mock"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Running something."}},"session_id":"mock-claude-session"}
{"mock":"ask-hook","tool":"Bash","toolUseId":"toolu_wide_open","input":{"command":"rm -rf /"}}
{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"mock-claude-session","result":"Done.","usage":{"input_tokens":40,"output_tokens":8}}
"#;

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

/// A project whose agent is the real adapter, piloting the wide-open wire.
fn fixture() -> PathBuf {
    let adapter = workspace_bin("meltemi-claude-acp");
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-claude-bypass-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let script = root.join("wide-open.ndjson");
    std::fs::write(&script, WIDE_OPEN.trim_start()).unwrap();
    // SAFETY: this test binary runs one test, and sets these before any adapter
    // is launched. The adapter inherits them through the daemon, which is the
    // same inheritance a real launch uses.
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
    root
}

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
async fn a_cli_told_to_ask_nobody_is_still_refused_what_the_proxy_refuses() {
    // Scenario: Compuerta dura sobre modo permisivo del CLI
    //
    // The wire announces the mode in which the CLI's own permission tool is
    // never consulted, and then tries to run `rm -rf /`. Nothing about the
    // adapter changes: the gate was installed with a matcher of everything, the
    // question reaches this client as a `permission/request`, the human says
    // no, and the call does not happen.
    let root = fixture();
    let root_str = root.display().to_string();

    let endpoint = test_endpoint("claude-bypass");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("claude-bypass", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-claude-bypass".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");

    // A human at the tray who says no to everything.
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
                    let deny = params["options"]
                        .as_array()
                        .and_then(|options| {
                            options
                                .iter()
                                .find(|option| option["kind"] == "reject_once")
                                .map(|option| option["optionId"].clone())
                        })
                        .expect("a refusal is always one of the offered options");
                    peer.respond(
                        id,
                        Ok(json!({ "outcome": { "outcome": "selected", "optionId": deny } })),
                    );
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(120),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "a CLI that was told to ask nobody", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");
    assert_eq!(
        result["deniedPermissions"], 1,
        "the refusal is counted where the human can see it: {result:#}"
    );

    assert_eq!(
        asked.lock().unwrap().clone(),
        vec!["toolu_wide_open".to_string()],
        "the permissive mode did not stop the question from being asked"
    );

    let events = session_events(&peer, &root_str).await;
    let updates: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "agent_update")
        .map(|event| &event["payload"]["update"])
        .collect();

    // The mode the CLI announced is in the session, not swallowed: a human
    // reading this log can see the CLI was running wide open. It rides the
    // second provenance update, the one written once the CLI has spoken —
    // which is the only moment the mode is a fact.
    let announced = updates
        .iter()
        .find_map(|update| {
            let meta = &update["_meta"]["meltemi"];
            meta.get("providerPermissionMode").map(|_| meta.clone())
        })
        .expect("the session records what the CLI announced");
    assert_eq!(
        announced["providerPermissionMode"], "bypassPermissions",
        "the session says which mode the CLI is in: {announced:#}"
    );

    // And the call itself did not happen: the wire reports back what the model
    // would have seen, which is the refusal.
    assert!(
        updates.iter().any(|update| {
            update["toolCallId"] == "toolu_wide_open" && update["status"] == "failed"
        }),
        "the denied call is recorded as one that did not run: {updates:#?}"
    );

    decider.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
