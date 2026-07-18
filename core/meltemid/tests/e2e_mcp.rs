// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of MCP passthrough (mcp-passthrough task 4.2): a session
//! with an MCP-announcing agent records the declared servers as injected (by
//! name); a session with an agent that does not announce MCP records the
//! honest non-delivery. The session log is read back via `session/log`.
//!
//! Runs against temporary fixtures and the mock, never this repo.

use std::path::{Path, PathBuf};

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

/// A fixture pointing the agent at the mock (with optional extra args) and
/// declaring one MCP server, plus a bare allow rule.
fn fixture(root: &Path, mock_args: &str) {
    let mock = mock_agent_bin();
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}'{mock_args}]\n\n\
             [[mcp.servers]]\nname = \"fs\"\ncommand = \"mcp-fs\"\n\
             [mcp.servers.env]\nKEY = \"$MELTEMI_TEST_TOKEN\"\n",
            mock.display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
}

async fn run(tag: &str, mock_args: &str) -> String {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-mcp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fixture(&root, mock_args);
    // SAFETY: single test per binary; env is not read concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-mcp".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");

    let proposed = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": format!("mcp {tag}"), "projectRoot": root.display().to_string() }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(proposed["status"], "completed", "{proposed:#}");

    // Read the session log back and return its concatenated text.
    let list = peer
        .request(
            methods::SESSION_LIST,
            &json!({ "projectRoot": root.display().to_string() }),
        )
        .await
        .expect("session/list ok");
    let sid = list["sessions"].as_array().unwrap()[0]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root.display().to_string(), "sessionId": sid }),
        )
        .await
        .expect("session/log ok");
    let text = log["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
    text
}

#[tokio::test]
async fn mcp_is_injected_by_name_when_the_agent_announces_support() {
    // Scenario: Agente con soporte recibe los servidores / Inyección auditada.
    let log = run("with", ", '--mcp'").await;
    assert!(
        log.contains("mcp_injected") && log.contains("fs"),
        "the injected server is recorded by name: {log}"
    );
    // The audit never carries the resolved secret or the env value.
    assert!(
        !log.contains("MELTEMI_TEST_TOKEN") && !log.contains("mcp-fs\""),
        "no env values or commands leak into the log: {log}"
    );
}

#[tokio::test]
async fn mcp_non_delivery_is_declared_without_support() {
    // Scenario: Degradación honesta sin soporte.
    let log = run("without", "").await;
    assert!(
        log.contains("mcp_not_delivered"),
        "the non-delivery is declared, never silent: {log}"
    );
    assert!(!log.contains("mcp_injected"), "nothing was injected: {log}");
}
