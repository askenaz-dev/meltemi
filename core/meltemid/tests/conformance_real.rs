// SPDX-License-Identifier: Apache-2.0

//! The conformance run against the **real** provider CLIs
//! (adaptadores-propios-acp task 5.2, design D10; the opt-in
//! `niveles-integracion-conformidad` 3.3 declared and never built).
//!
//! Everything CI runs is a fixture. A fixture is a frozen observation, and a
//! frozen observation says nothing about whether the contract it froze still
//! holds — so the only thing that can answer that question is the binary the
//! user actually has. This is where that run lives:
//!
//! ```text
//! meltemid ─ACP→ meltemi-{claude,codex}-acp ─provider wire→ the official CLI
//! ```
//!
//! It never runs in CI, twice over: it is `#[ignore]`d, and it returns without
//! doing anything unless `MELTEMI_CONFORMANCE_REAL=1` is set. The procedure,
//! per platform, is `docs/conformidad-manual.md`; the result is persisted into
//! the real data directory with its date and the version of the CLI that
//! answered, which is what `fleet/list` then reports as the verified level.
//!
//! What it costs is real: a real session on the user's own account, spending
//! whatever a turn spends. That is the price of knowing, and it is why nobody
//! is opted in by default.
//!
//! Each dialect is run in **two legs**, because the four criteria level 2
//! declares cannot all be observed in one session:
//!
//! 1. A turn that runs to its end — streaming, permissions, session.
//! 2. A turn that is stopped the moment the CLI speaks inside it —
//!    cancellation.
//!
//! The second leg was expected to be the cheap one — cut off at its first
//! words — and measured against the real CLI it is not: the stop lands after
//! the provider has already produced most of a turn, and it cost within a
//! tenth of what the first leg cost (`docs/conformidad-manual.md`). Budget for
//! **two full turns per dialect**, not one and a bit.
//!
//! The second leg exists because a cancellation is the one property no fixture
//! can settle here. On the headless dialect the only stop the provider
//! documents is the end of the CLI's input, and whether a real binary actually
//! notices that end is a fact about the binary, not about the adapter — a
//! scripted wire answers whatever it was scripted to answer.
//!
//! A criterion this run could not exercise is **left out**, never reported as
//! passed: `conformance::verified_level` refuses to award a level whose declared
//! criteria are not all there, so an incomplete run reports an incomplete
//! result rather than a flattering one. A leg that never got its turn in flight
//! reports nothing rather than a failure, because a stop that was never sent
//! has demonstrated nothing about stopping.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{
    ConformanceCriterion, ConformanceResult, InitializeParams, PROTOCOL_VERSION, PeerInfo, methods,
};
use meltemid::conformance;
use meltemid::rpc::{Incoming, Peer};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// The opt-in. Without it this file does nothing at all.
const OPT_IN: &str = "MELTEMI_CONFORMANCE_REAL";

/// Narrows the run to one catalog id. Worth having because each dialect spends
/// a turn on a different provider's account, and somebody re-anchoring one of
/// them should not have to pay for the other.
const ONLY: &str = "MELTEMI_CONFORMANCE_AGENT";

/// How long the stop leg waits for the CLI to speak inside its turn before
/// giving up on stopping one.
const TO_FIRST_WORD: Duration = Duration::from_secs(90);

/// The two adapter-piloted entries of the shipped registry, by the id the
/// catalog knows them as, with the adapter binary each is piloted through and
/// the official CLI each pilots.
const DIALECTS: [(&str, &str, &str); 2] = [
    ("claude-code", "meltemi-claude-acp", "claude"),
    ("codex-cli", "meltemi-codex-acp", "codex"),
];

