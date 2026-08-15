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

    // `ssh -N -R <remote_socket>:<local_endpoint> <target>`: listen on the
    // remote socket and forward every connection back to the local daemon.
    let ssh_command = format!("ssh -N -R {REMOTE_SOCKET}:{local_endpoint} {target}");

    let mut config_snippet = String::from("Host meltemi-tunnel\n");
    config_snippet.push_str(&format!("    HostName {host}\n"));
    if let Some(user) = user {
        config_snippet.push_str(&format!("    User {user}\n"));
    }
    config_snippet.push_str("    RequestTTY no\n");
    config_snippet.push_str(&format!(
        "    RemoteForward {REMOTE_SOCKET} {local_endpoint}\n"
    ));
    // Then simply: `ssh -N meltemi-tunnel`.

    let note = format!(
        "The remote sshd must allow Unix-socket forwarding and clean stale sockets \
         (`StreamLocalBindUnlink yes`). On the far end, reach the daemon with \
         `MELTEMI_ENDPOINT={REMOTE_SOCKET} meltemi <subcommand>` (from git-bash on Windows, \
         prefix `MSYS_NO_PATHCONV=1` so the socket path is not mangled — see docs/plataformas.md)."
    );

    Ok(TunnelPlan {
        ssh_command,
        config_snippet,
        local_endpoint: local_endpoint.to_string(),
        remote_endpoint: REMOTE_SOCKET.to_string(),
        note,
    })
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
            "ssh -N -R ~/.meltemi/meltemid.sock:/run/user/1000/meltemi/meltemid.sock me@server"
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
}
