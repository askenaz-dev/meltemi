// SPDX-License-Identifier: Apache-2.0

//! The integrated run's scripts (corrida-integrada): they spend real turns, so
//! nothing here runs them — this pins what they promise. Their daemon is their
//! own, the matrix is the same in both, and no workflow ever runs them.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("core/meltemid lives two levels under the repo root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

const SCRIPTS: [&str; 2] = ["scripts/smoke-agents.ps1", "scripts/smoke-agents.sh"];

// Scenario: La corrida no toca el daemon del usuario
#[test]
fn the_run_isolates_its_daemon_by_the_five_variables() {
    for script in SCRIPTS {
        let text = read(script);
        // Every one of the five, set by the script itself…
        for variable in [
            "MELTEMI_ENDPOINT",
            "MELTEMI_DATA_DIR",
            "MELTEMI_CONFIG_DIR",
            "MELTEMI_DAEMON_BIN",
            "MELTEMI_START_HERE",
        ] {
            let set = text.lines().any(|line| {
                let line = line.trim_start();
                (line.starts_with(&format!("$env:{variable} ="))
                    || line.starts_with(&format!("export {variable}=")))
                    && !line.contains("$env:APPDATA")
            });
            assert!(set, "{script} sets {variable} itself");
        }
        // …under the run's own temporary base, never the user's places.
        for user_place in [
            "APPDATA",
            "Application Support",
            ".local/share/meltemi",
            "meltemid-S-",
        ] {
            assert!(
                !text.contains(user_place),
                "{script} names a place of the user's own: {user_place}"
            );
        }
        // An agent command inherited from the shell would replace the fleet.
        assert!(text.contains("MELTEMI_AGENT_COMMAND"), "{script} clears it");
        // Teardown stops only what it started.
        assert!(text.contains("stop"), "{script} asks its daemon to stop");
    }
    let ps = read("scripts/smoke-agents.ps1");
    assert!(
        ps.contains("Where-Object { $_.Path -eq $meltemid }"),
        "only its own daemon binary is stopped"
    );
}

#[test]
fn both_scripts_run_the_same_seven_verbs_and_declare_what_is_not_installed() {
    for script in SCRIPTS {
        let text = read(script);
        for verb in [
            "'session'",
            "'propose'",
            "'assign'",
            "'dispatch'",
            "'direct'",
            "'cancel'",
        ] {
            let bare = verb.trim_matches('\'');
            assert!(
                text.contains(verb) || text.contains(&format!("--json {bare}")),
                "{script} runs `{bare}`"
            );
        }
        assert!(text.contains("**no medido**: no instalado"), "{script}");
        assert!(text.contains("path-prefix"), "{script} writes its rule");
        assert!(
            !text.contains("tool = \"execute\""),
            "{script} grants no command"
        );
    }
}

// Scenario: La corrida no corre en CI
#[test]
fn no_workflow_runs_the_integrated_run() {
    let workflows = repo_root().join(".github").join("workflows");
    let entries = std::fs::read_dir(&workflows).expect("the workflows directory");
    let mut seen = 0;
    for entry in entries.flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        seen += 1;
        assert!(
            !text.contains("smoke-agents"),
            "{} runs the integrated run",
            entry.path().display()
        );
    }
    assert!(seen > 0, "the workflows were read");
}
