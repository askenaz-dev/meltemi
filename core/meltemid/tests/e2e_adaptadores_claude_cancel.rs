// SPDX-License-Identifier: Apache-2.0

//! End-to-end cancellation through the headless-session adapter
//! (adaptadores-propios-acp task 3.6), with the real daemon piloting the real
//! adapter binary and a scripted provider that **will not stop**.
//!
//! This lives in its own test binary because it needs its own scripted wire,
//! and a wire is selected by environment variable — which is per process.
//!
//! The scenario is the one this dialect makes ugly. Its surface documents no
//! interruption at all: the end of the CLI's input stops the *next* turn, not
//! the one running. So a wire that ignores the end of its input and keeps
//! working is exactly the case that decides whether a human who pressed stop is
//! made to wait. They are not: after a grace the adapter ends the process,
//! rather than let a turn that will never finish hold the session and its
//! worktree open.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// A provider that opens a turn, speaks once, and then works forever.
///
/// The last directive is what makes it hang: the wire stops reading its input
/// and never ends the turn, so neither the polite stop nor the end of the
/// conversation reaches it. Everything before it is the same shape the fixture
/// freezes for a real session — the session announced only after the first
/// input, which is where this CLI announces it (task 5.3).
const NEVER_STOPS: &str = r#"
{"mock":"await-input"}
{"mock":"once"}
{"type":"system","subtype":"init","cwd":"/mock/project","session_id":"mock-claude-session","tools":["Read"],"mcp_servers":[],"model":"mock-sonnet","permissionMode":"default","slash_commands":[],"apiKeySource":"none","claude_code_version":"2.0.0-mock"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Thinking about it forever."}},"session_id":"mock-claude-session"}
{"mock":"never-stop"}
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

/// A project whose agent is the real adapter, piloting the wire that will not
/// stop.
fn fixture() -> PathBuf {
    let adapter = workspace_bin("meltemi-claude-acp");
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-claude-cancel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let script = root.join("never-stops.ndjson");
    std::fs::write(&script, NEVER_STOPS.trim_start()).unwrap();
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

async fn client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-claude-cancel".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// The session's log, as parsed events.
async fn session_events(peer: &Peer, root: &str) -> Vec<Value> {
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list ok");
    let Some(id) = list["sessions"][0]["sessionId"].as_str() else {
        return Vec::new();
    };
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
async fn cancelling_ends_a_turn_the_cli_refuses_to_end() {
    // Scenario: Eventos de sesión mapeados en streaming
    //
    // The cancellation this dialect has to earn: there is no interruption to
    // send, the polite stop is ignored, and the turn still ends — because the
    // adapter stops waiting on a CLI that will not, and ends it.
    let root = fixture();
    let root_str = root.display().to_string();

    let endpoint = test_endpoint("claude-cancel");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("claude-cancel", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    let peer = client(&endpoint).await;

    let proposing = tokio::spawn({
        let peer = peer.clone();
        let root_str = root_str.clone();
        async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "a turn that will not end", "projectRoot": root_str }),
            )
            .await
        }
    });

    // Cancel only once the turn is demonstrably in flight — the CLI has spoken.
    // Cancelling into a session that has not started its turn yet would prove
    // nothing about the adapter.
    let mut session_id = None;
    for _ in 0..200 {
        let events = session_events(&peer, &root_str).await;
        if events.iter().any(|event| event["type"] == "agent_update")
            && let Some(started) = events
                .iter()
                .find(|event| event["type"] == "session_started")
        {
            session_id = started["payload"]["sessionId"].as_str().map(str::to_string);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let session_id = session_id.expect("the turn started and the CLI spoke");

    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));

    let outcome = match tokio::time::timeout(Duration::from_secs(120), proposing).await {
        Ok(joined) => joined.expect("the task ran"),
        Err(_) => {
            let events = session_events(&peer, &root_str).await;
            panic!("the cancelled turn hung; the session log says: {events:#?}");
        }
    };
    assert!(outcome.is_ok(), "propose answered: {outcome:?}");

    let events = session_events(&peer, &root_str).await;
    let kinds: Vec<&str> = events
        .iter()
        .filter_map(|event| event["type"].as_str())
        .collect();
    assert!(
        kinds.contains(&"turn_completed") || kinds.contains(&"session_ended"),
        "the session closed rather than hanging: {kinds:?}"
    );

    // Nothing lists as running any more: the CLI process went with it.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    assert!(
        list["sessions"]
            .as_array()
            .expect("sessions")
            .iter()
            .all(|session| session["state"] != "active"),
        "no session is left running: {list:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
