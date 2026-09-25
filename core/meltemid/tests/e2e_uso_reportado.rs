// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the usage an agent reports over ACP (usage-desde-acp
//! 2.2), driving an ephemeral daemon and the scripted `mock-agent` — which
//! reports its context and its cumulative cost as each turn closes when asked
//! to (`--usage`, `--usage-no-cost`, `--usage-currency`):
//!
//! - the accounting reads what the agent reported, as the agent's figures and
//!   apart from the tokens, which stay absent with the reason that says what
//!   the protocol did carry;
//! - every conversation of a session adds its cost (`implement` opens one per
//!   task), currencies never mix, and a report without a cost leaves the cost
//!   absent;
//! - a session whose agent reported nothing keeps saying so.
//!
//! Everything runs against temporary fixtures and the mock — never this repo,
//! never a real agent (constitution §"tests e2e").

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{Value, json};
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
        format!(r"\\.\pipe\meltemid-e2e-uso-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-uso-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A fixture whose fleet runs the mock under four entries: `usd` reports its
/// cost in dollars, `eur` in euros, `bare` reports context and no cost, and
/// `silent` reports nothing. Every permission is allowed.
fn fixture(tag: &str) -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo build --workspace` first",
        mock.display()
    );
    let root = std::env::temp_dir().join(format!("meltemi-e2e-uso-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    let mock = mock.display().to_string().replace('\\', "/");
    let entry = |id: &str, args: &str| {
        format!(
            "[[agents]]\nid = \"{id}\"\nname = \"{id}\"\nlevel = 1\nbin = \"mock-agent\"\n\
             candidate-paths = ['{mock}']\nacp-args = [{args}]\n\n"
        )
    };
    let registry = root.join(".meltemi").join("registry.toml");
    std::fs::write(
        &registry,
        format!(
            "version = \"uso-e2e\"\n\n{}{}{}{}",
            entry("usd", "\"--usage\""),
            entry("eur", "\"--usage\", \"--usage-currency\", \"EUR\""),
            entry("bare", "\"--usage-no-cost\""),
            entry("silent", ""),
        ),
    )
    .unwrap();
    let registry = registry.display().to_string().replace('\\', "/");
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\nid = \"silent\"\n\n[fleet]\nregistry = '{registry}'\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("uso-{tag}"), shutdown_tx);
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
                name: "e2e-uso-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn call(peer: &Peer, method: &str, params: Value) -> Value {
    tokio::time::timeout(Duration::from_secs(90), peer.request(method, &params))
        .await
        .unwrap_or_else(|_| panic!("{method} answered"))
        .unwrap_or_else(|error| panic!("{method}: {error}"))
}

async fn usage(peer: &Peer, root: &str) -> Value {
    call(
        peer,
        methods::ANALYTICS_USAGE,
        json!({ "projectRoot": root, "granularity": "total" }),
    )
    .await
}

fn reason_count(coverage: &Value, kind: &str) -> u64 {
    coverage["reasons"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|reason| reason["kind"] == kind)
        .map(|reason| reason["sessions"].as_u64().unwrap_or(0))
        .sum()
}

// Scenario: El contexto y el costo reportados se leen del protocolo
// Scenario: Contexto sin costo deja el costo ausente
// Scenario: Monedas distintas no se mezclan
// Scenario: Sesión ACP marcada como no reportada por el protocolo
// Scenario: Sesión ACP no fabrica evento de uso
#[tokio::test]
async fn the_accounting_reads_what_the_agent_reported_as_its_own_figures() {
    let root = fixture("free");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("free").await;
    let peer = init_client(&endpoint).await;

    let mut reported = None;
    for agent in ["usd", "eur", "bare", "silent"] {
        let result = call(
            &peer,
            methods::SESSION_START,
            json!({ "projectRoot": root_str, "instruction": "look", "agent": agent }),
        )
        .await;
        assert_eq!(result["status"], "completed", "{agent}: {result:#}");
        if agent == "usd" {
            reported = Some(result["sessionId"].as_str().unwrap().to_string());
        }
    }

    // ACP fabricates no usage event: what the agent reported stays in the log
    // as the update it is, and the accounting reads it from there.
    let log = call(
        &peer,
        methods::SESSION_LOG,
        json!({ "sessionId": reported.unwrap(), "projectRoot": root_str }),
    )
    .await;
    let lines: Vec<&str> = log["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        lines.iter().any(|line| line.contains("\"usage_update\"")),
        "the report is in the log: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("\"usage_reported\"")),
        "and no usage event was made of it"
    );

    let answer = usage(&peer, &root_str).await;
    let cells = answer["cells"].as_array().expect("cells");
    assert_eq!(cells.len(), 1, "one binary, one cell: {answer:#}");
    let cell = &cells[0];
    let report = &cell["agentReport"];
    assert_eq!(report["sessions"], 3, "three agents reported: {report:#}");
    // The fullest report of a single turn: a thousand tokens of 200 000.
    assert_eq!(report["contextUsed"], 1_000);
    assert_eq!(report["contextSize"], 200_000);
    // One sum per currency, each with its sessions; the context-only session
    // adds no cost, and nothing is converted.
    assert_eq!(
        report["cost"],
        json!([
            { "currency": "EUR", "amount": 0.125, "sessions": 1 },
            { "currency": "USD", "amount": 0.125, "sessions": 1 }
        ])
    );
    // The tokens stay absent, each session with the reason that fits it.
    assert!(cell.get("tokens").is_none(), "{cell:#}");
    assert_eq!(
        reason_count(
            &cell["coverage"],
            "protocol_carries_context_not_consumption"
        ),
        3
    );
    assert_eq!(
        reason_count(&cell["coverage"], "protocol_carries_no_usage"),
        1
    );
    // The totals sum the cells, and the numbers travel with the disclosure.
    assert_eq!(answer["totals"]["agentReport"]["sessions"], 3);
    assert_eq!(answer["totals"]["agentReport"]["cost"], report["cost"]);
    assert!(
        answer["totals"]["agentReport"].get("contextUsed").is_none(),
        "a maximum is not a sum"
    );
    assert!(
        answer["disclosure"]
            .as_array()
            .unwrap()
            .contains(&json!("agent-reported-context-and-cost"))
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Cada conversación de una sesión aporta su costo
#[tokio::test]
async fn implement_adds_the_cost_of_every_task_conversation() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = fixture("implement");
    std::fs::create_dir_all(root.join(".meltemi/changes/add-demo")).unwrap();
    std::fs::write(
        root.join(".meltemi/changes/add-demo/tasks.md"),
        "## 1. Work\n\n- [ ] 1.1 First task\n- [ ] 1.2 Second task\n",
    )
    .unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("implement").await;
    let peer = init_client(&endpoint).await;

    let implemented = call(
        &peer,
        methods::SDD_IMPLEMENT,
        json!({ "projectRoot": root_str, "change": "add-demo", "agent": "usd" }),
    )
    .await;
    assert_eq!(
        implemented["committed"],
        json!(["1.1", "1.2"]),
        "{implemented:#}"
    );

    // One Meltemi session, two agent processes: each task's cumulative figure
    // starts again, and the session adds them — the last task alone would say
    // an eighth.
    let answer = usage(&peer, &root_str).await;
    let report = &answer["cells"][0]["agentReport"];
    assert_eq!(report["sessions"], 1, "{answer:#}");
    assert_eq!(
        report["cost"],
        json!([{ "currency": "USD", "amount": 0.25, "sessions": 1 }])
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
