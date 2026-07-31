// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of naming the agent on the two verbs a person actually
//! starts work with (lanzador-conversacional), driving an ephemeral daemon and
//! the scripted `mock-agent` against temporary fixtures — never this repo, never
//! a real agent, never the network.
//!
//! The fleet has had profiles, ids and a resolution order for a while, and
//! `worktree/dispatch` has named its agent per competitor since
//! flota-multiproveedor. `propose` and `sdd/explore` could not: choosing a
//! provider meant editing configuration between turns.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn mock_bin() -> String {
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
    .display()
    .to_string()
    .replace('\\', "/")
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-agente-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-agente-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture whose registry declares two agents — one that resolves to the mock
/// and one whose binary does not exist anywhere — plus a launch profile over the
/// first, carrying an env marker the mock echoes into what it writes. That
/// marker is how a test sees WHICH auth context actually ran.
fn fixture(tag: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-agente-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let mock = mock_bin();
    assert!(
        std::path::Path::new(&mock).exists(),
        "run `cargo test` at the workspace root"
    );
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"agent-e2e\"\n\
             [[agents]]\nid = \"provider-a\"\nname = \"Provider A\"\nlevel = 1\n\
             bin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock}']\n\
             [[agents]]\nid = \"provider-absent\"\nname = \"Provider Absent\"\nlevel = 1\n\
             bin = \"meltemi-no-such-binary\"\nacp-args = []\ncandidate-paths = []\n"
        ),
    )
    .unwrap();
    let registry = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock}']\n\n\
             [fleet]\nregistry = '{registry}'\n\n\
             [[fleet.profile]]\nname = \"work-sub\"\nagent = \"provider-a\"\n\
             env = {{ MELTEMI_MOCK_MARKER = \"work-sub-ctx\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("agente-{tag}"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-agente-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// The proposal the mock wrote for a change, which carries the marker of the
/// auth context the subprocess ran under.
fn proposal(root: &std::path::Path, change: &str) -> String {
    std::fs::read_to_string(
        root.join(".meltemi")
            .join("changes")
            .join(change)
            .join("proposal.md"),
    )
    .expect("the agent wrote the proposal")
}

// Scenario: Propose con agente nombrado
#[tokio::test]
async fn propose_runs_the_agent_the_request_named() {
    let root = fixture("named");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("named").await;
    let peer = init_client(&endpoint).await;

    let proposed = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "named agent", "projectRoot": root_str, "agent": "work-sub" }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(proposed["status"], "completed", "{proposed:#}");

    // The profile's auth context is what ran: the mock echoes the env marker it
    // was launched with, and only the profile supplies one.
    let written = proposal(&root, proposed["changeName"].as_str().unwrap());
    assert!(
        written.contains("marker=work-sub-ctx"),
        "the named profile's context launched the binary: {written}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Propose sin agente se comporta como siempre
#[tokio::test]
async fn propose_without_an_agent_behaves_as_it_always_did() {
    let root = fixture("unnamed");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("unnamed").await;
    let peer = init_client(&endpoint).await;

    let proposed = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "no agent named", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(proposed["status"], "completed", "{proposed:#}");
    assert!(proposed["changeName"].is_string(), "{proposed:#}");
    assert!(proposed["proposalPath"].is_string(), "{proposed:#}");
    assert_eq!(proposed["deniedPermissions"], 0, "{proposed:#}");

    // The project's configured `agent.command` ran, which carries no profile
    // context: an empty marker is the proof that no profile leaked in.
    let written = proposal(&root, proposed["changeName"].as_str().unwrap());
    assert!(
        written.contains("marker=\n") || written.contains("marker="),
        "{written}"
    );
    assert!(
        !written.contains("work-sub-ctx"),
        "no profile was named, so no profile context ran: {written}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Propose con agente no detectado rehúsa sin degradar
#[tokio::test]
async fn propose_with_an_undetected_agent_refuses_instead_of_running_another() {
    let root = fixture("absent");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("absent").await;
    let peer = init_client(&endpoint).await;

    let refused = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "absent agent", "projectRoot": root_str, "agent": "provider-absent" }),
        )
        .await
        .expect_err("an agent that is not on this machine cannot draft anything");
    assert_eq!(refused.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);

    // Nothing degraded to the configured agent behind the refusal: no session
    // ran, and no change was scaffolded.
    let sessions = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    assert_eq!(
        sessions["sessions"].as_array().map(Vec::len),
        Some(0),
        "no provider was launched in its place: {sessions:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Explore con agente nombrado sigue sin escribir
#[tokio::test]
async fn explore_with_a_named_agent_still_writes_nothing() {
    let root = fixture("explore");
    let root_str = root.display().to_string();
    std::fs::write(root.join("source.txt"), "untouched\n").unwrap();
    let (endpoint, daemon) = spawn_daemon("explore").await;
    let peer = init_client(&endpoint).await;

    let explored = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SDD_EXPLORE,
            &json!({ "projectRoot": root_str, "topic": "how should this work", "agent": "work-sub" }),
        ),
    )
    .await
    .expect("sdd/explore returned")
    .expect("sdd/explore ok");
    assert_eq!(explored["phase"], "explored", "{explored:#}");

    // Choosing an agent relaxes no guarantee: deliberation writes nothing,
    // whichever binary deliberates.
    assert_eq!(
        std::fs::read_to_string(root.join("source.txt")).unwrap(),
        "untouched\n"
    );
    assert!(!root.join(".meltemi").join("changes").exists());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Explore con agente no detectado rehúsa sin degradar
#[tokio::test]
async fn explore_with_an_undetected_agent_refuses_instead_of_deliberating() {
    let root = fixture("explore-absent");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("explore-absent").await;
    let peer = init_client(&endpoint).await;

    let refused = peer
        .request(
            methods::SDD_EXPLORE,
            &json!({
                "projectRoot": root_str,
                "topic": "anything",
                "agent": "provider-absent",
            }),
        )
        .await
        .expect_err("an agent that is not on this machine cannot deliberate");
    assert_eq!(refused.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);

    let sessions = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    assert_eq!(
        sessions["sessions"].as_array().map(Vec::len),
        Some(0),
        "nobody deliberated in its place: {sessions:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// The events of the one session recorded for `root`.
async fn only_session_events(peer: &Peer, root: &str) -> Vec<serde_json::Value> {
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list ok");
    let sessions = list["sessions"].as_array().expect("sessions");
    assert_eq!(sessions.len(), 1, "one session ran: {list:#}");
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root, "sessionId": sessions[0]["sessionId"] }),
        )
        .await
        .expect("session/log ok");
    log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.as_str().unwrap()).ok())
        .collect()
}

