// SPDX-License-Identifier: Apache-2.0

//! The CLI and the on-demand start (acceso-remoto-afinado): at an explicit
//! endpoint that does not answer, a verb reports the daemon unreachable at
//! once, naming the tunnel and how to ask for a start — and, asked, it starts
//! the real `meltemid` there and proceeds.
//!
//! One test in phases, because it sets process-global environment variables
//! that the daemon it starts inherits. Every location is temporary.

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use meltemi::cli::Command;
use meltemi::exit::ExitCode;
use meltemi::run::execute;
use meltemi_client::bootstrap::{ENV_DAEMON_BIN, ENV_START_HERE};
use meltemi_client::paths::{ENV_CONFIG_DIR, ENV_DATA_DIR, ENV_ENDPOINT};

use common::test_endpoint;

/// The built `meltemid` next to the test executable (a workspace build makes
/// it).
fn meltemid_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "meltemid.exe"
    } else {
        "meltemid"
    })
}

fn set(name: &str, value: &str) {
    // SAFETY: this test binary runs a single test, one phase after another.
    unsafe { std::env::set_var(name, value) }
}

fn unset(name: &str) {
    // SAFETY: as in `set`.
    unsafe { std::env::remove_var(name) }
}

// Scenario: Un endpoint explícito caído se reporta sin arrancar nada
// Scenario: Arranque bajo demanda al no haber daemon
#[tokio::test]
async fn a_verb_starts_the_daemon_only_where_it_may() {
    let daemon = meltemid_bin();
    assert!(
        daemon.exists(),
        "meltemid not found at {}; run `cargo build --workspace` first",
        daemon.display()
    );
    let endpoint = test_endpoint("on-demand");
    let base = std::env::temp_dir().join(format!("meltemi-cli-on-demand-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    set(ENV_ENDPOINT, &endpoint);
    set(ENV_DATA_DIR, &base.join("data").display().to_string());
    set(ENV_CONFIG_DIR, &base.join("config").display().to_string());
    set(ENV_DAEMON_BIN, &daemon.display().to_string());
    unset(ENV_START_HERE);

    // Unasked: unreachable at once, with the tunnel and the way to ask named.
    let started = Instant::now();
    let refused = execute(Command::Status, &endpoint, false).await;
    let elapsed = started.elapsed();

    // Asked: `status` starts the daemon and answers; `stop` ends it.
    set(ENV_START_HERE, "1");
    let status = execute(Command::Status, &endpoint, false).await;
    let stopped = execute(Command::Stop, &endpoint, false).await;
    unset(ENV_START_HERE);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut quiet = false;
    while Instant::now() < deadline {
        if meltemi_client::transport::connect(&endpoint).await.is_err() {
            quiet = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let refused = refused.expect_err("an explicit endpoint that does not answer is refused");
    assert_eq!(refused.exit, ExitCode::Unreachable);
    assert!(
        refused.message.contains("check the tunnel"),
        "{}",
        refused.message
    );
    assert!(
        refused.message.contains("MELTEMI_START_HERE=1"),
        "{}",
        refused.message
    );
    assert!(elapsed < Duration::from_secs(2), "at once: {elapsed:?}");

    let status = status.expect("asked, the verb starts the daemon and proceeds");
    assert!(status.json["daemonVersion"].is_string(), "{}", status.json);
    stopped.expect("the daemon it started stops when asked");
    assert!(quiet, "and it is gone");
    let _ = std::fs::remove_dir_all(&base);
}
