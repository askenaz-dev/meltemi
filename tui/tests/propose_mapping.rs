// SPDX-License-Identifier: Apache-2.0

//! End-to-end propose mapping (task 5.3). Isolated in its own test binary
//! because it clears a process-global environment variable; being the only
//! test here keeps that mutation free of concurrent readers.

mod common;

use std::time::Duration;

use meltemi::cli::Command;
use meltemi::run::execute;

use common::{mock_agent_bin, spawn_daemon};

#[tokio::test]
async fn propose_maps_to_the_propose_method() {
    // Scenario: propose -> initialize + propose. The scaffold (change name and
    // proposal path) is returned regardless of the agent turn's outcome; in
    // scriptable mode the permission is denied by default, which is enough to
    // exercise the RPC mapping end to end.
    let agent = mock_agent_bin();
    assert!(
        agent.exists(),
        "mock-agent binary not found at {}; run `cargo test` at the workspace root",
        agent.display()
    );

    let fixture = std::env::temp_dir().join(format!("meltemi-cli-fixture-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join(".meltemi")).unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("config.toml"),
        format!("[agent]\ncommand = ['{}']\n", agent.display()),
    )
    .unwrap();
    // Make sure no env override shadows the fixture config.
    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let (endpoint, handle) = spawn_daemon("propose").await;

    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        execute(
            Command::Propose {
                idea: "add greeting".into(),
                project_root: Some(fixture.display().to_string()),
            },
            &endpoint,
        ),
    )
    .await
    .expect("propose returned within 30s")
    .expect("propose succeeds");

    assert_eq!(
        outcome.json["changeName"], "add-greeting",
        "propose must map to the RPC and return the scaffolded change name, got: {}",
        outcome.json
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}