#[tokio::test]
async fn the_log_of_a_proposal_and_of_a_deliberation_name_the_agent_that_ran() {
    // A reconstruction from the log alone must recover which agent ran and
    // under which subscription. Before this, only dispatch and implement wrote
    // that down: every proposal and every authoring turn was anonymous.
    let root = fixture("resolved");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("resolved").await;
    let peer = init_client(&endpoint).await;

    tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "logged resolution", "projectRoot": root_str, "agent": "work-sub" }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");

    let events = only_session_events(&peer, &root_str).await;
    let resolved = events
        .iter()
        .find(|event| event["type"] == "agent_resolved")
        .unwrap_or_else(|| panic!("the proposal says who wrote it: {events:#?}"));
    assert_eq!(resolved["payload"]["source"], "profile", "{resolved:#}");
    assert_eq!(resolved["payload"]["profile"], "work-sub");
    assert_eq!(resolved["payload"]["agentId"], "provider-a");
    assert!(
        resolved["payload"]["binary"]
            .as_str()
            .unwrap_or_default()
            .contains("mock-agent"),
        "{resolved:#}"
    );
    // §2: the profile's name is recorded, its auth context never is.
    assert!(
        !serde_json::to_string(resolved)
            .unwrap()
            .contains("work-sub-ctx"),
        "the env overlay must never reach the log: {resolved:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_deliberation_records_its_resolution_too() {
    let root = fixture("resolved-explore");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("resolved-explore").await;
    let peer = init_client(&endpoint).await;

    tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SDD_EXPLORE,
            &json!({ "projectRoot": root_str, "topic": "anything", "agent": "work-sub" }),
        ),
    )
    .await
    .expect("sdd/explore returned")
    .expect("sdd/explore ok");

    let events = only_session_events(&peer, &root_str).await;
    let resolved = events
        .iter()
        .find(|event| event["type"] == "agent_resolved")
        .unwrap_or_else(|| panic!("the deliberation says who deliberated: {events:#?}"));
    assert_eq!(resolved["payload"]["source"], "profile", "{resolved:#}");
    assert_eq!(resolved["payload"]["profile"], "work-sub");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// A fixture with the same registry but NO configured agent at all, so a verb
/// that needs one refuses with 2000 rather than 2001.
fn fixture_without_a_configured_agent(tag: &str) -> PathBuf {
    let root = fixture(tag);
    let registry = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[fleet]\nregistry = '{registry}'\n"),
    )
    .unwrap();
    root
}