fn workspace_bin(name: &str) -> PathBuf {
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
        format!(r"\\.\pipe\meltemid-real-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-real-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// Whether the official CLI is on this machine's `PATH` under the name the
/// registry declares. Detection only — nothing is launched here.
fn cli_present(bin: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|dir| {
        if dir.as_os_str().is_empty() {
            return false;
        }
        if cfg!(windows) {
            ["exe", "cmd", "bat", "ps1"]
                .iter()
                .any(|ext| dir.join(format!("{bin}.{ext}")).is_file())
                || dir.join(bin).is_file()
        } else {
            dir.join(bin).is_file()
        }
    })
}

/// A throwaway project piloted by the real adapter against the real CLI.
///
/// The registry is substituted so the adapter is found at the path this
/// checkout built it to — an installed Meltemi finds it beside the daemon
/// instead, which is the one difference between this run and a user's. The CLI
/// layer is the bare name, resolved on the `PATH` exactly as in production.
fn fixture(root: &Path, id: &str, adapter: &Path, cli_bin: &str) {
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join("registry.toml"),
        format!(
            "version=\"manual-run\"\n[[agents]]\nid=\"{id}\"\nname=\"{id}\"\nlevel=2\n\
             bin='{adapter}'\nadapter='{adapter}'\ncli-bin=\"{cli_bin}\"\n",
            adapter = adapter.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\nid = \"{id}\"\n\n[fleet]\nregistry = '{}'\n",
            root.join("registry.toml").display()
        ),
    )
    .unwrap();
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
                name: "conformance-real".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

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
            &json!({ "projectRoot": root, "sessionId": id, "limit": 2000 }),
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

fn updates(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter(|event| event["type"] == "agent_update")
        .map(|event| &event["payload"]["update"])
        .collect()
}

/// One real turn: what it answered, what it asked about, and its session log.
struct Turn {
    outcome: Result<Value, String>,
    asked: Vec<String>,
    events: Vec<Value>,
}

/// Answers permission requests the way a human at the tray would: allow once,
/// every time. Returns the task and the list of calls it was asked about.
fn deciding(
    peer: &Peer,
    incoming: mpsc::UnboundedReceiver<Incoming>,
) -> (tokio::task::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
    let task = tokio::spawn({
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
                        .and_then(|options| {
                            options
                                .iter()
                                .find(|option| option["kind"] == "allow_once")
                                .map(|option| option["optionId"].clone())
                        })
                        .unwrap_or(Value::Null);
                    peer.respond(
                        id,
                        Ok(json!({ "outcome": { "outcome": "selected", "optionId": allow } })),
                    );
                }
            }
        }
    });
    (task, asked)
}

/// Drives one real session to its end.
async fn drive(root: &Path, tag: &str, idea: &str, timeout: Duration) -> Turn {
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon(tag).await;
    let (peer, incoming) = init_client(&endpoint).await;
    let (decider, asked) = deciding(&peer, incoming);

    let outcome = match tokio::time::timeout(
        timeout,
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": idea, "projectRoot": root_str }),
        ),
    )
    .await
    {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(format!("{error}")),
        Err(_) => Err(format!("no answer within {timeout:?}")),
    };
    let events = session_events(&peer, &root_str).await;

    decider.abort();
    peer.close();
    daemon.abort();
    Turn {
        outcome,
        asked: asked.lock().unwrap().clone(),
        events,
    }
}

/// What the second leg observed about a stop.
struct Stopped {
    /// Whether the turn was demonstrably in flight when the stop went out: the
    /// prompt had been sent and the CLI had spoken *inside* that turn. A stop
    /// sent into a session with no turn is answered "there is nothing to
    /// interrupt", which is correct and proves nothing.
    in_flight: bool,
    /// How the turn answered, if it answered at all.
    outcome: Result<Value, String>,
    /// How long the answer took from the moment the stop went out.
    took: Duration,
    /// Whether any session was still listed as running afterwards.
    left_running: bool,
}

