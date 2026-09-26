// SPDX-License-Identifier: Apache-2.0

//! The front-matter reader against the REAL files, not against examples written
//! here (harness-global-y-por-agente design D3).
//!
//! A reader tested only against its author's idea of a format passes for as
//! long as the idea holds. These four `RULE.md` are the published rules of the
//! hub, copied byte for byte (see `fixtures/fdh-rules/PROVENANCE.md`), and
//! every claim the design makes about the format is asserted against them.

use std::path::PathBuf;

use meltemi_spec::frontmatter::{FrontValue, parse_front_matter};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fdh-rules")
}

fn rules() -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(fixture_dir())
        .expect("the fixture directory exists")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".RULE.md"))
        })
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a name")
                .trim_end_matches(".RULE.md")
                .to_string();
            (name, std::fs::read_to_string(&path).expect("read the rule"))
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 4, "the four published rules are vendored");
    found
}

#[test]
fn every_published_rule_reads_with_no_diagnostic() {
    for (name, content) in rules() {
        let (front, body) = parse_front_matter(&content)
            .unwrap_or_else(|| panic!("`{name}` has a front-matter block"));
        assert!(
            front.problems.is_empty(),
            "`{name}` was not fully read: {:?}",
            front.problems
        );
        assert!(
            !body.trim().is_empty(),
            "`{name}` keeps its Markdown body after the block"
        );
    }
}

/// The design says nine keys, not the four the proposal named. If the hub ever
/// drops to four this test says so instead of the design quietly being wrong.
#[test]
fn the_real_front_matter_carries_more_keys_than_the_core_uses() {
    const CORE_USES: [&str; 4] = ["name", "scope", "severity", "description"];
    for (name, content) in rules() {
        let (front, _) = parse_front_matter(&content).expect("a block");
        for key in CORE_USES {
            assert!(
                front.get(key).is_some(),
                "`{name}` declares `{key}`, which the core reads"
            );
        }
        let extra: Vec<&str> = front
            .entries
            .iter()
            .map(|(key, _)| key.as_str())
            .filter(|key| !CORE_USES.contains(key))
            .collect();
        assert!(
            extra.len() >= 4,
            "`{name}` carries keys the core does not use, and they survive: {extra:?}"
        );
        // The ones the design named by hand, so a rename upstream is loud.
        for key in ["kind", "version", "agents_supported", "tags"] {
            assert!(
                extra.contains(&key),
                "`{name}` still declares `{key}`: {extra:?}"
            );
        }
    }
}

/// The defect the design found: every published rule carries a trailing comment
/// on `version`, and without trimming it the value comes back glued to it.
#[test]
fn the_trailing_comment_on_version_is_not_part_of_the_value() {
    for (name, content) in rules() {
        let (front, _) = parse_front_matter(&content).expect("a block");
        let version = front.scalar("version").expect("every rule is versioned");
        assert!(
            !version.contains('#'),
            "`{name}` version came back with its comment: {version:?}"
        );
        assert!(
            version.split('.').count() == 3,
            "`{name}` version reads as a version: {version:?}"
        );
    }
}

/// Why no YAML dependency: the real files use inline lists and scalars, and
/// nothing else. A block list appearing upstream turns this red.
#[test]
fn the_real_files_use_only_the_two_shapes_the_reader_covers() {
    for (name, content) in rules() {
        let (front, _) = parse_front_matter(&content).expect("a block");
        let lists: Vec<&str> = front
            .entries
            .iter()
            .filter(|(_, value)| matches!(value, FrontValue::List(_)))
            .map(|(key, _)| key.as_str())
            .collect();
        assert!(
            lists.contains(&"scope") && lists.contains(&"tags"),
            "`{name}` writes its lists inline: {lists:?}"
        );
        let scope = front.list("scope").expect("scope is a list");
        assert!(
            scope.iter().all(|glob| glob.starts_with("**/")),
            "`{name}` scope globs survive their quotes: {scope:?}"
        );
    }
}

/// The identity the harness resolves by: the name in the block must be the
/// directory's. Here the fixture flattens the directory into the file name, so
/// the same equality is what gets asserted.
#[test]
fn the_declared_name_is_the_one_the_harness_resolves_by() {
    for (name, content) in rules() {
        let (front, _) = parse_front_matter(&content).expect("a block");
        assert_eq!(
            front.scalar("name"),
            Some(name.as_str()),
            "the block names the rule its location names"
        );
    }
}
