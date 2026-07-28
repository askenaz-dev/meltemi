// SPDX-License-Identifier: Apache-2.0

//! Integration-level conformance suite (niveles-integracion-conformidad tasks
//! 3.2/3.3/4.1; adaptadores-propios-acp task 5.1). Executable pass/fail criteria
//! per level, run against **simulated agents only** — never the network, never
//! real agents in CI. Running against real agents is manual and opt-in; that
//! procedure lives in `docs/conformidad-manual.md` and in its own test binary,
//! and this file never launches one.
//!
//! Levels exercised: L1 native ACP (`mock-agent`), L2 adapter, L3 structured
//! headless (`mock-headless`), L4 artifacts (context projection). The result per
//! agent is persisted with its run date; the last run is what `fleet/list`
//! reports as verified.
//!
//! Level 2 is the one this file used to fake. It declared a criterion called
//! "adapter bridges to ACP" and satisfied it with `mock-agent` standing in as
//! its own adapter — which proved that the daemon can launch a level-2 entry and
//! nothing whatsoever about a bridge. It now runs the real thing:
//!
//! ```text
//! meltemid ─ACP→ meltemi-{claude,codex}-acp ─provider wire→ mock-{claude,codex}-wire
//! ```
//!
//! The adapters are the binaries that ship in the installers, piloted by the
//! real daemon; what stands in for the provider is the scripted wire, whose
//! every line is validated against the schema the official CLI dumps for itself.
//! Four criteria, because four properties are what make a bridge worth having:
//! the turn arrives as it is produced, the provider's questions reach the
//! permission proxy, a cancelled turn actually ends, and the session records
//! what was really launched.
//!
//! One test function, on purpose: a scripted wire is selected by environment
//! variable, and an environment is per process. Sequential runs can hand each
//! launch its own script; parallel ones would hand each other's.

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{
    ConformanceCriterion, ConformanceResult, InitializeParams, PROTOCOL_VERSION, PeerInfo, methods,
};
use meltemid::conformance;
use meltemid::levels::{map_headless_line, usage_from_headless_line};
use meltemid::rpc::{Incoming, Peer};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// The date this run is stamped with. A fixed stamp because the mocks are
/// fixed: what varies between runs of this suite is nothing at all.
const RUN_AT: &str = "2026-07-28T00:00:00Z";

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
        format!(r"\\.\pipe\meltemid-conf-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-conf-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture project that pilots an agent at the given level via a substitute
/// registry, with a bare allow rule so the mock's write proceeds.
fn fixture(root: &Path, id: &str, registry_body: &str) {
    fixture_with_rules(
        root,
        id,
        registry_body,
        Some("[[rule]]\neffect = \"allow\"\n"),
    );
}

/// As [`fixture`], with the permission rules spelled out — or without any,
/// which is how a level-2 run is set up: a project that allows everything by
/// rule never asks the proxy anything, and asking is the criterion.
fn fixture_with_rules(root: &Path, id: &str, registry_body: &str, rules: Option<&str>) {
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(root.join("registry.toml"), registry_body).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\nid = \"{id}\"\n\n[fleet]\nregistry = '{}'\n",
            root.join("registry.toml").display()
        ),
    )
    .unwrap();
    if let Some(rules) = rules {
        std::fs::write(root.join(".meltemi").join("permissions.toml"), rules).unwrap();
    }
}

/// A level-2 registry entry piloted by one of Meltemi's own adapters, with the
/// scripted wire declared as the provider's CLI layer — which is what it is
/// standing in for.
fn adapter_registry(id: &str, adapter: &Path, wire: &Path) -> String {
    let adapter = adapter.display().to_string().replace('\\', "/");
    let wire = wire.display().to_string().replace('\\', "/");
    format!(
        "version=\"conf\"\n[[agents]]\nid=\"{id}\"\nname=\"{id}\"\nlevel=2\n\
         bin='{adapter}'\nadapter='{adapter}'\ncli-bin='{wire}'\n"
    )
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
                name: "conformance".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// The session log of a project, as parsed events.
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
        .filter_map(|line| serde_json::from_str(line.as_str()?).ok())
        .collect()
}

