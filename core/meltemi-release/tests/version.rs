// SPDX-License-Identifier: Apache-2.0

//! The version and its carriers (release-con-el-push D2), over fixture trees
//! and over this repository itself.

mod common;

use std::path::{Path, PathBuf};

use meltemi_release::{CARRIERS, Version, agreed_version, read_from_tree, set_version};

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

/// A tree holding the five carrier files, every one at `version`.
fn fixture(name: &str, version: &str) -> PathBuf {
    let root = common::scratch(name);
    common::write_carriers(&root, version);
    root
}

fn read(root: &Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).expect("readable")
}

#[test]
fn this_repository_s_carriers_agree() {
    // Scenario: Versión divergente entre portadores
    // The guardian over the real tree: the five files that carry the version
    // say the same thing, or this names the ones that do not.
    let version = agreed_version(read_from_tree(&repo_root()));
    assert!(version.is_ok(), "{}", version.unwrap_err());
}

#[test]
fn divergent_carriers_are_named_and_refused() {
    // Scenario: Versión divergente entre portadores
    let root = fixture("divergent", "0.1.1");
    let tauri = read(&root, "mobile/tauri.conf.json").replace("0.1.1", "0.1.0");
    std::fs::write(root.join("mobile/tauri.conf.json"), tauri).expect("write");

    let refused = agreed_version(read_from_tree(&root)).expect_err("divergence is refused");
    assert!(
        refused.contains("mobile/tauri.conf.json says 0.1.0"),
        "{refused}"
    );
    assert!(
        refused.contains("Cargo.toml [workspace.package] says 0.1.1"),
        "{refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn setting_a_version_writes_every_carrier_and_nothing_else() {
    let root = fixture("forward", "0.1.1");
    let before_dependency = read(&root, "desktop/ui/package-lock.json");
    let new = Version::parse("0.2.0").expect("valid");

    let changed = set_version(&root, new).expect("0.2.0 moves forward");
    assert_eq!(
        changed.len(),
        5,
        "five files, one of them carrying the version twice"
    );
    assert_eq!(agreed_version(read_from_tree(&root)), Ok(new));
    let lock = read(&root, "desktop/ui/package-lock.json");
    assert_eq!(
        lock.matches("\"0.2.0\"").count(),
        2,
        "both entries of the lockfile move"
    );
    assert!(
        lock.contains("\"9.9.9\""),
        "a dependency's version is not a carrier"
    );
    assert_eq!(
        before_dependency.replace("\"0.1.1\"", "\"0.2.0\""),
        lock,
        "only the two version lines changed"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_version_that_does_not_move_forward_is_refused_and_writes_nothing() {
    let root = fixture("backward", "0.2.0");
    for not_forward in ["0.2.0", "0.1.9"] {
        let refused = set_version(&root, Version::parse(not_forward).expect("valid"))
            .expect_err("a version only moves forward");
        assert!(
            refused.contains("not greater than the current version 0.2.0"),
            "{refused}"
        );
    }
    for carrier in CARRIERS {
        assert!(
            read(&root, carrier.path).contains("\"0.2.0\""),
            "{} untouched",
            carrier.label()
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn drifted_carriers_are_brought_back_together_but_never_backwards() {
    let root = fixture("drifted", "0.1.1");
    let desktop = read(&root, "desktop/tauri.conf.json").replace("0.1.1", "0.3.0");
    std::fs::write(root.join("desktop/tauri.conf.json"), desktop).expect("write");

    let refused =
        set_version(&root, Version::parse("0.2.0").expect("valid")).expect_err("behind 0.3.0");
    assert!(refused.contains("current version 0.3.0"), "{refused}");
    set_version(&root, Version::parse("0.3.1").expect("valid")).expect("ahead of every carrier");
    assert_eq!(
        agreed_version(read_from_tree(&root))
            .map(|v| v.to_string())
            .as_deref(),
        Ok("0.3.1")
    );
    let _ = std::fs::remove_dir_all(&root);
}
