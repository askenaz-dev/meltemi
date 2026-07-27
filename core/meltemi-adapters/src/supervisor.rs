// SPDX-License-Identifier: Apache-2.0

//! Supervision of the provider subprocess (adaptadores-propios-acp task 1.2).
//!
//! The adapter's only way to the provider is this subprocess: the official CLI,
//! launched with the authentication it manages itself (constitution §2). This
//! module owns its life — launching it, talking to it over
//! newline-delimited JSON, and ending it: politely first, and then with
//! certainty, because a provider that hangs must not outlive the session that
//! asked for it.
//!
//! Everything here is generic over the process and over the streams, so the
//! bridge is tested in memory (a fake process, a duplex pipe) and behaves the
//! same on the three platforms. The concrete instantiation over a real child
//! process is [`SpawnedProvider`].

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::diagnostic::Refusal;
use crate::ndjson::{Frame, FrameReader, FrameWriter};

/// How the official CLI is launched: program, arguments and working directory.
///
/// The environment is **inherited, never composed**: the adapter does not read
/// or invent authentication material, and the daemon has already applied the
/// launch profile's overlay to the adapter's own process.
#[derive(Debug, Clone)]
pub struct ProviderCommand {
    /// The official CLI, as it is looked up on the PATH (or an absolute path).
    pub program: String,
    /// The arguments that select the provider's documented programmatic mode.
    pub args: Vec<String>,
    /// The session's working directory: the project root or one of its
    /// worktrees.
    pub cwd: PathBuf,
}

/// How long a clean shutdown waits before the process is killed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownPolicy {
    /// Time the provider gets to exit on its own after its input closes.
    pub grace: Duration,
}

impl Default for ShutdownPolicy {
    fn default() -> Self {
        // Long enough for a CLI to flush a final message and exit, short
        // enough that a hung provider does not hold a session hostage.
        Self {
            grace: Duration::from_secs(5),
        }
    }
}

/// What ending the subprocess actually cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// It exited on its own once its input closed.
    Exited,
    /// It outlived the grace and was killed.
    Killed,
}

/// What the supervisor needs from the process in order to end it — and nothing
/// more, so a test can stand in for one.
pub trait ProcessControl: Send {
    /// Waits up to `grace` for the process to exit on its own. `Ok(true)` when
    /// it exited, `Ok(false)` when the grace ran out with it still alive.
    fn wait_within(
        &mut self,
        grace: Duration,
    ) -> impl Future<Output = std::io::Result<bool>> + Send;

    /// Ends the process without asking.
    fn kill(&mut self) -> impl Future<Output = std::io::Result<()>> + Send;
}

/// A real child process.
pub struct ChildProcess(Child);

