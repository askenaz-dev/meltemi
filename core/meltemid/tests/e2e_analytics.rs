// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of local usage accounting (analitica-consumo-local task
//! 5.2), driving an ephemeral daemon over the real transport against synthetic
//! records in a controlled data directory:
//!
//! - two projects and two subscriptions aggregate into their own cells;
//! - an ACP session reports **no** token data, with the protocol as the stated
//!   reason, while a level-3 run whose official output declared counters
//!   contributes the measured ones — and the two never share a figure;
//! - the counters come from a real `mock-headless` run through the real capture
//!   seam: no network, no real agent, nothing estimated.
//!
//! Runs against temporary fixtures, never this repo (constitution
//! §"tests e2e").

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{
    InitializeParams, PROTOCOL_VERSION, PeerInfo, SessionEventKind, TurnStatus, methods,
};
use meltemid::levels::usage_from_headless_line;
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::session_index::{self, SessionRecord};
use meltemid::session_log::SessionLog;
use meltemid::transport::{Listener, connect};

fn bin(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    })
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(
            r"\\.\pipe\meltemid-e2e-analytics-{}-{tag}",
            std::process::id()
        )
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-analytics-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, PathBuf, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!(
        "meltemi-e2e-analytics-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let data_dir = base.join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(data_dir.clone(), base.join("config"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, data_dir, handle)
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
                name: "e2e-analytics-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Records one synthetic session: an index entry plus a log with its facts.
#[allow(clippy::too_many_arguments)]
fn synthetic_session(
    data_dir: &Path,
    project_root: &Path,
    session_id: &str,
    agent: &str,
    profile: Option<&str>,
    level: u8,
    started_at: &str,
    ended_at: &str,
    usage: Option<SessionEventKind>,
) {
    let key = meltemid::paths::project_key(project_root);
    session_index::append(
        data_dir,
        &key,
        &SessionRecord {
            mode: None,
            model: None,
            effort: None,
            session_id: session_id.into(),
            agent_command: vec![agent.into()],
            project_root: project_root.display().to_string(),
            level,
            started_at: started_at.into(),
            ended_at: Some(ended_at.into()),
            final_status: Some(TurnStatus::Completed),
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: Some(agent.into()),
            profile: profile.map(String::from),
            source: None,
            title: None,
        },
    )
    .expect("index record");

    let mut log = SessionLog::create(data_dir, &key, session_id).expect("session log");
    log.append(SessionEventKind::SessionStarted {
        mode: None,
        session_id: session_id.into(),
        agent_command: vec![agent.into()],
        project_root: project_root.display().to_string(),
        title: None,
    })
    .expect("start");
    log.append(SessionEventKind::AgentResolved {
        model: None,
        effort: None,
        binary: agent.into(),
        source: meltemi_proto::FleetResolutionSource::Profile,
        profile: profile.map(String::from),
        agent_id: Some(agent.into()),
        level,
    })
    .expect("resolution");
    log.append(SessionEventKind::PromptSent {
        text: "do the thing".into(),
    })
    .expect("prompt");
    if let Some(event) = usage {
        log.append(event).expect("usage");
    }
    log.append(SessionEventKind::TurnCompleted {
        stop_reason: TurnStatus::Completed,
    })
    .expect("turn");
    log.append(SessionEventKind::SessionEnded {
        reason: "completed".into(),
    })
    .expect("end");
}

/// The usage event a real `mock-headless` run yields through the real capture
/// seam — the counters are read, never invented.
fn usage_from_mock_headless() -> SessionEventKind {
    let headless = bin("mock-headless");
    assert!(
        headless.exists(),
        "run `cargo test` at the workspace root so mock-headless is built"
    );
    let output = Command::new(&headless).output().expect("run mock-headless");
    assert!(output.status.success());
    let captured: Vec<SessionEventKind> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(usage_from_headless_line)
        .collect();
    assert_eq!(
        captured.len(),
        1,
        "the official-shaped line is the only usage report"
    );
    captured.into_iter().next().unwrap()
}

fn cell_of<'a>(result: &'a Value, agent: &str, profile: &str) -> &'a Value {
    result["cells"]
        .as_array()
        .expect("cells array")
        .iter()
        .find(|cell| cell["agent"] == agent && cell["profile"] == profile)
        .unwrap_or_else(|| panic!("no cell for {agent}/{profile} in {result:#}"))
}