/// Drives one real session and stops it the moment the CLI speaks inside its
/// turn.
///
/// This is the only leg that can say anything about cancellation against a real
/// binary — on the headless dialect the stop *is* the end of the CLI's input,
/// and whether a real process notices that end is a fact about the process, not
/// about the adapter.
///
/// It is not the cheap leg it looks like. "The first words the session sees" is
/// already late: the provider has produced most of a turn by then, and the
/// measured cost is within a tenth of the full leg's.
async fn stop_mid_turn(
    root: &Path,
    tag: &str,
    idea: &str,
    to_start: Duration,
    to_answer: Duration,
) -> Stopped {
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon(tag).await;
    let (peer, incoming) = init_client(&endpoint).await;
    let (decider, _asked) = deciding(&peer, incoming);

    let proposing = tokio::spawn({
        let peer = peer.clone();
        let root_str = root_str.clone();
        let idea = idea.to_string();
        async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": idea, "projectRoot": root_str }),
            )
            .await
        }
    });

    // Wait for the turn to be demonstrably in flight, then stop it at once.
    let mut session_id = None;
    let watching = Instant::now();
    while watching.elapsed() < to_start {
        let events = session_events(&peer, &root_str).await;
        if events.iter().any(|event| event["type"] == "prompt_sent")
            && events
                .iter()
                .any(|event| event["payload"]["update"]["sessionUpdate"] == "agent_message_chunk")
            && let Some(started) = events
                .iter()
                .find(|event| event["type"] == "session_started")
        {
            session_id = started["payload"]["sessionId"].as_str().map(str::to_string);
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let in_flight = session_id.is_some();
    let stopped_at = Instant::now();
    if let Some(id) = &session_id {
        peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": id }));
    }

    let outcome = match tokio::time::timeout(to_answer, proposing).await {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(error))) => Err(format!("{error}")),
        Ok(Err(error)) => Err(format!("the turn's task did not finish: {error}")),
        Err(_) => Err(format!("no answer within {to_answer:?} of the stop")),
    };
    let took = stopped_at.elapsed();

    let left_running = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .ok()
        .and_then(|list| {
            list["sessions"]
                .as_array()
                .map(|sessions| sessions.iter().any(|session| session["state"] == "active"))
        })
        .unwrap_or(true);

    decider.abort();
    peer.close();
    daemon.abort();
    Stopped {
        in_flight,
        outcome,
        took,
        left_running,
    }
}

/// Runs the criteria of one dialect against its real CLI and prints what it
/// found, criterion by criterion.
async fn run_dialect(
    id: &str,
    adapter: &Path,
    timeout: Duration,
) -> (Vec<ConformanceCriterion>, Option<String>) {
    let base = std::env::temp_dir().join(format!("meltemi-real-{id}-{}", std::process::id()));
    let root = base.join("turn");
    fixture(
        &root,
        id,
        adapter,
        DIALECTS.iter().find(|d| d.0 == id).unwrap().2,
    );

    println!("  leg 1/2: one real session, run to its end (this spends a real turn)…");
    let turn = drive(
        &root,
        &format!("real-{id}"),
        "Write a one-paragraph proposal for adding a `hello` command.",
        timeout,
    )
    .await;
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
    let version = provenance
        .as_ref()
        .and_then(|meta| meta["providerVersion"].as_str())
        .map(str::to_string);

    if let Err(error) = &turn.outcome {
        println!("  the session did not complete: {error}");
        for event in turn.events.iter().filter(|e| e["type"] == "error") {
            println!("  session log error: {event}");
        }
    }

    let mut criteria = vec![
        ConformanceCriterion {
            level: 2,
            name: "streaming".into(),
            passed: turn.outcome.is_ok() && !streamed.trim().is_empty(),
        },
        ConformanceCriterion {
            level: 2,
            name: "session".into(),
            passed: version.is_some(),
        },
    ];
    // A criterion the run could not put a question to is not reported at all —
    // an agent that needed no permission has not demonstrated the channel.
    if !turn.asked.is_empty() {
        criteria.push(ConformanceCriterion {
            level: 2,
            name: "permissions".into(),
            passed: turn.asked.iter().all(|asked| {
                seen.iter()
                    .any(|update| update["toolCallId"] == asked.as_str())
            }),
        });
    } else {
        println!("  the agent asked for no permission; that criterion is left unreported");
    }

    // Leg two: a session opened to be stopped. Its own project root, because a
    // stop leaves a session behind on purpose and the first leg's log is
    // evidence.
    let stop_root = base.join("stop");
    fixture(
        &stop_root,
        id,
        adapter,
        DIALECTS.iter().find(|d| d.0 == id).unwrap().2,
    );
    println!("  leg 2/2: one real session, stopped at its first words…");
    let stopped = stop_mid_turn(
        &stop_root,
        &format!("real-stop-{id}"),
        "Write a one-paragraph proposal for adding a `hello` command.",
        // A CLI that has not spoken a word of its turn in this long is not
        // going to be stopped mid-turn, and waiting out the whole timeout for
        // it only makes a run that already failed take longer to say so.
        TO_FIRST_WORD,
        timeout,
    )
    .await;

    if stopped.in_flight {
        let status = stopped
            .outcome
            .as_ref()
            .ok()
            .and_then(|value| value["status"].as_str())
            .unwrap_or_default()
            .to_string();
        if let Err(error) = &stopped.outcome {
            println!("  the stopped turn did not answer: {error}");
        }
        println!(
            "  the stop was answered in {:?} as `{}`; anything still running: {}",
            stopped.took,
            if status.is_empty() { "—" } else { &status },
            stopped.left_running
        );
        criteria.push(ConformanceCriterion {
            level: 2,
            name: "cancellation".into(),
            passed: status == "cancelled" && !stopped.left_running,
        });
    } else {
        // No stop was ever sent, so nothing was learned about stopping. A
        // failure here would be a finding about this run, not about the bridge.
        println!(
            "  the turn never got in flight within the patience, so no stop was sent; \
             that criterion is left unreported"
        );
    }

    let _ = std::fs::remove_dir_all(&base);
    (criteria, version)
}

