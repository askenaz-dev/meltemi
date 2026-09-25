// SPDX-License-Identifier: Apache-2.0

//! Auditable SSH tunnel helper (control-remoto-asistido, design D3/D4).
//!
//! A pure formatter. Run on the machine where `meltemid` lives, it composes the
//! exact `ssh` invocation — using the user's OWN ssh — that pushes this daemon's
//! local endpoint out to a remote host, the equivalent `~/.ssh/config` snippet,
//! and the `MELTEMI_ENDPOINT` the far end sets. It opens no sockets, adds no
//! dependency, introduces no network transport into the daemon, and never
//! touches key material (§3, §2).
//!
//! The daemon's local endpoint is a Unix domain socket on macOS/Linux, which
//! OpenSSH can reverse-forward (`ssh -R`), and a named pipe on Windows, which it
//! cannot: so a Windows daemon refuses honestly (D4) instead of printing a
//! command that cannot work. A Windows machine can still be the *client* that
//! runs the printed command to reach a remote Unix daemon.

/// A composed tunnel: everything the user needs to reach this daemon remotely,
/// through their own `ssh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelPlan {
    /// The arguments given to `ssh`: the one source of what is printed and of
    /// what `--exec` runs, so the two cannot drift (acceso-remoto-afinado D5).
    pub ssh_args: Vec<String>,
    /// The exact `ssh -R` command that reverse-forwards the local endpoint.
    pub ssh_command: String,
    /// The equivalent `~/.ssh/config` block, for a named `ssh meltemi-tunnel`.
    pub config_snippet: String,
    /// The daemon endpoint being forwarded (this machine's socket).
    pub local_endpoint: String,
    /// The value the far end sets as `MELTEMI_ENDPOINT` to reach the daemon.
    pub remote_endpoint: String,
    /// Operator notes: prerequisites and the git-bash/MSYS gotcha.
    pub note: String,
}

/// An honest refusal: the platform cannot forward its endpoint with standard
/// OpenSSH, so nothing is printed that would silently fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelRefusal {
    pub reason: String,
    pub remedy: String,
}

/// The conventional path the far end listens on and points `MELTEMI_ENDPOINT`
/// at. One tunnel per host; the user can edit it freely.
const REMOTE_SOCKET: &str = "~/.meltemi/meltemid.sock";

/// The heartbeat and the forward check, as `ssh` options (acceso-remoto-afinado
/// D5): with no answer from the other end for about 90 seconds `ssh` ends,
/// instead of living on as a process and dead as a path; and a forward that
/// cannot be set up ends it at once, instead of leaving it connected and
/// forwarding nothing.
const HEARTBEAT: [(&str, &str); 3] = [
    ("ServerAliveInterval", "30"),
    ("ServerAliveCountMax", "3"),
    ("ExitOnForwardFailure", "yes"),
];

