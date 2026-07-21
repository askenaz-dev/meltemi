// SPDX-License-Identifier: Apache-2.0

//! Client-side daemon bootstrap (task 3.3): connect to the daemon, starting
//! it on demand if nothing is listening.
//!
//! Single-instance ownership lives on the daemon side
//! ([`crate::transport::Listener::bind`] fails with `AddrInUse` when the
//! endpoint is already owned, and `run` exits in that case). This is the
//! mirror image on the client side: try to connect and, if no daemon
//! answers, spawn `meltemid` detached and retry until it is accepting or a
//! startup budget elapses. Two clients racing to start the daemon is safe:
//! the loser's daemon exits immediately on the `AddrInUse` bind.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::transport::{self, Stream};

/// Environment override for the daemon binary the client launches.
pub const ENV_DAEMON_BIN: &str = "MELTEMI_DAEMON_BIN";

/// How long to wait for a freshly spawned daemon to start accepting.
const STARTUP_BUDGET: Duration = Duration::from_secs(5);
/// Delay between connection attempts while waiting for startup.
const RETRY_DELAY: Duration = Duration::from_millis(50);

/// Connects to the daemon at `endpoint`, launching it on demand if needed.
///
/// Fast path: if a daemon is already accepting, connects immediately.
/// Otherwise spawns the daemon detached and retries the connection until it
/// succeeds or [`STARTUP_BUDGET`] elapses.
pub async fn connect_or_start(endpoint: &str) -> std::io::Result<Stream> {
    if let Ok(stream) = transport::connect(endpoint).await {
        return Ok(stream);
    }
    spawn_daemon()?;
    let deadline = Instant::now() + STARTUP_BUDGET;
    loop {
        match transport::connect(endpoint).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                if Instant::now() >= deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("daemon did not start accepting within {STARTUP_BUDGET:?}: {e}"),
                    ));
                }
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

/// Resolves the daemon binary: `MELTEMI_DAEMON_BIN` if set, otherwise a
/// `meltemid` executable next to the current executable (the common case for
/// bundled clients and the e2e suite), otherwise the bare name `meltemid`
/// (resolved on `PATH`).
fn daemon_binary() -> PathBuf {
    if let Some(path) = std::env::var_os(ENV_DAEMON_BIN) {
        return PathBuf::from(path);
    }
    if let Ok(mut exe) = std::env::current_exe() {
        exe.set_file_name(exe_file_name("meltemid"));
        if exe.exists() {
            return exe;
        }
    }
    PathBuf::from("meltemid")
}

fn exe_file_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Spawns the daemon detached, with no console and no inherited stdio, so it
/// outlives the launching client.
fn spawn_daemon() -> std::io::Result<()> {
    let bin = daemon_binary();
    let mut command = Command::new(&bin);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    configure_detached(&mut command);
    command.spawn().map(drop).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("failed to start daemon `{}`: {e}", bin.display()),
        )
    })
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // DETACHED_PROCESS (0x8) | CREATE_NEW_PROCESS_GROUP (0x200): the daemon
    // gets no console and is not signaled when the launching client exits.
    command.creation_flags(0x0000_0008 | 0x0000_0200);
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // New process group so a terminal signal to the client's foreground group
    // (e.g. Ctrl-C) does not reach the daemon. Full daemonization (setsid +
    // double fork) is deferred; this is sufficient for Fase 0.
    command.process_group(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Listener;

    fn test_endpoint(tag: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\meltemid-bootstrap-{}-{tag}", std::process::id())
        }
        #[cfg(unix)]
        {
            std::env::temp_dir()
                .join(format!(
                    "meltemid-bootstrap-{}-{tag}.sock",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned()
        }
    }

    #[tokio::test]
    async fn fast_path_connects_without_spawning() {
        // A daemon is already listening: connect_or_start must take the fast
        // path and return a stream without ever trying to spawn a binary.
        let endpoint = test_endpoint("fastpath");
        let mut listener = Listener::bind(&endpoint).await.expect("bind");
        let accept = tokio::spawn(async move {
            // Hold the accepted stream open for the duration of the test.
            let _held = listener.accept().await.expect("accept");
            tokio::time::sleep(Duration::from_secs(2)).await;
        });

        let stream = connect_or_start(&endpoint).await;
        assert!(stream.is_ok(), "fast path must connect: {:?}", stream.err());
        accept.abort();
    }

    #[test]
    fn daemon_binary_honors_env_override() {
        // SAFETY: single-threaded test; ENV_DAEMON_BIN is read only here.
        unsafe {
            std::env::set_var(ENV_DAEMON_BIN, "/custom/path/to/meltemid");
        }
        assert_eq!(daemon_binary(), PathBuf::from("/custom/path/to/meltemid"));
        // SAFETY: as above.
        unsafe {
            std::env::remove_var(ENV_DAEMON_BIN);
        }
    }
}