#[tokio::test]
#[ignore = "spends a real session on the user's own account; see docs/conformidad-manual.md"]
async fn conformance_against_the_real_clis_is_opt_in() {
    if std::env::var(OPT_IN).is_err() {
        println!(
            "{OPT_IN} is not set: nothing was run. The procedure is in docs/conformidad-manual.md."
        );
        return;
    }

    let data_dir = meltemid::paths::data_dir();
    let run_at = meltemid::clock::now_rfc3339();
    println!("conformance against real CLIs — {run_at}");
    println!("results are appended to {}", data_dir.display());

    let only = std::env::var(ONLY).ok();
    let mut ran = 0;
    for (id, adapter_name, cli_bin) in DIALECTS {
        if only.as_deref().is_some_and(|wanted| wanted != id) {
            continue;
        }
        println!("\n[{id}]");
        let adapter = workspace_bin(adapter_name);
        if !adapter.exists() {
            println!("  {adapter_name} is not built here; run `cargo build --workspace` first");
            continue;
        }
        if !cli_present(cli_bin) {
            println!("  the official `{cli_bin}` CLI is not on this PATH; skipped");
            continue;
        }

        let (criteria, version) = run_dialect(id, &adapter, Duration::from_secs(300)).await;
        let verified = conformance::verified_level(&criteria);
        for criterion in &criteria {
            println!(
                "  {:<12} {}",
                criterion.name,
                if criterion.passed { "pass" } else { "FAIL" }
            );
        }
        let missing = conformance::missing_criteria(2, &criteria);
        if !missing.is_empty() {
            println!("  not exercised: {missing:?}");
        }
        println!(
            "  verified level: {verified} (CLI version: {})",
            version.as_deref().unwrap_or("unknown")
        );

        conformance::persist(
            &data_dir,
            &ConformanceResult {
                agent_id: id.to_string(),
                verified_level: verified,
                agent_version: version,
                run_at: run_at.clone(),
                criteria,
            },
        )
        .expect("the result is persisted");
        ran += 1;
    }

    assert!(
        ran > 0,
        "no dialect could be run: no official CLI was found on this PATH \
         (or {ONLY} named one that is not there)"
    );
    println!("\n{ran} dialect(s) run; `meltemi fleet` now reports the verified level.");
}