/// Composes the tunnel for a daemon on this platform. `daemon_is_windows` is the
/// platform of the machine running the helper (where the daemon lives);
/// `local_endpoint` is that daemon's endpoint; `target` is `user@host` (how the
/// remote end is reached), or a placeholder when the user gave none.
///
/// Pure and side-effect free, so the whole composition — and the Windows
/// refusal — is unit-testable without a network or an `ssh`.
pub fn compose(
    daemon_is_windows: bool,
    local_endpoint: &str,
    target: Option<&str>,
) -> Result<TunnelPlan, TunnelRefusal> {
    if daemon_is_windows {
        return Err(TunnelRefusal {
            reason: "this daemon's endpoint is a Windows named pipe, which standard OpenSSH \
                     cannot forward"
                .to_string(),
            remedy: "Reach this machine with the bridge instead: from the remote side run \
                     `ssh <this-host> meltemi bridge`, which carries the endpoint over the \
                     ssh connection's own stdio and needs no forwarding at all. Forwarding \
                     itself only works from a macOS/Linux daemon; a Windows machine can \
                     still be the client that runs the printed command against one."
                .to_string(),
        });
    }

    let target = target.unwrap_or("user@host");
    let (user, host) = split_target(target);

    // `ssh -N <heartbeat> -R <remote_socket>:<local_endpoint> <target>`: listen
    // on the remote socket and forward every connection back to the local
    // daemon, and end — visibly — when the path is gone.
    let mut ssh_args = vec!["-N".to_string()];
    for (option, value) in HEARTBEAT {
        ssh_args.push("-o".to_string());
        ssh_args.push(format!("{option}={value}"));
    }
    ssh_args.push("-R".to_string());
    ssh_args.push(format!("{REMOTE_SOCKET}:{local_endpoint}"));
    ssh_args.push(target.to_string());
    let ssh_command = format!("ssh {}", ssh_args.join(" "));

    let mut config_snippet = String::from("Host meltemi-tunnel\n");
    config_snippet.push_str(&format!("    HostName {host}\n"));
    if let Some(user) = user {
        config_snippet.push_str(&format!("    User {user}\n"));
    }
    config_snippet.push_str("    RequestTTY no\n");
    for (option, value) in HEARTBEAT {
        config_snippet.push_str(&format!("    {option} {value}\n"));
    }
    config_snippet.push_str(&format!(
        "    RemoteForward {REMOTE_SOCKET} {local_endpoint}\n"
    ));
    // Then simply: `ssh -N meltemi-tunnel`.

    let note = format!(
        "ServerAliveInterval and ServerAliveCountMax end the tunnel when the other end has \
         not answered for about 90 seconds, so a dead path does not live on as a live process; \
         ExitOnForwardFailure ends it at once when the forward cannot be set up. \
         The remote sshd must allow Unix-socket forwarding and clean stale sockets \
         (`StreamLocalBindUnlink yes`). On the far end, reach the daemon with \
         `MELTEMI_ENDPOINT={REMOTE_SOCKET} meltemi <subcommand>` (from git-bash on Windows, \
         prefix `MSYS_NO_PATHCONV=1` so the socket path is not mangled — see docs/plataformas.md)."
    );

    Ok(TunnelPlan {
        ssh_args,
        ssh_command,
        config_snippet,
        local_endpoint: local_endpoint.to_string(),
        remote_endpoint: REMOTE_SOCKET.to_string(),
        note,
    })
}

/// How the `ssh` that `--exec` ran came to an end (acceso-remoto-afinado D6):
/// `Ok` with the words when it ended on its own with success, `Err` with them
/// when it failed or was ended by a signal — a tunnel that died is never
/// reported as a success, and the far end no longer reaches the daemon either
/// way.
///
/// # Errors
///
/// Any status but a clean `0`.
pub fn ending(status: Option<i32>) -> Result<String, String> {
    const AFTER: &str = "the far end no longer reaches this daemon";
    match status {
        Some(0) => Ok(format!(
            "the tunnel ended: ssh exited normally, and {AFTER}; run it again to reconnect"
        )),
        Some(code) => Err(format!(
            "the tunnel is down: ssh exited with status {code}, and {AFTER}. The usual causes: \
             the other end stopped answering (the heartbeat ends ssh within about 90 seconds), \
             the forward could not be set up (ExitOnForwardFailure ends ssh at once: a stale \
             socket on the far end, or an sshd that does not allow it), or the login failed; \
             run it again once the cause is fixed"
        )),
        None => Err(format!(
            "the tunnel is down: ssh was ended by a signal, and {AFTER}"
        )),
    }
}

