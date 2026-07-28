// SPDX-License-Identifier: Apache-2.0

//! Conformance result store (niveles-integracion-conformidad D3/D4).
//!
//! A conformance run's outcome per agent is persisted under the user data
//! directory as append-only JSONL, stamped with the run date and the agent
//! version. `fleet/list` reads the latest result to report the **verified**
//! level alongside the declared one — declared ≠ verified is visible, not
//! shameful.
//!
//! What a level *means* is declared here rather than in whichever suite happens
//! to run it: [`declared_criteria`] is the list a run has to report in full, and
//! [`verified_level`] awards a level only when every one of them passed. A suite
//! that quietly stopped exercising one criterion would otherwise stay green and
//! keep claiming the same level, which is the failure mode a conformance suite
//! exists to prevent.

use std::path::{Path, PathBuf};

use meltemi_proto::{ConformanceCriterion, ConformanceResult};

/// The criteria each integration level declares, in the order a run reports
/// them.
///
/// Level 2 — the adapter-piloted level — names the four properties a bridge has
/// to hold for a session to be worth anything: the turn arrives as it is
/// produced, the provider's questions reach the permission proxy, a cancelled
/// turn actually ends, and the session is recorded with what was really
/// launched. They are named per level and not per dialect because a result is
/// persisted per agent: which dialect answered is the agent's own identity, not
/// a criterion's.
#[must_use]
pub fn declared_criteria(level: u8) -> &'static [&'static str] {
    match level {
        1 => &["acp_session_completes"],
        2 => &["streaming", "permissions", "cancellation", "session"],
        3 => &["structured_output_mapped", "usage_counters_captured"],
        4 => &["projection_includes_target"],
        _ => &[],
    }
}

/// The levels a run can award, highest first.
pub const LEVELS: [u8; 4] = [4, 3, 2, 1];

/// The declared criteria of a level that a run never reported.
///
/// Missing is not the same as failed, and the difference is the whole point: a
/// failed criterion is a finding about the agent, a missing one is a finding
/// about the suite.
#[must_use]
pub fn missing_criteria(level: u8, criteria: &[ConformanceCriterion]) -> Vec<&'static str> {
    declared_criteria(level)
        .iter()
        .filter(|declared| {
            !criteria
                .iter()
                .any(|c| c.level == level && c.name == **declared)
        })
        .copied()
        .collect()
}

/// The level a set of criteria verifies: the highest level whose declared
/// criteria are **all** reported and **all** passed.
///
/// Levels are not cumulative here, because they are not cumulative in the
/// product either: an agent piloted through an adapter is a level-2 agent and
/// never reports the native-ACP criterion of level 1. What the rule does refuse
/// is the arithmetic that came before it — awarding a level because *something*
/// at that level passed.
#[must_use]
pub fn verified_level(criteria: &[ConformanceCriterion]) -> u8 {
    LEVELS
        .into_iter()
        .find(|&level| {
            !declared_criteria(level).is_empty()
                && missing_criteria(level, criteria).is_empty()
                && criteria
                    .iter()
                    .filter(|c| c.level == level)
                    .all(|c| c.passed)
        })
        .unwrap_or(0)
}

/// The conformance store file under the data directory.
fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("conformance").join("results.jsonl")
}

/// Appends a conformance result (creating the store if needed).
pub fn persist(data_dir: &Path, result: &ConformanceResult) -> std::io::Result<()> {
    use std::io::Write;
    let path = store_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(result).expect("ConformanceResult serializes");
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())
}

/// The latest persisted result for each agent id (last write wins).
pub fn latest_by_agent(data_dir: &Path) -> std::collections::HashMap<String, ConformanceResult> {
    let mut latest = std::collections::HashMap::new();
    let Ok(contents) = std::fs::read_to_string(store_path(data_dir)) else {
        return latest;
    };
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(result) = serde_json::from_str::<ConformanceResult>(line) {
            latest.insert(result.agent_id.clone(), result);
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every criterion a level declares, all passing.
    fn full(level: u8) -> Vec<ConformanceCriterion> {
        declared_criteria(level)
            .iter()
            .map(|name| ConformanceCriterion {
                level,
                name: (*name).to_string(),
                passed: true,
            })
            .collect()
    }

    #[test]
    fn a_level_is_awarded_only_when_every_criterion_it_declares_passed() {
        let mut criteria = full(2);
        assert_eq!(verified_level(&criteria), 2);

        // One failure sinks the level. Not "mostly level 2" — a bridge whose
        // cancellation does not work is not a bridge anybody should be told is
        // verified.
        criteria
            .iter_mut()
            .find(|c| c.name == "cancellation")
            .expect("the level declares it")
            .passed = false;
        assert_eq!(verified_level(&criteria), 0);
    }

    #[test]
    fn a_criterion_the_run_never_reported_is_missing_rather_than_passed() {
        // The dangerous shape: a suite that stopped exercising something and
        // went green anyway. Absence has to weigh as much as failure.
        let partial: Vec<_> = full(2)
            .into_iter()
            .filter(|c| c.name != "session")
            .collect();
        assert_eq!(missing_criteria(2, &partial), vec!["session"]);
        assert_eq!(verified_level(&partial), 0);
    }

    #[test]
    fn levels_are_not_cumulative_because_the_product_is_not() {
        // An adapter-piloted agent reports level 2 and nothing at level 1;
        // that is a verified level-2 agent, not an unverified one.
        assert_eq!(verified_level(&full(2)), 2);
        // And the highest fully-passed level is the one awarded.
        let mut mixed = full(3);
        mixed.extend(full(4));
        assert_eq!(verified_level(&mixed), 4);
    }

    #[test]
    fn latest_result_wins_per_agent() {
        let dir = std::env::temp_dir().join(format!("meltemi-conf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mk = |level: u8, at: &str| ConformanceResult {
            agent_id: "acme".into(),
            verified_level: level,
            agent_version: Some("1.0".into()),
            run_at: at.into(),
            criteria: vec![ConformanceCriterion {
                level,
                name: "streaming".into(),
                passed: true,
            }],
        };
        persist(&dir, &mk(1, "2026-07-11T10:00:00Z")).unwrap();
        persist(&dir, &mk(2, "2026-07-12T10:00:00Z")).unwrap();

        let latest = latest_by_agent(&dir);
        assert_eq!(latest.get("acme").map(|r| r.verified_level), Some(2));
        std::fs::remove_dir_all(&dir).ok();
    }
}
