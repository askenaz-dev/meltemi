// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the two ways an agent announces a mode, and of what it
//! changes on its own account (apagado-entero-y-modos task 2.2).
//!
//! The unit tests in `session_config` settle the derivation: which form wins,
//! what the synthesised option looks like, and what an agent announcing neither
//! ends up with. They cannot settle the part that only exists once there is a
//! process on the other end of a pipe:
//!
//! - that setting the synthesised option sends the protocol's **mode** verb and
//!   not the configuration-option verb, which the agent never announced;
//! - that a mode the agent changes by itself, with nobody asking, reaches the
//!   session's announcement and its log;
//! - that a new list of options replaces the old one whole, rather than merging
//!   into it.
//!
//! The discriminator for the first is built into the mock rather than asserted
//! by reading the daemon's mind: under `--modes` the mock does NOT announce
//! configuration options, so its configuration-option verb answers with an
//! empty list. A daemon that took the wrong verb would come back with nothing
//! announced, and the assertion would say so.
//!
//! Runs against temporary fixtures, never this repo, and the CI `mock-agent`,
//! never a real agent or the network (constitution §"tests e2e").

use std::path::PathBuf;
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
        format!(r"\\.\pipe\meltemid-e2e-modos-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-modos-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture repo whose config points the agent at the mock with `mock_args`,
/// and whose permissions allow writes so a turn runs without client escalation.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-modos-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let mock = mock_agent_bin().display().to_string().replace('\\', "/");
    let mut command = format!("'{mock}'");
    for arg in mock_args {
        command.push_str(&format!(", '{arg}'"));
    }
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = [{command}]\n"),
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
    let state = DaemonState::for_test(&format!("modos-{tag}"), shutdown_tx);
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
                name: "e2e-modos-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Polls `session/list` until a live session for `root` appears.
async fn wait_for_live_session(peer: &Peer, root: &str) -> String {
    const LIVE: [&str; 4] = [
        "starting",
        "active",
        "waiting_permission",
        "waiting_instruction",
    ];
    let mut seen = String::new();
    for _ in 0..200 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list");
        if let Some(sessions) = list["sessions"].as_array() {
            if let Some(live) = sessions
                .iter()
                .find(|s| LIVE.iter().any(|state| s["state"] == *state))
            {
                return live["sessionId"].as_str().unwrap().to_string();
            }
            seen = sessions
                .iter()
                .map(|s| s["state"].as_str().unwrap_or("?").to_string())
                .collect::<Vec<_>>()
                .join(", ");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("no live session appeared for {root}; the states seen were: [{seen}]");
}

/// The full log, parsed.
async fn log_events(peer: &Peer, root: &str, session_id: &str) -> Vec<Value> {
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root, "sessionId": session_id }),
        )
        .await
        .expect("session/log");
    log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l.as_str().unwrap()).ok())
        .collect()
}

/// Every announcement the session has made, in order.
async fn announcements(peer: &Peer, root: &str, session_id: &str) -> Vec<Value> {
    log_events(peer, root, session_id)
        .await
        .into_iter()
        .filter(|e| e["type"] == "config_options_announced")
        .collect()
}

/// Waits until the session has announced something, and returns the latest.
///
/// The announcement arrives with the handshake, so a session that merely
/// exists has not necessarily made one yet — reading earlier is a race, and
/// `wait` is what keeps this test honest rather than lucky.
async fn wait_for_announcements(
    peer: &Peer,
    root: &str,
    session_id: &str,
    at_least: usize,
) -> Vec<Value> {
    let mut seen = Vec::new();
    for _ in 0..400 {
        seen = announcements(peer, root, session_id).await;
        if seen.len() >= at_least {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "expected at least {at_least} announcements for {session_id}; saw {}",
        seen.len()
    );
}

/// The options of an announcement, as a list.
fn options_of(announcement: &Value) -> Vec<Value> {
    announcement["payload"]["options"]
        .as_array()
        .expect("an announcement carries options")
        .clone()
}

fn option_with_id<'a>(options: &'a [Value], id: &str) -> Option<&'a Value> {
    options.iter().find(|option| option["id"] == id)
}

