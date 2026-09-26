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

const REPORT: &str = "docs/qa/2026-09-26-corrida-integrada.md";

// Scenario: Cada agente listo pasa por la misma matriz
#[test]
fn the_published_run_has_every_ready_agent_through_the_seven_verbs() {
    let report = read(REPORT);
    assert!(report.contains("# Corrida integrada — 2026-09-26"), "dated");
    for (agent, version) in [
        ("opencode", "1.18.32"),
        ("claude-code", "2.1.280"),
        ("codex-cli", "0.156.1"),
    ] {
        assert!(
            report
                .lines()
                .any(|line| line.starts_with(&format!("| `{agent}` |")) && line.contains(version)),
            "{agent} is listed with its version"
        );
        let section = report
            .split(&format!("### `{agent}`"))
            .nth(1)
            .unwrap_or_else(|| panic!("{agent} has its section"));
        let section = section.split("\n### ").next().unwrap_or(section);
        for verb in [
            "| session (read) |",
            "| session (write, no rule) |",
            "| session (write, rule) |",
            "| propose |",
            "| dispatch |",
            "| direct |",
            "| cancel |",
        ] {
            assert!(section.contains(verb), "{agent}: {verb}");
        }
    }
    // What the run found is written down, not left to the table.
    assert!(report.contains("## Hallazgos"));
    assert!(report.contains("**OpenCode 1.18.32 edita sin pedir permiso.**"));
}

// Scenario: Lo no instalado se declara
#[test]
fn the_published_run_declares_what_was_not_installed() {
    let report = read(REPORT);
    for agent in [
        "gemini-cli",
        "copilot-cli",
        "cursor-cli",
        "kiro-cli",
        "kilo-code",
        "aider",
        "antigravity",
    ] {
        assert!(
            report.contains(&format!("| `{agent}` | **no medido**: no instalado |")),
            "{agent} is declared as not measured"
        );
    }
    assert!(report.contains("## Lo no medido"));
}
