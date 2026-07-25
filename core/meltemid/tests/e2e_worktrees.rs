// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of worktree orchestration (orquestacion-worktrees task
//! 5.2), driving an ephemeral daemon against temporary **git** fixture repos:
//!
//! - a race assigns one task to two agents in isolated worktrees from the same
//!   base, labels them competitors, and yields comparable diffs;
//! - parallel vs serialized batching is planned and reported;
//! - cleanup is safe: dirty needs confirmation, foreign worktrees are refused;
//! - assisted merge applies a file only with explicit confirmation;
//! - a non-git directory degrades honestly (refused with a remedy).
//!
//! Runs against temporary fixtures, never this repo (constitution
//! §"tests e2e"). Requires `git` on PATH; skips with a note if absent.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tokio::sync::mpsc;

use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, error_codes, methods};
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

/// Runs git in `dir`, asserting success (test-only helper).
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

/// Creates a temp git repo with one committed file and a `.meltemi` dir.
fn git_fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-wt-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    git(&root, &["init", "-q"]);
    // Local identity so commits work in a clean CI environment.
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    // Deterministic bytes across platforms: no CRLF translation on checkout.
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("shared.txt"), "base\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-wt-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-wt-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-wtd-{}-{tag}", std::process::id()));
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
                name: "e2e-worktrees-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

#[tokio::test]
async fn a_race_creates_isolated_worktrees_and_comparable_diffs() {
    // Scenarios: Carrera etiquetada; ambos aislados desde la misma base; cada
    // resultado disponible como diff contra la base común.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("race");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("race").await;
    let peer = init_client(&endpoint).await;

    // Assign one task to two agents: a race.
    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [{
                    "change": "add-thing",
                    "task": "1.1",
                    "agents": ["claude", "gemini"],
                    "files": ["shared.txt"],
                }],
            }),
        )
        .await
        .expect("assign ok");

    let worktrees = assign["worktrees"].as_array().expect("worktrees");
    assert_eq!(worktrees.len(), 2, "one worktree per agent: {assign:#}");
    let base_rev = assign["baseRev"].as_str().expect("baseRev");
    for w in worktrees {
        assert_eq!(w["competitor"], true, "racers are competitors");
        assert_eq!(w["baseRev"], base_rev, "both from the same fixed base");
        assert!(
            Path::new(w["path"].as_str().unwrap()).is_dir(),
            "worktree exists on disk: {w:#}"
        );
    }
    // Distinct paths and branches per agent.
    assert_ne!(worktrees[0]["path"], worktrees[1]["path"]);
    assert_ne!(worktrees[0]["branch"], worktrees[1]["branch"]);

    // The managed list reports both (own registry).
    let list = peer
        .request(methods::WORKTREE_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("list ok");
    assert_eq!(list["worktrees"].as_array().unwrap().len(), 2);

    // Simulate divergent agent work: each competitor edits+commits the file.
    for w in worktrees {
        let path = PathBuf::from(w["path"].as_str().unwrap());
        let agent = w["agent"].as_str().unwrap();
        std::fs::write(path.join("shared.txt"), format!("{agent} was here\n")).unwrap();
        git(&path, &["add", "-A"]);
        git(&path, &["commit", "-q", "-m", "work"]);
    }

    // Each result is a diff against the common base — comparable side by side.
    let diff = peer
        .request(
            methods::WORKTREE_DIFF,
            &json!({ "projectRoot": root_str, "change": "add-thing", "task": "1.1" }),
        )
        .await
        .expect("diff ok");
    let competitors = diff["competitors"].as_array().expect("competitors");
    assert_eq!(competitors.len(), 2);
    for c in competitors {
        assert!(
            c["changedFiles"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "shared.txt"),
            "competitor changed shared.txt: {c:#}"
        );
        assert!(
            c["diff"].as_str().unwrap().contains("was here"),
            "competitor diff carries its change: {c:#}"
        );
    }

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn parallel_and_serialized_batches_are_reported() {
    // Scenarios: Paralelo sin solapamiento; Solapamiento serializado con motivo.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("batch");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("batch").await;
    let peer = init_client(&endpoint).await;

    // Two tasks sharing a file must serialize; the reason names the file.
    let overlap = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [
                    { "change": "c", "task": "1.1", "agents": ["claude"], "files": ["shared.txt"] },
                    { "change": "c", "task": "1.2", "agents": ["gemini"], "files": ["shared.txt"] },
                ],
            }),
        )
        .await
        .expect("assign ok");
    let batches = overlap["batches"].as_array().expect("batches");
    assert_eq!(batches.len(), 2, "overlap serializes into two batches");
    assert!(
        batches[1]["serializedReason"]
            .as_str()
            .is_some_and(|r| r.contains("shared.txt")),
        "the serialized batch names the shared file: {overlap:#}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn cleanup_is_safe_and_never_touches_foreign_worktrees() {
    // Scenario: Limpieza segura
    // dirty exige confirmación; ajeno jamás tocado.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("clean");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("clean").await;
    let peer = init_client(&endpoint).await;

    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [{ "change": "c", "task": "1.1", "agents": ["claude"], "files": [] }],
            }),
        )
        .await
        .expect("assign ok");
    let wt = assign["worktrees"][0]["path"].as_str().unwrap().to_string();

    // Dirty it, then a plain removal is refused (needs confirmation).
    std::fs::write(Path::new(&wt).join("dirty.txt"), "uncommitted\n").unwrap();
    let refused = peer
        .request(
            methods::WORKTREE_REMOVE,
            &json!({ "projectRoot": root_str, "path": wt }),
        )
        .await
        .expect_err("dirty removal must be refused");
    assert_eq!(refused.code, error_codes::WORKTREE_REFUSED, "{refused}");
    assert!(Path::new(&wt).is_dir(), "refused removal left it in place");

    // With force it is removed.
    let removed = peer
        .request(
            methods::WORKTREE_REMOVE,
            &json!({ "projectRoot": root_str, "path": wt, "force": true }),
        )
        .await
        .expect("forced removal ok");
    assert_eq!(removed["removed"], true);
    assert!(!Path::new(&wt).is_dir(), "worktree gone after force");

    // A foreign path (never created by the daemon) is refused and untouched.
    let foreign = std::env::temp_dir().join(format!("foreign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&foreign);
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("keep.txt"), "mine\n").unwrap();
    let foreign_refused = peer
        .request(
            methods::WORKTREE_REMOVE,
            &json!({ "projectRoot": root_str, "path": foreign.display().to_string(), "force": true }),
        )
        .await
        .expect_err("a foreign worktree must be refused");
    assert_eq!(foreign_refused.code, error_codes::WORKTREE_REFUSED);
    assert!(
        foreign.join("keep.txt").is_file(),
        "a foreign directory is never touched"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&foreign);
}

