// SPDX-License-Identifier: Apache-2.0

//! End-to-end test (task 6.1): a JSON-RPC client drives the daemon, which
//! drives the scripted `mock-agent` subprocess over ACP; a permission flows
//! back to the client and, once granted, the agent writes the proposal.
//!
//! This exercises the whole Fase 0 chain — client protocol, session, ACP
//! bridge, permission passthrough and the propose flow — against a real agent
//! subprocess. It runs entirely against a temporary fixture repository and
//! the CI `mock-agent`, never the real repo or a real agent (constitution
//! §"tests e2e").

use std::path::PathBuf;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{
    InitializeParams, PROTOCOL_VERSION, PeerInfo, PermissionOptionKind, PermissionOutcome,
    PermissionRequestParams, PermissionRequestResult, methods,
};
use meltemid::rpc::{Incoming, Peer};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// Resolves the built `mock-agent` binary next to the test executable
/// (`target/<profile>/mock-agent`). A workspace `cargo test` builds it.
fn mock_agent_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop(); // drop the test executable name -> .../deps
    if dir.ends_with("deps") {
        dir.pop(); // -> .../<profile>
    }
    let name = if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    };
    dir.join(name)
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

fn init_params() -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: "e2e-client".into(),
            version: "0.0.0".into(),
        },
    }
}

/// Background client behaviour: auto-approve permission requests, ignore
/// streamed events. Mirrors what a real client's approval loop does.
async fn auto_approve(peer: Peer, mut incoming: mpsc::UnboundedReceiver<Incoming>) {
    while let Some(message) = incoming.recv().await {
        if let Incoming::Request { id, method, params } = message
            && method == methods::PERMISSION_REQUEST
        {
            let request: PermissionRequestParams =
                serde_json::from_value(params).expect("permission params");
            let allow = request
                .options
                .into_iter()
                .find(|o| {
                    matches!(
                        o.kind,
                        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                    )
                })
                .map(|o| PermissionOutcome::Selected {
                    option_id: o.option_id,
                })
                .unwrap_or(PermissionOutcome::Cancelled);
            let result = PermissionRequestResult { outcome: allow };
            peer.respond(id, Ok(serde_json::to_value(result).unwrap()));
        }
    }
}

#[tokio::test]
async fn propose_end_to_end_writes_proposal_via_agent() {
    let agent = mock_agent_bin();
    assert!(
        agent.exists(),
        "mock-agent binary not found at {}; run `cargo test` at the workspace root",
        agent.display()
    );

    // Fixture repository whose .meltemi/config.toml points the agent at the
    // mock. A single-quoted TOML literal keeps Windows backslashes intact.
    let fixture = std::env::temp_dir().join(format!("meltemi-e2e-fixture-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join(".meltemi")).unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{}']\n", agent.display()),
    )
    .unwrap();

    // Make sure no env override shadows the fixture config.
    // SAFETY: this test binary runs this single test; no other thread reads
    // the variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    // Start the daemon in-process with isolated data/config directories.
    let endpoint = test_endpoint("propose");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("e2e", shutdown_tx);
    tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    // Connect as a client and run the approval loop in the background.
    let stream = connect(&endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    let approver = tokio::spawn(auto_approve(peer.clone(), incoming));

    peer.request(methods::INITIALIZE, &init_params())
        .await
        .expect("initialize");

    // A generous guard so a regression that deadlocks the chain fails fast
    // instead of hanging CI.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "add greeting", "projectRoot": fixture.display().to_string() }),
        ),
    )
    .await
    .expect("propose did not return within 30s")
    .expect("propose succeeds");

    // The turn completed and the agent (after the granted permission) filled
    // in the proposal.
    assert_eq!(result["status"], "completed", "result: {result:#}");
    assert_eq!(result["changeName"], "add-greeting");
    let proposal_path = result["proposalPath"].as_str().expect("proposalPath");
    let content = std::fs::read_to_string(proposal_path).expect("read proposal");
    assert!(
        content.contains("mock-agent"),
        "proposal must be filled by the agent, got: {content}"
    );

    // The proposal lives under the fixture's .meltemi/changes tree.
    assert!(proposal_path.contains(".meltemi"));

    // Proposing the same idea again is refused before any agent runs
    // (propose-flow: name collision).
    let duplicate = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "add greeting", "projectRoot": fixture.display().to_string() }),
        )
        .await;
    let error = duplicate.expect_err("a second identical propose must fail");
    assert_eq!(
        error.code, 3000,
        "expected change_already_exists, got: {error}"
    );

    approver.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}
