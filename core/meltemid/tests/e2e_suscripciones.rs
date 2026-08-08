// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of subscription linking (vincular-suscripciones), driving
//! an ephemeral daemon over the real transport:
//!
//! - linking creates the profile, the catalog lists it, and a dispatched
//!   session resolves the underlying binary UNDER the linked context (proven
//!   by the mock echoing the exact variable the registry declared);
//! - the composed login gesture travels whole and the context directory is
//!   created empty;
//! - refusals: no declared variable, invalid name, taken name, and unlinking
//!   a hand-written profile;
//! - unlinking never touches the context directory.
//!
//! Runs against temporary fixtures and the CI `mock-agent`, never this repo,
//! never real agents, never the network.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn mock_agent_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A registry whose provider declares `MELTEMI_MOCK_MARKER` as its auth
/// context variable. The mock echoes that variable into its artifact, so the
/// dispatched turn PROVES the linked context reached the subprocess.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-subs-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();
    let mock_dir = mock_agent_dir().display().to_string().replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/registry.toml"),
        format!(
            "version = \"subs-e2e\"\n\n\
             [[agents]]\nid = \"provider-a\"\nname = \"Provider A\"\nlevel = 1\nbin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock_dir}']\nauth-context-var = \"MELTEMI_MOCK_MARKER\"\nlogin-hint = \"provider-a login\"\n\n\
             [[agents]]\nid = \"provider-b\"\nname = \"Provider B\"\nlevel = 1\nbin = \"mock-agent\"\nacp-args = []\ncandidate-paths = ['{mock_dir}']\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/tasks.md"),
        "## 1. Build\n\n- [ ] 1.1 Add the toggle\n",
    )
    .unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-subs-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-subs-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// The daemon with ITS OWN config dir carrying the fixture registry override,
/// so links resolve against the fixture's providers.
async fn spawn_daemon(
    tag: &str,
    registry: &Path,
) -> (String, PathBuf, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-subsd-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::create_dir_all(base.join("config")).unwrap();
    std::fs::write(
        base.join("config").join("config.toml"),
        format!(
            "[fleet]\nregistry = '{}'\n",
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

async fn init_client(endpoint: &str) -> Peer {
    let stream = connect(endpoint).await.expect("connect");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-subs-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn linking_creates_the_profile_and_a_session_honors_it() {
    // Scenario: Vincular crea el perfil y la sesión lo honra
    // Scenario: El vínculo entrega el gesto de login
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = fixture("honor");
    let (endpoint, base, daemon) =
        spawn_daemon("honor", &root.join(".meltemi/registry.toml")).await;
    let peer = init_client(&endpoint).await;

    // Link a subscription on the declaring provider.
    let linked = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": "provider-a", "name": "work" }),
        )
        .await
        .expect("link ok");
    assert_eq!(linked["profile"], "work");
    assert_eq!(linked["agent"], "provider-a");
    // The gesture travels whole, and the context directory exists, empty.
    let gesture = &linked["gesture"];
    assert_eq!(gesture["var"], "MELTEMI_MOCK_MARKER");
    let ctx = PathBuf::from(gesture["value"].as_str().unwrap());
    assert!(ctx.is_dir(), "the context dir was created");
    assert_eq!(
        std::fs::read_dir(&ctx).unwrap().count(),
        0,
        "…and created EMPTY: nothing of Meltemi's lives in a credentials dir"
    );
    assert_eq!(gesture["hint"], "provider-a login");
    assert!(
        gesture["posix"]
            .as_str()
            .unwrap()
            .contains("MELTEMI_MOCK_MARKER='")
    );
    assert!(
        gesture["powershell"]
            .as_str()
            .unwrap()
            .contains("$env:MELTEMI_MOCK_MARKER")
    );

    // The catalog lists the link as a profile row naming its agent.
    let fleet = peer
        .request(methods::FLEET_LIST, &json!({}))
        .await
        .expect("fleet ok");
    let row = fleet["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "work")
        .expect("the link is a catalog row");
    assert_eq!(row["source"], "profile");
    assert_eq!(row["underlyingAgent"], "provider-a");

    // And a session naming the profile resolves the underlying binary UNDER
    // the linked context: the mock echoes the variable's value — the context
    // path — into its artifact.
    let dispatched = peer
        .request(
            methods::WORKTREE_DISPATCH,
            &json!({
                "projectRoot": root.display().to_string(),
                "change": "dark-mode",
                "task": "1.1",
                "agent": "work",
            }),
        )
        .await
        .expect("dispatch ok");
    assert_eq!(dispatched["resolution"]["source"], "profile");
    assert_eq!(dispatched["resolution"]["profile"], "work");
    let worktree = PathBuf::from(dispatched["worktree"].as_str().unwrap());
    let artifact = std::fs::read_to_string(worktree.join("task-1-1.md")).unwrap();
    let expected = format!("marker={}", ctx.display());
    assert!(
        artifact.contains(&expected),
        "the linked context reached the subprocess: wanted `{expected}` in `{artifact}`"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&base);
}

#[tokio::test]
async fn refusals_name_their_cause_and_their_remedy() {
    // Scenario: Vincular sobre un agente sin variable declarada rehúsa
    // Scenario: El nombre inválido como ruta rehúsa
    // Scenario: Nombre ya vinculado rehúsa
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = fixture("refuse");
    let (endpoint, base, daemon) =
        spawn_daemon("refuse", &root.join(".meltemi/registry.toml")).await;
    let peer = init_client(&endpoint).await;

    // provider-b declares no variable: refuse, remedy names the manual path.
    let no_var = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": "provider-b", "name": "spare" }),
        )
        .await
        .expect_err("no declared variable refuses");
    assert_eq!(no_var.code, 2005, "{no_var}");

    // An invalid name refuses before any directory exists.
    let bad = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": "provider-a", "name": "Not A Name" }),
        )
        .await
        .expect_err("invalid name refuses");
    assert_eq!(bad.code, 2005);
    assert!(
        !base.join("data/subscriptions/Not A Name").exists(),
        "a refusal creates nothing"
    );

    // A taken name refuses with the unlink remedy.
    peer.request(
        methods::SUBSCRIPTION_LINK,
        &json!({ "agent": "provider-a", "name": "work" }),
    )
    .await
    .expect("first link ok");
    let taken = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": "provider-a", "name": "work" }),
        )
        .await
        .expect_err("taken name refuses");
    assert_eq!(taken.code, 2005);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&base);
}

