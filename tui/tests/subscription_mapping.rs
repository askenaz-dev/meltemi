// SPDX-License-Identifier: Apache-2.0

//! End-to-end mapping of the subscription verbs (vincular-suscripciones 3.1):
//! `link` prints the composed login gesture and `unlink` of a hand-written
//! profile carries the daemon's refusal — the CLI's command layer against an
//! ephemeral in-process daemon whose config dir this test owns.

use tokio::sync::mpsc;

use meltemi::cli::Command;
use meltemi::run::execute;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::Listener;

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemi-submap-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemi-submap-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// An ephemeral daemon whose config dir carries a registry with one linkable
/// provider (the mock binary need not exist: linking is detection-free).
async fn spawn(tag: &str) -> (String, std::path::PathBuf, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-submap-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    let registry = base.join("config").join("registry.toml");
    std::fs::write(
        &registry,
        "version = \"submap\"\n\n[[agents]]\nid = \"provider-a\"\nname = \"Provider A\"\nlevel = 1\nbin = \"provider-a\"\nauth-context-var = \"PROVIDER_CONTEXT_DIR\"\nlogin-hint = \"provider-a login\"\n",
    )
    .unwrap();
    std::fs::write(
        base.join("config").join("config.toml"),
        format!(
            "[fleet]\nregistry = '{}'\n\n[[fleet.profile]]\nname = \"manual\"\nagent = \"provider-a\"\nenv = {{ PROVIDER_CONTEXT_DIR = '/home/u/ctx/manual' }}\n",
            registry.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(base.join("data"), base.join("config"), shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, base, handle)
}

#[tokio::test]
async fn link_confirms_and_prints_the_composed_login_gesture() {
    // Scenario: link crea y responde con el gesto de login
    let (endpoint, base, handle) = spawn("link").await;

    let outcome = execute(
        Command::Link {
            agent: "provider-a".into(),
            name: "work".into(),
        },
        &endpoint,
        false,
    )
    .await
    .expect("link succeeds");

    assert!(outcome.human.contains("linked `work`"), "{}", outcome.human);
    // The gesture is printed in both shells: the one thing the user must run.
    assert!(
        outcome.human.contains("$env:PROVIDER_CONTEXT_DIR")
            && outcome.human.contains("PROVIDER_CONTEXT_DIR='"),
        "the composed gesture is the output's point: {}",
        outcome.human
    );
    assert!(outcome.human.contains("provider-a login"));
    assert_eq!(outcome.json["gesture"]["var"], "PROVIDER_CONTEXT_DIR");

    handle.abort();
    let _ = std::fs::remove_dir_all(&base);
}

#[tokio::test]
async fn unlink_of_a_hand_written_profile_refuses_with_the_remedy() {
    // Scenario: unlink de un vínculo manual rehúsa con remedio
    let (endpoint, base, handle) = spawn("manual").await;

    let error = execute(
        Command::Unlink {
            name: "manual".into(),
        },
        &endpoint,
        false,
    )
    .await
    .expect_err("a hand-written profile is not the store's to remove");
    assert_eq!(
        error.exit,
        meltemi::exit::ExitCode::Contract,
        "the refusal is a contract error"
    );
    assert!(
        error.message.contains("config.toml"),
        "the daemon's remedy travels with the refusal: {}",
        error.message
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(&base);
}
