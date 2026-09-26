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

// ---- the intermediary the platform puts in the way --------------------------
//
// Everything above is about a provider that behaves: it ends when its input
// ends. What follows is about the shape the platform imposes on a provider that
// does not. On Windows the official CLI of more than one provider installs as a
// `.cmd` shim, so what this adapter launches is `cmd.exe` running a script, and
// what does the work is a process underneath it. Ending the shim alone leaves
// that process running — measured before this was written, and the reason the
// supervisor now launches inside a scope (apagado-entero-y-modos design D1).

#[cfg(windows)]
mod through_an_intermediary {
    use super::*;
    use std::process::Command;
    use std::time::Instant;

    /// A shim that ignores its input entirely and outlives any grace, with a
    /// grandchild of its own to leave behind.
    ///
    /// `ping` waits without doing anything and ships with every Windows, so
    /// nothing here depends on a tool that might not be installed — and nothing
    /// here uses a kill utility, which is the point.
    fn a_shim_that_ignores_its_input(dir: &std::path::Path) -> ProviderCommand {
        std::fs::create_dir_all(dir).expect("a temp directory");
        let shim = dir.join("provider.cmd");
        std::fs::write(&shim, "@echo off\r\nping -n 120 127.0.0.1 >nul\r\n")
            .expect("writing the shim");
        ProviderCommand {
            program: shim.display().to_string(),
            args: Vec::new(),
            cwd: dir.to_path_buf(),
        }
    }

    /// The identifiers of every process whose parent is `parent`.
    fn children_of(parent: u32) -> Vec<u32> {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-CimInstance Win32_Process -Filter \"ParentProcessId={parent}\" | \
                     ForEach-Object {{ $_.ProcessId }}"
                ),
            ])
            .output()
            .expect("powershell ships with the platform this test only runs on");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }

    /// Whether a process id belongs to something still running.
    ///
    /// Read from the process list rather than by opening a handle: a process
    /// that has terminated but not been reaped can still be opened, and would
    /// answer "alive" to the wrong question.
    fn alive(pid: u32) -> bool {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\" | \
                     ForEach-Object {{ $_.ProcessId }}"
                ),
            ])
            .output()
            .expect("powershell ships with the platform this test only runs on");
        String::from_utf8_lossy(&out.stdout).trim() == pid.to_string()
    }

    /// Waits for `condition`, up to `limit`. Returns whether it came true.
    fn within(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        condition()
    }

    /// The process running a given command line, waited for.
    ///
    /// Deliberately not "a child of this test process": these tests run in
    /// parallel and each of their helpers spawns processes of its own, so
    /// picking a child by shape would pick whichever one answered first. The
    /// shim's path is unique per test, and that is what identifies it.
    fn process_running(command_line: &str) -> Option<u32> {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -and                      $_.CommandLine.Contains('{command_line}') }} | ForEach-Object {{ $_.ProcessId }}"
                ),
            ])
            .output()
            .expect("powershell ships with the platform this test only runs on");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.trim().parse().ok())
    }

    /// Launches the shim and waits until it has the grandchild this is about.
    /// Returns the provider, the shim's identifier and the grandchildren.
    fn launched(
        dir: &std::path::Path,
    ) -> (meltemi_adapters::supervisor::SpawnedProvider, u32, Vec<u32>) {
        let command = a_shim_that_ignores_its_input(dir);
        let shim_path = command.program.clone();
        let provider = spawn(&command, "the official test CLI").expect("the shim launches");
        // The supervisor does not expose what it launched — it has no reason to
        // — so the shim is found by the one thing that is unique to this test:
        // the path of its own script.
        let mut shim = None;
        within(Duration::from_secs(10), || {
            shim = process_running(&shim_path);
            shim.is_some()
        });
        let shim = shim.expect("the shim is running under its own path");
        let grandchildren = {
            let mut found = Vec::new();
            within(Duration::from_secs(10), || {
                found = children_of(shim);
                !found.is_empty()
            });
            found
        };
        assert!(
            !grandchildren.is_empty(),
            "the shim never created the process this test is about; without it \
             the test proves nothing"
        );
        (provider, shim, grandchildren)
    }

    fn assert_all_gone(shim: u32, grandchildren: &[u32]) {
        for pid in grandchildren {
            assert!(
                within(Duration::from_secs(10), || !alive(*pid)),
                "the process under the shim ({pid}) outlived the provider: \
                 ending the shim is not ending what the shim launched"
            );
        }
        assert!(
            within(Duration::from_secs(10), || !alive(shim)),
            "the shim itself outlived the provider"
        );
    }

    // Scenario: El intermediario del proveedor no deja huérfanos
    #[tokio::test]
    async fn a_provider_that_ignores_the_end_of_its_input_takes_its_grandchild_with_it() {
        let dir = std::env::temp_dir().join(format!(
            "meltemi-adapters-intermediary-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (mut provider, shim, grandchildren) = launched(&dir);

        // A grace short enough that the test does not sit through a human's,
        // and long enough that a shim which really was ending would have.
        let outcome = provider
            .shutdown(ShutdownPolicy {
                grace: Duration::from_millis(500),
            })
            .await
            .expect("ending a provider that ignores its input");
        assert_eq!(
            outcome,
            ShutdownOutcome::Killed,
            "this shim ignores its input by construction; an `Exited` here means \
             the fixture stopped being the thing the test needs"
        );

        assert_all_gone(shim, &grandchildren);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: El adaptador cae y el proveedor no le sobrevive
    #[tokio::test]
    async fn dropping_the_provider_takes_the_whole_tree_with_it() {
        // The half a destructor cannot keep and the kernel can. Dropping is
        // what a destructor does; the guarantee this asserts is the platform's
        // behaviour when the last handle to the scope closes, which is also
        // what happens to a process that dies without running anything.
        let dir =
            std::env::temp_dir().join(format!("meltemi-adapters-dropped-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (provider, shim, grandchildren) = launched(&dir);

        drop(provider);

        assert_all_gone(shim, &grandchildren);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
