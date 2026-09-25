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
    // Scenario: Daemon inalcanzable
    // Scenario: Fallo de arranque se reporta como inalcanzable. Nothing is
    // listening, and the on-demand launcher is pointed at a binary that does
    // not exist, so the bootstrap cannot spawn a daemon and gives up. The
    // endpoint is a test's, not the user's own, so the start is asked for
    // explicitly — unasked, nothing would be started there at all
    // (acceso-remoto-afinado D1).
    let endpoint = test_endpoint("unreachable");

    // SAFETY: this test binary runs this single test; no other thread reads the
    // variables concurrently.
    unsafe {
        std::env::set_var(
            meltemi_client::bootstrap::ENV_DAEMON_BIN,
            "meltemi-no-such-daemon-binary",
        );
        std::env::set_var(meltemi_client::bootstrap::ENV_START_HERE, "1");
    }

    let error = execute(Command::Status, &endpoint, false)
        .await
        .expect_err("status must fail when the daemon is unreachable");
    assert_eq!(error.exit, ExitCode::Unreachable);
    // The start was attempted and failed: the launcher's own error, not the
    // refusal of an endpoint nobody asked to start.
    assert!(
        error.message.contains("meltemi-no-such-daemon-binary"),
        "{}",
        error.message
    );

    // SAFETY: as above.
    unsafe {
        std::env::remove_var(meltemi_client::bootstrap::ENV_DAEMON_BIN);
        std::env::remove_var(meltemi_client::bootstrap::ENV_START_HERE);
    }
}