// Scenario: La negativa trae los candidatos detectados
// Scenario: El error y la vista Flota no discrepan
#[tokio::test]
async fn a_refusal_to_resolve_hands_over_the_fleet_it_could_have_used() {
    let root = fixture_without_a_configured_agent("candidates");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("candidates").await;
    let peer = init_client(&endpoint).await;

    let refused = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "nothing configured", "projectRoot": root_str }),
        )
        .await
        .expect_err("no agent is configured");
    assert_eq!(
        refused.code,
        meltemi_proto::error_codes::AGENT_COMMAND_NOT_CONFIGURED
    );
    let data = refused.data.clone().expect("a refusal carries data");
    let offered = data["candidates"]
        .as_array()
        .expect("the refusal offers what this machine has");

    // One entry of the fixture registry resolves to the mock and the other
    // resolves to nothing, so the offer is exactly the first.
    let ids: Vec<&str> = offered
        .iter()
        .filter_map(|candidate| candidate["id"].as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["provider-a"],
        "detected agents only: an offer nobody can take is not an offer: {data:#}"
    );
    assert_eq!(offered[0]["detected"], true);
    assert!(
        offered[0]["installState"].is_string(),
        "each candidate carries its install state: {data:#}"
    );

    // And the state agrees with the Fleet view, because both come from the same
    // detection path: a client comparing the two must never see a machine
    // described two ways.
    let fleet = peer
        .request(methods::FLEET_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("fleet/list ok");
    for candidate in offered {
        let row = fleet["agents"]
            .as_array()
            .expect("agents")
            .iter()
            .find(|agent| agent["id"] == candidate["id"])
            .unwrap_or_else(|| panic!("the fleet lists {}: {fleet:#}", candidate["id"]));
        assert_eq!(
            row["installState"], candidate["installState"],
            "the error and the Fleet view describe the same machine"
        );
        assert_eq!(row["detected"], candidate["detected"]);
        assert_eq!(row["remedy"], candidate["remedy"]);
    }

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn naming_an_undetected_agent_also_offers_the_ones_that_are_there() {
    // The other refusal code: something WAS named, and it is not here. The offer
    // matters more in this one — the user has already said what they want.
    let root = fixture("candidates-2001");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("candidates-2001").await;
    let peer = init_client(&endpoint).await;

    let refused = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "absent", "projectRoot": root_str, "agent": "provider-absent" }),
        )
        .await
        .expect_err("the named agent is not on this machine");
    assert_eq!(refused.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);
    let data = refused.data.clone().expect("a refusal carries data");
    let ids: Vec<&str> = data["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .filter_map(|candidate| candidate["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["provider-a"], "{data:#}");
    assert!(
        !ids.contains(&"provider-absent"),
        "the agent that is missing is not among the ways out of missing it: {data:#}"
    );
    // The prose survives beside the structure: a scriptable client still prints
    // a diagnosis and a remedy.
    assert!(!data["detail"].as_str().unwrap_or_default().is_empty());
    assert!(!data["remedy"].as_str().unwrap_or_default().is_empty());

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La negativa no filtra material de autenticación
#[tokio::test]
async fn the_refusal_payload_carries_no_environment_and_nothing_shaped_like_a_secret() {
    // The fixture's profile injects an env marker, and the process running this
    // test has an env of its own. Neither may appear in a refusal: §2 is not a
    // matter of the payload happening to be small today.
    let root = fixture_without_a_configured_agent("hygiene");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("hygiene").await;
    let peer = init_client(&endpoint).await;

    // SAFETY: single-threaded setup before any concurrent env reader here.
    unsafe {
        std::env::set_var("MELTEMI_TEST_SECRET", "sk-do-not-leak-me");
    }
    let refused = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "hygiene", "projectRoot": root_str }),
        )
        .await
        .expect_err("no agent is configured");
    let payload = serde_json::to_string(&refused.data.clone().expect("data")).expect("serializes");

    assert!(
        !payload.contains("sk-do-not-leak-me"),
        "an environment value reached the payload: {payload}"
    );
    assert!(
        !payload.contains("MELTEMI_TEST_SECRET"),
        "an environment NAME reached the payload: {payload}"
    );
    assert!(
        !payload.contains("work-sub-ctx"),
        "a profile's auth context reached the payload: {payload}"
    );
    for suspicious in ["token", "secret", "password", "api_key", "apiKey", "Bearer"] {
        assert!(
            !payload.to_lowercase().contains(&suspicious.to_lowercase()),
            "something shaped like a credential reached the payload (`{suspicious}`): {payload}"
        );
    }
    // What it DOES carry is the vocabulary a surface can act on.
    let data = refused.data.clone().expect("data");
    let candidate = &data["candidates"][0];
    let keys: Vec<&str> = candidate
        .as_object()
        .expect("a candidate is an object")
        .keys()
        .map(String::as_str)
        .collect();
    for key in &keys {
        assert!(
            ["id", "detected", "installState", "remedy", "remedyCommand"].contains(key),
            "a candidate must carry ids, detection and remedies only, not `{key}`: {data:#}"
        );
    }

    // SAFETY: same reasoning as above.
    unsafe {
        std::env::remove_var("MELTEMI_TEST_SECRET");
    }
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
