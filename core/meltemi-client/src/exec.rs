// SPDX-License-Identifier: Apache-2.0

//! Speaking the protocol through a process the user named
//! (via-ssh-del-cliente).
//!
//! `meltemi bridge` already puts a daemon's local endpoint on its own stdio —
//! including the Windows named pipe that no SSH forwarding reaches — so
//! `ssh pc meltemi bridge` is a complete JSON-RPC channel. What was missing was
//! a client that could speak through a process instead of through a socket.
//! This module is that, and nothing more: the crossing is the user's own `ssh`,
//! and Meltemi neither sees nor carries any key material (§2).

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// How many of the process's last error lines are kept for the diagnosis.
///
/// Bounded because a noisy command must not grow in the client's memory without
/// a ceiling, and because the useful part of what `ssh` says is the end of it
/// (design D6).
const STDERR_LINES: usize = 20;

/// The prefix that marks an endpoint as a command to run rather than a socket
/// to dial.
///
/// The form lives in the endpoint because the endpoint is already the single
/// place all three surfaces choose who to talk to, and it already travels
/// through the environment, the user's configuration and the arguments. A
/// parallel option would have made a second place to decide the same thing,
/// with the inevitable question of which one wins (design D1).
pub const EXEC_PREFIX: &str = "exec:";

/// Whether this endpoint names a command.
#[must_use]
pub fn is_exec(endpoint: &str) -> bool {
    endpoint.starts_with(EXEC_PREFIX)
}

/// Splits a command line into program and arguments.
///
/// Whitespace separates, and double quotes group — the one rule anybody writing
/// a path with a space needs. Deliberately NOT a shell: no expansion, no
/// globbing, no pipelines, and no second interpreter between what the user
/// wrote and what runs (design D2). A command that needs a pipeline names a
/// script, which is what a shell would have done with less control.
///
/// # Errors
///
/// An empty command names nothing to run, and an unclosed quote is a command
/// whose author meant something the splitter cannot guess. Both refuse rather
/// than run half of what was written.
pub fn split(command: &str) -> Result<(String, Vec<String>), String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;

    for ch in command.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                // An empty pair of quotes is a real, empty argument: `-o ""`
                // must not silently vanish.
                started = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if in_quotes {
        return Err(format!(
            "the command `{command}` has an unclosed quote; close it, or name a script \
             instead of writing the whole line here"
        ));
    }
    if started {
        parts.push(current);
    }

    let mut parts = parts.into_iter();
    let program = parts
        .next()
        .filter(|program| !program.is_empty())
        .ok_or_else(|| {
            "the endpoint names no command to run; write it as `exec:<program> <arguments>`, \
             for example `exec:ssh my-pc meltemi bridge`"
                .to_string()
        })?;
    Ok((program, parts.collect()))
}

/// The command an `exec:` endpoint names, split and ready to launch.
///
/// # Errors
///
/// Returns the splitter's refusal as an invalid-input error, so a caller that
/// only handles `io::Error` still gets the diagnosis.
pub fn command_of(endpoint: &str) -> io::Result<(String, Vec<String>)> {
    let command = endpoint.strip_prefix(EXEC_PREFIX).unwrap_or(endpoint);
    split(command).map_err(|reason| io::Error::new(io::ErrorKind::InvalidInput, reason))
}

/// How this endpoint is worth showing in a surface's chrome: the program and
/// its first argument, which for the case this exists for is `ssh <host>`.
///
/// A surface that said nothing about talking to another machine would make an
/// action on the wrong one look like an action on your own (design D8).
#[must_use]
pub fn shown(endpoint: &str) -> Option<String> {
    if !is_exec(endpoint) {
        return None;
    }
    let (program, args) = command_of(endpoint).ok()?;
    // Just the program's own name, not the path somebody may have spelled out.
    let program = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&program)
        .trim_end_matches(".exe")
        .to_string();
    Some(match args.first() {
        Some(first) => format!("{program} {first}"),
        None => program,
    })
}