#[tokio::test]
async fn assisted_merge_applies_a_file_only_with_confirmation() {
    // Scenario: Elección y aplicación selectiva
    // cada aplicación explícita.
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("merge");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("merge").await;
    let peer = init_client(&endpoint).await;

    let assign = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [{
                    "change": "c", "task": "1.1",
                    "agents": ["claude", "gemini"], "files": ["shared.txt"],
                }],
            }),
        )
        .await
        .expect("assign ok");
    let wts = assign["worktrees"].as_array().unwrap();
    let claude = wts.iter().find(|w| w["agent"] == "claude").unwrap()["path"]
        .as_str()
        .unwrap()
        .to_string();
    let gemini = wts.iter().find(|w| w["agent"] == "gemini").unwrap()["path"]
        .as_str()
        .unwrap()
        .to_string();

    // Gemini writes a distinctive version of the file.
    std::fs::write(Path::new(&gemini).join("shared.txt"), "gemini wins\n").unwrap();

    // Without confirmation, nothing is applied (human decision required).
    let refused = peer
        .request(
            methods::WORKTREE_MERGE_FILE,
            &json!({
                "projectRoot": root_str, "target": claude, "source": gemini,
                "file": "shared.txt",
            }),
        )
        .await
        .expect_err("apply without confirm must be refused");
    assert_eq!(refused.code, error_codes::WORKTREE_REFUSED, "{refused}");
    assert_eq!(
        std::fs::read_to_string(Path::new(&claude).join("shared.txt")).unwrap(),
        "base\n",
        "nothing applied without confirmation"
    );

    // With explicit confirmation, gemini's file lands in claude's worktree.
    let applied = peer
        .request(
            methods::WORKTREE_MERGE_FILE,
            &json!({
                "projectRoot": root_str, "target": claude, "source": gemini,
                "file": "shared.txt", "confirm": true,
            }),
        )
        .await
        .expect("apply with confirm ok");
    assert_eq!(applied["applied"], true);
    assert_eq!(
        std::fs::read_to_string(Path::new(&claude).join("shared.txt")).unwrap(),
        "gemini wins\n",
        "the chosen file was applied into the base worktree"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_non_git_directory_degrades_honestly() {
    // Scenario: Proyecto sin git
    // rehusar con diagnóstico y remedio.
    let root = std::env::temp_dir().join(format!("meltemi-e2e-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("nogit").await;
    let peer = init_client(&endpoint).await;

    let refused = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &json!({
                "projectRoot": root_str,
                "tasks": [{ "change": "c", "task": "1.1", "agents": ["claude"], "files": [] }],
            }),
        )
        .await
        .expect_err("assign on a non-git dir must be refused");
    assert_eq!(
        refused.code,
        error_codes::WORKTREE_UNAVAILABLE,
        "expected worktree_unavailable, got: {refused}"
    );
    // The refusal carries an actionable remedy (git init).
    let remedy = refused.data.as_ref().and_then(|d| d["remedy"].as_str());
    assert!(
        remedy.is_some_and(|r| r.contains("git init")),
        "the refusal offers a remedy: {refused}"
    );

    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
