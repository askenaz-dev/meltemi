// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests of the change workshop (rama-por-change task 4.1),
//! driving an ephemeral daemon against temporary **git** fixture repos:
//!
//! - the workshop is created from the default branch tip (detected — one
//!   fixture's default branch is deliberately not `main`) and re-encountered
//!   on every later ask;
//! - a chosen branch is consent, a unique workshop never collides, a foreign
//!   homonymous branch is refused untouched;
//! - landing previews without `confirm`, merges `--no-ff` with it, aborts on
//!   conflict leaving the default branch intact, and refuses a dirty workshop;
//! - retiring a workshop with unlanded commits demands confirmation that says
//!   how many, and never deletes the branch.
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
/// `default_branch` is set via `symbolic-ref` before the first commit, so the
/// fixture works the same on every git the toolchain supports.
fn git_fixture(tag: &str, default_branch: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-e2e-ws-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    git(&root, &["init", "-q"]);
    git(
        &root,
        &[
            "symbolic-ref",
            "HEAD",
            &format!("refs/heads/{default_branch}"),
        ],
    );
    git(&root, &["config", "user.email", "e2e@meltemi.test"]);
    git(&root, &["config", "user.name", "Meltemi E2E"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("shared.txt"), "base\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    root
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-ws-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-ws-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let base = std::env::temp_dir().join(format!("meltemi-e2e-wsd-{}-{tag}", std::process::id()));
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
                name: "e2e-workspace-client".into(),
                version: "0.0.0".into(),
            },
        },
    )
    .await
    .expect("initialize");
    peer
}

/// Commits one edit inside a workshop so its branch moves ahead of the base.
fn commit_in(workshop: &Path, file: &str, contents: &str, message: &str) {
    std::fs::write(workshop.join(file), contents).unwrap();
    git(workshop, &["add", "-A"]);
    git(workshop, &["commit", "-q", "-m", message]);
}

