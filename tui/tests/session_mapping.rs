// SPDX-License-Identifier: Apache-2.0

//! End-to-end mapping of the free-session verb (lanzador-conversacional 5.1).
//! Isolated in its own test binary, and written as a single test, because it
//! clears a process-global environment variable: being the only reader keeps
//! that mutation free of concurrent threads.
//!
//! Everything runs against a temporary fixture and the scripted `mock-agent` —
//! never this repository, never a real agent, never the network.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use meltemi::cli::{Action, Command};
use meltemi::run::execute;

use common::{mock_agent_bin, spawn_daemon};

/// A fixture repository whose registry declares an agent that exists nowhere,
/// plus a launch profile over it spelled with capitals. That profile is the
/// instrument: naming it exactly must refuse (its binary is absent), while any
/// other spelling is a free label that falls back to the configured agent and
/// works. So the refusal itself proves the capitals crossed the wire.
fn fixture() -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo test` at the workspace root",
        mock.display()
    );
    let mock = mock.display().to_string().replace('\\', "/");

    let root = std::env::temp_dir().join(format!("meltemi-cli-free-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        "version = \"cli-free\"\n\
         [[agents]]\nid = \"provider-absent\"\nname = \"Provider Absent\"\nlevel = 1\n\
         bin = \"meltemi-no-such-binary\"\nacp-args = []\ncandidate-paths = []\n",
    )
    .unwrap();
    let registry = root
        .join(".meltemi/registry.toml")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!(
            "[agent]\ncommand = ['{mock}']\n\n\
             [fleet]\nregistry = '{registry}'\n\n\
             [[fleet.profile]]\nname = \"Work-Sub\"\nagent = \"provider-absent\"\n"
        ),
    )
    .unwrap();
    root
}

/// The `Command` an argument line resolves to: the mapping is exercised through
/// the grammar the binary ships with, not a hand-built value.
fn command_of(args: &[&str]) -> Command {
    let owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    match meltemi::cli::plan(&owned, false).action {
        Action::Run(command) => command,
        other => panic!("`{args:?}` must resolve to a runnable command, got {other:?}"),
    }
}

// Scenario: El arranque de sesión libre tiene su subcomando
// Scenario: Arranque con agente nombrado desde la CLI
#[tokio::test]
async fn the_free_session_verb_maps_to_session_start() {
    let root = fixture();
    let root_str = root.display().to_string();
    // Make sure no env override shadows the fixture config.
    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }

    let (endpoint, handle) = spawn_daemon("free").await;

    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        execute(
            command_of(&["session", "look at the failing build", &root_str]),
            &endpoint,
            false,
        ),
    )
    .await
    .expect("session returned within 30s")
    .expect("session succeeds");

    // What the scriptable client gets is final, and the id comes first: it is
    // the argument every other verb takes.
    let session_id = outcome.json["sessionId"]
        .as_str()
        .unwrap_or_else(|| panic!("the result names the session it created: {}", outcome.json));
    assert!(!session_id.is_empty());
    assert_eq!(outcome.json["status"], "completed", "{}", outcome.json);
    assert!(
        outcome.human.contains(session_id) && outcome.human.contains("[completed]"),
        "the human output presents the id and the turn's outcome: {}",
        outcome.human
    );

    // A temp directory is no git repository, and that is declared with the
    // remedy that fits — never the other cause's.
    assert_eq!(
        outcome.json["checkpointUnavailable"], "not_a_git_repo",
        "{}",
        outcome.json
    );
    assert!(
        outcome.human.contains("no restore point") && outcome.human.contains("remedy:"),
        "the absence of a restore point is stated, not hidden: {}",
        outcome.human
    );

    // In scriptable mode there is no approver, so the denied permission is
    // declared rather than passed over.
    assert!(
        outcome.human.contains("may be incomplete"),
        "{}",
        outcome.human
    );

    // `--agent` reaches the daemon exactly as it was typed. `Work-Sub` is a
    // profile over an agent this machine does not have, so it must refuse…
    let refused = tokio::time::timeout(
        Duration::from_secs(30),
        execute(
            command_of(&["session", "ship it", &root_str, "--agent", "Work-Sub"]),
            &endpoint,
            false,
        ),
    )
    .await
    .expect("session returned within 30s")
    .expect_err("a profile whose agent is not on this machine cannot run here");
    assert_eq!(refused.exit, meltemi::exit::ExitCode::Contract);

    // …while `work-sub` matches no profile and no catalog id, so it is a free
    // label that falls back to the configured agent and runs. The pair is the
    // proof: had the flag been lowercased on the way, the first call would have
    // succeeded like this one instead of refusing.
    let fallback = tokio::time::timeout(
        Duration::from_secs(30),
        execute(
            command_of(&["session", "ship it", &root_str, "--agent", "work-sub"]),
            &endpoint,
            false,
        ),
    )
    .await
    .expect("session returned within 30s")
    .expect("a free label falls back to the configured agent");
    assert_eq!(fallback.json["status"], "completed", "{}", fallback.json);

    handle.abort();
    let _ = std::fs::remove_dir_all(&root);
}
