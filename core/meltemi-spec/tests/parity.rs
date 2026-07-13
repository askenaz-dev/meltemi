// SPDX-License-Identifier: Apache-2.0

//! Parity with `openspec archive` (design D5).
//!
//! Applying a real archived change's delta onto the previous living spec must
//! reproduce the current living truth. This anchors the merge semantics to the
//! tool that performs `/archive` during the bootstrap. `propose-flow` is a
//! good anchor: it was created by `fase-0-fundacion` with `ADDED` only and has
//! not been modified since, so folding its delta onto an empty spec must yield
//! exactly today's living `propose-flow`.

use std::path::{Path, PathBuf};

use meltemi_spec::{Spec, apply_delta, parse_delta, parse_spec_file};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

/// Finds an archived change's delta spec for `capability`, if any.
fn archived_delta(capability: &str) -> Option<PathBuf> {
    let archive = repo_root().join("openspec").join("changes").join("archive");
    for entry in std::fs::read_dir(&archive).ok()?.filter_map(Result::ok) {
        let candidate = entry.path().join("specs").join(capability).join("spec.md");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[test]
fn folding_propose_flow_delta_matches_the_living_truth() {
    let capability = "propose-flow";
    let delta_path = archived_delta(capability)
        .expect("an archived propose-flow delta should exist (fase-0-fundacion)");

    let content = std::fs::read_to_string(&delta_path).unwrap();
    let delta = parse_delta(capability, &content, delta_path.clone());

    // Fold the delta onto an empty living spec — reproducing what archive did.
    let empty = Spec {
        capability: capability.to_string(),
        requirements: Vec::new(),
        deltas: Vec::new(),
        source: PathBuf::from(format!("specs/{capability}/spec.md")),
    };
    let (merged, diagnostics) = apply_delta(&empty, &delta);
    assert!(
        diagnostics.is_empty(),
        "folding the archived delta produced diagnostics: {diagnostics:?}"
    );

    // Compare against today's living truth.
    let living = parse_spec_file(
        &repo_root()
            .join("openspec")
            .join("specs")
            .join(capability)
            .join("spec.md"),
    )
    .expect("living propose-flow spec");

    let merged_names: Vec<&str> = merged
        .requirements
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    let living_names: Vec<&str> = living
        .requirements
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        merged_names, living_names,
        "merged requirements must match the living truth, in order"
    );

    // And each requirement's scenarios must match too.
    for (m, l) in merged.requirements.iter().zip(&living.requirements) {
        let ms: Vec<&str> = m.scenarios.iter().map(|s| s.name.as_str()).collect();
        let ls: Vec<&str> = l.scenarios.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            ms, ls,
            "scenarios of `{}` must match the living truth",
            m.name
        );
    }
}
