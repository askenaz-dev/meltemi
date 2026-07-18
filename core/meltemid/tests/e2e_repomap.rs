// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of repo context (gestion-contexto-repo task 4.2): a prompt
//! with an `@` reference reaches the agent expanded (the session log records
//! the expansion and the fenced content), and `repo/map` returns the tree
//! honoring gitignore. Runs against a temporary fixture and the mock.

use std::path::PathBuf;

use serde_json::json;
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

#[tokio::test]
async fn at_reference_reaches_the_agent_expanded_and_repo_map_honors_gitignore() {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-repo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{}']\n", mock_agent_bin().display()),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(root.join(".gitignore"), "secret.txt\n").unwrap();
    std::fs::write(root.join("ref.txt"), "THE-INJECTED-CONTENT").unwrap();
    std::fs::write(root.join("secret.txt"), "hidden").unwrap();
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let endpoint = test_endpoint("repo");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("e2e-repo", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-repo".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    let root_str = root.display().to_string();

    // repo/map excludes the gitignored file and reports sizes.
    let map = peer
        .request(methods::REPO_MAP, &json!({ "projectRoot": root_str }))
        .await
        .expect("repo/map ok");
    let paths: Vec<&str> = map["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"ref.txt"), "tracked file listed: {paths:?}");
    assert!(
        !paths.iter().any(|p| p.contains("secret")),
        "ignored excluded"
    );

    // A propose whose idea references @ref.txt expands it into the prompt.
    let proposed = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "review @ref.txt now", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(proposed["status"], "completed", "{proposed:#}");

    // The session log records the expansion (path + bytes) and the expanded
    // prompt carries the fenced content — the context is reconstructable.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let sid = list["sessions"].as_array().unwrap()[0]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str, "sessionId": sid }),
        )
        .await
        .expect("session/log ok");
    let text = log["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("refs_expanded"), "expansion audited: {text}");
    assert!(text.contains("ref.txt"), "the reference path is recorded");
    assert!(
        text.contains("THE-INJECTED-CONTENT"),
        "the expanded content reached the prompt (prompt_sent): {text}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
