// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the permission proxy (proxy-permisos task 5.2), driving
//! an ephemeral daemon and the scripted `mock-agent`:
//!
//! - a rule resolves the request without ever escalating to a client;
//! - a request with no rule becomes a first-class pending entry that survives
//!   reconnection, is resolved by `permission/decide` from another connection,
//!   and reconciles a second decision as "already resolved";
//! - a turn that denied a permission declares it honestly in the result.
//!
//! Everything runs against temporary fixtures and the mock — never this repo,
//! never a real agent (constitution §"tests e2e").

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::{Incoming, Peer};
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
            name: "e2e-perm-client".into(),
            version: "0.0.0".into(),
        },
    }
}

/// Writes a fixture repo whose config points the agent at the mock, optionally
/// with a `permissions.toml`. Returns the fixture root.
fn fixture(tag: &str, permissions: Option<&str>) -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo test` at the workspace root",
        mock.display()
    );
    let root = std::env::temp_dir().join(format!("meltemi-e2e-perm-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{}']\n", mock.display()),
    )
    .unwrap();
    if let Some(rules) = permissions {
        std::fs::write(root.join(".meltemi").join("permissions.toml"), rules).unwrap();
    }
    root
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
    peer.request(methods::INITIALIZE, &init_params())
        .await
        .expect("initialize");
    (peer, incoming)
}

#[tokio::test]
async fn a_rule_resolves_without_escalating_to_a_client() {
    // Scenario: Regla permite sin escalar. A bare allow rule matches the mock's
    // request; the client is never asked, and the proposal is written.
    let root = fixture("allow", Some("[[rule]]\neffect = \"allow\"\n"));
    // SAFETY: single test in this binary; no concurrent env readers.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-allow").await;
    let (peer, incoming) = init_client(&endpoint).await;

    // The client must never receive a permission/request when a rule decides.
    let escalated = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watcher = tokio::spawn({
        let escalated = escalated.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { method, .. } = &message
                    && method == methods::PERMISSION_REQUEST
                {
                    escalated.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "rule allows", "projectRoot": root.display().to_string() }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");

    assert_eq!(result["status"], "completed", "result: {result:#}");
    assert_eq!(
        result["deniedPermissions"], 0,
        "a rule allowed it, no denials"
    );
    let proposal = std::fs::read_to_string(result["proposalPath"].as_str().unwrap()).unwrap();
    assert!(
        proposal.contains("mock-agent"),
        "the agent wrote the proposal"
    );
    assert!(
        !escalated.load(std::sync::atomic::Ordering::SeqCst),
        "the rule must resolve without escalating to the client"
    );

    watcher.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn pending_survives_reconnection_and_decide_reconciles() {
    // Scenarios: Bandeja sobrevive la reconexión; Decide tras reconexión; Doble
    // resolución reconciliada.
    let root = fixture("queue", None); // no rules → the request escalates
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-queue").await;

    // Connection A drives the propose and HOLDS the live push (never answers),
    // so resolution must come from the tray on another connection.
    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {
            // Ignore everything, including permission/request: hold it.
        }
    });
    let propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root.display().to_string();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "queue then decide", "projectRoot": root }),
                )
                .await
        }
    });

    // Poll the queue on FRESH connections until the request appears — each poll
    // is a reconnection, proving the queue is daemon-owned, not per-connection.
    let (request_id, allow_option) = {
        let mut found = None;
        for _ in 0..100 {
            let (peer_b, _incoming_b) = init_client(&endpoint).await;
            let pending = peer_b
                .request(methods::PERMISSION_PENDING, &json!({}))
                .await
                .expect("pending ok");
            peer_b.close();
            if let Some(entry) = pending["pending"].as_array().and_then(|a| a.first()) {
                let request_id = entry["requestId"].as_str().unwrap().to_string();
                let allow = entry["options"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["kind"] == "allow_once")
                    .expect("an allow option")["optionId"]
                    .as_str()
                    .unwrap()
                    .to_string();
                found = Some((request_id, allow));
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        found.expect("the escalated request appears in the queue")
    };

    // A fresh (reconnected) client resolves it by id.
    let (peer_c, _incoming_c) = init_client(&endpoint).await;
    let decide = peer_c
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": allow_option }),
        )
        .await
        .expect("decide ok");
    assert_eq!(decide["status"], "applied", "the first decision applies");

    // A second decision for the same request reconciles as already resolved.
    let again = peer_c
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": allow_option }),
        )
        .await
        .expect("second decide ok");
    assert_eq!(
        again["status"], "already_resolved",
        "the losing path is told it was already resolved"
    );
    peer_c.close();

    // The propose completes: the agent got the allow and wrote the proposal.
    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose finished")
        .expect("join ok")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "result: {result:#}");
    assert_eq!(result["deniedPermissions"], 0, "the request was allowed");

    hold.abort();
    peer_a.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_denied_turn_declares_the_denial() {
    // Scenario: propose declara denegaciones (H1) at the daemon level. The
    // client actively denies the pushed request.
    let root = fixture("deny", None);
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-deny").await;
    let (peer, incoming) = init_client(&endpoint).await;

    // Answer every pushed permission with a cancel (deny).
    let denier = tokio::spawn({
        let peer = peer.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { id, method, .. } = message
                    && method == methods::PERMISSION_REQUEST
                {
                    peer.respond(id, Ok(json!({ "outcome": { "outcome": "cancelled" } })));
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "denied run", "projectRoot": root.display().to_string() }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");

    assert_eq!(result["status"], "completed", "result: {result:#}");
    assert_eq!(
        result["deniedPermissions"], 1,
        "the denied permission is declared honestly: {result:#}"
    );

    denier.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// ---- espera-humana ----------------------------------------------------------

/// A fixture whose config carries a `[permissions]` section besides the agent.
fn fixture_cfg(tag: &str, permissions_toml: &str) -> PathBuf {
    let root = fixture(tag, None);
    let mock = mock_agent_bin();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}']\n\n[permissions]\n{permissions_toml}",
            mock.display()
        ),
    )
    .unwrap();
    root
}

/// Polls the queue over fresh connections until an entry appears.
async fn wait_for_pending(endpoint: &str) -> serde_json::Value {
    for _ in 0..200 {
        let (peer, _incoming) = init_client(endpoint).await;
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("pending ok");
        peer.close();
        if let Some(entry) = pending["pending"].as_array().and_then(|a| a.first()) {
            return entry.clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the escalated request never appeared in the queue");
}

/// Polls `session/list` until the one session of `root` reads `ended`, then
/// returns the parsed events of its log.
async fn wait_for_ended_session_events(endpoint: &str, root: &str) -> Vec<serde_json::Value> {
    let (peer, _incoming) = init_client(endpoint).await;
    let mut session_id = None;
    for _ in 0..200 {
        let list = peer
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root }))
            .await
            .expect("session/list ok");
        if let Some(first) = list["sessions"].as_array().and_then(|a| a.first())
            && first["state"] == "ended"
        {
            session_id = Some(first["sessionId"].as_str().unwrap().to_string());
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let session_id = session_id.expect("the session ended");
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root, "sessionId": session_id }),
        )
        .await
        .expect("session/log ok");
    peer.close();
    log["lines"]
        .as_array()
        .expect("log lines")
        .iter()
        .filter_map(|l| l.as_str())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .collect()
}

#[tokio::test]
async fn the_owning_connections_death_does_not_resolve_the_request() {
    // Scenario: La caída de la conexión dueña no resuelve la petición.
    let root = fixture("owner-drop", None); // no rules → escalates; default wait
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-owner-drop").await;
    let root_str = root.display().to_string();

    // Connection A starts the propose and holds the live push unanswered.
    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {}
    });
    let _propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root_str.clone();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "outlive the owner", "projectRoot": root }),
                )
                .await
        }
    });
    let entry = wait_for_pending(&endpoint).await;
    let request_id = entry["requestId"].as_str().unwrap().to_string();
    let allow = entry["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "allow_once")
        .expect("an allow option")["optionId"]
        .as_str()
        .unwrap()
        .to_string();

    // A second client stays connected while the OWNING connection dies.
    let (peer_b, _incoming_b) = init_client(&endpoint).await;
    hold.abort();
    peer_a.close();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The request is still pending — the dead push resolved nothing.
    let pending = peer_b
        .request(methods::PERMISSION_PENDING, &json!({}))
        .await
        .expect("pending ok");
    let still = pending["pending"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["requestId"] == request_id.as_str());
    assert!(still, "the request survived the owner's death: {pending:#}");

    // The surviving client decides, and the agent receives the allow.
    let decide = peer_b
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": allow }),
        )
        .await
        .expect("decide ok");
    assert_eq!(decide["status"], "applied");
    peer_b.close();

    let events = wait_for_ended_session_events(&endpoint, &root_str).await;
    assert!(
        events.iter().any(|e| e["type"] == "permission_decided"
            && e["payload"]["decidedBy"] == "client"
            && e["payload"]["denied"] == false),
        "the human decision is on record, not a transport deny: {events:#?}"
    );

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_deadline_free_wait_is_declared_without_expiry() {
    // Scenario: Espera sin plazo declarada sin plazo.
    let root = fixture("nodeadline", None); // default policy: while-connected
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-nodeadline").await;
    let root_str = root.display().to_string();

    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {}
    });
    let propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root_str.clone();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "wait without a clock", "projectRoot": root }),
                )
                .await
        }
    });

    let entry = wait_for_pending(&endpoint).await;
    assert!(
        entry.get("expiresInSeconds").is_none(),
        "no invented countdown: {entry:#}"
    );
    assert_eq!(entry["expired"], false, "{entry:#}");

    // Decide so the turn completes.
    let request_id = entry["requestId"].as_str().unwrap();
    let allow = entry["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "allow_once")
        .expect("an allow option")["optionId"]
        .as_str()
        .unwrap()
        .to_string();
    let (peer_b, _incoming_b) = init_client(&endpoint).await;
    peer_b
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": allow }),
        )
        .await
        .expect("decide ok");
    peer_b.close();

    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose finished")
        .expect("join ok")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    hold.abort();
    peer_a.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_configured_bound_expires_audited_as_timeout() {
    // Scenario: Cota configurada vence auditada.
    let root = fixture_cfg("bounded", "wait = 1\n");
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-bounded").await;
    let root_str = root.display().to_string();

    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {}
    });
    let propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root_str.clone();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "expire on the bound", "projectRoot": root }),
                )
                .await
        }
    });

    // Nobody decides: the 1-second bound expires and denies audited.
    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose finished")
        .expect("join ok")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");
    assert_eq!(result["deniedPermissions"], 1, "{result:#}");

    let events = wait_for_ended_session_events(&endpoint, &root_str).await;
    assert!(
        events.iter().any(|e| e["type"] == "permission_decided"
            && e["payload"]["decidedBy"] == "timeout"
            && e["payload"]["denied"] == true),
        "the expiry is audited as timeout: {events:#?}"
    );

    hold.abort();
    peer_a.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn with_no_clients_the_constitutional_deny_fires_after_the_grace() {
    // Scenario: Sin clientes, denegación constitucional tras la gracia.
    let root = fixture_cfg("noclients", "no-client-grace = 0\n");
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-noclients").await;
    let root_str = root.display().to_string();

    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {}
    });
    let _propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root_str.clone();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "deny without clients", "projectRoot": root }),
                )
                .await
        }
    });
    let _entry = wait_for_pending(&endpoint).await;

    // The LAST client disconnects; with zero grace the constitutional deny
    // fires as soon as the vigil observes the empty registry.
    hold.abort();
    peer_a.close();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let events = wait_for_ended_session_events(&endpoint, &root_str).await;
    assert!(
        events.iter().any(|e| e["type"] == "permission_decided"
            && e["payload"]["decidedBy"] == "default_deny"
            && e["payload"]["denied"] == true),
        "the no-client deny is explicit and audited: {events:#?}"
    );

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_session_awaiting_a_decision_declares_itself_waiting() {
    // Scenario: Sesión esperando se declara esperando.
    // Scenario: Resuelta la petición, la sesión vuelve a activa.
    let root = fixture("waitingstate", None); // no rules → the request escalates
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-waitingstate").await;
    let root_str = root.display().to_string();

    // Connection A starts the propose and holds the live push unanswered, so
    // the session sits blocked on a human.
    let (peer_a, incoming_a) = init_client(&endpoint).await;
    let hold = tokio::spawn(async move {
        let mut incoming_a = incoming_a;
        while incoming_a.recv().await.is_some() {}
    });
    let propose = tokio::spawn({
        let peer_a = peer_a.clone();
        let root = root_str.clone();
        async move {
            peer_a
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "declare the wait", "projectRoot": root }),
                )
                .await
        }
    });

    let entry = wait_for_pending(&endpoint).await;
    let request_id = entry["requestId"].as_str().unwrap().to_string();
    let allow = entry["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "allow_once")
        .expect("an allow option")["optionId"]
        .as_str()
        .unwrap()
        .to_string();

    // Another client observes the blocked session — the state the surfaces
    // have always known how to paint, now actually set.
    let (peer_b, _incoming_b) = init_client(&endpoint).await;
    let mut waiting_seen = false;
    for _ in 0..100 {
        let list = peer_b
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
            .await
            .expect("session/list ok");
        if list["sessions"]
            .as_array()
            .and_then(|a| a.first())
            .is_some_and(|s| s["state"] == "waiting_permission")
        {
            waiting_seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        waiting_seen,
        "the session blocked on a human decision declares waiting_permission"
    );

    // Decide: the session goes back to work, and says so.
    peer_b
        .request(
            methods::PERMISSION_DECIDE,
            &json!({ "requestId": request_id, "optionId": allow }),
        )
        .await
        .expect("decide ok");
    let mut active_again = false;
    for _ in 0..100 {
        let list = peer_b
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
            .await
            .expect("session/list ok");
        let state = list["sessions"]
            .as_array()
            .and_then(|a| a.first())
            .map(|s| s["state"].clone());
        // `active` while the turn finishes, or already `ended` if it raced us:
        // either way it is no longer blocked on a human.
        if matches!(
            state.as_ref().and_then(|s| s.as_str()),
            Some("active") | Some("ended")
        ) {
            active_again = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(active_again, "the decided session is no longer waiting");
    peer_b.close();

    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose finished")
        .expect("join ok")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    hold.abort();
    peer_a.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_late_client_watches_a_session_it_did_not_start() {
    // Scenario: Cliente que llega a mitad de sesión.
    // Scenario: Sin declarar interés no llega el stream.
    let root = fixture("watch", Some("[[rule]]\neffect = \"allow\"\n"));
    // A deliberately slow turn: the mock holds it open before emitting
    // anything, so the watcher has a window to join a session already in
    // flight — which is the whole point of the scenario.
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}', '--turn-delay-ms', '600']\n",
            mock_agent_bin().display()
        ),
    )
    .unwrap();
    // SAFETY: no concurrent env readers in this test binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("perm-watch").await;
    let root_str = root.display().to_string();

    // A second connection that will NOT watch: it must stay silent.
    let (peer_silent, mut incoming_silent) = init_client(&endpoint).await;
    let silent_events = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let silent_counter = silent_events.clone();
    let silent = tokio::spawn(async move {
        while let Some(message) = incoming_silent.recv().await {
            if let Incoming::Notification { method, .. } = message
                && method == methods::SESSION_EVENT
            {
                silent_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });

    // A third connection that WILL watch. It learns the session id from
    // session/list, exactly as a phone joining mid-turn would.
    let (peer_watcher, mut incoming_watcher) = init_client(&endpoint).await;
    let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let watcher = tokio::spawn(async move {
        while let Some(message) = incoming_watcher.recv().await {
            if let Incoming::Notification { method, params } = message
                && method == methods::SESSION_EVENT
                && let Some(id) = params.get("sessionId").and_then(|v| v.as_str())
            {
                let _ = events_tx.send(id.to_string());
            }
        }
    });

    // The owner starts the turn. A slow mock keeps it alive long enough for
    // the watcher to join mid-session.
    let (peer_owner, incoming_owner) = init_client(&endpoint).await;
    let drain_owner = tokio::spawn(async move {
        let mut incoming_owner = incoming_owner;
        while incoming_owner.recv().await.is_some() {}
    });
    let propose = tokio::spawn({
        let peer_owner = peer_owner.clone();
        let root = root_str.clone();
        async move {
            peer_owner
                .request(
                    methods::PROPOSE,
                    &json!({ "idea": "stream to a late client", "projectRoot": root }),
                )
                .await
        }
    });

    // Find the live session and watch it.
    let mut session_id = None;
    for _ in 0..200 {
        let list = peer_watcher
            .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
            .await
            .expect("session/list ok");
        if let Some(first) = list["sessions"].as_array().and_then(|a| a.first()) {
            session_id = first["sessionId"].as_str().map(str::to_string);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let session_id = session_id.expect("a session appeared");
    let watch = peer_watcher
        .request(
            methods::SESSION_WATCH,
            &json!({ "sessionId": session_id, "watch": true }),
        )
        .await
        .expect("session/watch ok");
    assert_eq!(watch["watching"], true, "{watch:#}");
    assert_eq!(watch["sessionId"], session_id.as_str());

    // The turn completes; the owner got its stream as always.
    let result = tokio::time::timeout(Duration::from_secs(30), propose)
        .await
        .expect("propose finished")
        .expect("join ok")
        .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    // The watcher received events of a session it never started. (A race at
    // the very end of a fast turn could leave it with none; give the tail a
    // moment before asserting, and assert on what actually arrived.)
    let mut seen = Vec::new();
    while let Ok(Some(id)) =
        tokio::time::timeout(Duration::from_millis(200), events_rx.recv()).await
    {
        seen.push(id);
    }
    assert!(
        !seen.is_empty(),
        "the watcher received the stream of a session it never started"
    );
    assert!(
        seen.iter().all(|id| id == &session_id),
        "only the watched session's events arrive: {seen:?}"
    );

    // And the connection that never asked heard nothing at all.
    assert_eq!(
        silent_events.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a connection that did not watch receives no session events"
    );

    silent.abort();
    watcher.abort();
    drain_owner.abort();
    peer_silent.close();
    peer_watcher.close();
    peer_owner.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

/// A fixture whose mock asks a question before it works.
fn fixture_asking(tag: &str) -> PathBuf {
    let root = fixture(tag, Some("[[rule]]\neffect = \"allow\"\n"));
    let mock = mock_agent_bin();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{}', '--ask']\n", mock.display()),
    )
    .unwrap();
    root
}

// Scenario: Una pregunta llega con las opciones del agente
#[tokio::test]
async fn a_question_reaches_the_tray_with_its_options_and_the_turn_goes_on_with_the_answer() {
    // The whole loop, without a real agent and without the network: the agent
    // asks with its own options, the human picks one, and the turn continues
    // carrying that choice. The allow rule covers the mock's ordinary write
    // permission, so what reaches the tray is the QUESTION and nothing else —
    // which is what makes the assertions about it unambiguous.
    let root = fixture_asking("asking");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("asking").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let running = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "pick a route" }),
            )
            .await
        })
    };

    // Wait for the question, and read it as the surfaces would.
    let mut question = None;
    for _ in 0..400 {
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("permission/pending");
        if let Some(first) = pending["pending"].as_array().and_then(|all| all.first()) {
            question = Some(first.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let question = question.expect("the question reached the tray");
    let options: Vec<&str> = question["options"]
        .as_array()
        .expect("options")
        .iter()
        .map(|option| option["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        options,
        ["Rewrite it (recommended)", "Patch it"],
        "the agent's own labels reached the human, recommendation included: {question:#}"
    );

    // Answer it the way a surface does.
    let chosen = question["options"][1]["optionId"]
        .as_str()
        .expect("an option id");
    peer.request(
        methods::PERMISSION_DECIDE,
        &json!({ "requestId": question["requestId"], "optionId": chosen }),
    )
    .await
    .expect("permission/decide");

    let result = tokio::time::timeout(Duration::from_secs(30), running)
        .await
        .expect("the turn ended once the question was answered")
        .expect("join")
        .expect("session/start ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    // And the turn CONTINUED with the answer rather than merely receiving one.
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "sessionId": result["sessionId"], "projectRoot": root_str }),
        )
        .await
        .expect("session/log");
    let text =
        log["lines"]
            .as_array()
            .expect("lines")
            .iter()
            .fold(String::new(), |mut all, line| {
                all.push_str(line.as_str().unwrap_or_default());
                all
            });
    assert!(
        text.contains("answering with option-1"),
        "the agent carried on with the option the human picked: {text}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Una regla no contesta una pregunta por ti
#[tokio::test]
async fn a_blanket_allow_rule_does_not_answer_a_question_on_your_behalf() {
    // The defect this change would otherwise have introduced. Before it, a
    // question was refused outright, so no rule could ever answer one. Now that
    // questions are relayed, a bare `effect = "allow"` would have picked the
    // agent's FIRST option — silently, on the user's behalf, because it happens
    // to carry an "allow" kind.
    //
    // A rule has no opinion about which answer is right. The criterion is read
    // off the wire: a request that offers no way to REFUSE is not a permission
    // request, and deny-by-default means nothing where there is no deny.
    let root = fixture_asking("rule-and-question");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("rule-and-question").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let running = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "pick a route" }),
            )
            .await
        })
    };

    // The fixture's rule allows everything. The question STILL reaches a human.
    let mut question = None;
    for _ in 0..400 {
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("permission/pending");
        if let Some(first) = pending["pending"].as_array().and_then(|all| all.first()) {
            question = Some(first.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let question =
        question.expect("an allow rule must not answer a question: it escalated, as it has to");
    assert_eq!(
        question["options"].as_array().map(Vec::len),
        Some(2),
        "and it arrived as the question it is: {question:#}"
    );

    running.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Manual retira lo que las reglas concederían
// Scenario: El deny del usuario sobrevive a cualquier modo
#[tokio::test]
async fn the_same_request_resolves_differently_under_different_modes() {
    // The proof that a mode does something: one fixture, one request, three
    // answers. The project's rule ALLOWS everything, so without a mode the turn
    // never troubles a human — that is the baseline the other two are measured
    // against.
    let root = fixture("modes", Some("[[rule]]\neffect = \"allow\"\n"));
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("modes").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    // No mode: the rule grants, nothing reaches the tray, the turn completes.
    let plain = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({ "projectRoot": root_str, "instruction": "write it" }),
        ),
    )
    .await
    .expect("an allow rule needs no human")
    .expect("session/start ok");
    assert_eq!(plain["status"], "completed", "{plain:#}");

    // Manual: the SAME rule, the SAME request — and now it asks, because manual
    // takes back what the rules would have granted. This is the behaviour no
    // bundle of rules could have expressed.
    let asking = {
        let peer = peer.clone();
        let root = root_str.clone();
        tokio::spawn(async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root, "instruction": "write it", "mode": "manual" }),
            )
            .await
        })
    };
    let mut reached = false;
    for _ in 0..400 {
        let pending = peer
            .request(methods::PERMISSION_PENDING, &json!({}))
            .await
            .expect("permission/pending");
        if pending["pending"].as_array().is_some_and(|all| !all.is_empty()) {
            reached = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        reached,
        "manual asks about what the user's own rules would have granted"
    );
    asking.abort();

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Sin modo, la resolución es la de siempre
#[tokio::test]
async fn a_denying_rule_still_denies_under_the_most_permissive_mode() {
    // The one thing no mode may lift. Autonomous is the mode that grants most,
    // and it does not reach a refusal the user wrote down.
    let root = fixture("mode-deny", Some("[[rule]]\neffect = \"deny\"\n"));
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("mode-deny").await;
    let (peer, _incoming) = init_client(&endpoint).await;

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        peer.request(
            methods::SESSION_START,
            &json!({
                "projectRoot": root_str,
                "instruction": "write it",
                "mode": "autonomous",
            }),
        ),
    )
    .await
    .expect("a denying rule needs no human either")
    .expect("session/start ok");
    assert_eq!(
        result["deniedPermissions"], 1,
        "the user's deny was applied, not lifted: {result:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
