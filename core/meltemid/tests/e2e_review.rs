// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of spec review (revision-specs-ux task 5.2): the checklist
//! over a change's spec deltas — decide items (approve/comment/reject),
//! resumption across calls, and a close blocked while items remain pending.
//! Runs against a temporary fixture; the change's specs are written directly.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

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

async fn client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-review".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

fn write_change(root: &Path, change: &str) {
    let specs = root
        .join(".meltemi")
        .join("changes")
        .join(change)
        .join("specs")
        .join("cap");
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("spec.md"),
        "## ADDED Requirements\n### Requirement: One\nThe system SHALL a.\n\
         #### Scenario: s\n- **WHEN** x\n- **THEN** y\n\n\
         ### Requirement: Two\nThe system SHALL b.\n\
         #### Scenario: s\n- **WHEN** x\n- **THEN** y\n",
    )
    .unwrap();
}

async fn decide(peer: &Peer, root: &str, change: &str, req: &str, decision: &str) -> Value {
    peer.request(
        methods::SDD_REVIEW_DECIDE,
        &json!({
            "projectRoot": root, "changeName": change,
            "capability": "cap", "requirement": req, "decision": decision,
        }),
    )
    .await
    .expect("decide ok")
}

#[tokio::test]
async fn review_checklist_decides_items_and_gates_the_close() {
    let root: PathBuf =
        std::env::temp_dir().join(format!("meltemi-e2e-review-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    write_change(&root, "add-two");

    let endpoint = test_endpoint("review");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("e2e-review", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let peer = client(&endpoint).await;
    let root_str = root.display().to_string();

    // The fresh checklist has two pending items and cannot close.
    let listed = peer
        .request(
            methods::SDD_REVIEW,
            &json!({ "projectRoot": root_str, "changeName": "add-two" }),
        )
        .await
        .expect("review ok");
    assert_eq!(listed["items"].as_array().unwrap().len(), 2);
    assert_eq!(listed["pending"], 2);
    assert_eq!(listed["canClose"], false, "cannot close with pending items");

    // Decide the first; the state persists (a second call resumes it).
    let after_one = decide(&peer, &root_str, "add-two", "One", "approve").await;
    assert_eq!(after_one["pending"], 1, "one still pending: {after_one:#}");
    assert_eq!(after_one["canClose"], false);
    let one = after_one["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["requirement"] == "One")
        .unwrap();
    assert_eq!(one["state"], "approved");

    // Decide the second: now the review can close.
    let after_two = decide(&peer, &root_str, "add-two", "Two", "reject").await;
    assert_eq!(after_two["pending"], 0);
    assert_eq!(after_two["canClose"], true, "all decided → closable");

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
