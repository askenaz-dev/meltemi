// SPDX-License-Identifier: Apache-2.0

//! Ending a provider process, against a real one (adaptadores-propios-acp
//! review correction).
//!
//! Every other test of the supervisor drives it over `tokio::io::duplex`, and
//! for the length of a conversation that is exactly right: the framing, the
//! ordering and the dialects are the same bytes either way, and an in-memory
//! pipe runs identically on the three platforms.
//!
//! It is wrong for exactly one thing, and it is this one. A duplex stream
//! honours `AsyncWrite::shutdown` by signalling end of stream; a child
//! process's input honours it by doing nothing at all — tokio's implementation
//! is `Poll::Ready(Ok(()))` on both platforms, and only dropping the handle
//! closes the pipe. So a stand-in politer than the thing it stands for kept an
//! entire class of defect invisible: every clean close waited out its grace and
//! killed the CLI, and a cancellation on the headless dialect — where closing
//! the input **is** the cancellation — sent nothing at all.
//!
//! Hence this file, and hence the fixture: a real child, on a real pipe, ended
//! the way a session ends it. The child is this crate's own adapter binary in
//! its permission-shim role, which is a genuine stdio process that ends when
//! its input does — no extra fixture binary, and nothing to build but the crate
//! under test.

use std::time::Duration;

use meltemi_adapters::supervisor::{ProviderCommand, ShutdownOutcome, ShutdownPolicy, spawn};

/// A real child process that reads its input until it ends, and then ends too.
fn a_process_that_ends_with_its_input() -> ProviderCommand {
    ProviderCommand {
        program: env!("CARGO_BIN_EXE_meltemi-claude-acp").to_string(),
        args: vec![
            meltemi_adapters::claude::shim::SHIM_ARG.to_string(),
            // No question ever travels here: the child is spawned to be ended,
            // and the channel it would ask over is never used.
            std::env::temp_dir()
                .join("meltemi-adapters-lifecycle-unused")
                .display()
                .to_string(),
        ],
        cwd: std::env::temp_dir(),
    }
}

/// Long enough that a child which really is ending has ended; short enough that
/// a test which is about to prove a kill does not sit through a human's grace.
const GRACE: Duration = Duration::from_secs(10);

#[tokio::test]
async fn closing_a_real_childs_input_ends_it_instead_of_waiting_for_the_kill() {
    // Scenario: Fin de entrada percibido por el proceso proveedor
    //
    // The property the duplex tests assert and cannot prove: a close is an end
    // of stream *to the process*, so a provider that ends with its input ends
    // on its own and is never killed.
    let mut provider = spawn(
        &a_process_that_ends_with_its_input(),
        "the official test CLI",
    )
    .expect("this crate's own binary is next to this test");

    // A line first, so the close is proved to land after real traffic rather
    // than on a pipe nothing ever used. This one is a notification the child
    // reads and does not answer, which is all this test needs of it.
    provider
        .send(&serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await
        .expect("the child's input takes a line");

    let outcome = provider
        .shutdown(ShutdownPolicy { grace: GRACE })
        .await
        .expect("ending a child that is ending cannot fail");
    assert_eq!(
        outcome,
        ShutdownOutcome::Exited,
        "a well-behaved provider exits on end of input; if this says `Killed`, \
         the close did not reach the process and every clean shutdown is a kill"
    );

    // And nothing may travel afterwards: the conversation is over, and a write
    // that looked as if it had gone somewhere would let a dialect believe it
    // had sent a turn.
    let refused = provider
        .send(&serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await
        .expect_err("a closed input carries nothing");
    assert_eq!(refused.kind(), std::io::ErrorKind::BrokenPipe);
}
