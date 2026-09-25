// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the levers in every verb that starts an agent
//! (modelo-por-opcion-anunciada 2.4), driving an ephemeral daemon and the
//! scripted `mock-agent` against a temporary **git** fixture:
//!
//! - a profile's model reaches the agent by its way — the flag its entry
//!   declares, or the option the agent announces — in `propose`, an authoring
//!   verb, `dispatch` and `implement`, and is recorded as what governed;
//! - a lever refused when the session opens reaches whoever asked as the 2007
//!   with its data, never as an agent that refused to go on, and `propose`
//!   leaves no scaffold behind;
//! - a lever refused before launch (2006) creates nothing: no scaffold, no
//!   worktree.
//!
//! Runs against temporary fixtures and the mock — never this repo, never a real
//! agent (constitution §"tests e2e"). Requires `git` on PATH; skips if absent.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::{Peer, RpcError};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

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
        format!(
            r"\\.\pipe\meltemid-e2e-leververbs-{}-{tag}",
            std::process::id()
        )
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-leververbs-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A git fixture whose fleet has two entries over the mock — `announcer`
/// declares no levers, so they go by what it announces; `flagged` declares the
/// model as a launch flag — and four profiles over them. Every permission is
/// allowed, and the change `add-demo` has one task.
fn fixture(tag: &str) -> Option<PathBuf> {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return None;
    }
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo build --workspace` first",
        mock.display()
    );
    let root = std::env::temp_dir().join(format!(
        "meltemi-e2e-leververbs-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/add-demo")).unwrap();
    let mock = mock.display().to_string().replace('\\', "/");
    let registry = root.join(".meltemi/registry.toml");
    std::fs::write(
        &registry,
        format!(
            "version = \"lever-verbs-e2e\"\n\n\
             [[agents]]\nid = \"announcer\"\nname = \"Announcer\"\nlevel = 1\n\
             bin = \"mock-agent\"\ncandidate-paths = ['{mock}']\n\
             acp-args = [\"--config-options\", \"--echo-levers\"]\n\n\
             [[agents]]\nid = \"flagged\"\nname = \"Flagged\"\nlevel = 1\n\
             bin = \"mock-agent\"\ncandidate-paths = ['{mock}']\n\
             acp-args = [\"--echo-levers\"]\nlevers = [\"model\"]\n"
        ),
    )
    .unwrap();
    let registry = registry.display().to_string().replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock}']\n\n\
             [fleet]\nregistry = '{registry}'\n\n\
             [[fleet.profile]]\nname = \"slow-sub\"\nagent = \"announcer\"\nmodel = \"slow\"\n\n\
             [[fleet.profile]]\nname = \"flag-sub\"\nagent = \"flagged\"\nmodel = \"some-model\"\n\n\
             [[fleet.profile]]\nname = \"unknown-sub\"\nagent = \"announcer\"\nmodel = \"gpt-x\"\n\n\
             [[fleet.profile]]\nname = \"effort-sub\"\nagent = \"flagged\"\neffort = \"high\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/add-demo/tasks.md"),
        "## 1. Work\n\n- [ ] 1.1 First task\n",
    )
    .unwrap();

    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    Some(root)
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(&format!("leververbs-{tag}"), shutdown_tx);
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
                name: "e2e-leververbs-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

async fn call(peer: &Peer, method: &str, params: Value) -> Result<Value, RpcError> {
    tokio::time::timeout(Duration::from_secs(90), peer.request(method, &params))
        .await
        .unwrap_or_else(|_| panic!("{method} answered"))
}

async fn session_ids(peer: &Peer, root: &str) -> HashSet<String> {
    peer.request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list")["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|session| session["sessionId"].as_str().map(str::to_string))
        .collect()
}

/// The one session a verb opened since `before`, as the history lists it, and
/// its log.
async fn opened_since(peer: &Peer, root: &str, before: &HashSet<String>) -> (Value, Vec<Value>) {
    let sessions = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
        .await
        .expect("session/list")["sessions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fresh: Vec<Value> = sessions
        .into_iter()
        .filter(|session| {
            session["sessionId"]
                .as_str()
                .is_some_and(|id| !before.contains(id))
        })
        .collect();
    assert_eq!(fresh.len(), 1, "the verb opened one session: {fresh:#?}");
    let session = fresh[0].clone();
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "sessionId": session["sessionId"], "projectRoot": root }),
        )
        .await
        .expect("session/log");
    let events = log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
        .collect();
    (session, events)
}

fn position(events: &[Value], kind: &str) -> Option<usize> {
    events.iter().position(|event| event["type"] == kind)
}

/// The profile's model was set on the agent's announcement BEFORE the first
/// prompt, and the history lists it as what governed.
fn assert_governed_by_announcement(verb: &str, session: &Value, events: &[Value]) {
    let applied = position(events, "levers_applied")
        .unwrap_or_else(|| panic!("{verb}: the model was confirmed: {events:#?}"));
    let prompt = position(events, "prompt_sent")
        .unwrap_or_else(|| panic!("{verb}: the prompt went out: {events:#?}"));
    assert!(
        applied < prompt,
        "{verb}: confirmed before the first prompt"
    );
    assert_eq!(events[applied]["payload"]["model"], "slow", "{verb}");
    assert_eq!(
        session["model"], "slow",
        "{verb}: the history lists what governed: {session:#}"
    );
}

fn refusal_of(verb: &str, error: &RpcError) -> Value {
    assert_eq!(
        error.code, 2007,
        "{verb}: a lever refused at open answers 2007, never an agent that refused: {error}"
    );
    let data = error.data.clone().expect("the refusal travels as data");
    assert_eq!(data["kind"], "lever_refused", "{verb}: {data:#}");
    data["leverRefused"].clone()
}

// Scenario: El modelo del perfil rige en todo verbo que arranca un agente
#[tokio::test]
async fn a_profiles_model_governs_every_verb_that_starts_an_agent() {
    let Some(root) = fixture("governs") else {
        return;
    };
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("governs").await;
    let peer = init_client(&endpoint).await;

    // propose: by the agent's announcement.
    let before = session_ids(&peer, &root_str).await;
    call(
        &peer,
        methods::PROPOSE,
        json!({ "projectRoot": root_str, "idea": "add a cache", "agent": "slow-sub" }),
    )
    .await
    .expect("propose ran");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    assert_governed_by_announcement("propose", &session, &events);

    // An authoring verb.
    let before = session_ids(&peer, &root_str).await;
    call(
        &peer,
        methods::SDD_EXPLORE,
        json!({ "projectRoot": root_str, "topic": "caching", "agent": "slow-sub" }),
    )
    .await
    .expect("explore ran");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    assert_governed_by_announcement("explore", &session, &events);

    // implement — first, because a dispatch of the same task and label works
    // in the same worktree and would leave it nothing new to commit.
    let before = session_ids(&peer, &root_str).await;
    let implemented = call(
        &peer,
        methods::SDD_IMPLEMENT,
        json!({ "projectRoot": root_str, "change": "add-demo", "agent": "slow-sub" }),
    )
    .await
    .expect("implement ran");
    assert_eq!(implemented["committed"], json!(["1.1"]), "{implemented:#}");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    assert_governed_by_announcement("implement", &session, &events);

    // dispatch.
    let before = session_ids(&peer, &root_str).await;
    let dispatched = call(
        &peer,
        methods::WORKTREE_DISPATCH,
        json!({ "projectRoot": root_str, "change": "add-demo", "task": "1.1", "agent": "slow-sub" }),
    )
    .await
    .expect("dispatch ran");
    assert_eq!(dispatched["status"], "completed", "{dispatched:#}");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    assert_governed_by_announcement("dispatch", &session, &events);

    // And by the flag its entry declares: the agent reads it from its launch
    // environment, and the resolution records it from the start.
    let before = session_ids(&peer, &root_str).await;
    call(
        &peer,
        methods::PROPOSE,
        json!({ "projectRoot": root_str, "idea": "add a flag", "agent": "flag-sub" }),
    )
    .await
    .expect("propose ran by flag");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    let resolved = &events[position(&events, "agent_resolved").expect("resolved")];
    assert_eq!(resolved["payload"]["model"], "some-model", "{resolved:#}");
    assert!(
        events.iter().any(|event| event
            .to_string()
            .contains("levers read: model=some-model effort=-")),
        "the flag reached the agent: {events:#?}"
    );
    assert!(
        position(&events, "levers_applied").is_none(),
        "no announcement was used"
    );
    assert_eq!(session["model"], "some-model", "{session:#}");

    // What a dispatch declares itself overrides the profile's default.
    let before = session_ids(&peer, &root_str).await;
    call(
        &peer,
        methods::WORKTREE_DISPATCH,
        json!({
            "projectRoot": root_str, "change": "add-demo", "task": "1.1",
            "agent": "slow-sub", "model": "fast",
        }),
    )
    .await
    .expect("dispatch ran with its own model");
    let (session, events) = opened_since(&peer, &root_str, &before).await;
    let applied = position(&events, "levers_applied").expect("confirmed");
    assert_eq!(events[applied]["payload"]["model"], "fast");
    assert_eq!(session["model"], "fast");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: El rehúso llega a quien lo pidió
#[tokio::test]
async fn a_lever_refused_at_open_reaches_whoever_asked_as_a_2007() {
    let Some(root) = fixture("refused") else {
        return;
    };
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("refused").await;
    let peer = init_client(&endpoint).await;
    let changes = root.join(".meltemi/changes");
    let change_dirs = || -> HashSet<String> {
        std::fs::read_dir(&changes)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    };

    // propose: the 2007, and no scaffold left behind, so the retry does not
    // collide with a change that was never written.
    let before = change_dirs();
    let error = call(
        &peer,
        methods::PROPOSE,
        json!({ "projectRoot": root_str, "idea": "add a cache", "agent": "unknown-sub" }),
    )
    .await
    .expect_err("a model the agent did not announce is refused");
    let refusal = refusal_of("propose", &error);
    assert_eq!(refusal["lever"], "model");
    assert_eq!(refusal["requested"], "gpt-x");
    assert_eq!(refusal["reason"], "not_a_value");
    assert_eq!(change_dirs(), before, "the scaffold is gone");
    call(
        &peer,
        methods::PROPOSE,
        json!({ "projectRoot": root_str, "idea": "add a cache", "agent": "slow-sub" }),
    )
    .await
    .expect("the retry does not collide with the refused scaffold");

    // dispatch: the 2007, never a turn the agent "refused".
    let error = call(
        &peer,
        methods::WORKTREE_DISPATCH,
        json!({ "projectRoot": root_str, "change": "add-demo", "task": "1.1", "agent": "unknown-sub" }),
    )
    .await
    .expect_err("dispatch answers the refusal");
    assert_eq!(refusal_of("dispatch", &error)["reason"], "not_a_value");

    // implement: the 2007, and the deployment stops with the task untouched.
    let error = call(
        &peer,
        methods::SDD_IMPLEMENT,
        json!({ "projectRoot": root_str, "change": "add-demo", "agent": "unknown-sub" }),
    )
    .await
    .expect_err("implement answers the refusal");
    assert_eq!(refusal_of("implement", &error)["reason"], "not_a_value");
    let tasks = std::fs::read_to_string(changes.join("add-demo/tasks.md")).unwrap();
    assert!(tasks.contains("- [ ] 1.1"), "nothing was ticked: {tasks}");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Una palanca que el agente no admite se rehúsa
#[tokio::test]
async fn a_lever_the_entry_does_not_take_is_refused_before_anything_is_created() {
    let Some(root) = fixture("before") else {
        return;
    };
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("before").await;
    let peer = init_client(&endpoint).await;

    // The flagged entry declares the model and not the effort.
    let changes_before: Vec<_> = std::fs::read_dir(root.join(".meltemi/changes"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    let error = call(
        &peer,
        methods::PROPOSE,
        json!({ "projectRoot": root_str, "idea": "add effort", "agent": "effort-sub" }),
    )
    .await
    .expect_err("an effort the entry does not take is refused");
    assert_eq!(error.code, 2006, "{error}");
    let changes_after: Vec<_> = std::fs::read_dir(root.join(".meltemi/changes"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(changes_after, changes_before, "no scaffold was written");

    let error = call(
        &peer,
        methods::WORKTREE_DISPATCH,
        json!({ "projectRoot": root_str, "change": "add-demo", "task": "1.1", "agent": "effort-sub" }),
    )
    .await
    .expect_err("dispatch refuses it too");
    assert_eq!(error.code, 2006, "{error}");
    assert!(
        !root.join(".meltemi/worktrees").exists(),
        "no worktree was created for a request that could never run"
    );
    assert!(
        session_ids(&peer, &root_str).await.is_empty(),
        "and no session"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