/// The protocol spoken over a process's stdio.
///
/// Reads are the process's output and writes are its input, so from the rest of
/// the client this is the same byte stream a socket would have been. Two things
/// it does that a socket does not:
///
/// - **It keeps the process's last error lines**, because `ssh` explains itself
///   on `stderr` and a connection failure without that is a "could not" with no
///   cause (design D6).
/// - **It ends the process when it is dropped**, inside a scope so that a client
///   which dies outright leaves nothing behind (design D7).
#[derive(Debug)]
pub struct ExecStream {
    /// What the command was, for the diagnosis.
    endpoint: String,
    stdin: ChildStdin,
    stdout: ChildStdout,
    child: Child,
    /// Windows binds the process tree's life to this; elsewhere it is inert and
    /// killing the child is enough, because there is no shell in between.
    scope: meltemi_process::Scope,
    stderr: Arc<Mutex<VecDeque<String>>>,
    /// Set once the process is known to have gone, so the diagnosis is built
    /// once and every later poll answers the same thing.
    ended: Option<String>,
}

impl ExecStream {
    /// What to say when the process is gone.
    ///
    /// Carries the command, the exit status and the tail of what the process
    /// wrote on `stderr`, which is where `ssh` says whether it was an unknown
    /// host, a refused key or a name that does not resolve. The remedy names the
    /// one thing Meltemi cannot do for the user, and deliberately does not offer
    /// to: accept the host and load the key from a terminal, once (design D5).
    fn diagnosis(&mut self) -> String {
        let status = match self.child.try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(code) => format!("exit code {code}"),
                None => "terminated by a signal".to_string(),
            },
            _ => "no longer running".to_string(),
        };
        let said = self
            .stderr
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("; "))
            .unwrap_or_default();
        let mut message = format!("`{}` ended ({status})", self.endpoint);
        if said.is_empty() {
            message.push_str(" and said nothing on its error output");
        } else {
            message.push_str(&format!(" saying: {said}"));
        }
        message.push_str(
            " — Meltemi adds no options to your command and never handles a passphrase: \
             if this is a first connection, accept the host and load the key into your agent \
             from a terminal once, then try again",
        );
        message
    }
}

impl Drop for ExecStream {
    fn drop(&mut self) {
        // The child first, then the scope: the same order the adapters settled
        // on, so the child is killed and reaped while it still exists and the
        // scope collects whatever was underneath it.
        let _ = self.child.start_kill();
        let _ = self.scope.end();
    }
}

impl AsyncRead for ExecStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        if let Some(said) = &me.ended {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, said.clone())));
        }
        let filled = buf.filled().len();
        match Pin::new(&mut me.stdout).poll_read(cx, buf) {
            Poll::Ready(Ok(())) if buf.filled().len() == filled => {
                // End of the process's output. NOT reported as a clean end of
                // stream: a socket closing is a daemon that went, and a command
                // closing is a command that failed with a reason on its error
                // output. Saying "connection closed" here would throw away the
                // only explanation there is.
                let said = me.diagnosis();
                me.ended = Some(said.clone());
                Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, said)))
            }
            other => other,
        }
    }
}

impl AsyncWrite for ExecStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();
        Pin::new(&mut me.stdin).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        Pin::new(&mut me.stdin).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        Pin::new(&mut me.stdin).poll_shutdown(cx)
    }
}