impl ProcessControl for ChildProcess {
    async fn wait_within(&mut self, grace: Duration) -> std::io::Result<bool> {
        match tokio::time::timeout(grace, self.0.wait()).await {
            Ok(status) => {
                status?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    async fn kill(&mut self) -> std::io::Result<()> {
        // tokio's `kill` also reaps the child, so nothing is left as a zombie.
        self.0.kill().await
    }
}

/// A provider process under supervision: its control handle and the two framed
/// halves of its stdio.
pub struct ProviderProcess<C, W, R> {
    control: C,
    stdin: FrameWriter<W>,
    stdout: FrameReader<R>,
}

/// The supervised provider as it exists at runtime: a real child process.
pub type SpawnedProvider = ProviderProcess<ChildProcess, ChildStdin, ChildStdout>;

impl<C: ProcessControl, W: AsyncWrite + Unpin, R: AsyncRead + Unpin> ProviderProcess<C, W, R> {
    /// Supervises an already-running process over the given framed streams.
    pub fn new(control: C, stdin: W, stdout: R) -> Self {
        Self {
            control,
            stdin: FrameWriter::new(stdin),
            stdout: FrameReader::new(stdout),
        }
    }

    /// Sends one message to the provider.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the write fails.
    pub async fn send(&mut self, message: &Value) -> std::io::Result<()> {
        self.stdin.write_frame(message).await
    }

    /// Receives the next message from the provider, or `None` when it closed
    /// its output.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the read fails.
    pub async fn receive(&mut self) -> std::io::Result<Option<Frame>> {
        self.stdout.next_frame().await
    }

    /// Ends the provider: closes its input, waits out the grace, and kills it
    /// if it is still there. A hung provider is a bug in the provider; leaving
    /// it running would make it a bug in Meltemi.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when waiting or killing fails.
    pub async fn shutdown(&mut self, policy: ShutdownPolicy) -> std::io::Result<ShutdownOutcome> {
        // Closing the input is the documented way out for both provider wires.
        let _ = self.stdin.close().await;
        end(&mut self.control, policy).await
    }

    /// Hands over the three halves separately.
    ///
    /// A dialect that runs a bidirectional conversation has to read and write
    /// at the same time, which a single owner cannot do; splitting is how the
    /// reader becomes a task of its own while the writer stays where turns are
    /// sent from. The control handle comes along because ending the process is
    /// still the session's job, and nobody else's.
    pub fn into_parts(self) -> (C, FrameWriter<W>, FrameReader<R>) {
        (self.control, self.stdin, self.stdout)
    }
}

/// Waits out the grace and kills what is still there.
///
/// A hung provider is a bug in the provider; leaving it running would make it a
/// bug in Meltemi.
///
/// # Errors
///
/// Returns the underlying I/O error when waiting or killing fails.
pub async fn end(
    control: &mut impl ProcessControl,
    policy: ShutdownPolicy,
) -> std::io::Result<ShutdownOutcome> {
    if control.wait_within(policy.grace).await? {
        return Ok(ShutdownOutcome::Exited);
    }
    control.kill().await?;
    Ok(ShutdownOutcome::Killed)
}

/// Which binary to launch: the override when it says something, the registry's
/// name otherwise.
///
/// Every dialect has the same escape hatch — an environment variable naming a
/// specific binary — because a CLI can live where the PATH does not reach, and
/// because it is how the end-to-end tests put a scripted wire where the
/// provider would be. It changes *which binary* is launched, never *what* is
/// spoken to it. An override set to whitespace is somebody's empty variable,
/// not an instruction.
#[must_use]
pub fn resolve_program(override_value: Option<String>, default_bin: &str) -> String {
    override_value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_bin.to_string())
}

/// Launches the official CLI as the session's provider process.
///
/// `layer` names the layer in a refusal, worded as the fleet catalog words it.
///
/// # Errors
///
/// Refuses — never falls back — when the official CLI cannot be launched. A
/// bridge that reached for another way in would be piloting something the user
/// did not authorize, which is the one thing this architecture exists to
/// prevent.
pub fn spawn(command: &ProviderCommand, layer: &str) -> Result<SpawnedProvider, Refusal> {
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .current_dir(&command.cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        // The provider's diagnostics travel to the adapter's own stderr, where
        // the daemon already collects them; they are never parsed as protocol.
        .stderr(std::process::Stdio::inherit())
        // If this adapter's future is dropped, the provider goes with it: no
        // orphan holding a worktree open.
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| launch_refusal(&command.program, layer, &error))?;

    let stdin = child.stdin.take().ok_or_else(|| {
        Refusal::new(
            "provider_cli_not_launchable",
            layer,
            format!(
                "`{}` was launched without a usable input stream",
                command.program
            ),
            "Reinstall or repair the provider CLI.",
        )
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        Refusal::new(
            "provider_cli_not_launchable",
            layer,
            format!(
                "`{}` was launched without a usable output stream",
                command.program
            ),
            "Reinstall or repair the provider CLI.",
        )
    })?;

    Ok(ProviderProcess::new(ChildProcess(child), stdin, stdout))
}

/// The refusal for a provider CLI that cannot be launched.
///
/// Public because launching the CLI is not always the session's own spawn: a
/// dialect may have to ask the binary something before it pilots it, and a
/// binary that is not there must refuse with the same words either way.
#[must_use]
pub fn launch_refusal(program: &str, layer: &str, error: &std::io::Error) -> Refusal {
    Refusal::new(
        "provider_cli_not_launchable",
        layer,
        format!("`{program}` could not be launched ({error})"),
        format!(
            "Install the provider CLI and sign in, then check that `{program}` is on your PATH \
             (`meltemi fleet` shows the exact command for this entry)."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A process that behaves as the test says, and remembers what was done to
    /// it.
    struct FakeProcess {
        /// Whether it exits on its own once its input closes.
        exits_on_its_own: bool,
        killed: bool,
    }

    impl ProcessControl for FakeProcess {
        async fn wait_within(&mut self, _grace: Duration) -> std::io::Result<bool> {
            Ok(self.exits_on_its_own)
        }

        async fn kill(&mut self) -> std::io::Result<()> {
            self.killed = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_well_behaved_provider_is_ended_by_closing_its_input() {
        // Clean shutdown: the provider sees end of input, exits, and nothing is
        // killed. The bytes the adapter wrote must have reached it first.
        let (provider_stdin, mut provider_view) = tokio::io::duplex(1024);
        let (_unused, provider_stdout) = tokio::io::duplex(1024);
        let mut provider = ProviderProcess::new(
            FakeProcess {
                exits_on_its_own: true,
                killed: false,
            },
            provider_stdin,
            provider_stdout,
        );

        provider.send(&json!({"type": "user"})).await.unwrap();
        let outcome = provider.shutdown(ShutdownPolicy::default()).await.unwrap();
        assert_eq!(outcome, ShutdownOutcome::Exited);
        assert!(!provider.control.killed, "a polite exit is never killed");

        let mut seen = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut provider_view, &mut seen)
            .await
            .unwrap();
        assert_eq!(
            seen, "{\"type\":\"user\"}\n",
            "the message was flushed as one line, and the input then closed"
        );
    }

    #[tokio::test]
    async fn a_hung_provider_is_killed_when_the_grace_runs_out() {
        // The provider ignores the end of its input. It does not get to hold
        // the session — or the worktree — hostage.
        let (provider_stdin, _view) = tokio::io::duplex(64);
        let (_unused, provider_stdout) = tokio::io::duplex(64);
        let mut provider = ProviderProcess::new(
            FakeProcess {
                exits_on_its_own: false,
                killed: false,
            },
            provider_stdin,
            provider_stdout,
        );

        let outcome = provider
            .shutdown(ShutdownPolicy {
                grace: Duration::from_millis(10),
            })
            .await
            .unwrap();
        assert_eq!(outcome, ShutdownOutcome::Killed);
        assert!(provider.control.killed, "the hung provider was killed");
    }

    #[tokio::test]
    async fn the_official_cli_missing_refuses_with_its_layer_and_remedy() {
        // Scenario: CLI oficial ausente al lanzar
        let launched = spawn(
            &ProviderCommand {
                program: "meltemi-no-such-provider-cli".into(),
                args: vec!["app-server".into()],
                cwd: std::env::temp_dir(),
            },
            "the official test CLI",
        );
        let Err(refusal) = launched else {
            panic!("a CLI that does not exist cannot be launched");
        };

        assert_eq!(refusal.kind, "provider_cli_not_launchable");
        assert_eq!(refusal.layer, "the official test CLI");
        assert!(
            refusal.detail.contains("meltemi-no-such-provider-cli"),
            "the refusal names the binary it tried: {}",
            refusal.detail
        );
        assert!(
            refusal.remedy.contains("PATH"),
            "the refusal carries a remedy the human can act on: {}",
            refusal.remedy
        );
    }
}
