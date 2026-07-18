// SPDX-License-Identifier: Apache-2.0

//! Migration verification (migracion-openspec-a-meltemi): the living truth and
//! the history now live under `.meltemi/`, and the engine verifies the migrated
//! specs are identical (requirements and scenarios) to their bootstrap-stage
//! origin under `openspec/`. The borrowed tool must have no operative
//! invocation left in the repository configuration.
//!
//! This is the inverted parity: the migrated living truth revalidates against
//! its source. It reads the repository's own artifacts.

use std::path::{Path, PathBuf};

use meltemi_spec::model::Spec;
use meltemi_spec::spec_parser::parse_spec;

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

/// The comparable shape of a spec: its requirement names and, per requirement,
/// its scenario names. Two specs with the same shape are identical in the sense
/// the migration must preserve.
fn shape(spec: &Spec) -> Vec<(String, Vec<String>)> {
    spec.requirements
        .iter()
        .map(|r| {
            (
                r.name.clone(),
                r.scenarios.iter().map(|s| s.name.clone()).collect(),
            )
        })
        .collect()
}

fn parse_at(path: &Path, capability: &str) -> Option<Spec> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_spec(capability, &content, path))
}

#[test]
fn the_migrated_living_truth_is_identical_to_its_origin() {
    // Scenario: Verdad viva idéntica tras migrar — every migrated spec matches
    // its bootstrap-stage source in requirements and scenarios.
    let root = repo_root();
    let source_dir = root.join("openspec").join("specs");
    let dest_dir = root.join(".meltemi").join("specs");

    assert!(
        dest_dir.is_dir(),
        "the migration must have populated `.meltemi/specs/`"
    );

    let mut compared = 0;
    for entry in std::fs::read_dir(&source_dir).expect("openspec/specs readable") {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            continue;
        }
        let capability = entry.file_name().to_string_lossy().into_owned();
        let source = entry.path().join("spec.md");
        if !source.is_file() {
            continue;
        }
        let dest = dest_dir.join(&capability).join("spec.md");
        let migrated = parse_at(&dest, &capability)
            .unwrap_or_else(|| panic!("capability `{capability}` was not migrated"));
        let origin = parse_at(&source, &capability).unwrap();
        assert_eq!(
            shape(&migrated),
            shape(&origin),
            "capability `{capability}` differs after migration"
        );
        compared += 1;
    }
    assert!(
        compared >= 20,
        "expected the full living truth migrated, got {compared}"
    );
}

#[test]
fn the_history_is_preserved_under_meltemi() {
    // Scenario: histórico conserva fechas y contenido.
    let root = repo_root();
    let source = root.join("openspec").join("changes").join("archive");
    let dest = root.join(".meltemi").join("changes").join("archive");
    assert!(dest.is_dir(), "the archived history must be migrated");

    // Every dated archive directory in the source exists in the destination.
    for entry in std::fs::read_dir(&source).expect("openspec archive readable") {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            let name = entry.file_name();
            assert!(
                dest.join(&name).is_dir(),
                "history `{}` is preserved under .meltemi/",
                name.to_string_lossy()
            );
        }
    }
}

#[test]
fn no_operative_reference_to_the_borrowed_tool_remains_in_ci() {
    // Scenario: Sin referencias operativas a la etapa anterior — CI/config do
    // not invoke the borrowed tool. Its tree may remain as consultable history
    // until the maintainer confirms its physical removal.
    let root = repo_root();
    let workflows = root.join(".github").join("workflows");
    if let Ok(entries) = std::fs::read_dir(&workflows) {
        for entry in entries.flatten() {
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            assert!(
                !content.contains("openspec") && !content.contains("/opsx"),
                "no CI workflow invokes the borrowed tool: {}",
                entry.file_name().to_string_lossy()
            );
        }
    }
}