#[tokio::test]
async fn unlink_leaves_the_context_behind_and_manual_profiles_alone() {
    // Scenario: Desvincular deja el contexto intacto
    // Scenario: Lo escrito a mano gana y no se desvincula por superficie
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = fixture("unlink");
    let (endpoint, base, daemon) =
        spawn_daemon("unlink", &root.join(".meltemi/registry.toml")).await;

    // A hand-written profile in the daemon's own config.toml.
    let config_path = base.join("config").join("config.toml");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config.push_str(
        "\n[[fleet.profile]]\nname = \"manual\"\nagent = \"provider-a\"\nenv = { MELTEMI_MOCK_MARKER = 'C:/ctx/manual' }\n",
    );
    std::fs::write(&config_path, config).unwrap();

    let peer = init_client(&endpoint).await;

    // Link, then let "the provider" store a credential in the context dir —
    // the TEST writes it; the daemon never does.
    let linked = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": "provider-a", "name": "work" }),
        )
        .await
        .expect("link ok");
    let ctx = PathBuf::from(linked["gesture"]["value"].as_str().unwrap());
    std::fs::write(ctx.join("auth.json"), "{\"the-provider\": \"wrote-this\"}").unwrap();

    // Unlink: the profile goes, the directory and its contents stay, named.
    let unlinked = peer
        .request(methods::SUBSCRIPTION_UNLINK, &json!({ "name": "work" }))
        .await
        .expect("unlink ok");
    assert_eq!(unlinked["profile"], "work");
    assert_eq!(
        unlinked["contextDir"].as_str().unwrap(),
        ctx.display().to_string()
    );
    assert!(
        ctx.join("auth.json").is_file(),
        "credentials are not ours to destroy"
    );
    let fleet = peer
        .request(methods::FLEET_LIST, &json!({}))
        .await
        .expect("fleet ok");
    assert!(
        !fleet["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == "work"),
        "the profile row is gone"
    );

    // A hand-written profile is not the store's to remove.
    let manual = peer
        .request(methods::SUBSCRIPTION_UNLINK, &json!({ "name": "manual" }))
        .await
        .expect_err("hand-written refuses");
    assert_eq!(manual.code, 2005, "{manual}");

    // And an unknown name says so.
    let unknown = peer
        .request(methods::SUBSCRIPTION_UNLINK, &json!({ "name": "ghost" }))
        .await
        .expect_err("unknown refuses");
    assert_eq!(unknown.code, 2005);

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&base);
}
