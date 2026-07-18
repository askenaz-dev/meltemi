// SPDX-License-Identifier: Apache-2.0

//! Conformance result store (niveles-integracion-conformidad D3/D4).
//!
//! A conformance run's outcome per agent is persisted under the user data
//! directory as append-only JSONL, stamped with the run date and the agent
//! version. `fleet/list` reads the latest result to report the **verified**
//! level alongside the declared one — declared ≠ verified is visible, not
//! shameful.

use std::path::{Path, PathBuf};

use meltemi_proto::ConformanceResult;

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
    use meltemi_proto::ConformanceCriterion;

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
