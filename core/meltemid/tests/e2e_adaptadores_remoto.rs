// SPDX-License-Identifier: Apache-2.0

//! End-to-end refusal of a Codex whose remote control is on (codex-al-dia task
//! 2.3), with the real daemon piloting the real adapter binary and a scripted
//! provider that reports its remote control as connected.
//!
//! This lives in its own test binary because it needs its own scripted wire,
//! and a wire is selected by environment variable — which is per process.
//!
//! The report arrives where the real server sends it (recorded 2026-09-24):
//! after the thread is asked for and before its answer. The session must not
//! open. With the remote control on, another client could attach to the thread
//! and answer its approvals outside the permission proxy — a remote channel
//! constitution §3 does not admit.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// A provider that opens the thread under the gate asked for, but reports its
/// remote control as connected first. The last directive is never reached: a
/// turn the adapter started would wait on it, and the test would time out.
const REMOTE_CONTROL_ON: &str = r#"
{"mock":"await-request","method":"initialize"}
{"mock":"respond","result":{"userAgent":"meltemi-codex-acp/0.156.1 (remote-control fixture) unknown (meltemi-codex-acp; 0.1.1)","codexHome":"/mock/codex-home","platformFamily":"unix","platformOs":"linux"}}
{"mock":"await-request","method":"thread/start"}
{"mock":"notify","method":"remoteControl/status/changed","params":{"status":"connected","serverName":"mock-machine","installationId":"mock-installation","environmentId":null}}
{"mock":"respond","result":{"approvalPolicy":"untrusted","approvalsReviewer":"user","cwd":"/mock/project","model":"mock-model","modelProvider":"mock","sandbox":{"type":"readOnly","networkAccess":false},"thread":{"id":"mock-thread-1","sessionId":"mock-thread-1","cliVersion":"0.156.1","createdAt":1790000000,"updatedAt":1790000000,"cwd":"/mock/project","modelProvider":"mock","preview":"remote control","ephemeral":false,"projectId":null,"status":{"type":"idle"},"source":"appServer","turns":[]}}}
{"mock":"await-request","method":"turn/start"}
{"mock":"await-request","method":"nothing/ever/asks/this"}
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

/// A project whose agent is the real adapter, piloting the wire whose remote
/// control is on.
fn fixture() -> PathBuf {
    let adapter = workspace_bin("meltemi-codex-acp");
    let root = std::env::temp_dir().join(format!(
        "meltemi-e2e-adapters-remote-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let script = root.join("remote-control-on.ndjson");
    std::fs::write(&script, REMOTE_CONTROL_ON.trim_start()).unwrap();
    // SAFETY: this test binary runs one test, and sets these before any adapter
    // is launched. The adapter inherits them through the daemon, which is the
    // same inheritance a real launch uses.
    unsafe {
        std::env::set_var("MELTEMI_CODEX_BIN", workspace_bin("mock-codex-wire"));
        std::env::set_var("MELTEMI_MOCK_CODEX_SCRIPT", &script);
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
                name: "e2e-adapters-remote".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn a_codex_whose_remote_control_is_on_does_not_get_a_session() {
    // Scenario: Un control remoto activo rehúsa la sesión
    let root = fixture();
    let root_str = root.display().to_string();

    let endpoint = test_endpoint("adapters-remote");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("adapters-remote", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    let peer = client(&endpoint).await;

    // The session is refused where it opens: the prompt never reaches a turn,
    // so the scripted wire never gets to wait on one.
    let outcome = tokio::time::timeout(
        Duration::from_secs(60),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "work under a remote control", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("the refusal comes at once, not after a turn that never ends");
    let error = outcome.expect_err("no session opens under a remote control that is on");
    let said = format!("{error:?}");
    assert!(
        said.contains("remote control") && said.contains("`connected`"),
        "the refusal says why, in the adapter's words: {said}"
    );

    assert!(
        said.contains("provider_remote_control"),
        "and by the kind the adapter gave it: {said}"
    );

    // Nothing lists as running: the provider went with the refusal.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().expect("sessions");
    assert!(
        sessions.iter().all(|session| session["state"] != "active"),
        "no session is left running: {list:#}"
    );

    // Refused where the session opens, not at its first turn: the log has the
    // losses row the adapter declares on opening, and no prompt ever went out.
    let id = sessions[0]["sessionId"]
        .as_str()
        .expect("the attempt was logged");
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str, "sessionId": id, "limit": 1000 }),
        )
        .await
        .expect("session/log ok");
    let kinds: Vec<String> = log["lines"]
        .as_array()
        .expect("the log's raw lines")
        .iter()
        .map(|line| {
            let event: Value = serde_json::from_str(line.as_str().expect("a JSONL line"))
                .expect("every logged line is JSON");
            event["type"].as_str().unwrap_or_default().to_string()
        })
        .collect();
    assert!(
        !kinds.iter().any(|kind| kind == "prompt_sent"),
        "no prompt reached a session that should never have opened: {kinds:?}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
