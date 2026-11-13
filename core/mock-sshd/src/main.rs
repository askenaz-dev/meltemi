// SPDX-License-Identifier: Apache-2.0

//! A test SSH server, for proving the companion's channel without a system
//! `sshd` (companero-que-conecta D8).
//!
//! It does the one thing the companion needs of a machine and nothing more:
//! it accepts exactly one public key — under any user name — and runs exactly
//! one command, `meltemi bridge`, with the binary and the environment it was
//! handed. Every other request is refused. Its host key is made fresh on
//! every start, so starting it again IS a reinstalled machine, which is what
//! the test of a changed host key needs and must not fake.
//!
//! Loopback only, and never in a product: `core/mock-sshd` is a fixture like
//! `mock-agent`, built for the e2e in `mobile/tests/channel.rs` and for the
//! emulator smoke, where the app on the emulator reaches it at `10.0.2.2`.
//!
//! ```text
//! mock-sshd --listen 127.0.0.1:0 --authorized <file> --bridge <meltemi> [--env K=V]...
//! ```
//!
//! On start it prints one JSON line — `{"port":…,"hostKey":"…"}` — and then
//! serves until killed.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use russh::keys::{Algorithm, PrivateKey, PublicKey, PublicKeyBase64};
use russh::server::{
    Auth, ChannelOpenHandle, Config, Handler, Msg, Server, Session, VerifiedPublicKey,
};
use russh::{Channel, ChannelId, MethodKind, MethodSet};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};

/// The one command this server runs.
const BRIDGE: &str = "meltemi bridge";

struct Options {
    listen: String,
    authorized: PublicKey,
    bridge: String,
    env: Vec<(String, String)>,
}

fn parse_args() -> Result<Options, String> {
    let mut listen = "127.0.0.1:0".to_string();
    let mut authorized = None;
    let mut bridge = None;
    let mut env = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("`{flag}` needs a value"));
        match flag.as_str() {
            "--listen" => listen = value()?,
            "--authorized" => {
                let path = value()?;
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("could not read {path}: {e}"))?;
                let line = text
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .ok_or_else(|| format!("{path} holds no key"))?;
                authorized = Some(
                    PublicKey::from_openssh(line)
                        .map_err(|e| format!("{path} is not an authorized_keys line: {e}"))?,
                );
            }
            "--bridge" => bridge = Some(value()?),
            "--env" => {
                let pair = value()?;
                let (key, val) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("`--env {pair}` is not KEY=VALUE"))?;
                env.push((key.to_string(), val.to_string()));
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Options {
        listen,
        authorized: authorized.ok_or("`--authorized <file>` is required")?,
        bridge: bridge.ok_or("`--bridge <meltemi>` is required")?,
        env,
    })
}

/// What every connection shares: the key that gets in and the bridge to run.
struct Shared {
    authorized: PublicKey,
    bridge: String,
    env: Vec<(String, String)>,
}

struct Gate {
    shared: Arc<Shared>,
}

impl Server for Gate {
    type Handler = Client;

    fn new_client(&mut self, _peer: Option<std::net::SocketAddr>) -> Client {
        Client {
            shared: Arc::clone(&self.shared),
            child: None,
            stdin: None,
        }
    }
}

/// One connection: at most one bridge process, fed by the one channel.
struct Client {
    shared: Arc<Shared>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

impl Handler for Client {
    type Error = russh::Error;

    /// Any user name, exactly one key: who the far side says it is, is its
    /// business; the key is what this server checks. russh hands it over only
    /// after verifying the signature of the request, so a key here is a key
    /// the far side holds, not one it merely named.
    async fn auth_publickey(
        &mut self,
        _user: &str,
        key: VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        if key.public_key.public_key_base64() == self.shared.authorized.public_key_base64() {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    /// Only the bridge runs here. Anything else is refused on the channel,
    /// which the companion reads as "could not run" — and the test reads as
    /// proof that the app asks for the bridge and nothing else.
    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if data != BRIDGE.as_bytes() {
            eprintln!(
                "mock-sshd: refused `{}`: only `{BRIDGE}` runs here",
                String::from_utf8_lossy(data)
            );
            session.channel_failure(channel)?;
            return Ok(());
        }
        let mut child = Command::new(&self.shared.bridge)
            .arg("bridge")
            .envs(self.shared.env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .inspect_err(|e| eprintln!("mock-sshd: could not run {}: {e}", self.shared.bridge))?;
        self.stdin = child.stdin.take();
        let mut stdout = child.stdout.take().expect("stdout is piped");
        self.child = Some(child);

        // The bridge's stdout is the channel's data, until the bridge ends.
        let handle = session.handle();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 16 * 1024];
            loop {
                match stdout.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if handle.data(channel, buffer[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                }
            }
            let _ = handle.eof(channel).await;
            let _ = handle.close(channel).await;
        });
        session.channel_success(channel)?;
        Ok(())
    }

    /// The channel's data is the bridge's stdin.
    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(stdin) = self.stdin.as_mut()
            && stdin.write_all(data).await.is_err()
        {
            self.stdin = None;
        }
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Closing the bridge's stdin is what ends it, as the bridge's own
        // rule says: the close of one end closes the bridge.
        self.stdin = None;
        Ok(())
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.stdin = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("mock-sshd: {error}");
            std::process::exit(2);
        }
    };

    // A fresh host key on every start: the second start of this server is a
    // reinstalled machine, which the test of a changed host key needs for real.
    let mut rng =
        russh::keys::ssh_key::rand_core::UnwrapErr(russh::keys::ssh_key::getrandom::SysRng);
    let host_key = PrivateKey::random(&mut rng, Algorithm::Ed25519).expect("a host key");
    let announced = host_key.public_key().public_key_base64();

    let listener = tokio::net::TcpListener::bind(&options.listen)
        .await
        .unwrap_or_else(|e| panic!("could not listen on {}: {e}", options.listen));
    let port = listener.local_addr().expect("a bound address").port();
    println!(
        "{}",
        serde_json::json!({ "port": port, "hostKey": announced })
    );

    let mut methods = MethodSet::empty();
    methods.push(MethodKind::PublicKey);
    let config = Config {
        keys: vec![host_key],
        methods,
        // A fixture: nobody is timing rejections here.
        auth_rejection_time: Duration::from_millis(1),
        auth_rejection_time_initial: Some(Duration::ZERO),
        inactivity_timeout: Some(Duration::from_secs(3600)),
        ..Config::default()
    };
    let mut gate = Gate {
        shared: Arc::new(Shared {
            authorized: options.authorized,
            bridge: options.bridge,
            env: options.env,
        }),
    };
    gate.run_on_socket(Arc::new(config), &listener)
        .await
        .expect("the server runs until killed");
}
