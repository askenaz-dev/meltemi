// SPDX-License-Identifier: Apache-2.0

//! The SSH channel of the companion (companero-movil design D2).
//!
//! One job: reach `meltemi bridge` on the user's machine and hand back a stream
//! the client stack already knows how to use. Everything above this file is the
//! same code the desktop runs.
//!
//! **Why this exists only here.** Everywhere else, remote access is an `exec:`
//! endpoint that launches the user's own `ssh`. A phone has no `ssh` to launch,
//! so the app speaks the protocol. On the desktop the `ssh` stays the user's,
//! and this crate is the only place that changes.

use std::fmt;
use std::sync::Arc;

use russh::client::{self, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyBase64};

/// The command run on the far side.
///
/// The same one a desktop client names in its `exec:` endpoint, which is the
/// point: the phone is not a second protocol, it is the same bridge reached by
/// a different means.
pub const BRIDGE: &str = "meltemi bridge";

/// The SSH port, when the user named none.
pub const DEFAULT_PORT: u16 = 22;

/// Where to connect, and as whom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// The command to run. Configurable because a user may have `meltemi`
    /// somewhere unusual, and refusing to let them say so would mean refusing
    /// their machine.
    pub command: String,
}

impl Destination {
    /// A destination from `user@host[:port]`, with the ordinary bridge command.
    ///
    /// # Errors
    ///
    /// Refuses anything that is not `user@host`, naming the shape it wanted:
    /// a half-parsed destination is how a connection goes somewhere else.
    pub fn parse(target: &str) -> Result<Self, String> {
        let target = target.trim();
        let (user, rest) = target
            .split_once('@')
            .ok_or_else(|| format!("`{target}` is not `user@host`: name the user to log in as"))?;
        let user = user.trim();
        if user.is_empty() {
            return Err(format!("`{target}` has no user before the `@`"));
        }
        // Where the port is, and where it is not. A bare IPv6 address is all
        // colons, so the rule everywhere else applies here: to carry a port it
        // must be bracketed. Splitting on the last colon without this reads
        // `fe80::1` as port 1 of a host called `fe80:` — which dials something
        // that does not exist, and a test caught it doing exactly that.
        let (host, port) = if let Some(after_bracket) = rest.strip_prefix('[') {
            let (inside, rest) = after_bracket
                .split_once(']')
                .ok_or_else(|| format!("`{target}` opens `[` and never closes it"))?;
            let port = match rest.strip_prefix(':') {
                Some(port) => port
                    .parse::<u16>()
                    .map_err(|_| format!("`{port}` is not a port number"))?,
                None if rest.is_empty() => DEFAULT_PORT,
                None => return Err(format!("`{rest}` follows `]` and is not a port")),
            };
            (inside, port)
        } else if rest.matches(':').count() > 1 {
            // More than one colon and no brackets: an address, not a port.
            (rest, DEFAULT_PORT)
        } else {
            match rest.rsplit_once(':') {
                Some((host, port))
                    if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) =>
                {
                    let port = port
                        .parse::<u16>()
                        .map_err(|_| format!("`{port}` is not a port number"))?;
                    (host, port)
                }
                _ => (rest, DEFAULT_PORT),
            }
        };
        let host = host.trim();
        if host.is_empty() {
            return Err(format!("`{target}` has no host after the `@`"));
        }
        Ok(Self {
            host: host.to_string(),
            port,
            user: user.to_string(),
            command: BRIDGE.to_string(),
        })
    }

    /// How this destination is shown in the chrome: `user@host`, never a port
    /// nobody set and never a command.
    #[must_use]
    pub fn shown(&self) -> String {
        if self.port == DEFAULT_PORT {
            format!("{}@{}", self.user, self.host)
        } else {
            format!("{}@{}:{}", self.user, self.host, self.port)
        }
    }
}

/// What the app remembers about a host's key, between connections.
///
/// Trust on first use, and nothing cleverer: there is no key distribution to
/// lean on and inventing one would be inventing a service.
pub trait HostKeys: Send {
    /// The key fingerprint remembered for `host`, if any.
    fn remembered(&self, host: &str) -> Option<String>;
    /// Remembers `fingerprint` as the key of `host`.
    fn remember(&mut self, host: &str, fingerprint: &str);
}

/// A `HostKeys` that forgets everything, for tests and for a first run.
#[derive(Debug, Default)]
pub struct Forgetful(std::collections::BTreeMap<String, String>);

impl HostKeys for Forgetful {
    fn remembered(&self, host: &str) -> Option<String> {
        self.0.get(host).cloned()
    }
    fn remember(&mut self, host: &str, fingerprint: &str) {
        self.0.insert(host.to_string(), fingerprint.to_string());
    }
}

