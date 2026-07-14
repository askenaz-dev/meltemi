// SPDX-License-Identifier: Apache-2.0

//! End-to-end unreachable-daemon mapping (task 5.3). Isolated in its own test
//! binary because it sets a process-global environment variable to force the
//! on-demand launch to fail; being the only test here keeps that mutation free
//! of concurrent readers.

mod common;

use meltemi::cli::Command;
use meltemi::exit::ExitCode;
use meltemi::run::execute;

use common::test_endpoint;

#[tokio::test]
async fn unreachable_daemon_is_reported_as_unreachable() {
    // Scenario: Fallo de arranque se reporta como inalcanzable. Nothing is
    // listening, and the on-demand launcher is pointed at a binary that does
    // not exist, so the bootstrap cannot spawn a daemon and gives up.
    let endpoint = test_endpoint("unreachable");

    // SAFETY: this test binary runs this single test; no other thread reads the
    // variable concurrently.
    unsafe {
        std::env::set_var(
            meltemid::bootstrap::ENV_DAEMON_BIN,
            "meltemi-no-such-daemon-binary",
        );
    }

    let error = execute(Command::Status, &endpoint)
        .await
        .expect_err("status must fail when the daemon is unreachable");
    assert_eq!(error.exit, ExitCode::Unreachable);

    // SAFETY: as above.
    unsafe {
        std::env::remove_var(meltemid::bootstrap::ENV_DAEMON_BIN);
    }
}
