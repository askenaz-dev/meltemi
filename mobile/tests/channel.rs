// SPDX-License-Identifier: Apache-2.0

//! The companion's channel, end to end, on loopback (companero-que-conecta
//! D8): the app's own SSH client → `core/mock-sshd` → `meltemi bridge` → a
//! real `meltemid` in this process. No system `sshd`, no network, and no
//! setting of this machine touched.
//!
//! Needs the workspace's binaries built — `cargo build --workspace` first,
//! which is what CI does before `cargo test`: `meltemi` for the bridge and
//! `mock-sshd` for the far side, both found next to this test's executable.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};

use meltemi_client::rpc::Peer;
use meltemi_client::transport::{Listener, Stream};
use meltemi_mobile::keys::{AppKey, InMemory};
use meltemi_mobile::ssh::{self, Destination, Forgetful, HostKeys, Refusal};
use meltemi_mobile::store::KnownHosts;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};

const IN_TIME: Duration = Duration::from_secs(60);

/// A workspace binary next to this test's executable; built by CI's
/// `cargo build --workspace` before `cargo test`.
fn binary(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let path = dir.join(file);
    assert!(
        path.exists(),
        "{} is not built: run `cargo build --workspace` first, as CI does",
        path.display()
    );
    path
}

/// A fresh, process-unique endpoint for an ephemeral test daemon.
fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemi-companion-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemi-companion-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// Spins an in-process daemon at a fresh endpoint.
async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind endpoint");
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}

/// A scratch directory for one test, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "meltemi-companion-channel-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The far side: a `mock-sshd` that accepts `authorized` and runs the real
/// `meltemi bridge` against `endpoint`. Killed when dropped.
struct FarSide {
    _child: tokio::process::Child,
    port: u16,
    host_key: String,
}