// Scenario: Modos sin opción de configuración se ofrecen como opción
// Scenario: El modo elegido viaja por el verbo de modos
#[tokio::test]
async fn a_mode_announced_by_the_older_field_is_offered_and_set_by_its_own_verb() {
    // The mock announces modes and NOTHING else: no configuration options at
    // all. Before this change that session announced nothing and no surface
    // could offer a live change.
    let root = fixture("por-el-verbo", &["--turn-delay-ms", "3000", "--modes"]);
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("por-el-verbo").await;
    let peer = init_client(&endpoint).await;

    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    let session_id = wait_for_live_session(&peer, &root_str).await;
    let announced = wait_for_announcements(&peer, &root_str, &session_id, 1).await;
    let options = options_of(&announced[0]);
    let mode = option_with_id(&options, "mode")
        .unwrap_or_else(|| panic!("the modes are offered as an option: {options:#?}"))
        .clone();
    assert_eq!(mode["category"], "mode", "{mode:#}");
    assert_eq!(mode["type"], "select", "{mode:#}");
    assert_eq!(mode["currentValue"], "default", "{mode:#}");
    let values: Vec<&str> = mode["values"]
        .as_array()
        .expect("values")
        .iter()
        .map(|value| value["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(values, ["default", "acceptEdits"], "{mode:#}");

    // The change, through the ordinary session-options door. What travels on
    // the wire underneath is the protocol's mode verb.
    let changed = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "mode", "value": "acceptEdits" }),
        )
        .await
        .expect("session/set-config-option on the mode option");
    let now = changed["options"].as_array().expect("options").clone();
    assert!(
        !now.is_empty(),
        "an empty announcement here means the daemon took the configuration-option \
         verb, which this agent never announced: {changed:#}"
    );
    let mode = option_with_id(&now, "mode")
        .expect("the mode option is still announced")
        .clone();
    assert_eq!(
        mode["currentValue"], "acceptEdits",
        "the agent accepted, so the chosen mode is the record: {changed:#}"
    );
    // And its values are untouched: setting a mode is not re-announcing one.
    assert_eq!(mode["values"].as_array().expect("values").len(), 2);

    // A value the agent never announced is refused before anything travels:
    // the same refusal the other verb gives, because the announcement is what
    // decides either way.
    let refused = peer
        .request(
            methods::SESSION_SET_CONFIG_OPTION,
            &json!({ "sessionId": session_id, "optionId": "mode", "value": "invented" }),
        )
        .await
        .expect_err("an unannounced mode is refused");
    assert_eq!(refused.code, 2007, "{refused}");

    // The change is on record where the session's history is read.
    let announced = announcements(&peer, &root_str, &session_id).await;
    let last = options_of(announced.last().expect("at least one announcement"));
    assert_eq!(
        option_with_id(&last, "mode").expect("mode")["currentValue"],
        "acceptEdits",
        "the log says what is announced now: {last:#?}"
    );

    let result = tokio::time::timeout(Duration::from_secs(60), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Un cambio de modo del agente actualiza la opción
#[tokio::test]
async fn a_mode_the_agent_changes_by_itself_reaches_the_announcement() {
    // Nobody asked. The agent changes its own mode at the start of the turn
    // and says so, which is the case that used to leave the daemon holding an
    // option that was no longer true.
    let root = fixture(
        "deriva-de-modo",
        &["--turn-delay-ms", "3000", "--modes", "--mode-drift"],
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("deriva-de-modo").await;
    let peer = init_client(&endpoint).await;

    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    let session_id = wait_for_live_session(&peer, &root_str).await;
    // Two announcements: the one the handshake made, and the one the agent's
    // own change caused. Waiting for the second is the whole test.
    let announced = wait_for_announcements(&peer, &root_str, &session_id, 2).await;

    let first = options_of(&announced[0]);
    assert_eq!(
        option_with_id(&first, "mode").expect("mode")["currentValue"],
        "default",
        "the session opened in the mode the agent announced: {first:#?}"
    );

    let second = options_of(announced.last().expect("the second announcement"));
    assert_eq!(
        option_with_id(&second, "mode").expect("mode")["currentValue"],
        "acceptEdits",
        "the agent moved, and the announcement moved with it: {second:#?}"
    );
    // Nothing else changed: this is a value moving, not a re-announcement.
    assert_eq!(
        option_with_id(&second, "mode").expect("mode")["values"],
        option_with_id(&first, "mode").expect("mode")["values"],
        "the modes on offer are the same ones"
    );

    let result = tokio::time::timeout(Duration::from_secs(60), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Una lista de opciones nueva reemplaza a la anterior
#[tokio::test]
async fn a_new_list_of_options_replaces_the_old_one_whole() {
    // The mock announces two options at the handshake and then sends a list
    // with one option of an id neither of them had. Merging would leave three;
    // replacing leaves one, and only one of those readings is what the agent
    // said it has.
    let root = fixture(
        "lista-nueva",
        &[
            "--turn-delay-ms",
            "3000",
            "--config-options",
            "--options-drift",
        ],
    );
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("lista-nueva").await;
    let peer = init_client(&endpoint).await;

    let propose = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::PROPOSE,
                &json!({ "idea": "add dark mode", "projectRoot": root }),
            )
            .await
        })
    };

    let session_id = wait_for_live_session(&peer, &root_str).await;
    let announced = wait_for_announcements(&peer, &root_str, &session_id, 2).await;

    let first = options_of(&announced[0]);
    assert!(option_with_id(&first, "model").is_some(), "{first:#?}");
    assert!(option_with_id(&first, "thinking").is_some(), "{first:#?}");

    let second = options_of(announced.last().expect("the second announcement"));
    let ids: Vec<&str> = second
        .iter()
        .map(|option| option["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        ["verbosity"],
        "the new list is exactly what the agent sent, and nothing of the old one \
         survived on its own account: {second:#?}"
    );

    let result = tokio::time::timeout(Duration::from_secs(60), propose)
        .await
        .expect("propose returned")
        .expect("join")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
