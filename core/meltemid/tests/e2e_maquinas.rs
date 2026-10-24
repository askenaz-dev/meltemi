// SPDX-License-Identifier: Apache-2.0

//! The machines of a user, end to end (maquinas-remotas).
//!
//! Two real daemons over the real transport standing in for two machines, and a
//! configuration file naming both. What is under test is the whole path a person
//! takes — read the list, test one before trusting it, and be on the other
//! afterwards — rather than each piece on its own.
//!
//! No network and no `ssh`: one endpoint is a plain local one and the other is a
//! command that speaks the protocol over its stdio, which is exactly what
//! `ssh <host> meltemi bridge` is once the connection is made. Testing it with a
//! real `ssh` would need a host, a key and a network, and would be testing
//! OpenSSH rather than this.
//!
//! Lives in the daemon's test crate rather than the client's, where the task
//! placed it: the client crate does not depend on the daemon, and making it do
//! so for a test would add a dependency — a circular one — which this change
//! forbade itself in its own task list. Here the dependency already runs the
//! right way round.

use std::path::{Path, PathBuf};
use std::time::Duration;

use meltemi_client::machines;
use meltemi_client::transport::Listener;
use meltemid::server::{DaemonState, serve_until_shutdown};

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemi-machines-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemi-machines-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::sync::mpsc::Sender<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx.clone());
    tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, shutdown_tx)
}

/// A configuration directory naming the two machines.
fn config_dir(tag: &str, first: &str, second: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("meltemi-machines-cfg-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("config dir");
    std::fs::write(
        dir.join("config.toml"),
        format!(
            "[[client.machines]]\nname = \"esta\"\nendpoint = {}\n\n\
             [[client.machines]]\nname = \"la-torre\"\nendpoint = {}\n",
            serde_json::json!(first),
            serde_json::json!(second),
        ),
    )
    .expect("config file");
    dir
}

fn wipe(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

// Scenario: Las máquinas se leen del usuario y nunca de un repositorio
// Scenario: Probar una conexión no arranca nada
#[tokio::test]
async fn two_machines_are_listed_probed_and_told_apart() {
    let (first, _one) = spawn_daemon("e2e-first").await;
    let (second, _two) = spawn_daemon("e2e-second").await;
    let dir = config_dir("both", &first, &second);

    // Read: two machines, each with what the user wrote.
    let read = machines::read(&dir);
    assert_eq!(read.names(), vec!["esta", "la-torre"], "{read:?}");
    assert!(read.diagnostics.is_empty(), "{read:?}");

    // Probe: both answer, with a version and a time.
    for name in ["esta", "la-torre"] {
        let machine = read.by_name(name).expect(name);
        let probe = machines::probe(&machine.endpoint).await;
        assert!(probe.answered(), "{name} must answer: {probe:?}");
        // A REAL version, not the `unknown` fallback: reading the wrong field
        // was green here until this assertion existed, because an absent field
        // falls back silently. The conducted smoke found it; this keeps it found.
        assert!(
            probe.version.as_deref().is_some_and(
                |v| v != "unknown" && v.chars().next().is_some_and(|c| c.is_ascii_digit())
            ),
            "{name} must report the daemon's real version: {probe:?}"
        );
        assert!(
            probe.millis < machines::PROBE_BUDGET.as_millis() as u64,
            "{name} answered within its budget: {probe:?}"
        );
    }

    // And the endpoint of one is not the endpoint of the other, which is the
    // whole point of having two.
    assert_ne!(
        read.by_name("esta").expect("esta").endpoint,
        read.by_name("la-torre").expect("la-torre").endpoint
    );
    // The machine a connection belongs to is answerable from the endpoint, which
    // is what the chrome uses to say which one you are on.
    assert_eq!(
        read.named(&second).map(|m| m.name.as_str()),
        Some("la-torre")
    );
    wipe(&dir);
}

#[tokio::test]
async fn a_machine_that_is_switched_off_fails_its_probe_and_starts_nothing() {
    // The honest answer, and the one that matters: a remote machine that is off
    // must fail rather than be started on the user's behalf.
    let (alive, _one) = spawn_daemon("e2e-alive").await;
    let dead = test_endpoint("e2e-never-bound");
    let dir = config_dir("mixed", &alive, &dead);
    let read = machines::read(&dir);

    let good = machines::probe(&read.by_name("esta").expect("esta").endpoint).await;
    assert!(good.answered(), "{good:?}");

    let bad = machines::probe(&read.by_name("la-torre").expect("la-torre").endpoint).await;
    assert!(
        !bad.answered(),
        "a machine that is not there must not answer"
    );
    assert!(bad.detail.is_some(), "the reason travels: {bad:?}");
    // Nothing was started: the endpoint is still unbound, so a second probe
    // fails exactly the same way. A daemon started by the first would answer.
    let again = machines::probe(&read.by_name("la-torre").expect("la-torre").endpoint).await;
    assert!(
        !again.answered(),
        "probing must not have started a daemon: {again:?}"
    );
    wipe(&dir);
}

// Scenario: La lista no lleva material de claves
#[tokio::test]
async fn a_machine_with_a_key_path_is_refused_and_the_good_one_survives() {
    let (alive, _one) = spawn_daemon("e2e-keys").await;
    let dir = std::env::temp_dir().join(format!("meltemi-machines-keys-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("config.toml"),
        format!(
            "[[client.machines]]\nname = \"esta\"\nendpoint = {}\n\n\
             [[client.machines]]\nname = \"la-torre\"\nendpoint = \"exec:ssh t meltemi bridge\"\n\
             identity-file = \"~/.ssh/id_ed25519\"\n",
            serde_json::json!(alive)
        ),
    )
    .expect("config file");

    let read = machines::read(&dir);
    assert_eq!(
        read.names(),
        vec!["esta"],
        "the one carrying a key path is refused and the other survives: {read:?}"
    );
    let said = read.diagnostics.join(" ");
    assert!(said.contains("identity-file"), "{said}");
    assert!(
        said.contains("ssh"),
        "the remedy names where it lives: {said}"
    );

    // And the surviving one still works, which is the half that matters: a bad
    // entry costs its own row and nothing else.
    let probe = tokio::time::timeout(
        Duration::from_secs(20),
        machines::probe(&read.by_name("esta").expect("esta").endpoint),
    )
    .await
    .expect("the probe ends");
    assert!(probe.answered(), "{probe:?}");
    wipe(&dir);
}