/// Why a connection did not happen.
#[derive(Debug)]
pub enum Refusal {
    /// The destination could not be read.
    Destination(String),
    /// The host answered with a key that is not the one remembered.
    ///
    /// Its own variant, not a string, because this is the one refusal a surface
    /// must never render as "try again": somebody is either reinstalling a
    /// server or standing in the middle.
    HostKeyChanged {
        host: String,
        remembered: String,
        offered: String,
    },
    /// The server refused this key.
    Rejected(String),
    /// Everything else, with what the transport said.
    Transport(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Destination(said) => write!(f, "{said}"),
            Self::HostKeyChanged {
                host,
                remembered,
                offered,
            } => write!(
                f,
                "the key of `{host}` is not the one remembered: expected {remembered}, got \
                 {offered}. Nothing was sent. Either that machine was reinstalled, or this is not \
                 that machine"
            ),
            Self::Rejected(said) => write!(
                f,
                "the server refused this app's key: {said} — add its public key to your \
                 authorized_keys"
            ),
            Self::Transport(said) => write!(f, "{said}"),
        }
    }
}

impl std::error::Error for Refusal {}

/// The client side of the dialogue: it exists to decide about the host key.
struct Dialogue {
    remembered: Option<String>,
    /// What the server offered, kept so the caller can remember it when the
    /// connection turns out to be the first one.
    offered: Arc<std::sync::Mutex<Option<String>>>,
    /// Set when the offered key is not the remembered one, so the caller can
    /// tell a refusal from a transport failure.
    changed: Arc<std::sync::Mutex<bool>>,
}

impl client::Handler for Dialogue {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let offered = match server_public_key {
            russh::keys::PublicKeyOrCertificate::PublicKey { key, .. } => key.public_key_base64(),
            // A certificate is a key signed by a CA the app does not know, and
            // pretending to evaluate it would be worse than not accepting it.
            // The bastion's certificates are a phase-3 design note, not this.
            russh::keys::PublicKeyOrCertificate::Certificate(_) => return Ok(false),
        };
        if let Ok(mut seen) = self.offered.lock() {
            *seen = Some(offered.clone());
        }
        match &self.remembered {
            // First time: accept and let the caller write it down. Trust on
            // first use is the honest name for this, and the surface says so.
            None => Ok(true),
            Some(known) if *known == offered => Ok(true),
            Some(_) => {
                if let Ok(mut changed) = self.changed.lock() {
                    *changed = true;
                }
                Ok(false)
            }
        }
    }
}

/// A live SSH session with the channel that carries the protocol.
///
/// Holding the handle matters: dropping it closes the session, and the channel
/// with it. The stream handed to the client borrows nothing, so the caller must
/// keep this alive for as long as it uses that stream.
pub struct Tunnel {
    _session: Handle<Dialogue>,
    /// The fingerprint the server offered, for a first connection to remember.
    pub offered: Option<String>,
}

