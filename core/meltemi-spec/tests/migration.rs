// SPDX-License-Identifier: Apache-2.0

//! Migration verification (migracion-openspec-a-meltemi): the living truth and
//! the history now live under `.meltemi/`, and the engine verifies the migration
//! PRESERVED every capability's requirements and scenarios from the
//! bootstrap-stage origin under `openspec/`. The invariant is preservation, not
//! eternal byte-identity — the migration lost nothing, and the living truth may
//! legitimately grow past its frozen origin as later changes edit `.meltemi/`.
//! The borrowed tool must have no operative invocation left in CI.
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

/// Scenarios a later change legitimately SUPERSEDED by refining their
/// requirement, with the change that did it. Preservation is about the migration
/// losing nothing, not about freezing the specs against evolution: a `MODIFIED`
/// requirement that splits one coarse scenario into finer ones has not dropped a
/// promise, it has sharpened it. Every entry here is a deliberate, reviewed and
/// archived decision — the list is the audit trail, so an accidental deletion
/// still fails this test.
const SUPERSEDED: &[(&str, &str, &str, &str)] = &[(
    "edit-surface",
    "Advertencia por sesión de agente activa",
    "Edición sobre worktree con agente trabajando",
    // The migration-era spec itself deferred the concurrency policy to the
    // phase-2 GUI change, which replaced this single warning with the three
    // states of the soft lock (turn in flight / between turns / free tree).
    "gui-tauri-paridad",
)];

fn parse_at(path: &Path, capability: &str) -> Option<Spec> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_spec(capability, &content, path))
}

#[test]
fn the_migration_preserved_every_capability_shape() {
    // Scenario: Verdad viva idéntica tras migrar
    // scoped "WHEN la migración
    // concluye": the migration lost nothing. It is a PRESERVATION invariant, not
    // eternal byte-identity: once the living truth hosts the method, later
    // changes legitimately grow `.meltemi/specs/` past its frozen origin
    // (`openspec/`), so every migration-era requirement and scenario must STILL
    // exist in the live spec — not that the two trees stay identical forever
    // (navegacion-del-metodo, per the archival-parity analysis). This test
    // retires together with the physical removal of `openspec/`.
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
        // Preservation: every origin requirement (and each of its scenarios)
        // still exists in the live spec. The live spec MAY carry more.
        let live = shape(&migrated);
        for (req, origin_scenarios) in shape(&origin) {
            let live_scenarios = live
                .iter()
                .find(|(name, _)| *name == req)
                .map(|(_, s)| s)
                .unwrap_or_else(|| {
                    panic!("requirement `{req}` of `{capability}` was dropped after migration")
                });
            for scenario in origin_scenarios {
                if live_scenarios.contains(&scenario) {
                    continue;
                }
                let superseded = SUPERSEDED.iter().find(|(cap, requirement, name, _)| {
                    *cap == capability && *requirement == req && *name == scenario
                });
                let Some((_, _, _, by)) = superseded else {
                    panic!(
                        "scenario `{scenario}` of `{req}` (`{capability}`) was dropped after \
                         migration; if a change superseded it, record it in SUPERSEDED"
                    );
                };
                // A supersession must be traceable to an ARCHIVED change, and the
                // requirement must still carry scenarios: a requirement left with
                // none is a loss, whatever the reason given.
                let archive = repo_root().join(".meltemi").join("changes").join("archive");
                let archived = std::fs::read_dir(&archive)
                    .map(|entries| {
                        entries.flatten().any(|entry| {
                            entry
                                .file_name()
                                .to_string_lossy()
                                .ends_with(&format!("-{by}"))
                        })
                    })
                    .unwrap_or(false);
                assert!(
                    archived,
                    "`{scenario}` claims to be superseded by `{by}`, which is not archived"
                );
                assert!(
                    !live_scenarios.is_empty(),
                    "`{req}` (`{capability}`) lost every scenario it had"
                );
            }
        }
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
    // Scenario: Sin referencias operativas a la etapa anterior
    // CI/config do
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
