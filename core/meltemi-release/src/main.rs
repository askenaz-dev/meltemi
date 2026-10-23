// SPDX-License-Identifier: Apache-2.0

//! `meltemi-release`: the release tooling of this repository.
//!
//! ```text
//! meltemi-release version [--root <dir>]
//! meltemi-release set-version <x.y.z> [--root <dir>]
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
//! Exit codes: 0 done, 1 refused or failed, 2 usage.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use meltemi_release::{
    CARRIERS, Version, agreed_version, lock_moved_only_the_workspace, read_from_tree, set_version,
};

const USAGE: &str = "usage: meltemi-release version [--root <dir>]\n       meltemi-release set-version <x.y.z> [--root <dir>]";

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
