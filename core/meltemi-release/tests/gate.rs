// SPDX-License-Identifier: Apache-2.0

//! The release gate (release-con-el-push D1, D5) over real git repositories:
//! one temporary repository per scenario, with `main` standing in for
//! `origin/main`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use meltemi_release::gate;

/// Runs git in `repo` with an identity of its own and no signing, so the test
/// does not depend on the machine's configuration.
fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Release Gate Test",
            "-c",
            "user.email=gate@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A repository on `main` whose first commit carries `version`.
fn repository(name: &str, version: &str) -> PathBuf {
    let root = common::scratch(name);
    git(&root, &["init", "-q", "-b", "main"]);
    commit_version(&root, version, "first");
    root
}

/// Writes `version` into every carrier and commits it; returns the commit.
fn commit_version(repo: &Path, version: &str, message: &str) -> String {
    common::write_carriers(repo, version);
    std::fs::write(repo.join("NOTES"), message).expect("a change to commit");
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

#[test]
fn a_new_version_on_main_opens_the_release_path() {
    // Scenario: Versión nueva en la rama principal publica
    let repo = repository("new-version", "0.1.0");
    git(&repo, &["tag", "v0.1.0"]);
    let commit = commit_version(&repo, "0.2.0", "bump");

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(decision.release, "{}", decision.reason);
    assert_eq!(decision.version.tag(), "v0.2.0");
    assert!(
        decision.reason.contains("newer than v0.1.0"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn the_first_version_ever_opens_the_release_path() {
    // Scenario: Versión nueva en la rama principal publica
    let repo = repository("first-ever", "0.1.0");
    let commit = git(&repo, &["rev-parse", "HEAD"]);

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(decision.release, "{}", decision.reason);
    assert!(
        decision.reason.contains("first version tag"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn a_push_without_a_new_version_does_not_publish() {
    // Scenario: Push sin versión nueva no publica
    let repo = repository("no-bump", "0.1.1");
    git(&repo, &["tag", "v0.1.1"]);
    let commit = commit_version(&repo, "0.1.1", "a fix, no new version");

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(!decision.release, "{}", decision.reason);
    assert!(
        decision.reason.contains("v0.1.1 already exists"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn a_version_already_tagged_elsewhere_is_not_published_again() {
    // Scenario: Versión ya publicada no se republica
    // The tag sits on an older commit; a later commit that still says 0.2.0
    // must not publish 0.2.0 a second time from somewhere else.
    let repo = repository("already-tagged", "0.2.0");
    git(&repo, &["tag", "v0.2.0"]);
    let commit = commit_version(&repo, "0.2.0", "later commit, same version");

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(!decision.release, "{}", decision.reason);
    assert!(
        decision.reason.contains("v0.2.0 already exists"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn a_version_behind_the_newest_tag_is_not_published() {
    // Scenario: Versión menor no se publica
    let repo = repository("behind", "0.2.0");
    git(&repo, &["tag", "v0.2.0"]);
    let commit = commit_version(&repo, "0.1.5", "a version that goes back");

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(!decision.release, "{}", decision.reason);
    assert!(
        decision
            .reason
            .contains("not greater than the newest version tag v0.2.0"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn a_commit_off_main_is_not_published() {
    // Scenario: Commit fuera de la rama principal no se publica
    let repo = repository("off-main", "0.1.0");
    git(&repo, &["tag", "v0.1.0"]);
    git(&repo, &["switch", "-q", "-c", "side"]);
    let commit = commit_version(&repo, "0.2.0", "a new version on a side branch");
    git(&repo, &["switch", "-q", "main"]);

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(!decision.release, "{}", decision.reason);
    assert!(
        decision.reason.contains("is not reachable from main"),
        "{}",
        decision.reason
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn divergent_carriers_in_the_commit_fail_the_gate_loudly() {
    // Scenario: Versión divergente entre portadores
    let repo = repository("divergent", "0.1.0");
    common::write_carriers(&repo, "0.2.0");
    let tauri = std::fs::read_to_string(repo.join("mobile/tauri.conf.json"))
        .expect("readable")
        .replace("0.2.0", "0.1.0");
    std::fs::write(repo.join("mobile/tauri.conf.json"), tauri).expect("write");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "forgot one carrier"]);
    let commit = git(&repo, &["rev-parse", "HEAD"]);

    let refused = gate(&repo, &commit, "main").expect_err("not a quiet no: an error");
    assert!(
        refused.contains("mobile/tauri.conf.json says 0.1.0"),
        "{refused}"
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn the_gate_judges_the_commit_and_not_the_working_tree() {
    // A working tree edited after the commit must not change the decision
    // about the commit.
    let repo = repository("commit-not-tree", "0.1.0");
    git(&repo, &["tag", "v0.1.0"]);
    let commit = commit_version(&repo, "0.1.0", "no new version");
    common::write_carriers(&repo, "9.0.0");

    let decision = gate(&repo, &commit, "main").expect("the gate answers");
    assert!(!decision.release, "{}", decision.reason);
    assert_eq!(decision.version.to_string(), "0.1.0");
    let _ = std::fs::remove_dir_all(&repo);
}