/// The agent updates of a session log, in order.
fn updates(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|event| event["type"] == "agent_update")
        .map(|event| &event["payload"]["update"])
        .collect()
}

async fn drive_propose(root: &Path, tag: &str) -> bool {
    let (endpoint, daemon) = spawn_daemon(tag).await;
    let (peer, mut incoming) = init_client(&endpoint).await;
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": format!("conformance {tag}"), "projectRoot": root.display().to_string() }),
        ),
    )
    .await;
    peer.close();
    daemon.abort();
    matches!(result, Ok(Ok(v)) if v["status"] == "completed")
}

/// One turn through an adapter: whether it completed, which tool calls a human
/// was asked about, and the session log it left behind.
struct Turn {
    completed: bool,
    asked: Vec<String>,
    events: Vec<Value>,
}

/// Drives one turn and answers every permission request with allow-once, the
/// way a human at the tray would.
async fn drive_adapter_turn(root: &Path, tag: &str) -> Turn {
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon(tag).await;
    let (peer, incoming) = init_client(&endpoint).await;

    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
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
        Duration::from_secs(120),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": format!("conformance {tag}"), "projectRoot": root_str }),
        ),
    )
    .await;
    let completed = matches!(&result, Ok(Ok(value)) if value["status"] == "completed");
    let events = session_events(&peer, &root_str).await;

    decider.abort();
    peer.close();
    daemon.abort();
    Turn {
        completed,
        asked: asked.lock().unwrap().clone(),
        events,
    }
}

/// Cancels a turn whose provider will not end it, and reports whether the
/// session closed anyway and left nothing running.
async fn drive_adapter_cancel(root: &Path, tag: &str) -> bool {
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon(tag).await;
    let (peer, mut incoming) = init_client(&endpoint).await;
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });

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

    // Cancel only once the turn is demonstrably in flight — the provider has
    // spoken. Cancelling into a session that has not started proves nothing.
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
    let Some(session_id) = session_id else {
        peer.close();
        daemon.abort();
        return false;
    };

    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));
    let answered = matches!(
        tokio::time::timeout(Duration::from_secs(60), proposing).await,
        Ok(Ok(Ok(_)))
    );
    let idle = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .is_ok_and(|list| {
            list["sessions"]
                .as_array()
                .is_some_and(|sessions| sessions.iter().all(|s| s["state"] != "active"))
        });

    peer.close();
    daemon.abort();
    answered && idle
}

