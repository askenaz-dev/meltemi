// SPDX-License-Identifier: Apache-2.0

//! The v0.1 acceptance run (hito-v01-aceptacion): an executable milestone
//! script that drives the full cycle over a fixture through the product's own
//! surfaces, and shows two distinct-profile agents working in parallel in
//! separate worktrees with per-task traceability. Runs with simulated agents
//! and no network (constitution §"tests e2e"). Requires `git`; skips if absent.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::rpc::Peer;
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn mock_agent_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    })
}

const LIVING: &str = "# widget-cap Specification\n\n## Purpose\nWidgets.\n\n## Requirements\n### Requirement: Existing widget\nIt exists.\n\n#### Scenario: Existing widget renders\n- **WHEN** shown\n- **THEN** it renders\n";

const DELTA: &str = "## ADDED Requirements\n\n### Requirement: Dark-mode toggle\nA toggle switches the theme.\n\n#### Scenario: Toggle flips the theme\n- **WHEN** the toggle is activated\n- **THEN** the theme flips\n\n#### Scenario: Toggle persists\n- **WHEN** the app restarts\n- **THEN** the last theme is remembered\n";

/// A git fixture: a configured mock agent, an allow rule, a living spec, a
/// seeded change (proposal + delta + two tasks), and a test naming its scenarios.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-hito-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/specs/widget-cap")).unwrap();
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode/specs/widget-cap")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();

    let mock = mock_agent_bin();
    assert!(mock.exists(), "run `cargo test` at the workspace root");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = ['{}']\n", mock.display()),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(root.join(".meltemi/specs/widget-cap/spec.md"), LIVING).unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/proposal.md"),
        "## Why\nUsers want a dark mode.\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/specs/widget-cap/spec.md"),
        DELTA,
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/tasks.md"),
        "## 1. Build\n\n- [ ] 1.1 Add the toggle control\n- [ ] 1.2 Persist the choice\n",
    )
    .unwrap();
    // A test naming the change's scenarios (verify links them).
    std::fs::write(
        root.join("tests/acceptance_link.rs"),
        "// Scenario: Toggle flips the theme\n// Scenario: Toggle persists\nfn t() {}\n",
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
        format!(r"\\.\pipe\meltemid-e2e-hito-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-e2e-hito-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-hitod-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data_dir = base.join("data");
    let config_dir = base.join("config");
    std::fs::create_dir_all(&data_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::new(data_dir, config_dir, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
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
                name: "e2e-hito-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn the_milestone_cycle_reaches_implemented_verified_archived() {
    // Scenario: Ciclo completo en terminal — the fixture ends implemented,
    // verified, and archived, every step via the product surfaces.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // SAFETY: single test process.
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let root = fixture("cycle");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("cycle").await;
    let peer = init_client(&endpoint).await;

    // 1. propose — the product surface scaffolds a change and delegates it.
    let proposed = peer
        .request(
            methods::PROPOSE,
            &json!({ "idea": "add a status badge to the header", "projectRoot": root_str }),
        )
        .await
        .expect("propose ok");
    assert!(
        proposed["proposalPath"]
            .as_str()
            .unwrap()
            .contains(".meltemi"),
        "propose scaffolds under .meltemi: {proposed:#}"
    );

    // 2. review — the seeded change's deltas as a checklist.
    let review = peer
        .request(
            methods::SDD_REVIEW,
            &json!({ "projectRoot": root_str, "changeName": "dark-mode" }),
        )
        .await
        .expect("review ok");
    assert!(
        review["items"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "review presents the checklist: {review:#}"
    );

    // 3. implement — deploy the agent over tasks.md (real mock turns).
    let implemented = peer
        .request(
            methods::SDD_IMPLEMENT,
            &json!({ "projectRoot": root_str, "change": "dark-mode", "agent": "claude" }),
        )
        .await
        .expect("implement ok");
    assert_eq!(
        implemented["committed"].as_array().unwrap().len(),
        2,
        "both tasks committed: {implemented:#}"
    );

    // 4. verify — the scenarios link to the acceptance test.
    let verified = peer
        .request(
            methods::SDD_VERIFY,
            &json!({ "projectRoot": root_str, "change": "dark-mode" }),
        )
        .await
        .expect("verify ok");
    assert_eq!(
        verified["complete"], true,
        "all scenarios verified: {verified:#}"
    );

    // 5. archive — fold the delta into the living truth.
    let archived = peer
        .request(
            methods::SDD_ARCHIVE,
            &json!({ "projectRoot": root_str, "change": "dark-mode" }),
        )
        .await
        .expect("archive ok");
    assert!(
        archived["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "widget-cap"),
        "archive folds widget-cap: {archived:#}"
    );

    // End state: the feature is in the living truth and the change is archived.
    let living = std::fs::read_to_string(root.join(".meltemi/specs/widget-cap/spec.md")).unwrap();
    assert!(
        living.contains("### Requirement: Dark-mode toggle"),
        "feature landed"
    );
    assert!(
        !root.join(".meltemi/changes/dark-mode").exists(),
        "the change is archived out of active"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn two_distinct_profiles_work_in_parallel_worktrees_with_traceability() {
    // Scenario: Paralelismo real de dos agentes — two profiles work in separate
    // worktrees from the same base, and their commits keep per-task traceability.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    unsafe {
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    let root = fixture("parallel");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("parallel").await;
    let peer = init_client(&endpoint).await;

    // Assign task 1.1 to two distinct-profile agents: a race on one task.
    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [{
                    "change": "dark-mode", "task": "1.1",
                    "agents": ["fast", "thorough"], "files": ["toggle.rs"],
                }],
            }),
        )
        .await
        .expect("assign ok");
    let wts = assign["worktrees"].as_array().unwrap();
    assert_eq!(wts.len(), 2, "one worktree per profile");
    let base = assign["baseRev"].as_str().unwrap();
    for w in wts {
        assert_eq!(w["competitor"], true, "labeled competitors");
        assert_eq!(w["baseRev"], base, "both from the same fixed base");
    }
    assert_ne!(wts[0]["branch"], wts[1]["branch"], "distinct branches");

    // Each profile produces distinct work; each is committed with traceability.
    let req = json!([{ "capability": "widget-cap", "requirement": "Dark-mode toggle" }]);
    for w in wts {
        let path = PathBuf::from(w["path"].as_str().unwrap());
        let agent = w["agent"].as_str().unwrap();
        std::fs::write(
            path.join("toggle.rs"),
            format!("// {agent} profile implementation\n"),
        )
        .unwrap();
        let committed = peer
            .request(
                methods::COMMIT_TASK,
                &json!({
                    "projectRoot": root_str, "change": "dark-mode", "task": "1.1", "agent": agent,
                    "title": format!("Add the toggle ({agent})"),
                    "requirements": req, "declaredFiles": ["toggle.rs"], "confirm": true,
                }),
            )
            .await
            .expect("commit ok");
        assert_eq!(committed["committed"], true, "{agent} committed");

        // The commit carries per-task and per-requirement traceability.
        let body = git(&path, &["log", "-1", "--pretty=%B"]);
        assert!(
            body.contains("Meltemi-Task: dark-mode/1.1"),
            "task trailer: {body}"
        );
        assert!(
            body.contains("Meltemi-Req: widget-cap/dark-mode-toggle"),
            "req trailer"
        );
        assert!(
            !body.to_ascii_lowercase().contains("co-authored-by"),
            "no co-authorship"
        );
    }

    // The two results are comparable side by side against the common base.
    let diff = peer
        .request(
            methods::WORKTREE_DIFF,
            &json!({ "projectRoot": root_str, "change": "dark-mode", "task": "1.1" }),
        )
        .await
        .expect("diff ok");
    assert_eq!(
        diff["competitors"].as_array().unwrap().len(),
        2,
        "two competitor diffs"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
