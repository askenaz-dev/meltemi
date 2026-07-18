// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of the fleet catalog (catalogo-flota tasks 5.2): a client
//! drives an ephemeral daemon whose registry is substituted by a fixture
//! pointing at the CI `mock-agent`. It exercises `fleet/list` (configured +
//! detected marking, per-query freshness), the session opened by catalog id,
//! and the 2001 `agent_not_detected` refusal without launching anything.
//!
//! Everything runs against a temporary fixture repository and the scripted
//! mock — never this repo, never a real agent (constitution §"tests e2e").

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{
    InitializeParams, PROTOCOL_VERSION, PeerInfo, PermissionOptionKind, PermissionOutcome,
    PermissionRequestParams, PermissionRequestResult, error_codes, methods,
};
use meltemid::rpc::{Incoming, Peer};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// Resolves the built `mock-agent` binary next to the test executable.
fn mock_agent_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
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
            name: "e2e-fleet-client".into(),
            version: "0.0.0".into(),
        },
    }
}

/// Auto-approves permission requests, like a user granting the mock's write.
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

/// Creates an executable file the passive detection can find, following the
/// platform's conventions (`<name>.cmd` shim on Windows, exec bit on Unix).
fn create_fake_binary(dir: &Path, name: &str) {
    #[cfg(windows)]
    {
        std::fs::write(dir.join(format!("{name}.cmd")), "@echo off\r\n").unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Finds one agent by id in a `fleet/list` response.
fn agent<'a>(result: &'a Value, id: &str) -> &'a Value {
    result["agents"]
        .as_array()
        .expect("agents array")
        .iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("agent `{id}` missing in: {result:#}"))
}

#[tokio::test]
async fn fleet_catalog_end_to_end() {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo test` at the workspace root",
        mock.display()
    );

    // Fixture repository and fixture registry: `mock` is detectable only via
    // its candidate path (the built mock binary); `ghost` is undetectable
    // until the freshness step creates its candidate.
    let fixture = std::env::temp_dir().join(format!("meltemi-e2e-fleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join(".meltemi")).unwrap();
    std::fs::create_dir_all(fixture.join("bin")).unwrap();
    let registry_path = fixture.join("registry.toml");
    std::fs::write(
        &registry_path,
        format!(
            "version = \"fixture-1\"\n\n\
             [[agents]]\n\
             id = \"mock\"\n\
             name = \"Mock Agent\"\n\
             level = 1\n\
             bin = \"meltemi-e2e-mock-absent\"\n\
             acp-args = []\n\
             candidate-paths = ['{}']\n\n\
             [[agents]]\n\
             id = \"ghost\"\n\
             name = \"Ghost Agent\"\n\
             level = 3\n\
             bin = \"meltemi-e2e-ghost-agent\"\n\
             candidate-paths = ['{}']\n",
            mock.display(),
            fixture.join("bin").join("ghost-agent").display(),
        ),
    )
    .unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("config.toml"),
        "[agent]\nid = \"mock\"\n\n\
         [[fleet.custom]]\n\
         id = \"local-tool\"\n\
         name = \"Local Tool\"\n\
         command = \"meltemi-e2e-local-tool --acp\"\n",
    )
    .unwrap();

    // The registry override travels through the environment; the agent
    // command must not be shadowed by one.
    // SAFETY: this test binary runs this single test; no other thread reads
    // the environment concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::set_var("MELTEMI_FLEET_REGISTRY", &registry_path);
    }

    // Ephemeral in-process daemon with isolated data/config directories.
    let endpoint = test_endpoint("fleet");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("e2e-fleet", shutdown_tx);
    tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    let approver = tokio::spawn(auto_approve(peer.clone(), incoming));
    peer.request(methods::INITIALIZE, &init_params())
        .await
        .expect("initialize");

    let root = fixture.display().to_string();

    // 1. fleet/list: substituted registry version, detection and configured
    //    marking, custom agent integrated (Scenario: Catálogo con configurado
    //    marcado / Registro sustituido / Agente custom listado).
    let result = peer
        .request(methods::FLEET_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("fleet/list succeeds");
    assert_eq!(result["registryVersion"], "fixture-1", "got: {result:#}");

    let mock_agent = agent(&result, "mock");
    assert_eq!(mock_agent["detected"], true);
    assert_eq!(mock_agent["configured"], true, "agent.id marks it");
    assert!(mock_agent["binaryPath"].as_str().is_some());
    assert_eq!(mock_agent["source"], "registry");

    let ghost = agent(&result, "ghost");
    assert_eq!(ghost["detected"], false);
    assert_eq!(ghost["configured"], false);
    assert!(ghost.get("binaryPath").is_none());

    let custom = agent(&result, "local-tool");
    assert_eq!(custom["source"], "custom");
    assert_eq!(custom["detected"], false);

    // 2. Detection refreshes per query (Scenario: La detección se refresca
    //    por consulta): the ghost binary appears, a new query reports it.
    create_fake_binary(&fixture.join("bin"), "ghost-agent");
    let result = peer
        .request(methods::FLEET_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("fleet/list succeeds again");
    assert_eq!(
        agent(&result, "ghost")["detected"],
        true,
        "a binary created after the previous query must be reported"
    );

    // 3. A session opened by catalog id launches the detected mock over ACP
    //    (Scenario: Sesión por id detectado).
    let proposed = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "fleet greeting", "projectRoot": root }),
        ),
    )
    .await
    .expect("propose did not return within 30s")
    .expect("propose by id succeeds");
    assert_eq!(proposed["status"], "completed", "result: {proposed:#}");
    let proposal = std::fs::read_to_string(proposed["proposalPath"].as_str().unwrap()).unwrap();
    assert!(
        proposal.contains("mock-agent"),
        "the mock resolved from the catalog id filled the proposal: {proposal}"
    );

    // 4. An id that is not in the catalog answers 2001 without launching
    //    anything (Scenario: Id no detectado).
    std::fs::write(
        fixture.join(".meltemi").join("config.toml"),
        "[agent]\nid = \"missing-id\"\n",
    )
    .unwrap();
    let refused = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "ghost run", "projectRoot": root }),
        )
        .await
        .expect_err("an unknown id must be refused");
    assert_eq!(
        refused.code,
        error_codes::AGENT_NOT_DETECTED,
        "expected 2001 agent_not_detected, got: {refused}"
    );
    let data = refused.data.expect("error data");
    assert_eq!(data["kind"], "agent_not_detected");
    assert!(data["remedy"].as_str().is_some(), "remedy is actionable");

    // No subprocess ran: the daemon holds no session after the refusal.
    let status = peer
        .request(methods::STATUS, &json!({}))
        .await
        .expect("status");
    assert_eq!(
        status["sessions"].as_array().map(Vec::len),
        Some(0),
        "a 2001 refusal must not leave any session behind"
    );

    // SAFETY: same single-test reasoning as above.
    unsafe {
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }
    approver.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}
