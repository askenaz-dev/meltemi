// SPDX-License-Identifier: Apache-2.0

//! Client-side daemon bootstrap (task 3.3): connect to the daemon, starting
//! it on demand if nothing is listening — at this user's own endpoint, or where
//! a start was asked for, and nowhere else (acceso-remoto-afinado D1).
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

/// Environment consent to start a daemon at an endpoint that is not this
/// user's own. Absent, empty or `0` is no; anything else is yes — the shape
/// `MELTEMI_BELL` and `MELTEMI_ASCII` already have (acceso-remoto-afinado D2).
pub const ENV_START_HERE: &str = "MELTEMI_START_HERE";

/// How long to wait for a freshly spawned daemon to start accepting.
const STARTUP_BUDGET: Duration = Duration::from_secs(5);
/// Delay between connection attempts while waiting for startup.
const RETRY_DELAY: Duration = Duration::from_millis(50);

/// Connects to the daemon at `endpoint`, launching it on demand if needed.
///
/// Fast path: if a daemon is already accepting, connects immediately.
/// Otherwise, at this user's own endpoint — or wherever [`ENV_START_HERE`]
/// asks — spawns the daemon detached and retries the connection until it
/// succeeds or [`STARTUP_BUDGET`] elapses. Anywhere else nothing is started:
/// an endpoint that does not answer may be a tunnel that dropped, another
/// tool's daemon or a typo, and a fresh local daemon under that name would
/// answer in the place of the one the user meant. The refusal is immediate and
/// says the ways on (acceso-remoto-afinado D1, D3).
pub async fn connect_or_start(endpoint: &str) -> std::io::Result<Stream> {
    if let Ok(stream) = transport::connect(endpoint).await {
        return Ok(stream);
    }
    let asked = std::env::var(ENV_START_HERE).ok();
    if !may_start(
        endpoint,
        &crate::paths::default_endpoint(),
        asked.as_deref(),
    ) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotConnected,
            refusal(endpoint),
        ));
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

/// Whether a client may start a daemon at `endpoint`: at this user's own
/// endpoint, always; anywhere else, only when [`ENV_START_HERE`] asks. The
/// comparison is by text on purpose: an endpoint written another way is not
/// recognized as the own one, and the error it gets names the remedy — the
/// safe side of being wrong.
#[must_use]
pub fn may_start(endpoint: &str, own: &str, start_here: Option<&str>) -> bool {
    // An endpoint that names a command is never a place to start a daemon, and
    // not even the opt-in changes that. Starting one because a remote `ssh` did
    // not answer would serve one machine in the name of another, and the local
    // daemon would then answer in place of the one the user meant.
    //
    // The comparison below would already never match an `exec:` endpoint, so
    // this is written out rather than left to that accident: a guarantee that
    // holds by coincidence is a guarantee the next refactor takes away without
    // noticing (via-ssh-del-cliente design D4).
    if crate::exec::is_exec(endpoint) {
        return false;
    }
    endpoint == own || !matches!(start_here, None | Some("") | Some("0"))
}

/// Why nothing was started at `endpoint`, and the two ways on.
#[must_use]
pub fn refusal(endpoint: &str) -> String {
    // A command endpoint gets its own refusal, because the two remedies the
    // other one offers cannot work here: there is no tunnel to check and the
    // start-here opt-in is closed by `may_start` above. Offering a way that is
    // shut would be worse than offering none.
    if crate::exec::is_exec(endpoint) {
        return format!(
            "`{endpoint}` names a command and it did not put a daemon on its output; nothing \
             is ever started locally for a command endpoint, because that would answer in \
             place of the machine you meant. Check the command on its own: run it in a \
             terminal and see whether it speaks JSON-RPC"
        );
    }
    format!(
        "`{endpoint}` is not accepting connections, and a daemon is started only at this \
         user's own endpoint: if it is a tunnel, check the tunnel; if the daemon should run \
         here, start it without {} (for example `meltemi status`), or set {ENV_START_HERE}=1 \
         to start one at this endpoint",
        crate::paths::ENV_ENDPOINT
    )
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
    fn a_daemon_starts_at_the_own_endpoint_or_where_asked() {
        let own = r"\\.\pipe\meltemid-S-1-5-21-own";
        assert!(may_start(own, own, None), "the own endpoint, unasked");
        for unasked in [None, Some(""), Some("0")] {
            assert!(
                !may_start("/home/me/.meltemi/meltemid.sock", own, unasked),
                "an explicit endpoint, {unasked:?}: nothing is started there"
            );
        }
        for asked in [Some("1"), Some("yes")] {
            assert!(
                may_start("/home/me/.meltemi/meltemid.sock", own, asked),
                "asked with {asked:?}"
            );
        }
    }

    #[test]
    fn the_refusal_names_the_tunnel_and_how_to_ask() {
        let said = refusal("/home/me/.meltemi/meltemid.sock");
        assert!(said.contains("/home/me/.meltemi/meltemid.sock"), "{said}");
        assert!(said.contains("check the tunnel"), "{said}");
        assert!(said.contains("MELTEMI_ENDPOINT"), "{said}");
        assert!(said.contains("MELTEMI_START_HERE=1"), "{said}");
    }

    #[tokio::test]
    async fn an_explicit_endpoint_that_does_not_answer_is_refused_at_once() {
        // Nothing listens here, and it is not this user's own endpoint: the
        // client says so at once instead of starting a daemon under its name
        // and waiting out the startup budget.
        let endpoint = test_endpoint("explicit-refused");
        let started = Instant::now();
        let Err(refused) = connect_or_start(&endpoint).await else {
            panic!("nothing answers here, and nothing may be started");
        };
        assert!(started.elapsed() < STARTUP_BUDGET, "at once");
        assert_eq!(refused.kind(), std::io::ErrorKind::NotConnected);
        assert!(
            refused.to_string().contains("check the tunnel"),
            "{refused}"
        );
        assert!(
            transport::connect(&endpoint).await.is_err(),
            "no daemon appeared"
        );
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

    // Scenario: El comando falla y no se arranca nada
    #[test]
    fn a_command_endpoint_never_starts_a_daemon_not_even_when_asked_to() {
        let own = "socket-of-this-user";
        let command = "exec:ssh my-pc meltemi bridge";

        assert!(
            !may_start(command, own, None),
            "a command endpoint is not a place to start a daemon"
        );
        // And the opt-in does NOT open it. This is the assertion the explicit
        // guard exists for: the text comparison alone refuses the first case by
        // accident and would allow this one.
        for asked in ["1", "yes", "true"] {
            assert!(
                !may_start(command, own, Some(asked)),
                "`{asked}` must not open a command endpoint to a local daemon"
            );
        }
        // The plain forms are untouched: the own endpoint still starts, and the
        // opt-in still works where it always did.
        assert!(may_start(own, own, None));
        assert!(may_start("some-other-socket", own, Some("1")));

        // And the refusal talks about the command, not about a tunnel: a remedy
        // pointing at the start-here variable would be a remedy that cannot
        // work, because the guard above closed it.
        let said = refusal(command);
        assert!(said.contains("names a command"), "{said}");
        assert!(
            said.contains("run it in a terminal"),
            "the remedy is about the command: {said}"
        );
        assert!(
            !said.contains("MELTEMI_START_HERE"),
            "and it does not offer a way that is shut: {said}"
        );
    }
}
