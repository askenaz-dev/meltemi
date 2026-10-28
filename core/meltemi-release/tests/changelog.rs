// SPDX-License-Identifier: Apache-2.0

//! The changelog written from the archive (antes-de-anunciar D3, D4).

mod common;

use std::path::Path;

use meltemi_release::{Version, archived_changes, changelog_section, summary_of, write_changelog};

fn archive(root: &Path, folder: &str, proposal: &str) {
    let dir = root.join(".meltemi/changes/archive").join(folder);
    std::fs::create_dir_all(&dir).expect("archive folder");
    std::fs::write(dir.join("proposal.md"), proposal).expect("proposal");
}

fn fixture(name: &str) -> std::path::PathBuf {
    let root = common::scratch(name);
    archive(
        &root,
        "2026-07-20-first",
        "# first\n\n## Why\n\nBecause. And more.\n\n## What Changes\n\n- **The first thing**: and its detail.\n- **The second**: x.\n",
    );
    archive(
        &root,
        "2026-07-26-second",
        "# second\n\n## Why\n\nIt was missing. Truly.\n",
    );
    archive(
        &root,
        "2026-10-28-third",
        "# third\n\n## What Changes\n\n- A plain bullet that says\n  what changed. And more.\n",
    );
    std::fs::create_dir_all(root.join(".meltemi/changes/archive/not-a-change")).unwrap();
    root
}

fn v(text: &str) -> Version {
    Version::parse(text).expect("a version")
}

#[test]
fn each_release_lists_the_changes_it_archives() {
    // Scenario: Cada release lista las changes que archiva
    let root = fixture("lists");
    let archived = archived_changes(&root).expect("the archive reads");
    assert_eq!(
        archived.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["first", "second", "third"],
        "oldest first, and a folder without a date is not a change"
    );

    let log = write_changelog("", v("0.1.0"), Some("2026-07-26"), &archived);
    let log = write_changelog(&log, v("0.2.0"), None, &archived);
    let first = changelog_section(&log, v("0.1.0")).expect("0.1.0");
    let second = changelog_section(&log, v("0.2.0")).expect("0.2.0");
    assert!(
        first.contains("[`first`]") && first.contains("[`second`]"),
        "{log}"
    );
    assert!(
        !first.contains("[`third`]"),
        "archived after its date: {log}"
    );
    assert!(second.contains("[`third`]"), "{log}");
    assert!(
        !second.contains("[`first`]") && !second.contains("[`second`]"),
        "a change already listed is not repeated: {log}"
    );
    assert!(
        log.find("## 0.2.0").unwrap() < log.find("## 0.1.0 — 2026-07-26").unwrap(),
        "newest first, and the date bound is the section's date: {log}"
    );
    assert!(
        first.contains("(.meltemi/changes/archive/2026-07-20-first/proposal.md)"),
        "each entry links its proposal: {first}"
    );

    // Writing the same version again gives the same changelog.
    assert_eq!(write_changelog(&log, v("0.2.0"), None, &archived), log);
    assert_eq!(
        write_changelog(&log, v("0.1.0"), Some("2026-07-26"), &archived),
        log
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_release_notes_come_from_that_section() {
    // Scenario: Las notas de la release salen de esa sección
    let root = fixture("notes");
    let archived = archived_changes(&root).expect("the archive reads");
    common::write_carriers(&root, "0.1.0");
    let log = write_changelog("", v("0.3.0"), None, &archived);
    std::fs::write(root.join("CHANGELOG.md"), &log).unwrap();

    // The real reader: the binary the pipeline runs.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_meltemi-release"))
        .args(["notes", "0.3.0", "--root"])
        .arg(&root)
        .output()
        .expect("run notes");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let notes = String::from_utf8_lossy(&out.stdout);
    for name in ["first", "second", "third"] {
        assert!(notes.contains(&format!("[`{name}`]")), "{notes}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_summary_is_the_line_the_proposal_opens_its_changes_with() {
    assert_eq!(
        summary_of("## What Changes\n\n- **The lead**: then detail.\n"),
        "The lead"
    );
    assert_eq!(
        summary_of("## What Changes\n\n- A plain bullet that says\n  what changed. And more.\n"),
        "A plain bullet that says what changed."
    );
    assert_eq!(
        summary_of("# x\n\n## Why\n\nIt was missing. Truly.\n\nNext paragraph.\n"),
        "It was missing."
    );
    assert_eq!(summary_of("# nothing to say\n"), "");
}

#[test]
fn the_command_writes_the_file_and_refuses_a_bad_date() {
    let root = fixture("command");
    let bin = env!("CARGO_BIN_EXE_meltemi-release");
    let out = std::process::Command::new(bin)
        .args(["changelog", "0.1.0", "--until", "26-07-2026", "--root"])
        .arg(&root)
        .output()
        .expect("run changelog");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a date not of the form yyyy-mm-dd is a usage error"
    );
    assert!(
        !root.join("CHANGELOG.md").exists(),
        "and nothing is written"
    );

    let out = std::process::Command::new(bin)
        .args(["changelog", "0.1.0", "--until", "2026-07-26", "--root"])
        .arg(&root)
        .output()
        .expect("run changelog");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_to_string(root.join("CHANGELOG.md")).expect("written");
    assert!(written.starts_with("# Changelog\n"), "{written}");
    assert!(written.contains("## 0.1.0 — 2026-07-26"), "{written}");
    let _ = std::fs::remove_dir_all(&root);
}