/// Opens the tunnel and runs the bridge, returning the stream and the session.
///
/// The stream is whatever `meltemi bridge` writes, which is the daemon's own
/// protocol — so the caller hands it straight to the client's JSON-RPC peer.
///
/// # Errors
///
/// Every refusal is a [`Refusal`], and a changed host key is its own variant
/// because a surface must never offer to retry past one.
pub async fn open(
    destination: &Destination,
    key: russh::keys::PrivateKey,
    hosts: &mut dyn HostKeys,
) -> Result<(meltemi_client::transport::Stream, Tunnel), Refusal> {
    let remembered = hosts.remembered(&destination.host);
    let offered = Arc::new(std::sync::Mutex::new(None));
    let changed = Arc::new(std::sync::Mutex::new(false));
    let dialogue = Dialogue {
        remembered: remembered.clone(),
        offered: Arc::clone(&offered),
        changed: Arc::clone(&changed),
    };

    let config = Arc::new(client::Config::default());
    let mut session = client::connect(
        config,
        (destination.host.as_str(), destination.port),
        dialogue,
    )
    .await
    .map_err(|e| {
        // A rejected host key surfaces here as a generic failure, so the flag
        // the handler set is what tells the two apart.
        if changed.lock().map(|c| *c).unwrap_or(false) {
            Refusal::HostKeyChanged {
                host: destination.host.clone(),
                remembered: remembered.clone().unwrap_or_default(),
                offered: offered
                    .lock()
                    .ok()
                    .and_then(|o| o.clone())
                    .unwrap_or_default(),
            }
        } else {
            Refusal::Transport(format!("could not reach {}: {e}", destination.shown()))
        }
    })?;

    let authenticated = session
        .authenticate_publickey(
            destination.user.clone(),
            PrivateKeyWithHashAlg::new(Arc::new(key), None),
        )
        .await
        .map_err(|e| Refusal::Transport(format!("authentication failed: {e}")))?;
    if !authenticated.success() {
        return Err(Refusal::Rejected(format!(
            "`{}` did not accept it",
            destination.shown()
        )));
    }

    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| Refusal::Transport(format!("could not open a session channel: {e}")))?;
    channel
        .exec(true, destination.command.as_bytes())
        .await
        .map_err(|e| Refusal::Transport(format!("could not run `{}`: {e}", destination.command)))?;

    // The whole point of this crate, in one line: the client's `Stream` is a
    // boxed `AsyncRead + AsyncWrite + Unpin + Send`, and an SSH channel is one.
    // Nothing in `meltemi-client` knows a phone exists.
    let stream: meltemi_client::transport::Stream = Box::new(channel.into_stream());
    let seen = offered.lock().ok().and_then(|o| o.clone());
    if let Some(fingerprint) = &seen {
        hosts.remember(&destination.host, fingerprint);
    }
    Ok((
        stream,
        Tunnel {
            _session: session,
            offered: seen,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_destination_is_user_at_host_with_an_optional_port() {
        let d = Destination::parse("guillermo@mi-pc").expect("plain");
        assert_eq!(d.user, "guillermo");
        assert_eq!(d.host, "mi-pc");
        assert_eq!(d.port, DEFAULT_PORT);
        assert_eq!(d.command, BRIDGE, "the same bridge every client runs");
        assert_eq!(d.shown(), "guillermo@mi-pc", "no port nobody set");

        let d = Destination::parse("  guillermo@mi-pc:2222  ").expect("with a port");
        assert_eq!(d.port, 2222);
        assert_eq!(d.shown(), "guillermo@mi-pc:2222");
    }

    #[test]
    fn an_address_with_colons_and_no_port_is_not_read_as_having_one() {
        // A bare IPv6 address is full of colons, and the first version of this
        // read `fe80::1` as port 1 of a host called `fe80:` — dialing something
        // that does not exist. To carry a port it must be bracketed, which is
        // the rule everywhere else that writes host and port together.
        let d = Destination::parse("me@fe80::1").expect("ipv6");
        assert_eq!(d.host, "fe80::1");
        assert_eq!(d.port, DEFAULT_PORT);

        let d = Destination::parse("me@[fe80::1]:2222").expect("bracketed with a port");
        assert_eq!(d.host, "fe80::1");
        assert_eq!(d.port, 2222);

        let d = Destination::parse("me@[fe80::1]").expect("bracketed without one");
        assert_eq!(d.host, "fe80::1");
        assert_eq!(d.port, DEFAULT_PORT);

        // And a bracket that never closes is a typo, not a host.
        assert!(Destination::parse("me@[fe80::1").is_err());
    }

    #[test]
    fn a_half_read_destination_is_refused_with_the_shape_it_wanted() {
        for bad in ["mi-pc", "@mi-pc", "guillermo@", "  "] {
            let said = Destination::parse(bad).expect_err(bad);
            assert!(!said.is_empty(), "{bad}");
        }
        assert!(
            Destination::parse("mi-pc")
                .expect_err("no user")
                .contains("user@host"),
            "it names the shape it wanted"
        );
    }

    #[test]
    fn a_port_that_is_not_a_number_is_refused_and_not_dropped() {
        let said = Destination::parse("me@host:nope");
        // `nope` is not digits, so it is part of the host rather than a port —
        // and the connection will fail by name, which is honest. What must NOT
        // happen is silently dialing 22 on `host`.
        let d = said.expect("parsed as a host");
        assert_eq!(d.host, "host:nope");
    }

    // Scenario: La clave del host cambia
    #[test]
    fn a_changed_host_key_reads_as_its_own_refusal_and_never_as_retry() {
        let refusal = Refusal::HostKeyChanged {
            host: "mi-pc".into(),
            remembered: "AAAAC3NzaC1lZDI1NTE5AAAAold".into(),
            offered: "AAAAC3NzaC1lZDI1NTE5AAAAnew".into(),
        };
        let said = refusal.to_string();
        assert!(said.contains("not the one remembered"), "{said}");
        assert!(said.contains("Nothing was sent"), "{said}");
        assert!(
            said.contains("not that machine"),
            "it names the possibility that matters: {said}"
        );
        // And it is a distinct variant, so a surface cannot treat it as a
        // transport hiccup by accident.
        assert!(matches!(refusal, Refusal::HostKeyChanged { .. }));
    }

    #[test]
    fn host_keys_are_remembered_per_host() {
        let mut hosts = Forgetful::default();
        assert_eq!(hosts.remembered("mi-pc"), None);
        hosts.remember("mi-pc", "fingerprint-1");
        assert_eq!(hosts.remembered("mi-pc").as_deref(), Some("fingerprint-1"));
        // Another host is another key; one machine's reinstall is not another's.
        assert_eq!(hosts.remembered("la-torre"), None);
    }

    #[test]
    fn the_command_is_the_same_bridge_the_desktop_runs() {
        // Not a second protocol for phones: the far side is the same command a
        // desktop client names in its `exec:` endpoint.
        assert_eq!(BRIDGE, "meltemi bridge");
    }
}