#[tokio::test]
async fn the_workshop_is_created_from_the_default_branch_and_reencountered() {
    // Scenario: El primer taller se crea desde la rama por defecto
    // Scenario: Pedirlo de nuevo reencuentra, no falla
    // Scenario: El taller no ensucia el estado del árbol principal
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // The default branch is deliberately NOT `main`: detection, not assumption.
    let root = git_fixture("basic", "trunk");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("basic").await;
    let peer = init_client(&endpoint).await;

    let first = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("workspace ok");
    assert_eq!(first["branch"], "mi-change", "the branch is the bare name");
    assert_eq!(first["baseBranch"], "trunk", "detected, never assumed");
    assert_eq!(first["reencountered"], false, "the first ask creates");
    let path = PathBuf::from(first["path"].as_str().expect("path"));
    assert!(path.is_dir(), "the workshop exists on disk");
    assert_eq!(
        git(&path, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        "mi-change",
        "the workshop stands on its own branch"
    );

    let again = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("second ask ok");
    assert_eq!(again["reencountered"], true, "asking again re-encounters");
    assert_eq!(
        again["path"], first["path"],
        "the same workshop, not a twin"
    );

    // The managed root stays out of the main tree's status, by the local
    // route: `info/exclude` gains the entry, the versioned `.gitignore` does
    // not exist and is not invented.
    let status = git(&root, &["status", "--porcelain"]);
    assert!(
        !status.contains(".meltemi/worktrees"),
        "the managed root is excluded from status: {status}"
    );
    assert!(
        !root.join(".gitignore").exists(),
        "no versioned .gitignore was invented"
    );
    let exclude = std::fs::read_to_string(root.join(".git/info/exclude")).unwrap_or_default();
    assert!(
        exclude.lines().any(|l| l.trim() == "/.meltemi/worktrees/"),
        "info/exclude carries the entry: {exclude}"
    );

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn a_chosen_branch_is_consent_and_a_unique_workshop_never_collides() {
    // Scenario: El taller sobre una rama elegida
    // Scenario: Un taller único no colisiona con nadie
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("chosen", "main");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("chosen").await;
    let peer = init_client(&endpoint).await;

    // A branch the daemon did not create, named explicitly: consent.
    git(&root, &["branch", "rama-del-humano"]);
    let chosen = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({
                "projectRoot": root_str,
                "change": "mi-change",
                "branch": "rama-del-humano",
            }),
        )
        .await
        .expect("naming the branch is consent");
    assert_eq!(chosen["branch"], "rama-del-humano");
    assert!(PathBuf::from(chosen["path"].as_str().unwrap()).is_dir());

    // A named branch that does not exist yet is minted from the base tip.
    let minted = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({
                "projectRoot": root_str,
                "change": "mi-change",
                "branch": "rama-nueva",
            }),
        )
        .await
        .expect("a missing named branch is created");
    assert_eq!(minted["branch"], "rama-nueva");

    // Two unique workshops for the same change: distinct, never re-encounters.
    let one = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change", "unique": true }),
        )
        .await
        .expect("first unique ok");
    let two = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change", "unique": true }),
        )
        .await
        .expect("second unique ok");
    for unique in [&one, &two] {
        assert_eq!(
            unique["reencountered"], false,
            "a unique workshop is always a creation: {unique:#}"
        );
        assert!(
            unique["branch"].as_str().unwrap().starts_with("mi-change-"),
            "the suffix rides the change name: {unique:#}"
        );
    }
    assert_ne!(one["branch"], two["branch"], "distinct branches");
    assert_ne!(one["path"], two["path"], "distinct workshops");

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn a_foreign_homonymous_branch_is_refused_untouched() {
    // Scenario: La rama ajena se rehúsa sin tocarse
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("foreign", "main");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("foreign").await;
    let peer = init_client(&endpoint).await;

    // A branch with the change's name that Meltemi did not create.
    git(&root, &["branch", "mi-change"]);
    let before = git(&root, &["rev-parse", "refs/heads/mi-change"]);

    let refused = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect_err("the implicit path over a foreign branch must refuse");
    assert_eq!(refused.code, error_codes::WORKTREE_REFUSED, "{refused}");

    let after = git(&root, &["rev-parse", "refs/heads/mi-change"]);
    assert_eq!(before, after, "the foreign branch was not touched");
    assert!(
        !root.join(".meltemi/worktrees/mi-change").exists(),
        "no workshop was created for the refused ask"
    );

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn landing_previews_without_confirm_and_merges_with_it() {
    // Scenario: Sin confirmación, la previsualización
    // Scenario: Con confirmación, el aterrizaje limpio
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("land", "main");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("land").await;
    let peer = init_client(&endpoint).await;

    let workspace = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("workspace ok");
    let workshop = PathBuf::from(workspace["path"].as_str().unwrap());
    commit_in(&workshop, "feature.txt", "the work\n", "add the feature");

    let tip_before = git(&root, &["rev-parse", "HEAD"]);
    let preview = peer
        .request(
            methods::CHANGE_LAND,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("preview ok");
    assert_eq!(preview["landed"], false, "no confirm, no merge");
    assert!(preview.get("mergeSha").is_none(), "a preview has no merge");
    assert_eq!(
        preview["commits"].as_array().unwrap().len(),
        1,
        "the preview says which commits would land: {preview:#}"
    );
    assert_eq!(
        preview["files"],
        json!(["feature.txt"]),
        "and which files they touch"
    );
    assert_eq!(
        git(&root, &["rev-parse", "HEAD"]).trim(),
        tip_before.trim(),
        "the preview merged nothing"
    );

    let landed = peer
        .request(
            methods::CHANGE_LAND,
            &json!({ "projectRoot": root_str, "change": "mi-change", "confirm": true }),
        )
        .await
        .expect("confirmed landing ok");
    assert_eq!(landed["landed"], true);
    assert!(landed["mergeSha"].is_string(), "the merge commit is named");
    // --no-ff: the change's shape stays visible — the tip is a merge commit
    // with two parents, not a fast-forwarded line.
    let parents = git(&root, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "a merge commit with two parents: {parents}"
    );
    assert!(
        root.join("feature.txt").exists(),
        "the landed work is on the default branch"
    );

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn conflicts_abort_cleanly_and_a_dirty_workshop_never_lands() {
    // Scenario: El conflicto se rehúsa y no deja el árbol a medias
    // Scenario: El taller sucio no aterriza
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("conflict", "main");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("conflict").await;
    let peer = init_client(&endpoint).await;

    let workspace = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("workspace ok");
    let workshop = PathBuf::from(workspace["path"].as_str().unwrap());

    // First: a dirty workshop refuses before anything else is considered.
    std::fs::write(workshop.join("shared.txt"), "workshop side\n").unwrap();
    let dirty = peer
        .request(
            methods::CHANGE_LAND,
            &json!({ "projectRoot": root_str, "change": "mi-change", "confirm": true }),
        )
        .await
        .expect_err("a dirty workshop must not land");
    assert_eq!(dirty.code, error_codes::WORKTREE_REFUSED, "{dirty}");

    // Commit the workshop side, then move the default branch incompatibly.
    git(&workshop, &["add", "-A"]);
    git(&workshop, &["commit", "-q", "-m", "workshop side"]);
    std::fs::write(root.join("shared.txt"), "default side\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "default side"]);
    let tip_before = git(&root, &["rev-parse", "HEAD"]);

    let refused = peer
        .request(
            methods::CHANGE_LAND,
            &json!({ "projectRoot": root_str, "change": "mi-change", "confirm": true }),
        )
        .await
        .expect_err("a conflicted merge must refuse");
    assert_eq!(refused.code, error_codes::WORKTREE_REFUSED, "{refused}");

    // The abort left no half-applied merge: no MERGE_HEAD, clean status, the
    // default branch exactly where it stood.
    assert!(
        !root.join(".git/MERGE_HEAD").exists(),
        "the merge was aborted, not abandoned midway"
    );
    assert_eq!(
        git(&root, &["status", "--porcelain"]).trim(),
        "",
        "the main tree is clean after the abort"
    );
    assert_eq!(
        git(&root, &["rev-parse", "HEAD"]).trim(),
        tip_before.trim(),
        "the default branch is intact"
    );

    peer.close();
    daemon.abort();
}

#[tokio::test]
async fn retiring_an_unlanded_workshop_demands_confirmation_and_keeps_the_branch() {
    // Scenario: Retirar con commits sin aterrizar exige confirmación
    // Scenario: Retirar el taller conserva la rama
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = git_fixture("retire", "main");
    let root_str = root.display().to_string();
    let (endpoint, daemon) = spawn_daemon("retire").await;
    let peer = init_client(&endpoint).await;

    let workspace = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("workspace ok");
    let workshop = PathBuf::from(workspace["path"].as_str().unwrap());
    commit_in(&workshop, "unlanded.txt", "not yet\n", "unlanded work");
    let path_str = workshop.display().to_string();

    let refused = peer
        .request(
            methods::WORKTREE_REMOVE,
            &json!({ "projectRoot": root_str, "path": path_str }),
        )
        .await
        .expect_err("unlanded commits must demand confirmation");
    assert_eq!(refused.code, error_codes::WORKTREE_REFUSED, "{refused}");
    assert!(
        refused.to_string().contains('1'),
        "the refusal says how many commits would remain: {refused}"
    );

    let removed = peer
        .request(
            methods::WORKTREE_REMOVE,
            &json!({ "projectRoot": root_str, "path": path_str, "force": true }),
        )
        .await
        .expect("confirmed retirement ok");
    assert_eq!(removed["removed"], true);
    assert!(!workshop.exists(), "the worktree is gone");
    // The branch stays: retiring the workshop retires the worktree only.
    git(&root, &["rev-parse", "--verify", "refs/heads/mi-change"]);

    // And asking for the workshop again re-mounts the surviving branch — the
    // daemon minted it, so it is not foreign.
    let remounted = peer
        .request(
            methods::CHANGE_WORKSPACE,
            &json!({ "projectRoot": root_str, "change": "mi-change" }),
        )
        .await
        .expect("re-mounting the surviving branch is not touching the foreign");
    assert_eq!(remounted["reencountered"], false, "a fresh mount, declared");
    assert!(
        PathBuf::from(remounted["path"].as_str().unwrap())
            .join("unlanded.txt")
            .exists(),
        "the unlanded work is right there on the branch"
    );

    peer.close();
    daemon.abort();
}