/// Splits `user@host` into `(Some(user), host)`, or `(None, host)` when no user
/// is given.
fn split_target(target: &str) -> (Option<&str>, &str) {
    match target.split_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => (Some(user), host),
        _ => (None, target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_a_reverse_forward_for_a_unix_endpoint() {
        // Scenario: Composición del túnel por plataforma
        let plan = compose(
            false,
            "/run/user/1000/meltemi/meltemid.sock",
            Some("me@server"),
        )
        .expect("a unix daemon composes a plan");
        // The exact reverse-forward of THIS endpoint, using the user's own ssh.
        assert_eq!(
            plan.ssh_command,
            "ssh -N -o ServerAliveInterval=30 -o ServerAliveCountMax=3 \
             -o ExitOnForwardFailure=yes \
             -R ~/.meltemi/meltemid.sock:/run/user/1000/meltemi/meltemid.sock me@server"
        );
        // The config snippet and the endpoint the far end must use.
        assert!(plan.config_snippet.contains("Host meltemi-tunnel"));
        assert!(plan.config_snippet.contains("HostName server"));
        assert!(plan.config_snippet.contains("User me"));
        assert!(plan.config_snippet.contains(
            "RemoteForward ~/.meltemi/meltemid.sock /run/user/1000/meltemi/meltemid.sock"
        ));
        assert_eq!(plan.remote_endpoint, "~/.meltemi/meltemid.sock");
        // The git-bash/MSYS gotcha is carried for the far end.
        assert!(plan.note.contains("MELTEMI_ENDPOINT"));
        assert!(plan.note.contains("MSYS_NO_PATHCONV"));
    }

    #[test]
    fn a_windows_daemon_refuses_honestly() {
        // Scenario: Plataforma sin reenvío posible rehúsa honesta
        let refusal = compose(true, r"\\.\pipe\meltemid-S-1-5-21", Some("me@server"))
            .expect_err("a Windows named pipe cannot be forwarded");
        assert!(refusal.reason.contains("named pipe"));
        // The refusal STANDS: forwarding a named pipe is still impossible, and
        // no command that cannot work is ever printed.
        assert!(refusal.remedy.to_lowercase().contains("linux") || refusal.remedy.contains("Unix"));
        // What improved is the remedy: it names the way that DOES work on this
        // platform, with its exact command (acceso-remoto-en-dos-vias 1.2).
        assert!(
            refusal.remedy.contains("meltemi bridge"),
            "the remedy names the bridge and its command: {}",
            refusal.remedy
        );
    }

    #[test]
    fn a_missing_target_uses_a_clear_placeholder() {
        let plan = compose(false, "/tmp/meltemid.sock", None).expect("composes with placeholder");
        assert!(plan.ssh_command.ends_with("user@host"));
        // No `User` line when the placeholder carries a user already: it is
        // parsed like any target.
        assert!(plan.config_snippet.contains("HostName host"));
        assert!(plan.config_snippet.contains("User user"));
    }

    #[test]
    fn a_bare_host_omits_the_user_line() {
        let plan = compose(false, "/tmp/meltemid.sock", Some("server.example")).expect("composes");
        assert!(plan.ssh_command.ends_with("server.example"));
        assert!(plan.config_snippet.contains("HostName server.example"));
        assert!(!plan.config_snippet.contains("User "));
    }

    // Scenario: El túnel lleva latido y cae si no puede reenviar
    #[test]
    fn the_tunnel_carries_a_heartbeat_and_ends_when_it_cannot_forward() {
        let plan = compose(false, "/tmp/meltemid.sock", Some("me@server")).expect("composes");
        // In the command…
        for option in [
            "ServerAliveInterval=30",
            "ServerAliveCountMax=3",
            "ExitOnForwardFailure=yes",
        ] {
            assert!(
                plan.ssh_command.contains(&format!("-o {option}")),
                "{}",
                plan.ssh_command
            );
        }
        // …in the snippet, with what each does…
        for line in [
            "    ServerAliveInterval 30",
            "    ServerAliveCountMax 3",
            "    ExitOnForwardFailure yes",
        ] {
            assert!(
                plan.config_snippet.contains(line),
                "{}",
                plan.config_snippet
            );
        }
        assert!(plan.note.contains("about 90 seconds"), "{}", plan.note);
        assert!(
            plan.note.contains("forward cannot be set up"),
            "{}",
            plan.note
        );
        // …and what `--exec` runs is exactly what was printed.
        assert_eq!(plan.ssh_command, format!("ssh {}", plan.ssh_args.join(" ")));
        assert_eq!(plan.ssh_args.last().map(String::as_str), Some("me@server"));
    }

    // Scenario: Un túnel que muere se reporta
    #[test]
    fn a_tunnel_that_dies_is_reported_and_never_as_a_success() {
        let ended = ending(Some(0)).expect("a clean exit is no failure");
        assert!(ended.contains("no longer reaches this daemon"), "{ended}");
        let failed = ending(Some(255)).expect_err("a failed ssh is not a success");
        assert!(failed.contains("status 255"), "{failed}");
        assert!(failed.contains("no longer reaches this daemon"), "{failed}");
        assert!(failed.contains("ExitOnForwardFailure"), "{failed}");
        let signalled = ending(None).expect_err("a signal is not a success");
        assert!(signalled.contains("signal"), "{signalled}");
    }
}
