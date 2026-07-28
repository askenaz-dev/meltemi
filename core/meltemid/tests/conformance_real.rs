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
//! A criterion this run could not exercise is **left out**, never reported as
//! passed: `conformance::verified_level` refuses to award a level whose declared
//! criteria are not all there, so an incomplete run reports an incomplete
//! result rather than a flattering one.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// Drives one real session, answering permission requests the way a human at
/// the tray would: allow once, every time.
async fn drive(root: &Path, tag: &str, idea: &str, timeout: Duration) -> Turn {
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

    println!("  running one real session (this spends a real turn)…");
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