async fn far_side(scratch: &Path, tag: &str, authorized: &str, endpoint: &str) -> FarSide {
    let file = scratch.join(format!("authorized-{tag}"));
    std::fs::write(&file, format!("{authorized}\n")).expect("write the authorized line");
    let mut child = tokio::process::Command::new(binary("mock-sshd"))
        .args(["--listen", "127.0.0.1:0", "--authorized"])
        .arg(&file)
        .arg("--bridge")
        .arg(binary("meltemi"))
        .arg("--env")
        .arg(format!("MELTEMI_ENDPOINT={endpoint}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .expect("mock-sshd starts");
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut lines = BufReader::new(stdout).lines();
    let announced = tokio::time::timeout(IN_TIME, lines.next_line())
        .await
        .expect("mock-sshd announces itself in time")
        .expect("readable")
        .expect("one line");
    let announced: Value = serde_json::from_str(&announced).expect("the announcement is JSON");
    FarSide {
        _child: child,
        port: u16::try_from(announced["port"].as_u64().expect("a port")).expect("a port"),
        host_key: announced["hostKey"]
            .as_str()
            .expect("the host key")
            .to_string(),
    }
}

/// What a client does with the stream: `initialize`, then `status`.
async fn initialize_and_status(stream: Stream) -> Value {
    let (peer, _incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi-mobile-test".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .expect("initialize answers through the bridge");
    let status = peer
        .request(methods::STATUS, &json!({}))
        .await
        .expect("status answers through the bridge");
    peer.close();
    status
}

fn destination_for(port: u16) -> Destination {
    Destination::parse(&format!("tester@127.0.0.1:{port}")).expect("a destination")
}

// Scenario: La app alcanza un daemon real por SSH en bucle local
#[tokio::test]
async fn the_app_reaches_a_real_daemon_over_ssh_on_loopback_and_only_through_the_bridge() {
    let scratch = Scratch::new("reach");
    let (endpoint, daemon) = spawn_daemon("companion-reach").await;
    let key = AppKey::load_or_create(&mut InMemory::default()).expect("the app's key");
    let far = far_side(&scratch.0, "reach", &key.authorized_line(), &endpoint).await;
    let destination = destination_for(far.port);
    let mut hosts = Forgetful::default();

    // The channel opens with the app's own key, and the first use fixes the
    // host key the server announced.
    let (stream, _tunnel) =
        tokio::time::timeout(IN_TIME, ssh::open(&destination, key.private(), &mut hosts))
            .await
            .expect("in time")
            .expect("the far side accepted the app's key");
    assert_eq!(
        hosts.remembered("127.0.0.1").as_deref(),
        Some(far.host_key.as_str()),
        "first use fixed the host key the server announced"
    );

    // And what comes through it is a real daemon's own answer, by the bridge.
    let status = tokio::time::timeout(IN_TIME, initialize_and_status(stream))
        .await
        .expect("in time");
    assert!(
        status
            .get("daemonVersion")
            .and_then(Value::as_str)
            .is_some_and(|v| !v.is_empty()),
        "a real daemon answered through the bridge: {status:#}"
    );

    // A command that is not the bridge is refused by the far side: the app
    // asks for the bridge and nothing else, and the server runs nothing else.
    let mut other = destination.clone();
    other.command = "cat".into();
    match tokio::time::timeout(IN_TIME, ssh::open(&other, key.private(), &mut hosts))
        .await
        .expect("in time")
    {
        Err(Refusal::Transport(said)) => {
            assert!(said.contains("could not run `cat`"), "{said}");
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("the test server must run only the bridge"),
    }

    // A key the far side does not know is rejected — the terminal kind, the
    // one that tells a person to add the key, and never to wait.
    let stranger = AppKey::load_or_create(&mut InMemory::default()).expect("another key");
    match tokio::time::timeout(
        IN_TIME,
        ssh::open(&destination, stranger.private(), &mut hosts),
    )
    .await
    .expect("in time")
    {
        Err(Refusal::Rejected(said)) => {
            assert!(said.contains("did not accept"), "{said}");
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("a key not in authorized_keys must not get in"),
    }

    drop(far);
    daemon.abort();
}

// Scenario: Un host reinstalado se nota en la prueba
#[tokio::test]
async fn a_reinstalled_host_is_refused_until_forgotten_on_purpose() {
    let scratch = Scratch::new("reinstalled");
    let (endpoint, daemon) = spawn_daemon("companion-reinstalled").await;
    let key = AppKey::load_or_create(&mut InMemory::default()).expect("the app's key");
    let mut hosts = KnownHosts::open(&scratch.0).expect("no hosts yet");

    // The machine, met once.
    let first = far_side(&scratch.0, "first", &key.authorized_line(), &endpoint).await;
    let first_key = first.host_key.clone();
    {
        let (stream, _tunnel) = tokio::time::timeout(
            IN_TIME,
            ssh::open(&destination_for(first.port), key.private(), &mut hosts),
        )
        .await
        .expect("in time")
        .expect("first use");
        tokio::time::timeout(IN_TIME, initialize_and_status(stream))
            .await
            .expect("in time");
    }
    assert_eq!(
        hosts.remembered("127.0.0.1").as_deref(),
        Some(first_key.as_str())
    );
    drop(first);

    // The same machine comes back reinstalled: a new host key, the same name.
    let second = far_side(&scratch.0, "second", &key.authorized_line(), &endpoint).await;
    assert_ne!(
        second.host_key, first_key,
        "a fresh start is a fresh host key"
    );
    let reinstalled = destination_for(second.port);
    match tokio::time::timeout(IN_TIME, ssh::open(&reinstalled, key.private(), &mut hosts))
        .await
        .expect("in time")
    {
        Err(Refusal::HostKeyChanged {
            host,
            remembered,
            offered,
        }) => {
            assert_eq!(host, "127.0.0.1");
            assert_eq!(remembered, first_key);
            assert_eq!(offered, second.host_key);
        }
        Err(other) => panic!("the wrong refusal: {other}"),
        Ok(_) => panic!("a changed host key must never be accepted in silence"),
    }
    // Nothing was accepted in silence: the old key is still the one on file.
    assert_eq!(
        hosts.remembered("127.0.0.1").as_deref(),
        Some(first_key.as_str())
    );

    // Forgetting is a person's choice; after it, the new key is fixed.
    assert!(hosts.forget("127.0.0.1").expect("written down"));
    let (stream, _tunnel) =
        tokio::time::timeout(IN_TIME, ssh::open(&reinstalled, key.private(), &mut hosts))
            .await
            .expect("in time")
            .expect("accepted after forgetting, on purpose");
    tokio::time::timeout(IN_TIME, initialize_and_status(stream))
        .await
        .expect("in time");
    assert_eq!(
        hosts.remembered("127.0.0.1").as_deref(),
        Some(second.host_key.as_str())
    );

    drop(second);
    daemon.abort();
}
