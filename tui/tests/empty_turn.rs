// SPDX-License-Identifier: Apache-2.0

//! The CLI's side of the empty turn (turno-vacio-es-error design D4), end to
//! end: the real grammar, the real dispatch, an in-process daemon and the
//! scripted `mock-agent` falling silent — never this repository, never a real
//! agent, never the network.
//!
//! Isolated in its own test binary, and written as a single test, because it
//! clears a process-global environment variable.

mod common;

use std::path::{Path, PathBuf};

use common::{mock_agent_bin, spawn_daemon};

/// A fixture whose agent is the mock with `mock_args`, allowed to do anything.
fn fixture(tag: &str, mock_args: &[&str]) -> PathBuf {
    let mock = mock_agent_bin();
    assert!(
        mock.exists(),
        "mock-agent binary not found at {}; run `cargo build --workspace` first",
        mock.display()
    );
    let mut command = format!("'{}'", mock.display().to_string().replace('\\', "/"));
    for arg in mock_args {
        command.push_str(&format!(", '{arg}'"));
    }
    let root = std::env::temp_dir().join(format!("meltemi-cli-empty-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = [{command}]\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    root
}

/// Runs one command line through the shipped grammar and dispatch, returning
/// the exit code, stdout and stderr.
async fn run(endpoint: &str, args: &[&str]) -> (i32, String, String) {
    let owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    let plan = meltemi::cli::plan(&owned, false);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        meltemi::dispatch(plan, endpoint, false, &mut out, &mut err),
    )
    .await
    .expect("the command answered");
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn root_str(root: &Path) -> String {
    root.display().to_string()
}

// Scenario: Turno vacío distinguible
// Scenario: La advertencia va a stderr con su remedio
// Scenario: La salida de máquina lleva el veredicto
#[tokio::test]
async fn an_empty_turn_warns_on_stderr_and_exits_fifteen() {
    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let (endpoint, daemon) = spawn_daemon("empty-turn").await;

    // The agent says nothing: exit 15, the warning and its remedy on stderr,
    // and stdout keeps only what the command itself answers.
    let silent = fixture("silent", &["--silent-turn"]);
    let (code, out, err) = run(
        &endpoint,
        &["session", "fix the failing build", &root_str(&silent)],
    )
    .await;
    assert_eq!(code, 15, "stdout: {out}\nstderr: {err}");
    assert!(
        err.contains("without saying or doing anything")
            && err.contains("provider")
            && err.contains("credentials"),
        "the warning and its remedy go to stderr: {err}"
    );
    assert!(
        !out.contains("without saying or doing anything"),
        "stdout carries only the command's output: {out}"
    );

    // Machine mode: the verdict is the answer's own field, and the code is the
    // same.
    let (code, out, err) = run(
        &endpoint,
        &[
            "--json",
            "session",
            "fix the failing build",
            &root_str(&silent),
        ],
    )
    .await;
    assert_eq!(code, 15, "stdout: {out}\nstderr: {err}");
    let answer: serde_json::Value = serde_json::from_str(out.trim()).expect("stdout is JSON");
    assert_eq!(answer["emptyTurn"], true, "{answer:#}");
    assert_eq!(answer["status"], "completed", "{answer:#}");

    // The same verb whose turn carried work exits 0, without the warning.
    let worked = fixture("worked", &[]);
    let (code, out, err) = run(
        &endpoint,
        &["session", "fix the failing build", &root_str(&worked)],
    )
    .await;
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    assert!(!err.contains("without saying or doing anything"), "{err}");

    daemon.abort();
    let _ = std::fs::remove_dir_all(&silent);
    let _ = std::fs::remove_dir_all(&worked);
}