#[tokio::test]
async fn accounting_spans_projects_and_subscriptions_and_measures_only_what_was_reported() {
    // Scenario: Celdas por proyecto, agente, perfil y período
    // Scenario: Agregación multiproyecto sin filtro de proyecto
    // Scenario: Contadores del stream oficial agregados
    // Scenario: Sesión ACP marcada como no reportada por el protocolo
    // Scenario: Cobertura declarada junto al total
    // Scenario: Medido y no reportado nunca comparten cifra
    let (endpoint, data_dir, daemon) = spawn_daemon("pair").await;
    let alpha = std::env::temp_dir().join(format!("meltemi-an-alpha-{}", std::process::id()));
    let beta = std::env::temp_dir().join(format!("meltemi-an-beta-{}", std::process::id()));
    for root in [&alpha, &beta] {
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(root).unwrap();
    }

    // Alpha: one ACP session under "work" (no usage in the protocol) and one
    // level-3 run under "personal" whose official output reported counters.
    synthetic_session(
        &data_dir,
        &alpha,
        "acp-1",
        "claude",
        Some("work"),
        1,
        "2026-07-10T10:00:00Z",
        "2026-07-10T10:02:00Z",
        None,
    );
    synthetic_session(
        &data_dir,
        &alpha,
        "headless-1",
        "claude",
        Some("personal"),
        3,
        "2026-07-11T10:00:00Z",
        "2026-07-11T10:01:00Z",
        Some(usage_from_mock_headless()),
    );
    // Beta: another project entirely, another agent.
    synthetic_session(
        &data_dir,
        &beta,
        "acp-2",
        "codex",
        None,
        1,
        "2026-07-12T10:00:00Z",
        "2026-07-12T10:03:00Z",
        None,
    );

    let peer = init_client(&endpoint).await;
    let result = peer
        .request(methods::ANALYTICS_USAGE, &json!({ "granularity": "month" }))
        .await
        .expect("analytics/usage ok");

    // Three dimension combinations across two projects, one query, no filter.
    let cells = result["cells"].as_array().expect("cells");
    assert_eq!(cells.len(), 3, "got: {result:#}");
    let roots: Vec<&str> = cells
        .iter()
        .map(|cell| cell["projectRoot"].as_str().unwrap())
        .collect();
    assert!(roots.iter().any(|root| root.contains("an-alpha")));
    assert!(roots.iter().any(|root| root.contains("an-beta")));

    // The ACP session keeps its activity and states why it has no tokens.
    let acp = cell_of(&result, "claude", "work");
    assert_eq!(acp["activity"]["prompts"], 1);
    assert_eq!(acp["activity"]["activeSeconds"], 120);
    assert!(
        acp["tokens"].is_null(),
        "no measurement means no token object at all: {acp:#}"
    );
    assert_eq!(acp["coverage"]["measuredSessions"], 0);
    assert_eq!(acp["coverage"]["unreportedSessions"], 1);
    assert_eq!(
        acp["coverage"]["reasons"][0]["kind"], "protocol_carries_no_usage",
        "the reason is stated, not implied: {acp:#}"
    );

    // The measured run reports exactly what the official output declared.
    let measured = cell_of(&result, "claude", "personal");
    assert_eq!(measured["tokens"]["input"], 120);
    assert_eq!(measured["tokens"]["output"], 34);
    assert!(
        measured["tokens"]["cachedInput"].is_null(),
        "a counter the output never declared stays absent: {measured:#}"
    );
    assert_eq!(measured["coverage"]["measuredSessions"], 1);
    assert_eq!(measured["coverage"]["unreportedSessions"], 0);

    // The totals declare their coverage: measured and unreported side by side,
    // never folded into one figure.
    assert_eq!(result["totals"]["tokens"]["input"], 120);
    assert_eq!(result["totals"]["coverage"]["measuredSessions"], 1);
    assert_eq!(result["totals"]["coverage"]["unreportedSessions"], 2);
    // And the disclosure travels with the numbers.
    let disclosure = result["disclosure"].as_array().expect("disclosure");
    assert!(disclosure.iter().any(|k| k == "nothing-is-estimated"));
    assert!(
        disclosure
            .iter()
            .any(|k| k == "nothing-leaves-this-machine")
    );

    // A project filter narrows to that project only.
    let scoped = peer
        .request(
            methods::ANALYTICS_USAGE,
            &json!({ "projectRoot": beta.display().to_string(), "granularity": "total" }),
        )
        .await
        .expect("scoped ok");
    let scoped_cells = scoped["cells"].as_array().unwrap();
    assert_eq!(scoped_cells.len(), 1, "got: {scoped:#}");
    assert_eq!(scoped_cells[0]["agent"], "codex");
    assert_eq!(scoped_cells[0]["period"], "total");

    // A subscription filter reaches the same records by profile.
    let by_profile = peer
        .request(
            methods::ANALYTICS_USAGE,
            &json!({ "profile": "personal", "granularity": "month" }),
        )
        .await
        .expect("profile filter ok");
    let filtered = by_profile["cells"].as_array().unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0]["profile"], "personal");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&alpha);
    let _ = std::fs::remove_dir_all(&beta);
}

#[tokio::test]
async fn an_invalid_query_is_refused_over_the_wire() {
    // Scenario: Parámetro inválido rehúsa
    let (endpoint, _data_dir, daemon) = spawn_daemon("invalid").await;
    let peer = init_client(&endpoint).await;

    let error = peer
        .request(
            methods::ANALYTICS_USAGE,
            &json!({ "since": "2026-07-10T00:00:00Z", "until": "2026-07-01T00:00:00Z" }),
        )
        .await
        .expect_err("an inverted range is refused");
    assert_eq!(error.code, meltemi_proto::error_codes::USAGE_QUERY_INVALID);
    let remedy = error
        .data
        .as_ref()
        .and_then(|data| data.get("remedy"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(!remedy.is_empty(), "the refusal explains how to fix it");

    // An unknown granularity is a parameter error too, never a silent default.
    let unknown = peer
        .request(
            methods::ANALYTICS_USAGE,
            &json!({ "granularity": "fortnight" }),
        )
        .await
        .expect_err("an unknown granularity is refused");
    assert!(
        unknown.message.to_lowercase().contains("analytics/usage"),
        "got: {unknown:?}"
    );

    peer.close();
    daemon.abort();
}
