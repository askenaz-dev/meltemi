// SPDX-License-Identifier: Apache-2.0

//! Ending an agent the platform put an intermediary in front of
//! (apagado-entero-y-modos task 1.3).
//!
//! Every other end-to-end test launches the simulated agent directly, which is
//! the shape of an agent whose installer ships an executable. It is not the
//! shape of the ones a package manager installs: on Windows those arrive as a
//! `.cmd` shim, so what the daemon launches is `cmd.exe` running a script, and
//! what speaks the protocol is a process underneath it. Ending the shim alone
//! leaves that process running — measured before this was written, and the
//! reason the daemon now launches inside a process scope.
//!
//! So this file puts the real intermediary in the chain:
//!
//! ```text
//! test client ─JSON-RPC→ meltemid ─launch→ cmd.exe (shim) ─launch→ mock-agent ─ACP→ meltemid
//! ```
//!
//! and asks the only question the other tests cannot: after the daemon has
//! ended the session, is the process underneath the shim gone?
//!
//! Windows only, and it says so rather than pretending: the shims elsewhere are
//! scripts with a shebang, the launcher's own process is the interpreter, and a
//! signal reaches the work. There is no intermediary to test.
#![cfg(windows)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// A workspace binary, next to this test's own executable.
fn workspace_bin(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let path = dir.join(format!("{name}.exe"));
    assert!(
        path.exists(),
        "{name} was not built at {}; run `cargo test` at the workspace root",
        path.display()
    );
    path
}

/// The fixture: a repository whose configured agent is a `.cmd` shim that
/// launches the simulated agent, exactly as an npm install would.
///
/// The marker is a unique argument the shim passes through. The simulated agent
/// ignores arguments it does not know, and it gives this test something to find
/// the grandchild by that no other test could answer to — these run in
/// parallel, and "a process running mock-agent" is not a unique description.
fn fixture(tag: &str) -> (PathBuf, String) {
    let root =
        std::env::temp_dir().join(format!("meltemi-e2e-apagado-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();

    let marker = format!("--e2e-apagado-{}-{tag}", std::process::id());
    let mock = workspace_bin("mock-agent");
    let shim = root.join("agent.cmd");
    // The turn is held open so the grandchild can be observed while it exists:
    // a test that looked for it after the session had ended would be asking
    // whether something absent is absent.
    std::fs::write(
        &shim,
        format!(
            "@echo off\r\n\"{}\" --turn-delay-ms 4000 {marker} %*\r\n",
            mock.display()
        ),
    )
    .unwrap();

    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}']\n",
            shim.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();

    let resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
    let shown = resolved.to_string_lossy();
    let resolved = shown
        .strip_prefix(r"\\?\")
        .map_or_else(|| resolved.clone(), PathBuf::from);
    (resolved, marker)
}

fn test_endpoint(tag: &str) -> String {
    format!(r"\\.\pipe\meltemid-apagado-{}-{tag}", std::process::id())
}

async fn client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-apagado".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Every process whose command line contains `needle`.
fn processes_matching(needle: &str) -> Vec<u32> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -and \
                 $_.CommandLine.Contains('{needle}') }} | ForEach-Object {{ $_.ProcessId }}"
            ),
        ])
        .output()
        .expect("powershell ships with the platform this test only runs on");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

/// Waits for `condition`, up to `limit`. Returns whether it came true.
async fn within(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    condition()
}

/// The session's log, as parsed events.
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
        .map(|line| {
            serde_json::from_str(line.as_str().expect("a JSONL line"))
                .expect("every logged line is JSON")
        })
        .collect()
}

// Scenario: Un lanzador intermedio no deja huérfanos
// Scenario: Cancelación de sesión
#[tokio::test]
async fn ending_a_session_ends_the_agent_the_shim_launched() {
    let (root, marker) = fixture("ends");
    let root_str = root.display().to_string();

    let endpoint = test_endpoint("ends");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("apagado-ends", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    let peer = client(&endpoint).await;

    let starting = tokio::spawn({
        let peer = peer.clone();
        let root_str = root_str.clone();
        async move {
            peer.request(
                methods::SESSION_START,
                &json!({ "projectRoot": root_str, "instruction": "look around" }),
            )
            .await
        }
    });

    // The grandchild, while it exists: `cmd.exe` has to parse the script and
    // launch, so it is not there the instant the session starts.
    let mut grandchildren = Vec::new();
    let appeared = within(Duration::from_secs(20), || {
        grandchildren = processes_matching(&marker);
        // The shim's own command line carries the marker too, so the process
        // this test is about is the one that is NOT the shim: at least two.
        grandchildren.len() >= 2
    })
    .await;
    assert!(
        appeared,
        "the shim never launched the agent underneath it; without that process \
         this test proves nothing (found: {grandchildren:?})"
    );

    let started = tokio::time::timeout(Duration::from_secs(60), starting)
        .await
        .expect("session/start returned")
        .expect("the task ran")
        .expect("session/start ok");
    assert_eq!(started["status"], "completed", "{started:#}");

    // The question this file exists to ask.
    for pid in &grandchildren {
        let gone = within(Duration::from_secs(20), || {
            !processes_matching(&marker).contains(pid)
        })
        .await;
        assert!(
            gone,
            "process {pid} outlived the session: ending the shim is not ending \
             what the shim launched"
        );
    }

    // And the session log says which process it was — the line somebody needs
    // when they are looking at a process that outlived something.
    let events = session_events(&peer, &root_str).await;
    let process = events
        .iter()
        .find(|event| event["type"] == "agent_process")
        .expect("the session log records the agent's process");
    assert!(
        process["payload"]["pid"]
            .as_u64()
            .is_some_and(|pid| pid > 0),
        "the recorded identifier names a process: {process:#}"
    );
    assert!(
        process["payload"]["binary"]
            .as_str()
            .is_some_and(|binary| binary.ends_with("agent.cmd")),
        "the recorded program is what the platform launched — the shim, not the \
         agent underneath it: {process:#}"
    );

    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
