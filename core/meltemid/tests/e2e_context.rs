// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of context projection (proyeccion-contexto task 5.2): a
//! client drives an ephemeral daemon to project a temporary fixture repo into
//! its multiple declared targets, then verifies the managed blocks, the
//! preservation of user content, and idempotence on a second call.
//!
//! Runs against a temporary fixture, never this repo (constitution
//! §"tests e2e").

use serde_json::json;
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

fn init_params() -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: "e2e-context-client".into(),
            version: "0.0.0".into(),
        },
    }
}

#[tokio::test]
async fn context_project_writes_managed_targets_and_is_idempotent() {
    // A fixture repo with constitution + an `always` rumbo, and a hand-written
    // AGENTS.md whose content must be preserved.
    let fixture = std::env::temp_dir().join(format!("meltemi-e2e-context-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join(".meltemi").join("rumbo")).unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("constitution.md"),
        "# Constitución\n\nprincipios del proyecto\n",
    )
    .unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("rumbo").join("product.md"),
        "---\ninclusion: siempre\n---\nEL RUMBO DE PRODUCTO\n",
    )
    .unwrap();
    std::fs::write(
        fixture.join("AGENTS.md"),
        "# Notas a mano\n\nno me borres.\n",
    )
    .unwrap();

    // Ephemeral daemon.
    let endpoint = test_endpoint("context");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("e2e-context", shutdown_tx);
    tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, _incoming) = Peer::start(stream);
    peer.request(methods::INITIALIZE, &init_params())
        .await
        .expect("initialize");

    let root = fixture.display().to_string();

    // 1. First projection: every declared target is written with a fingerprint.
    let result = peer
        .request(methods::CONTEXT_PROJECT, &json!({ "projectRoot": root }))
        .await
        .expect("context/project ok");
    let targets = result["targets"].as_array().expect("targets array");
    assert!(targets.len() >= 2, "multiple targets declared: {result:#}");
    assert!(
        targets
            .iter()
            .any(|t| t["path"] == "AGENTS.md" && t["written"] == true),
        "AGENTS.md is written: {result:#}"
    );
    for target in targets {
        let fp = target["fingerprint"].as_str().unwrap();
        assert_eq!(fp.len(), 64, "fingerprint is a full sha-256");
    }

    // The managed block landed, and the user's content survived.
    let agents = std::fs::read_to_string(fixture.join("AGENTS.md")).unwrap();
    assert!(agents.contains("no me borres."), "user content preserved");
    assert!(
        agents.contains("meltemi:context:begin"),
        "managed block present"
    );
    assert!(agents.contains("EL RUMBO DE PRODUCTO"), "rumbo projected");
    assert!(
        agents.contains("principios del proyecto"),
        "constitution projected"
    );

    // 2. Second projection without source changes writes nothing (idempotent).
    let again = peer
        .request(methods::CONTEXT_PROJECT, &json!({ "projectRoot": root }))
        .await
        .expect("second context/project ok");
    assert!(
        again["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["written"] == false),
        "re-projection without source changes writes nothing: {again:#}"
    );

    // 3. Invalid project root is a contract error, not a panic.
    let bad = peer
        .request(
            methods::CONTEXT_PROJECT,
            &json!({ "projectRoot": fixture.join("does-not-exist").display().to_string() }),
        )
        .await;
    assert!(bad.is_err(), "an invalid project root is refused");

    peer.close();
    let _ = std::fs::remove_dir_all(&fixture);
}
