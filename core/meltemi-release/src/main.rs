// SPDX-License-Identifier: Apache-2.0

//! `meltemi-release`: the release tooling of this repository.
//!
//! ```text
//! meltemi-release version [--root <dir>]
//! meltemi-release set-version <x.y.z> [--root <dir>]
//! meltemi-release gate --commit <sha> [--main <ref>] [--root <dir>]
//! meltemi-release notes <x.y.z> [--root <dir>]
//! ```
//!
//! `version` prints the version every carrier agrees on. `set-version` is the
//! maintainer's gesture (release-con-el-push D2): it writes the new version
//! into every carrier, refreshes the workspace's entries in `Cargo.lock`, and
//! refuses a version that does not move forward. Since this change a commit of
//! a new version on `main` is a publication, so the tool is run by a person,
//! deliberately, and never by an agent unless the maintainer says so in words
//! (AGENTS.md).
//!
//! `gate` is the first job of the release pipeline (release-con-el-push D1):
//! it prints `release`, `version`, `tag` and `reason` as `key=value` lines,
//! ready to append to `$GITHUB_OUTPUT`. A "no" exits 0; carriers that disagree
//! exit 1, so the run turns red instead of quietly publishing nothing.
//!
//! `notes` prints the section of `CHANGELOG.md` for a version, which becomes
//! the release's notes (release-con-el-push D4).
//!
//! Exit codes: 0 done, 1 refused or failed, 2 usage, 3 no notes for that
//! version (the pipeline then generates them).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use meltemi_release::{
    CARRIERS, Version, agreed_version, changelog_section, gate, lock_moved_only_the_workspace,
    read_from_tree, set_version,
};

const USAGE: &str = "usage: meltemi-release version [--root <dir>]\n       meltemi-release set-version <x.y.z> [--root <dir>]\n       meltemi-release gate --commit <sha> [--main <ref>] [--root <dir>]\n       meltemi-release notes <x.y.z> [--root <dir>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (root, rest) = match split_root(&args) {
        Ok(parts) => parts,
        Err(message) => return usage(&message),
    };
    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    let outcome = match rest.as_slice() {
        ["version"] => agreed_version(read_from_tree(&root)).map(|version| println!("{version}")),
        ["set-version", new] => match Version::parse(new) {
            Some(new) => set(&root, new),
            None => return usage(&format!("`{new}` is not a version of the form x.y.z")),
        },
        ["gate", "--commit", commit] => decide(&root, commit, "origin/main"),
        ["gate", "--commit", commit, "--main", main]
        | ["gate", "--main", main, "--commit", commit] => decide(&root, commit, main),
        ["notes", version] => match Version::parse(version) {
            Some(version) => return notes(&root, version),
            None => return usage(&format!("`{version}` is not a version of the form x.y.z")),
        },
        _ => return usage("unknown command"),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("meltemi-release: {message}");
            ExitCode::from(1)
        }
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("meltemi-release: {message}");
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

/// Takes `--root <dir>` out of the arguments; the root defaults to `.`.
fn split_root(args: &[String]) -> Result<(PathBuf, Vec<String>), String> {
    let mut root = PathBuf::from(".");
    let mut rest = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--root" {
            root = iter
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| "`--root` needs a directory".to_string())?;
        } else {
            rest.push(arg.clone());
        }
    }
    Ok((root, rest))
}

/// Prints the changelog section of `version`, or exits 3 when there is none.
fn notes(root: &Path, version: Version) -> ExitCode {
    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md")).unwrap_or_default();
    match changelog_section(&changelog, version) {
        Some(section) => {
            println!("{section}");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("meltemi-release: CHANGELOG.md has no section for {version}");
            ExitCode::from(3)
        }
    }
}

/// Prints the gate's decision as `key=value` lines.
fn decide(root: &Path, commit: &str, main: &str) -> Result<(), String> {
    let decision = gate(root, commit, main)?;
    println!("release={}", decision.release);
    println!("version={}", decision.version);
    println!("tag={}", decision.version.tag());
    println!("reason={}", decision.reason);
    Ok(())
}

/// Sets the version and refreshes the lockfile, or leaves the tree exactly as
/// it was found.
fn set(root: &Path, new: Version) -> Result<(), String> {
    let mut originals: Vec<(PathBuf, String)> = Vec::new();
    let mut files: Vec<PathBuf> = CARRIERS.iter().map(|c| root.join(c.path)).collect();
    files.push(root.join("Cargo.lock"));
    files.dedup();
    for file in &files {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        originals.push((file.clone(), text));
    }
    let restore = |why: String| -> String {
        for (file, text) in &originals {
            let _ = std::fs::write(file, text);
        }
        format!("{why}; every file was put back as it was")
    };

    let changed = set_version(root, new)?;
    let lock_before = &originals.last().expect("Cargo.lock was read").1;
    let status = Command::new("cargo")
        .args(["update", "--workspace"])
        .current_dir(root)
        .status()
        .map_err(|e| restore(format!("cargo update --workspace could not start: {e}")))?;
    if !status.success() {
        return Err(restore(format!(
            "cargo update --workspace failed ({status})"
        )));
    }
    let lock_after = std::fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|e| restore(format!("Cargo.lock: {e}")))?;
    lock_moved_only_the_workspace(lock_before, &lock_after).map_err(restore)?;

    for path in &changed {
        println!("  {}", path.display());
    }
    println!("  Cargo.lock");
    println!(
        "version {new} written; commit it on main to publish {}",
        new.tag()
    );
    Ok(())
}