/// The level-2 criteria of one dialect, exercised through the real adapter
/// binary against its scripted provider wire.
///
/// `turn_script` and `cancel_script` are written into the fixture and selected
/// through `script_env`; a `None` script leaves the wire on its embedded
/// default, which is the transcript its own conformance test validates.
async fn level_2_criteria(
    tag: &str,
    adapter: &Path,
    wire: &Path,
    bin_env: &str,
    script_env: &str,
    turn_script: Option<&str>,
    cancel_script: &str,
) -> (Vec<ConformanceCriterion>, Option<String>) {
    let base = std::env::temp_dir().join(format!("meltemi-conf-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let turn_root = base.join("turn");
    let cancel_root = base.join("cancel");
    std::fs::create_dir_all(&turn_root).unwrap();
    std::fs::create_dir_all(&cancel_root).unwrap();

    // SAFETY: this suite is a single test function; every launch below is
    // sequential and reads these before the next one writes them. The adapter
    // inherits them through the daemon, which is the inheritance a real launch
    // uses.
    unsafe { std::env::set_var(bin_env, wire) };

    // --- The turn: streaming, permissions, session provenance. ---
    //
    // No rules at all here: the question has to reach the tray, which is the
    // criterion. A blanket allow would let the proxy answer for the human and
    // the run would prove nothing about the channel it exists to test.
    fixture_with_rules(&turn_root, tag, &adapter_registry(tag, adapter, wire), None);
    match turn_script {
        Some(script) => {
            let path = turn_root.join("turn.ndjson");
            std::fs::write(&path, script).unwrap();
            // SAFETY: as above.
            unsafe { std::env::set_var(script_env, &path) };
        }
        // SAFETY: as above.
        None => unsafe { std::env::remove_var(script_env) },
    }
    let turn = drive_adapter_turn(&turn_root, &format!("{tag}-turn")).await;
    let seen = updates(&turn.events);

    let streamed: String = seen
        .iter()
        .filter(|update| update["sessionUpdate"] == "agent_message_chunk")
        .filter_map(|update| update["content"]["text"].as_str())
        .collect();
    let provenance = seen.iter().find_map(|update| {
        let meta = &update["_meta"]["meltemi"];
        meta.is_object().then(|| meta.clone())
    });
    // No real provider binary ran, and the log is where that is provable: the
    // effective binary the adapter launched is the scripted wire, by name.
    let piloted_the_fixture = provenance.as_ref().is_some_and(|meta| {
        meta["providerBin"].as_str().is_some_and(|launched| {
            launched.contains(&*wire.file_stem().unwrap().to_string_lossy())
        })
    });
    let version = provenance
        .as_ref()
        .and_then(|meta| meta["providerVersion"].as_str())
        .map(str::to_string);

    let mut criteria = vec![
        ConformanceCriterion {
            level: 2,
            name: "streaming".into(),
            passed: turn.completed && streamed.contains("Working on it."),
        },
        ConformanceCriterion {
            level: 2,
            name: "permissions".into(),
            // The provider asked, a human decided, and what they decided about
            // had been shown to them as the tool call it was.
            passed: !turn.asked.is_empty()
                && turn.asked.iter().all(|id| {
                    seen.iter()
                        .any(|update| update["toolCallId"] == id.as_str())
                }),
        },
        ConformanceCriterion {
            level: 2,
            name: "session".into(),
            passed: piloted_the_fixture && version.is_some(),
        },
    ];

    // --- Cancellation: a provider that will not end its turn. ---
    fixture_with_rules(
        &cancel_root,
        tag,
        &adapter_registry(tag, adapter, wire),
        None,
    );
    let path = cancel_root.join("cancel.ndjson");
    std::fs::write(&path, cancel_script).unwrap();
    // SAFETY: as above.
    unsafe { std::env::set_var(script_env, &path) };
    criteria.push(ConformanceCriterion {
        level: 2,
        name: "cancellation".into(),
        passed: drive_adapter_cancel(&cancel_root, &format!("{tag}-cancel")).await,
    });

    let _ = std::fs::remove_dir_all(&base);
    (criteria, version)
}

/// The headless dialect's turn: it streams, then asks about one tool through
/// the hard gate the adapter installs.
const HEADLESS_TURN: &str = r#"
{"type":"system","subtype":"init","session_id":"conf-session","cwd":"/mock/project","model":"mock-sonnet","tools":["Read","Write"],"mcp_servers":[],"permissionMode":"default","apiKeySource":"none","capabilities":["interrupt_receipt_v1"],"slash_commands":[]}
{"mock":"await-input"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Working"}},"session_id":"conf-session"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" on it."}},"session_id":"conf-session"}
{"mock":"ask-hook","tool":"Write","toolUseId":"toolu_conf","input":{"file_path":"NOTES.md","content":"scripted"}}
{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"conf-session","result":"Done.","usage":{"input_tokens":120,"output_tokens":34}}
"#;

/// The headless dialect's turn that never ends: it speaks once and then works
/// forever, ignoring the end of its input — the only stop this surface
/// documents.
const HEADLESS_NEVER_STOPS: &str = r#"
{"type":"system","subtype":"init","session_id":"conf-session","cwd":"/mock/project","model":"mock-sonnet","tools":["Read"],"mcp_servers":[],"permissionMode":"default","apiKeySource":"none","capabilities":["interrupt_receipt_v1"],"slash_commands":[]}
{"mock":"await-input"}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Working on it forever."}},"session_id":"conf-session"}
{"mock":"never-stop"}
"#;

/// The server dialect's turn that never ends: it opens the turn, speaks, and
/// then waits on a request nothing ever sends — acknowledging the interruption
/// and ending nothing.
const SERVER_HELD_OPEN: &str = r#"
{"mock":"await-request","method":"initialize"}
{"mock":"respond","result":{"userAgent":"codex_cli_rs/0.77.0 (conformance fixture) (meltemi-codex-acp; 0.1.0)"}}
{"mock":"await-request","method":"thread/start"}
{"mock":"respond","result":{"approvalPolicy":"on-request","cwd":"/mock/project","model":"mock-model","modelProvider":"mock","sandbox":{"type":"readOnly"},"thread":{"id":"conf-thread","cliVersion":"0.77.0","createdAt":1784000000,"cwd":"/mock/project","modelProvider":"mock","path":"/mock/threads/conf-thread.jsonl","preview":"held open","source":"appServer","turns":[]}}}
{"mock":"await-request","method":"turn/start"}
{"mock":"notify","method":"turn/started","params":{"threadId":"conf-thread","turn":{"id":"turn-1","status":"inProgress","items":[]}}}
{"mock":"notify","method":"item/agentMessage/delta","params":{"threadId":"conf-thread","turnId":"turn-1","itemId":"item-1","delta":"Working on it forever."}}
{"mock":"await-request","method":"nothing/ever/asks/this"}
"#;

#[tokio::test]
async fn conformance_suite_runs_every_level_against_simulated_agents() {
    // Scenario: Conformidad en CI con simulados
    //
    // CI-safety: this suite must never opt into real agents on its own. The
    // opt-in run has its own binary and its own procedure.
    assert!(
        std::env::var("MELTEMI_CONFORMANCE_REAL").is_err(),
        "the conformance suite runs against mocks; real agents are manual opt-in \
         (docs/conformidad-manual.md)"
    );

    let mock = bin("mock-agent");
    let headless = bin("mock-headless");
    let claude_adapter = bin("meltemi-claude-acp");
    let codex_adapter = bin("meltemi-codex-acp");
    let claude_wire = bin("mock-claude-wire");
    let codex_wire = bin("mock-codex-wire");
    for binary in [
        &mock,
        &headless,
        &claude_adapter,
        &codex_adapter,
        &claude_wire,
        &codex_wire,
    ] {
        assert!(
            binary.exists(),
            "{} was not built; run `cargo test` at the workspace root",
            binary.display()
        );
    }
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }

    let base = std::env::temp_dir().join(format!("meltemi-conf-suite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut results = Vec::new();
    let mut criteria = Vec::new();

    // --- Level 1: native ACP session streams and completes. ---
    let l1 = base.join("l1");
    fixture(
        &l1,
        "sim-l1",
        &format!(
            "version=\"conf\"\n[[agents]]\nid=\"sim-l1\"\nname=\"Sim L1\"\nlevel=1\nbin='{}'\n",
            mock.display()
        ),
    );
    criteria.push(ConformanceCriterion {
        level: 1,
        name: "acp_session_completes".into(),
        passed: drive_propose(&l1, "l1").await,
    });

    // --- Level 3: structured headless output maps, and the rest is kept raw. ---
    let output = StdCommand::new(&headless)
        .output()
        .expect("run mock-headless");
    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let mapped: Vec<_> = lines.iter().map(|l| map_headless_line(l)).collect();
    let has_text = mapped
        .iter()
        .any(|e| matches!(e, meltemi_proto::SessionEventKind::AgentUpdate { .. }));
    criteria.push(ConformanceCriterion {
        level: 3,
        name: "structured_output_mapped".into(),
        passed: output.status.success() && has_text && lines.len() >= 3,
    });
    // The usage counters the official shape declares are captured, and only
    // the declared ones (analitica-consumo-local D3).
    let usage: Vec<_> = lines
        .iter()
        .filter_map(|l| usage_from_headless_line(l))
        .collect();
    let measured_only_what_was_declared = matches!(
        usage.first(),
        Some(meltemi_proto::SessionEventKind::UsageReported {
            input_tokens: Some(_),
            output_tokens: Some(_),
            cached_input_tokens: None,
            ..
        })
    );
    criteria.push(ConformanceCriterion {
        level: 3,
        name: "usage_counters_captured".into(),
        passed: usage.len() == 1 && measured_only_what_was_declared,
    });

    // --- Level 4: projection is the only channel; it includes the target. ---
    let l4 = base.join("l4");
    std::fs::create_dir_all(l4.join(".meltemi").join("rumbo")).unwrap();
    std::fs::write(l4.join(".meltemi").join("constitution.md"), "# C\n").unwrap();
    std::fs::write(
        l4.join(".meltemi").join("rumbo").join("product.md"),
        "---\ninclusion: siempre\n---\nR\n",
    )
    .unwrap();
    let written =
        meltemid::context::project_and_write_with(&l4, Some("GEMINI.md")).expect("projection");
    criteria.push(ConformanceCriterion {
        level: 4,
        name: "projection_includes_target".into(),
        passed: written.iter().any(|w| w.path == "GEMINI.md" && w.wrote),
    });

    results.push(ConformanceResult {
        agent_id: "sim-l1".into(),
        verified_level: conformance::verified_level(&criteria),
        agent_version: Some("mock-1".into()),
        run_at: RUN_AT.into(),
        criteria,
    });

    // --- Level 2: the real adapters, piloting scripted provider wires. ---
    //
    // Scenario: Nivel de adaptador ejercido por los adaptadores propios
    //
    // One result per adapter-piloted agent, because a result is per agent and
    // these are two agents: two dialects, two provider surfaces, two binaries
    // that ship in the installers. Neither run touches a real provider — the
    // effective binary each session logged is the scripted wire, asserted below
    // rather than assumed.
    let (headless_criteria, headless_version) = level_2_criteria(
        "own-headless",
        &claude_adapter,
        &claude_wire,
        "MELTEMI_CLAUDE_BIN",
        "MELTEMI_MOCK_CLAUDE_SCRIPT",
        Some(HEADLESS_TURN.trim_start()),
        HEADLESS_NEVER_STOPS.trim_start(),
    )
    .await;
    results.push(ConformanceResult {
        agent_id: "own-headless".into(),
        verified_level: conformance::verified_level(&headless_criteria),
        agent_version: headless_version,
        run_at: RUN_AT.into(),
        criteria: headless_criteria,
    });

    let (server_criteria, server_version) = level_2_criteria(
        "own-server",
        &codex_adapter,
        &codex_wire,
        "MELTEMI_CODEX_BIN",
        "MELTEMI_MOCK_CODEX_SCRIPT",
        None,
        SERVER_HELD_OPEN.trim_start(),
    )
    .await;
    results.push(ConformanceResult {
        agent_id: "own-server".into(),
        verified_level: conformance::verified_level(&server_criteria),
        agent_version: server_version,
        run_at: RUN_AT.into(),
        criteria: server_criteria,
    });

    // Every criterion the levels declare was reported, and every one passed. A
    // level a result speaks to at all, it has to speak to in full.
    for result in &results {
        for level in conformance::LEVELS {
            if result.criteria.iter().all(|c| c.level != level) {
                continue;
            }
            let missing = conformance::missing_criteria(level, &result.criteria);
            assert!(
                missing.is_empty(),
                "`{}` reported level {level} without {missing:?}",
                result.agent_id
            );
        }
        for c in &result.criteria {
            assert!(
                c.passed,
                "`{}`: level {} criterion `{}` failed",
                result.agent_id, c.level, c.name
            );
        }
    }
    assert_eq!(
        results
            .iter()
            .filter(|r| r.verified_level == 2)
            .map(|r| r.agent_id.as_str())
            .collect::<Vec<_>>(),
        vec!["own-headless", "own-server"],
        "both dialects verified the whole of level 2"
    );

    // Scenario: Resultado persistido
    //
    // Each agent's run is stored with its date and the version of what actually
    // answered, and read back by the same reader `fleet/list` uses.
    let data_dir = base.join("data");
    for result in &results {
        conformance::persist(&data_dir, result).unwrap();
    }
    let latest = conformance::latest_by_agent(&data_dir);
    for result in &results {
        let stored = latest
            .get(&result.agent_id)
            .unwrap_or_else(|| panic!("`{}` was persisted", result.agent_id));
        assert_eq!(stored.verified_level, result.verified_level);
        assert_eq!(stored.run_at, RUN_AT, "the run is stamped with its date");
        assert!(
            stored.agent_version.is_some(),
            "`{}` recorded the version of what answered",
            result.agent_id
        );
    }

    let _ = std::fs::remove_dir_all(&base);
}
