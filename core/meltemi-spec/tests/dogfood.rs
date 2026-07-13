// SPDX-License-Identifier: Apache-2.0

//! Dogfooding (design M7): the engine validates Meltemi's own artifacts.
//!
//! The `.meltemi/` tree (constitution + rumbo) and the living-truth specs must
//! be structurally conformant to the `artifact-format` canon. During the
//! bootstrap the living specs live under `openspec/specs/`, so they are
//! validated directly from there. A non-conformant artifact anywhere in the
//! project fails this test — the format guarding itself.

use std::path::{Path, PathBuf};

use meltemi_spec::{MeltemiTree, parse_spec_file, validate_spec, validate_tree};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/core/meltemi-spec
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn own_meltemi_tree_is_conformant() {
    let tree = MeltemiTree::discover(&repo_root()).expect("discover .meltemi");
    assert!(tree.exists, ".meltemi/ must exist in the repo");
    assert!(
        tree.constitution.is_some(),
        ".meltemi/constitution.md must exist"
    );
    assert!(!tree.rumbo.is_empty(), ".meltemi/rumbo/ must have files");

    let diagnostics = validate_tree(&tree);
    assert!(
        diagnostics.is_empty(),
        "the project's own .meltemi/ is non-conformant:\n{}",
        render(&diagnostics)
    );
}

#[test]
fn living_specs_are_conformant() {
    // Bootstrap: the living truth is under openspec/specs/ until migration.
    let specs_dir = repo_root().join("openspec").join("specs");
    assert!(specs_dir.is_dir(), "openspec/specs/ must exist");

    let mut checked = 0usize;
    let mut diagnostics = Vec::new();
    for capability in std::fs::read_dir(&specs_dir)
        .unwrap()
        .filter_map(Result::ok)
    {
        let spec_file = capability.path().join("spec.md");
        if spec_file.is_file() {
            let spec = parse_spec_file(&spec_file).expect("parse living spec");
            assert!(
                !spec.requirements.is_empty(),
                "living spec {} parsed with no requirements",
                spec.capability
            );
            diagnostics.extend(validate_spec(&spec));
            checked += 1;
        }
    }

    assert!(
        checked >= 5,
        "expected at least the 5 fase-0/ola-1 capabilities, got {checked}"
    );
    assert!(
        diagnostics.is_empty(),
        "living specs are non-conformant:\n{}",
        render(&diagnostics)
    );
}

fn render(diagnostics: &[meltemi_spec::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| format!("  {d}"))
        .collect::<Vec<_>>()
        .join("\n")
}
