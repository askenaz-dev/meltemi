// SPDX-License-Identifier: Apache-2.0

//! A verb operating through a command instead of a socket
//! (via-ssh-del-cliente).
//!
//! The fixture is the shape the feature is for, with the network taken out of
//! it: a script that names an endpoint and runs `meltemi bridge`, which is
//! exactly what `ssh my-pc meltemi bridge` does once the connection is made.
//! Testing it with a real `ssh` would need a host, a key and a network, and
//! would be testing OpenSSH rather than this.
//!
//! The script is also not an accident: the design says a command that needs more
//! than a program and arguments names a script, and on Windows a `.cmd` puts
//! `cmd.exe` between the client and the work — so this exercises the process
//! scope at the same time.

mod common;

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use common::spawn_daemon;

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_meltemi"))
}

/// Writes the script that puts the test daemon on its own stdio, and answers the
/// `exec:` endpoint that names it.
fn bridge_script(dir: &std::path::Path, endpoint: &str) -> String {
    std::fs::create_dir_all(dir).expect("a temp directory");
    let cli = cli().display().to_string();
    #[cfg(windows)]
    {
        let script = dir.join("bridge.cmd");
        std::fs::write(
            &script,
            format!("@echo off\r\nset MELTEMI_ENDPOINT={endpoint}\r\n\"{cli}\" bridge\r\n"),
        )
        .expect("write the script");
        format!("exec:\"{}\"", script.display())
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("bridge.sh");
        std::fs::write(
            &script,
            format!("#!/bin/sh\nexport MELTEMI_ENDPOINT='{endpoint}'\nexec '{cli}' bridge\n"),
        )
        .expect("write the script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("make it executable");
        format!("exec:{}", script.display())
    }
}

// Scenario: El cliente habla por el comando que el usuario nombró
// Scenario: Un verbo cualquiera opera a través del comando
#[tokio::test]
async fn a_verb_answers_through_the_command_as_if_the_daemon_were_local() {
    let (endpoint, daemon) = spawn_daemon("exec-endpoint").await;
    let dir = std::env::temp_dir().join(format!("meltemi-exec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let exec = bridge_script(&dir, &endpoint);

    let out = tokio::process::Command::new(cli())
        .args(["--json", "status"])
        .env("MELTEMI_ENDPOINT", &exec)
        .stdin(Stdio::null())
        .output();
    let out = tokio::time::timeout(Duration::from_secs(60), out)
        .await
        .expect("the verb answered")
        .expect("it ran");

    assert!(
        out.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let answer: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "the answer is JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    assert!(
        answer
            .get("daemonVersion")
            .and_then(Value::as_str)
            .is_some(),
        "the daemon's own answer came back through the command: {answer:#}"
    );
    assert!(
        answer.get("sessions").is_some(),
        "and the whole answer, not a fragment of it: {answer:#}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    daemon.abort();
}

// Scenario: El comando falla y no se arranca nada
#[tokio::test]
async fn a_command_that_fails_is_refused_and_starts_no_daemon() {
    // A command that exits at once. Nothing must be started locally to cover
    // for it: a local daemon would answer in place of the machine the user
    // meant.
    let exec = if cfg!(windows) {
        r#"exec:cmd /c "exit 3""#
    } else {
        r#"exec:sh -c "exit 3""#
    };
    let out = tokio::process::Command::new(cli())
        .args(["--json", "status"])
        .env("MELTEMI_ENDPOINT", exec)
        // Even asked to, it must not: the guard is unconditional.
        .env("MELTEMI_START_HERE", "1")
        .stdin(Stdio::null())
        .output();
    let out = tokio::time::timeout(Duration::from_secs(60), out)
        .await
        .expect("it refused rather than hanging")
        .expect("it ran");

    assert!(!out.status.success(), "a dead command is not a success");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        said.contains("ended") || said.contains("names a command"),
        "the refusal explains itself: {said}"
    );
    assert!(
        !said.contains("MELTEMI_START_HERE"),
        "and it does not point at a way that is shut: {said}"
    );
}
