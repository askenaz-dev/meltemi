// SPDX-License-Identifier: Apache-2.0

//! Integration-level conformance suite (niveles-integracion-conformidad tasks
//! 3.2/3.3/4.1). Executable pass/fail criteria per level, run against
//! **simulated agents only** — never the network, never real agents in CI.
//! Running against real agents is manual and opt-in via
//! `MELTEMI_CONFORMANCE_REAL=1`; this file never launches one.
//!
//! Levels exercised: L1 native ACP (`mock-agent`), L2 adapter (the same mock
//! standing in as an ACP adapter), L3 structured headless (`mock-headless`),
//! L4 artifacts (context projection). The result per agent is persisted with
//! its run date; the last run is what `fleet/list` reports as verified.

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{
    ConformanceCriterion, ConformanceResult, InitializeParams, PROTOCOL_VERSION, PeerInfo, methods,
};
use meltemid::conformance;
use meltemid::levels::map_headless_line;
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn bin(name: &str) -> PathBuf {
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
        format!(r"\\.\pipe\meltemid-conf-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-conf-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// A fixture project that pilots `mock-agent` at the given level via a
/// substitute registry, with a bare allow rule so the mock's write proceeds.
fn fixture(root: &Path, id: &str, registry_body: &str) {
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(root.join("registry.toml"), registry_body).unwrap();
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\nid = \"{id}\"\n\n[fleet]\nregistry = '{}'\n",
            root.join("registry.toml").display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi").join("permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
}

async fn drive_propose(root: &Path, tag: &str) -> bool {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "conformance".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": format!("conformance {tag}"), "projectRoot": root.display().to_string() }),
        ),
    )
    .await;
    peer.close();
    daemon.abort();
    matches!(result, Ok(Ok(v)) if v["status"] == "completed")
}

#[tokio::test]
async fn conformance_suite_runs_every_level_against_simulated_agents() {
    // CI-safety (Scenario: Conformidad en CI con simulados): this suite must
    // never opt into real agents on its own.
    assert!(
        std::env::var("MELTEMI_CONFORMANCE_REAL").is_err(),
        "the conformance suite runs against mocks; real agents are manual opt-in"
    );

    let mock = bin("mock-agent");
    let headless = bin("mock-headless");
    assert!(
        mock.exists() && headless.exists(),
        "run `cargo test` at the workspace root"
    );
    // SAFETY: single test in this binary.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
        std::env::remove_var("MELTEMI_FLEET_REGISTRY");
    }

    let base = std::env::temp_dir().join(format!("meltemi-conf-suite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut criteria = Vec::new();

    // --- Level 1: native ACP session streams and completes. ---
    let l1 = base.join("l1");
    fixture(
        &l1,
        "sim-l1",
        &format!(
            "version=\"conf\"\n[[agents]]\nid=\"sim-l1\"\nname=\"Sim L1\"\nlevel=1\nbin='{}'\n",
            mock.display()
        ),
    );
    criteria.push(ConformanceCriterion {
        level: 1,
        name: "acp_session_completes".into(),
        passed: drive_propose(&l1, "l1").await,
    });

    // --- Level 2: adapter bridges to ACP transparently. ---
    let l2 = base.join("l2");
    fixture(
        &l2,
        "sim-l2",
        &format!(
            "version=\"conf\"\n[[agents]]\nid=\"sim-l2\"\nname=\"Sim L2\"\nlevel=2\n\
             bin='{}'\nadapter='{}'\n",
            mock.display(),
            mock.display()
        ),
    );
    criteria.push(ConformanceCriterion {
        level: 2,
        name: "adapter_bridges_to_acp".into(),
        passed: drive_propose(&l2, "l2").await,
    });

    // --- Level 3: structured headless output maps, and the rest is kept raw. ---
    let output = StdCommand::new(&headless)
        .output()
        .expect("run mock-headless");
    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let mapped: Vec<_> = lines.iter().map(|l| map_headless_line(l)).collect();
    let has_text = mapped
        .iter()
        .any(|e| matches!(e, meltemi_proto::SessionEventKind::AgentUpdate { .. }));
    criteria.push(ConformanceCriterion {
        level: 3,
        name: "structured_output_mapped".into(),
        passed: output.status.success() && has_text && lines.len() >= 3,
    });

    // --- Level 4: projection is the only channel; it includes the target. ---
    let l4 = base.join("l4");
    std::fs::create_dir_all(l4.join(".meltemi").join("rumbo")).unwrap();
    std::fs::write(l4.join(".meltemi").join("constitution.md"), "# C\n").unwrap();
    std::fs::write(
        l4.join(".meltemi").join("rumbo").join("product.md"),
        "---\ninclusion: siempre\n---\nR\n",
    )
    .unwrap();
    let written =
        meltemid::context::project_and_write_with(&l4, Some("GEMINI.md")).expect("projection");
    criteria.push(ConformanceCriterion {
        level: 4,
        name: "projection_includes_target".into(),
        passed: written.iter().any(|w| w.path == "GEMINI.md" && w.wrote),
    });

    // Every criterion passed against the mocks.
    for c in &criteria {
        assert!(c.passed, "level {} criterion `{}` failed", c.level, c.name);
    }

    // Persist the result and read it back (Scenario: Resultado persistido).
    let verified_level = criteria
        .iter()
        .filter(|c| c.passed)
        .map(|c| c.level)
        .max()
        .unwrap_or(0);
    let result = ConformanceResult {
        agent_id: "sim-l1".into(),
        verified_level,
        agent_version: Some("mock-1".into()),
        run_at: "2026-07-18T00:00:00Z".into(),
        criteria,
    };
    let data_dir = base.join("data");
    conformance::persist(&data_dir, &result).unwrap();
    let latest = conformance::latest_by_agent(&data_dir);
    assert_eq!(
        latest.get("sim-l1").map(|r| r.verified_level),
        Some(verified_level),
        "the run is persisted with its verified level"
    );

    let _ = std::fs::remove_dir_all(&base);
}