/// Launches the command an `exec:` endpoint names and speaks the protocol over
/// its stdio.
///
/// # Errors
///
/// Returns the splitter's refusal for a command that names nothing, and the
/// platform's error when the program cannot be launched at all — a program that
/// is not there is a typo or a missing tool, and saying so beats a timeout.
pub async fn connect(endpoint: &str) -> io::Result<ExecStream> {
    let (program, args) = command_of(endpoint)?;

    // The scope is opened BEFORE the launch, so there is no window in which a
    // process exists outside it (design D7).
    let scope = meltemi_process::Scope::new()?;
    let mut child = Command::new(&program)
        .args(&args)
        // The endpoint does NOT travel into the command's environment. A
        // command that itself runs `meltemi` — a local helper, a wrapper script
        // — would otherwise inherit this very endpoint and dial back into
        // itself. Removed rather than overwritten, so the child falls back to
        // whatever its own machine's default is, and a script that wants a
        // particular one sets it itself.
        .env_remove(crate::paths::ENV_ENDPOINT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "could not launch `{program}` for endpoint `{endpoint}`: {e} — check the \
                     program's name and that it is on your PATH"
                ),
            )
        })?;
    if let Some(pid) = child.id() {
        scope.adopt_pid(pid)?;
    }

    let stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    // The collector. Bounded, and discarded with the stream: what it holds is
    // only ever read to explain a failure.
    let kept: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let writer = Arc::clone(&kept);
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(mut held) = writer.lock() {
                if held.len() == STDERR_LINES {
                    held.pop_front();
                }
                held.push_back(line);
            }
        }
    });

    Ok(ExecStream {
        endpoint: endpoint.to_string(),
        stdin,
        stdout,
        child,
        scope,
        stderr: kept,
        ended: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A command that stays up long enough to be observed, on either platform.
    fn a_long_lived_command() -> String {
        if cfg!(windows) {
            "exec:ping -n 60 127.0.0.1".to_string()
        } else {
            "exec:sleep 60".to_string()
        }
    }

    /// Whether a process id belongs to something still running.
    fn alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\" | \
                         ForEach-Object {{ $_.ProcessId }}"
                    ),
                ])
                .output()
                .expect("powershell ships with Windows");
            String::from_utf8_lossy(&out.stdout).trim() == pid.to_string()
        }
        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .is_ok_and(|status| status.success())
        }
    }

    // Scenario: Al cerrar el cliente no queda el proceso
    #[tokio::test]
    async fn dropping_the_stream_ends_the_process_it_launched() {
        let stream = connect(&a_long_lived_command())
            .await
            .expect("the command launches");
        let pid = stream.child.id().expect("a running child has an id");
        assert!(alive(pid), "the process is up before the stream is dropped");

        drop(stream);

        // Killing is not instant; give the platform a moment to reap.
        let mut gone = false;
        for _ in 0..100 {
            if !alive(pid) {
                gone = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(
            gone,
            "process {pid} outlived the stream: a client that closes would leave an ssh \
             holding a remote session"
        );
    }

    // Scenario: Un comando que no llega a hablar el protocolo se rehúsa con su causa
    #[tokio::test]
    async fn a_command_that_dies_says_why_and_what_to_do() {
        // A command that exits at once with something on its error output, which
        // is the shape of every first connection to an unknown host.
        let endpoint = if cfg!(windows) {
            r#"exec:cmd /c "echo Host key verification failed. 1>&2 && exit 255""#
        } else {
            r#"exec:sh -c "echo 'Host key verification failed.' >&2; exit 255""#
        };
        let mut stream = connect(endpoint).await.expect("it launches, then dies");

        // Reading is where the death surfaces, and it must NOT look like a clean
        // end of stream: that would throw away the only explanation there is.
        let mut buf = [0u8; 64];
        let error = loop {
            match tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await {
                Ok(0) => panic!("a dead command must not read as a clean end of stream"),
                Ok(_) => continue,
                Err(error) => break error,
            }
        };
        let said = error.to_string();
        assert!(said.contains("ended"), "{said}");
        assert!(
            said.contains("Host key verification failed"),
            "the process's own words travel, because that is where ssh explains itself: {said}"
        );
        assert!(
            said.contains("accept the host"),
            "and the remedy with them: {said}"
        );
        assert!(
            said.contains("never handles a passphrase"),
            "and what Meltemi does not do: {said}"
        );

        // Asked again, it answers the same thing rather than a different one.
        let again = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
            .await
            .expect_err("still gone");
        assert_eq!(again.to_string(), said);
    }

    #[tokio::test]
    async fn a_program_that_is_not_there_says_so_instead_of_timing_out() {
        let refusal = connect("exec:meltemi-no-such-program-anywhere")
            .await
            .expect_err("nothing to launch");
        let said = refusal.to_string();
        assert!(said.contains("could not launch"), "{said}");
        assert!(said.contains("on your PATH"), "the remedy: {said}");
    }

    #[tokio::test]
    async fn the_endpoint_does_not_travel_into_the_command() {
        // A command that runs `meltemi` would otherwise inherit this very
        // endpoint and dial back into itself (design D9). Observed through the
        // command's own view of its environment.
        // SAFETY: the value is restored below, and this test does not run
        // concurrently with another that reads it.
        unsafe { std::env::set_var(crate::paths::ENV_ENDPOINT, "exec:should-not-be-inherited") };
        let endpoint = if cfg!(windows) {
            r#"exec:cmd /c "echo seen=[%MELTEMI_ENDPOINT%]""#
        } else {
            r#"exec:sh -c "echo seen=[$MELTEMI_ENDPOINT]""#
        };
        let mut stream = connect(endpoint).await.expect("it launches");
        let mut said = String::new();
        // Read until the process closes its output; the error that follows is
        // the death, which this test is not about.
        let mut buf = [0u8; 256];
        while let Ok(n) = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await {
            if n == 0 {
                break;
            }
            said.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        unsafe { std::env::remove_var(crate::paths::ENV_ENDPOINT) };

        // Asserted by absence rather than by an empty pair of brackets: `cmd`
        // leaves `%VAR%` literal when the variable is unset, so "seen=[]" is a
        // Unix-shaped expectation. What matters on every platform is that the
        // value never reached the child.
        assert!(
            !said.contains("should-not-be-inherited"),
            "the child saw an endpoint it should not have: {said:?}"
        );
        assert!(
            said.contains("seen="),
            "the child ran and reported what it saw: {said:?}"
        );
    }

    // Scenario: El comando se lanza sin intérprete intermedio
    #[test]
    fn a_command_splits_into_a_program_and_its_arguments() {
        let (program, args) = split("ssh my-pc meltemi bridge").expect("a plain command");
        assert_eq!(program, "ssh");
        assert_eq!(args, ["my-pc", "meltemi", "bridge"]);
    }

    #[test]
    fn double_quotes_group_an_argument_that_has_spaces() {
        let (program, args) = split(r#""C:/Program Files/OpenSSH/ssh.exe" my-pc "meltemi bridge""#)
            .expect("quotes group");
        assert_eq!(program, "C:/Program Files/OpenSSH/ssh.exe");
        assert_eq!(args, ["my-pc", "meltemi bridge"]);
    }

    #[test]
    fn an_empty_pair_of_quotes_is_a_real_empty_argument() {
        // `-o ""` must not vanish: an argument the user wrote is an argument
        // the program receives, even when it is empty.
        let (_, args) = split(r#"ssh -o "" host"#).expect("an empty argument");
        assert_eq!(args, ["-o", "", "host"]);
    }

    #[test]
    fn an_unclosed_quote_refuses_instead_of_guessing() {
        let refusal = split(r#"ssh "my pc meltemi bridge"#).expect_err("unclosed");
        assert!(refusal.contains("unclosed quote"), "{refusal}");
        assert!(refusal.contains("name a script"), "the remedy: {refusal}");
    }

    #[test]
    fn an_empty_command_names_nothing_and_says_so() {
        for empty in ["", "   ", "\t"] {
            let refusal = split(empty).expect_err("nothing to run");
            assert!(refusal.contains("names no command"), "{refusal}");
            assert!(
                refusal.contains("exec:ssh my-pc meltemi bridge"),
                "the shape is shown: {refusal}"
            );
        }
    }

    #[test]
    fn the_prefix_is_what_tells_a_command_from_a_socket() {
        assert!(is_exec("exec:ssh pc meltemi bridge"));
        assert!(!is_exec(r"\\.\pipe\meltemid-guill"));
        assert!(!is_exec("/tmp/meltemid.sock"));

        let (program, args) = command_of("exec:ssh pc meltemi bridge").expect("split");
        assert_eq!(program, "ssh");
        assert_eq!(args, ["pc", "meltemi", "bridge"]);
    }

    #[test]
    fn the_chrome_shows_the_program_and_the_host_and_nothing_of_a_socket() {
        assert_eq!(
            shown("exec:ssh my-pc meltemi bridge").as_deref(),
            Some("ssh my-pc")
        );
        // A spelled-out path shows its name, not the whole path.
        assert_eq!(
            shown(r#"exec:"C:/Program Files/OpenSSH/ssh.exe" my-pc meltemi bridge"#).as_deref(),
            Some("ssh my-pc")
        );
        assert_eq!(shown("exec:local-helper").as_deref(), Some("local-helper"));
        // A socket has no via to show.
        assert_eq!(shown(r"\\.\pipe\meltemid-guill"), None);
    }
}
