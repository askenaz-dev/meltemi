// SPDX-License-Identifier: Apache-2.0

//! Running the linking gesture is explicit and visible (identidad-propia).
//!
//! The claim under test is not "a flag exists" but that `--exec` runs **the
//! user's own binary** with **stdio inherited**, so whatever that client prints —
//! the URL the user has to open — reaches them unfiltered. That is provable
//! rather than assertable by reading the source: the simulated mesh client
//! refuses any argv that is not `status --json` and complains on stderr, so if
//! its complaint turns up in this test's pipe, the chain client → CLI → caller
//! was never interrupted. If the CLI had captured or dropped the child's output,
//! nothing would arrive.

mod common;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use common::spawn_daemon;

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_meltemi"))
}

/// The simulated mesh client, copied beside the test under a neutral name.
fn mock_mesh(dir: &Path) -> PathBuf {
    let mut built = std::env::current_exe().expect("current exe");
    built.pop();
    if built.ends_with("deps") {
        built.pop();
    }
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let built = built.join(format!("mock-mesh{exe}"));
    assert!(
        built.is_file(),
        "{} is missing: run `cargo build --workspace --bins` first",
        built.display()
    );
    let copy = dir.join(format!("mock-mesh{exe}"));
    std::fs::copy(&built, &copy).expect("copy the simulated client");
    copy
}

/// The user configuration the test daemon will read, naming `client`.
///
/// Written into the directory `DaemonState::for_test` derives from the tag, which
/// is also why the tag is spelled the same in both places.
fn write_user_config(tag: &str, client: &Path) {
    let config_dir = std::env::temp_dir()
        .join(format!("meltemid-state-{}-{tag}", std::process::id()))
        .join("config");
    std::fs::create_dir_all(&config_dir).expect("the config directory");
    std::fs::write(
        config_dir.join("config.toml"),
        format!(
            "[identity]\nmesh = \"tailscale\"\nlogin-server = \"https://headscale.example\"\ncommand = {}\n",
            serde_json::json!(client.to_string_lossy())
        ),
    )
    .expect("the user configuration");
}

/// Runs the CLI against `endpoint`, returning (code, stdout, stderr).
///
/// `tokio::process`, not `std::process`: the daemon under test runs as a task on
/// this very runtime, so a blocking wait here would stop it from ever accepting
/// the connection — a deadlock that looks exactly like a hung CLI.
async fn run(endpoint: &str, args: &[&str]) -> (Option<i32>, String, String) {
    let out = tokio::process::Command::new(cli())
        .args(args)
        .env("MELTEMI_ENDPOINT", endpoint)
        // Nothing must start a daemon of its own: the one spawned by the test is
        // the daemon under test.
        .env_remove("MELTEMI_START_HERE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run the CLI");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// Scenario: Ejecutar es explícito y visible
#[tokio::test]
async fn exec_runs_the_users_own_client_and_its_output_reaches_the_user() {
    let tag = "identity-exec";
    let dir = std::env::temp_dir().join(format!("meltemi-identity-exec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temp directory");
    let client = mock_mesh(&dir);
    write_user_config(tag, &client);
    let (endpoint, _daemon) = spawn_daemon(tag).await;

    // Without `--exec`: the gesture is printed and NOTHING is launched. The proof
    // that nothing ran is the absence of the client's refusal, which it emits for
    // any argv but the read-only one.
    let (code, stdout, stderr) = run(&endpoint, &["identity-link"]).await;
    assert_eq!(code, Some(0), "stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("tailscale") || stdout.contains("mock-mesh"),
        "the gesture is printed: {stdout}"
    );
    assert!(
        !stdout.contains("mock-mesh refuses") && !stderr.contains("mock-mesh refuses"),
        "printing the gesture must launch no process: {stdout} {stderr}"
    );

    // With `--exec`: the user's own binary runs. This client refuses anything but
    // `status --json`, so it complains and fails — and its complaint arriving here
    // is what proves the stdio was INHERITED rather than captured or dropped.
    let (code, stdout, stderr) = run(&endpoint, &["identity-link", "--exec"]).await;
    let both = format!("{stdout}{stderr}");
    assert!(
        both.contains("mock-mesh refuses"),
        "the client's own words reach the user unfiltered: {both}"
    );
    assert!(
        both.contains("never invoke anything that changes mesh state"),
        "the whole message, not a summary of it: {both}"
    );
    // A client that ended badly is never reported as a success.
    assert_eq!(
        code,
        Some(10),
        "a failed link exits non-zero: {stdout} {stderr}"
    );
    assert!(
        both.contains("not linked") || both.contains("identity_link_failed"),
        "and says what did not happen: {both}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
