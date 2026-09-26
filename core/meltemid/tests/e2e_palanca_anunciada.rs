// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the announced lever (modelo-por-opcion-anunciada 2.3),
//! driving an ephemeral daemon and the scripted `mock-agent`:
//!
//! - a model or an effort the agent's entry does not take as a flag is set on
//!   the option the agent announces, before the first prompt, and confirmed by
//!   its answer;
//! - anything else — no such option or two of them, a value it did not
//!   announce, an answer that leaves another value or moves the mode, no answer
//!   in time — ends the session without a prompt, refused as data (2007);
//! - a cancellation while a lever is being set sends no prompt either;
//! - the history lists what governed, never what was refused.
//!
//! Everything runs against temporary fixtures and the mock — never this repo,
//! never a real agent (constitution §"tests e2e").

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::{Incoming, Peer, RpcError};
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
        format!(r"\\.\pipe\meltemid-e2e-lever-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-lever-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

fn fixture_root(tag: &str) -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo build --workspace` first",
        mock.display()
    );
    let root = std::env::temp_dir().join(format!("meltemi-e2e-lever-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    // Every permission allowed: what is under test is the lever, not the proxy.
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    root
}

/// A fixture whose agent is the mock run as a LITERAL command with `switches`.
/// A literal command declares no levers, so every lever goes by what the mock
/// announces (modelo-por-opcion-anunciada D2). `extra` is appended to the
/// project config.
fn fixture(tag: &str, switches: &[&str], extra: &str) -> PathBuf {
    let root = fixture_root(tag);
    let command: Vec<String> = std::iter::once(mock_agent_bin().display().to_string())
        .chain(switches.iter().map(|switch| (*switch).to_string()))
        .map(|arg| format!("'{arg}'"))
        .collect();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = [{}]\n{extra}", command.join(", ")),
    )
    .unwrap();
    root
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("lever-{tag}"), shutdown_tx);
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
                name: "e2e-lever-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    (peer, incoming)
}

/// Starts an attached free session with `levers` merged into its params.
async fn start(peer: &Peer, root: &str, levers: Value) -> Result<Value, RpcError> {
    let mut params = json!({ "projectRoot": root, "instruction": "look" });
    for (key, value) in levers.as_object().expect("levers are an object") {
        params[key] = value.clone();
    }
    tokio::time::timeout(
        Duration::from_secs(60),
        peer.request(methods::SESSION_START, &params),
    )
    .await
    .expect("session/start answered")
}

/// The session's log, parsed.
async fn events(peer: &Peer, root: &str, session_id: &str) -> Vec<Value> {
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "sessionId": session_id, "projectRoot": root }),
        )
        .await
        .expect("session/log");
    log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
        .collect()
}

/// The sessions the history lists for the project.
async fn listed(peer: &Peer, root: &str) -> Vec<Value> {
    peer.request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list")["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn position(events: &[Value], kind: &str) -> Option<usize> {
    events.iter().position(|event| event["type"] == kind)
}

fn find<'a>(events: &'a [Value], kind: &str) -> Option<&'a Value> {
    events.iter().find(|event| event["type"] == kind)
}

/// What a refusal carries in its error, checked for the shape every refusal
/// shares: the 2007, and the record in its data.
fn refusal_of(error: &RpcError) -> Value {
    assert_eq!(
        error.code, 2007,
        "a lever refused at open is a 2007: {error}"
    );
    let data = error.data.clone().expect("the refusal travels as data");
    assert_eq!(data["kind"], "lever_refused", "{data:#}");
    data["leverRefused"].clone()
}

/// The one session a refused start left behind, with its log: the refusal
/// answers without a session id, so the history is where it is found.
async fn refused_session(peer: &Peer, root: &str) -> (Value, Vec<Value>) {
    let sessions = listed(peer, root).await;
    assert_eq!(
        sessions.len(),
        1,
        "one session opened and ended: {sessions:#?}"
    );
    let session = sessions[0].clone();
    let id = session["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();
    (session, events(peer, root, &id).await)
}

/// A session refused at open ended on its own reason, with no prompt and no
/// turn — and the log recorded the same refusal the error carried.
fn assert_refused_without_a_prompt(events: &[Value], refusal: &Value) {
    assert!(
        position(events, "prompt_sent").is_none(),
        "no prompt goes out before the lever is confirmed: {events:#?}"
    );
    assert!(
        position(events, "turn_completed").is_none(),
        "no turn ran, so none completed: {events:#?}"
    );
    let ended = find(events, "session_ended").expect("the session ended");
    assert_eq!(ended["payload"]["reason"], "lever_refused");
    let logged = find(events, "lever_refused").expect("the refusal is in the log");
    assert_eq!(&logged["payload"], refusal, "the log and the error agree");
}

async fn close(peer: Peer, daemon: tokio::task::JoinHandle<()>, root: PathBuf) {
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La opción anunciada fija el modelo antes del primer prompt
// Scenario: Sin declaración la palanca es lo que el agente anuncia
// Scenario: Lo que rigió queda en el registro
#[tokio::test]
async fn an_announced_model_is_set_and_confirmed_before_the_first_prompt() {
    // The mock is a FLEET ENTRY that declares no levers: the model goes by
    // what it announces, and its environment never carries it.
    let root = fixture_root("model");
    let mock = mock_agent_bin().display().to_string().replace('\\', "/");
    let registry = root.join(".meltemi").join("registry.toml");
    std::fs::write(
        &registry,
        format!(
            "version = \"lever-e2e\"\n\
             [[agents]]\nid = \"mock\"\nname = \"Mock\"\nlevel = 1\nbin = \"mock-agent\"\n\
             candidate-paths = ['{mock}']\n\
             acp-args = [\"--config-options\", \"--echo-levers\"]\n"
        ),
    )
    .unwrap();
    let registry = registry.display().to_string().replace('\\', "/");
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\nid = \"mock\"\n\n[fleet]\nregistry = '{registry}'\n"),
    )
    .unwrap();
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("model").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    // `slow` is not the mock's default, so only setting it can leave it there.
    let result = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect("the announced model is taken");
    assert_eq!(result["status"], "completed", "{result:#}");
    assert_eq!(result["model"], "slow", "the start says what governed");

    let session_id = result["sessionId"].as_str().expect("session id");
    let log = events(&peer, &root_str, session_id).await;
    let applied = position(&log, "levers_applied").expect("the confirmation is recorded");
    let prompt = position(&log, "prompt_sent").expect("the prompt went out");
    assert!(
        applied < prompt,
        "the model is confirmed BEFORE the first prompt: {log:#?}"
    );
    assert_eq!(log[applied]["payload"]["model"], "slow");
    // The agent's answer is recorded as the announcement it is.
    let answered = log[..applied]
        .iter()
        .rev()
        .find(|event| event["type"] == "config_options_announced")
        .expect("the answer is recorded");
    let model = answered["payload"]["options"]
        .as_array()
        .expect("options")
        .iter()
        .find(|option| option["category"] == "model")
        .expect("the model option");
    assert_eq!(model["currentValue"], "slow", "{model:#}");
    // The resolution carries only what travelled by flag: nothing did.
    let resolved = find(&log, "agent_resolved").expect("the resolution");
    assert!(resolved["payload"]["model"].is_null(), "{resolved:#}");
    // And the agent's environment never carried it: the announced route is
    // the option, not the flag.
    assert!(
        log.iter()
            .any(|event| event.to_string().contains("levers read: model=- effort=-")),
        "the launch environment carried no lever: {log:#?}"
    );

    // The history lists what governed, taken from the record that knew it.
    let sessions = listed(&peer, &root_str).await;
    let session = sessions
        .iter()
        .find(|session| session["sessionId"] == session_id)
        .expect("the session is listed");
    assert_eq!(session["model"], "slow", "{session:#}");

    close(peer, daemon, root).await;
}

// Scenario: La opción anunciada fija el esfuerzo antes del primer prompt
#[tokio::test]
async fn an_announced_effort_is_set_before_the_first_prompt_as_a_level_or_a_toggle() {
    // A reasoning selector, the realistic shape of it — and the model first,
    // then the effort against what the model left on offer.
    let root = fixture("effort", &["--config-options", "--reasoning"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("effort").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let result = start(
        &peer,
        &root_str,
        json!({ "model": "slow", "effort": "high" }),
    )
    .await
    .expect("both announced levers are taken");
    assert_eq!(result["model"], "slow");
    assert_eq!(result["effort"], "high");
    let log = events(&peer, &root_str, result["sessionId"].as_str().unwrap()).await;
    let applied = position(&log, "levers_applied").expect("recorded");
    assert!(applied < position(&log, "prompt_sent").expect("prompted"));
    assert_eq!(log[applied]["payload"]["model"], "slow");
    assert_eq!(log[applied]["payload"]["effort"], "high");
    close(peer, daemon, root).await;

    // A toggle takes the agent's own two words.
    let root = fixture("toggle", &["--config-options"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("toggle").await;
    let (peer, _incoming) = init_client(&endpoint).await;
    let result = start(&peer, &root_str, json!({ "effort": "true" }))
        .await
        .expect("the toggle is set");
    assert_eq!(result["effort"], "true");
    let log = events(&peer, &root_str, result["sessionId"].as_str().unwrap()).await;
    let applied = position(&log, "levers_applied").expect("recorded");
    assert!(applied < position(&log, "prompt_sent").expect("prompted"));
    assert_eq!(log[applied]["payload"]["effort"], "true");
    close(peer, daemon, root).await;
}

// Scenario: Un valor no anunciado se rehúsa listando lo anunciado
// Scenario: Lo rehusado no figura como lo que rigió
#[tokio::test]
async fn a_value_the_agent_did_not_announce_is_refused_listing_what_it_did() {
    let root = fixture("not-a-value", &["--config-options"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("not-a-value").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let error = start(&peer, &root_str, json!({ "model": "gpt-x" }))
        .await
        .expect_err("a model the agent did not announce is refused");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["lever"], "model");
    assert_eq!(refusal["requested"], "gpt-x");
    assert_eq!(refusal["reason"], "not_a_value");
    assert_eq!(refusal["values"], json!(["fast", "slow"]), "{refusal:#}");

    let (session, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    // What was asked lives in the refusal; the history does not list it as
    // what governed.
    assert!(session["model"].is_null(), "{session:#}");
    assert!(session["finalStatus"].is_null(), "no turn ran: {session:#}");

    close(peer, daemon, root).await;

    // A toggle refuses anything but its own two words, and says so.
    let root = fixture("not-a-toggle", &["--config-options"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("not-a-toggle").await;
    let (peer, _incoming) = init_client(&endpoint).await;
    let error = start(&peer, &root_str, json!({ "effort": "high" }))
        .await
        .expect_err("a toggle is not a level");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "not_a_value");
    assert_eq!(refusal["values"], json!(["true", "false"]));
    close(peer, daemon, root).await;
}

// Scenario: Sin una opción de la categoría la palanca se rehúsa al abrir
#[tokio::test]
async fn without_exactly_one_option_of_its_category_a_lever_is_refused_at_open() {
    // Nothing announced at all.
    let root = fixture("not-announced", &[], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("not-announced").await;
    let (peer, _incoming) = init_client(&endpoint).await;
    let error = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect_err("an agent that announces no model option cannot take one");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "not_announced");
    let (_, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    close(peer, daemon, root).await;

    // Two options of the category: choosing one would be a guess, so every
    // announced option is listed for the human to pick by its id.
    let root = fixture("ambiguous", &["--config-options", "--config-twins"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("ambiguous").await;
    let (peer, _incoming) = init_client(&endpoint).await;
    let error = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect_err("two model options are refused, not guessed between");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "ambiguous");
    let ids: Vec<&str> = refusal["options"]
        .as_array()
        .expect("the options are listed")
        .iter()
        .filter_map(|option| option["id"].as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["model", "thinking", "model-backup"],
        "{refusal:#}"
    );
    let (_, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    close(peer, daemon, root).await;
}

// Scenario: Lo que el agente no confirma se rehúsa sin prompt
#[tokio::test]
async fn an_answer_that_leaves_another_value_is_refused_without_a_prompt() {
    // The agent that falls back on its own: asked for `slow`, it keeps `fast`.
    let root = fixture("clamp", &["--config-options", "--config-clamp"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("clamp").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let error = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect_err("what the agent did not confirm is refused");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "not_confirmed");
    assert_eq!(refusal["requested"], "slow");
    assert_eq!(refusal["left"], "fast", "it names what the agent left");
    let (session, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    assert!(session["model"].is_null(), "neither governed: {session:#}");
    close(peer, daemon, root).await;
}

// Scenario: Un cambio de modo al fijar la palanca se rehúsa
#[tokio::test]
async fn setting_a_lever_that_moves_the_mode_is_refused_without_a_prompt() {
    let root = fixture(
        "mode-shift",
        &["--config-options", "--config-mode-shift"],
        "",
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("mode-shift").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let error = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect_err("a mode nobody asked for is refused");
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "mode_changed");
    assert_eq!(
        refusal["left"], "auto",
        "it names the mode the agent moved to"
    );
    let (_, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    close(peer, daemon, root).await;

    // A mode announced by the protocol's older field is watched too, and a
    // model that leaves it where it was is not refused for it.
    let root = fixture("inherited-mode", &["--config-options", "--modes"], "");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("inherited-mode").await;
    let (peer, _incoming) = init_client(&endpoint).await;
    let result = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect("an unmoved mode lets the model through");
    assert_eq!(result["model"], "slow");
    close(peer, daemon, root).await;
}

// Scenario: Un agente que no contesta a tiempo se rehúsa
#[tokio::test]
async fn an_agent_that_does_not_answer_in_time_is_refused_without_a_prompt() {
    let root = fixture(
        "hang",
        &["--config-options", "--config-hang"],
        "\n[sessions]\nlever-wait = 1\n",
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("hang").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let started = Instant::now();
    let error = start(&peer, &root_str, json!({ "model": "slow" }))
        .await
        .expect_err("no answer within the wait is refused");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the wait is the configured one, not the default"
    );
    let refusal = refusal_of(&error);
    assert_eq!(refusal["reason"], "no_answer");
    let (_, log) = refused_session(&peer, &root_str).await;
    assert_refused_without_a_prompt(&log, &refusal);
    close(peer, daemon, root).await;
}

// Scenario: Cancelar mientras se fija la palanca no envía prompt
#[tokio::test]
async fn cancelling_while_a_lever_is_being_set_sends_no_prompt() {
    // The agent never answers, and the wait is long: only the cancellation
    // can end this session quickly.
    let root = fixture(
        "cancel",
        &["--config-options", "--config-hang"],
        "\n[sessions]\nlever-wait = 30\n",
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cancel").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let result = start(&peer, &root_str, json!({ "model": "slow", "detach": true }))
        .await
        .expect("a detached start answers at once");
    let session_id = result["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();

    // Wait until the agent has announced — the lever is being set from then.
    let started = Instant::now();
    loop {
        let log = events(&peer, &root_str, &session_id).await;
        if position(&log, "config_options_announced").is_some() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the agent announced: {log:#?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    peer.notify(methods::SESSION_CANCEL, &json!({ "sessionId": session_id }));

    let log = loop {
        let log = events(&peer, &root_str, &session_id).await;
        if position(&log, "session_ended").is_some() {
            break log;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the cancellation ended the session well before the wait: {log:#?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(
        position(&log, "prompt_sent").is_none(),
        "no prompt goes out: {log:#?}"
    );
    assert!(
        position(&log, "lever_refused").is_none(),
        "cancelled, not refused"
    );
    assert!(
        position(&log, "levers_applied").is_none(),
        "nothing was confirmed"
    );
    let completed = find(&log, "turn_completed").expect("the session ends cancelled");
    assert_eq!(completed["payload"]["stopReason"], "cancelled");

    close(peer, daemon, root).await;
}
