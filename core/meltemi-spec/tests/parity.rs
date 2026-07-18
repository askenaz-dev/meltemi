// SPDX-License-Identifier: Apache-2.0

//! Parity with `openspec archive` (design D5).
//!
//! Applying a capability's archived-change deltas, in archive order, onto an
//! empty living spec must reproduce the current living truth. This anchors the
//! merge semantics to the tool that performs `/archive` during the bootstrap.
//! `propose-flow` is a good anchor: `fase-0-fundacion` created it and later
//! changes only `ADDED` to it, so folding every archived delta in chronological
//! order must yield exactly today's living `propose-flow`.

use std::path::{Path, PathBuf};

use meltemi_spec::{Spec, apply_delta, parse_delta, parse_spec_file};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

/// Every archived change's delta spec for `capability`, in archive order
/// (archives are date-prefixed, so a name sort is chronological).
fn archived_deltas(capability: &str) -> Vec<PathBuf> {
    let archive = repo_root().join("openspec").join("changes").join("archive");
    let mut deltas: Vec<PathBuf> = std::fs::read_dir(&archive)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path().join("specs").join(capability).join("spec.md"))
        .filter(|p| p.is_file())
        .collect();
    deltas.sort();
    deltas
}

#[test]
fn folding_propose_flow_delta_matches_the_living_truth() {
    let capability = "propose-flow";
    let delta_paths = archived_deltas(capability);
    assert!(
        !delta_paths.is_empty(),
        "an archived propose-flow delta should exist (fase-0-fundacion)"
    );

    // Fold every archived delta onto an empty living spec, in archive order —
    // reproducing what successive `/archive` runs did.
    let mut merged = Spec {
        capability: capability.to_string(),
        requirements: Vec::new(),
        deltas: Vec::new(),
        source: PathBuf::from(format!("specs/{capability}/spec.md")),
    };
    for delta_path in &delta_paths {
        let content = std::fs::read_to_string(delta_path).unwrap();
        let delta = parse_delta(capability, &content, delta_path.clone());
        let (next, diagnostics) = apply_delta(&merged, &delta);
        assert!(
            diagnostics.is_empty(),
            "folding {} produced diagnostics: {diagnostics:?}",
            delta_path.display()
        );
        merged = next;
    }

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
