// SPDX-License-Identifier: Apache-2.0

//! Session metadata index (sesiones-reanudables D1).
//!
//! Beyond the per-session JSONL log, the daemon keeps an append-only index of
//! session metadata per project — `sessions/index.jsonl` next to the logs — so
//! sessions can be listed, inspected and resumed after they end or after the
//! daemon restarts. The **logs are the source of truth**: when the index is
//! missing or damaged, it is rebuilt from the logs. A session with no recorded
//! end is `interrupted` (a crash leaves no ghost "active" sessions).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use meltemi_proto::{FleetResolutionSource, SessionEvent, SessionEventKind, TurnStatus};

/// One session's persisted metadata (one JSON line in `index.jsonl`). A start
/// record has no end fields; the end record is complete, so a last-wins fold
/// per session id reconstructs the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub session_id: String,
    pub agent_command: Vec<String>,
    pub project_root: String,
    /// The integration level the session ran at (1-4); defaults to 1 for
    /// records written before levels existed.
    #[serde(default = "one_level")]
    pub level: u8,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_status: Option<TurnStatus>,
    /// The agent's own session id (needed to request a resume/load).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
    /// Whether the agent announced session-load support in its handshake.
    #[serde(default)]
    pub supports_load: bool,
    /// When this session resumed another, the original session id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumed_from: Option<String>,
    /// The catalog id the agent resolved to, when the resolution named one
    /// (multiproyecto-suscripciones design D4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// The launch profile — the subscription — the session ran under: the NAME
    /// only, never its env overlay (§2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The autonomy mode the session declared, when it declared one. Persisted
    /// because a listing that cannot say what a session was allowed to decide
    /// cannot show it, and `#[serde(default)]` because records written before
    /// modes existed declared none (modos-de-autonomia).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<meltemi_proto::AutonomyMode>,
    /// The model that effectively governed, recoverable from the log's own
    /// resolution event like the agent and the title are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The effort that effectively governed, on the same terms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// HOW the agent resolved (profile, catalog id, configured). Recorded by
    /// the writers that resolved through the fleet; absent elsewhere, because
    /// it cannot be derived from the other two — a catalog id and a configured
    /// agent that names one look identical (tablero-de-carrera design D2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FleetResolutionSource>,
    /// The title derived from the instruction that opened the session. Absent
    /// for sessions that no user sentence started — a dispatched race lane —
    /// and for every session recorded before titles existed
    /// (titulo-de-sesion design D2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Serde default for the level field (level 1).
fn one_level() -> u8 {
    1
}

impl SessionRecord {
    /// Whether the session can be resumed: the agent supports load and its
    /// session id is known.
    #[must_use]
    pub fn resumable(&self) -> bool {
        self.supports_load && self.agent_session_id.is_some()
    }
}

/// The sessions directory for a project key.
pub fn sessions_dir(data_dir: &Path, project_key: &str) -> PathBuf {
    data_dir.join("projects").join(project_key).join("sessions")
}

fn index_path(data_dir: &Path, project_key: &str) -> PathBuf {
    sessions_dir(data_dir, project_key).join("index.jsonl")
}

/// Appends a metadata record to a project's index (creating it if needed).
pub fn append(data_dir: &Path, project_key: &str, record: &SessionRecord) -> std::io::Result<()> {
    use std::io::Write;
    let path = index_path(data_dir, project_key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(record).expect("SessionRecord serializes");
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())
}

/// The metadata records of a project, most recent first. Reads the index and
/// folds it last-wins; when the index is absent, rebuilds from the logs.
pub fn records_for_project(data_dir: &Path, project_key: &str) -> Vec<SessionRecord> {
    let dir = sessions_dir(data_dir, project_key);
    let mut folded: BTreeMap<String, SessionRecord> = BTreeMap::new();

    let index = index_path(data_dir, project_key);
    let from_index = std::fs::read_to_string(&index).ok();
    if let Some(contents) = &from_index {
        for line in contents.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(record) = serde_json::from_str::<SessionRecord>(line) {
                // Last write wins, but keep an end that a later start would
                // otherwise clobber (records are appended start-then-end).
                merge_into(&mut folded, record);
            }
        }
    }

    // Rebuild from logs when the index gave us nothing (missing/damaged): the
    // logs are the source of truth. Also fills in sessions the index missed.
    for record in rebuild_from_logs(&dir) {
        folded
            .entry(record.session_id.clone())
            .or_insert_with(|| record.clone());
    }

    let mut records: Vec<SessionRecord> = folded.into_values().collect();
    // Most recent first.
    records.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    records
}

/// Merges a record into the fold, preferring recorded ends and known agent
/// ids over the partial start record.
fn merge_into(folded: &mut BTreeMap<String, SessionRecord>, record: SessionRecord) {
    match folded.get_mut(&record.session_id) {
        None => {
            folded.insert(record.session_id.clone(), record);
        }
        Some(existing) => {
            if record.ended_at.is_some() {
                existing.ended_at = record.ended_at;
                existing.final_status = record.final_status;
            }
            if record.agent_session_id.is_some() {
                existing.agent_session_id = record.agent_session_id;
            }
            existing.supports_load |= record.supports_load;
            if record.resumed_from.is_some() {
                existing.resumed_from = record.resumed_from;
            }
            // A known resolution is never clobbered by a record that omits it.
            if record.agent_id.is_some() {
                existing.agent_id = record.agent_id;
            }
            if record.profile.is_some() {
                existing.profile = record.profile;
            }
            if record.source.is_some() {
                existing.source = record.source;
            }
            // Same reason, and the one that bites hardest: the closing record
            // carries no title, so without this the title would show while the
            // session ran and vanish the moment it ended.
            if record.title.is_some() {
                existing.title = record.title;
            }
        }
    }
}

/// Reconstructs metadata from every session log in `dir` (skipping the index).
pub fn rebuild_from_logs(dir: &Path) -> Vec<SessionRecord> {
    let mut records = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return records;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_log = path.extension().is_some_and(|e| e == "jsonl")
            && path.file_name().is_some_and(|n| n != "index.jsonl");
        if !is_log {
            continue;
        }
        if let Some(record) = record_from_log(&path) {
            records.push(record);
        }
    }
    records
}

/// Builds a metadata record by folding one session's JSONL log.
fn record_from_log(path: &Path) -> Option<SessionRecord> {
    let contents = std::fs::read_to_string(path).ok()?;
    let session_id = path.file_stem()?.to_str()?.to_string();

    let mut agent_command = Vec::new();
    let mut project_root = String::new();
    let mut started_at = None;
    let mut ended_at = None;
    let mut final_status = None;
    let mut agent_id = None;
    let mut profile = None;
    let mut source = None;
    let mut title = None;
    let mut mode = None;
    let mut model = None;
    let mut effort = None;

    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(event) = serde_json::from_str::<SessionEvent>(line) else {
            continue;
        };
        if started_at.is_none() {
            started_at = Some(event.ts.clone());
        }
        match event.kind {
            SessionEventKind::SessionStarted {
                agent_command: cmd,
                project_root: root,
                title: name,
                mode: declared,
                ..
            } => {
                agent_command = cmd;
                project_root = root;
                // Recoverable from the log alone, like the title: a rebuilt
                // record that could not say what the session was allowed to
                // decide would be a record missing the fact that matters most.
                mode = declared;
                // The start event is what makes the title recoverable without
                // the index, the same job the resolution event does for the
                // agent (titulo-de-sesion design D3).
                title = name;
            }
            // The resolution event is what makes agent and subscription
            // recoverable without the index (multiproyecto-suscripciones D2).
            SessionEventKind::AgentResolved {
                agent_id: id,
                profile: prof,
                source: src,
                model: ran_model,
                effort: ran_effort,
                ..
            } => {
                agent_id = id;
                profile = prof;
                source = Some(src);
                // Recoverable from the log alone, like the agent itself: a
                // rebuilt record can still say with which model a session ran.
                model = ran_model;
                effort = ran_effort;
            }
            SessionEventKind::TurnCompleted { stop_reason } => final_status = Some(stop_reason),
            SessionEventKind::SessionEnded { .. } => ended_at = Some(event.ts.clone()),
            _ => {}
        }
    }

    // A log with no start event is not a usable session record.
    let started_at = started_at?;
    if agent_command.is_empty() {
        return None;
    }
    Some(SessionRecord {
        session_id,
        agent_command,
        project_root,
        // The log does not carry the level; a reconstructed record defaults
        // to 1 (the index carries the real level when present).
        level: 1,
        started_at,
        ended_at,
        final_status,
        // The log does not carry the agent session id or the load capability,
        // so a reconstructed session is inspectable but not resumable.
        agent_session_id: None,
        supports_load: false,
        resumed_from: None,
        title,
        // Agent, subscription and HOW it resolved come from the resolution
        // event when the log recorded one; a log without it states nothing
        // rather than guessing.
        agent_id,
        profile,
        source,
        mode,
        model,
        effort,
    })
}

/// Every project key that has a sessions directory under `data_dir`.
pub fn all_project_keys(data_dir: &Path) -> Vec<String> {
    let projects = data_dir.join("projects");
    let Ok(entries) = std::fs::read_dir(&projects) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().join("sessions").is_dir())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("meltemi-sidx-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn started(id: &str, ts: &str) -> SessionRecord {
        SessionRecord {
            mode: None,
            model: None,
            effort: None,
            session_id: id.into(),
            agent_command: vec!["mock-agent".into()],
            project_root: "/repo".into(),
            level: 1,
            started_at: ts.into(),
            ended_at: None,
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: None,
            profile: None,
            source: None,
            title: None,
        }
    }

    // Scenario: El título sobrevive al cierre de la sesión
    #[test]
    fn the_title_survives_the_record_that_closes_the_session() {
        let data = temp("title-fold");
        let mut opening = started("s-title", "2026-08-09T10:00:00Z");
        opening.title = Some("Corregir el login".into());
        append(&data, "k", &opening).unwrap();

        // The closing record carries no title — that is the ordinary shape, and
        // it is exactly what would erase it without the fold's own branch.
        let mut closing = started("s-title", "2026-08-09T10:00:00Z");
        closing.ended_at = Some("2026-08-09T10:05:00Z".into());
        closing.final_status = Some(TurnStatus::Completed);
        assert!(closing.title.is_none(), "the closing record names nothing");
        append(&data, "k", &closing).unwrap();

        let folded = records_for_project(&data, "k");
        let record = folded.iter().find(|r| r.session_id == "s-title").unwrap();
        assert_eq!(
            record.title.as_deref(),
            Some("Corregir el login"),
            "a title shown while the session ran must not vanish when it ends"
        );
        assert!(
            record.ended_at.is_some(),
            "and the end still won its fields"
        );
    }

    #[test]
    fn append_then_fold_keeps_the_end_over_the_start() {
        let data = temp("fold");
        append(&data, "k", &started("s1", "2026-07-11T10:00:00Z")).unwrap();
        // The end record completes s1.
        append(
            &data,
            "k",
            &SessionRecord {
                mode: None,
                model: None,
                effort: None,
                ended_at: Some("2026-07-11T10:05:00Z".into()),
                final_status: Some(TurnStatus::Completed),
                agent_session_id: Some("agent-s1".into()),
                supports_load: true,
                ..started("s1", "2026-07-11T10:00:00Z")
            },
        )
        .unwrap();

        let records = records_for_project(&data, "k");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].ended_at.as_deref(), Some("2026-07-11T10:05:00Z"));
        assert!(records[0].resumable(), "load-capable ended session resumes");
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn records_sort_most_recent_first() {
        let data = temp("sort");
        append(&data, "k", &started("old", "2026-07-11T09:00:00Z")).unwrap();
        append(&data, "k", &started("new", "2026-07-11T11:00:00Z")).unwrap();
        let records = records_for_project(&data, "k");
        assert_eq!(records[0].session_id, "new");
        assert_eq!(records[1].session_id, "old");
        std::fs::remove_dir_all(&data).ok();
    }

    // Scenario: El título se recupera del registro
    #[test]
    fn a_rebuilt_record_recovers_the_title_from_the_log() {
        let data = temp("title-rebuild");
        let dir = sessions_dir(&data, "k");
        std::fs::create_dir_all(&dir).unwrap();
        // Two logs and no index: one session named, one not — which is what a
        // dispatched lane and every pre-title session look like.
        std::fs::write(
            dir.join("named.jsonl"),
            format!(
                "{}\n",
                event_line(
                    "session_started",
                    "2026-08-09T10:00:00Z",
                    r#""sessionId":"named","agentCommand":["mock-agent"],"projectRoot":"/repo","title":"Corregir el login""#
                )
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("unnamed.jsonl"),
            format!(
                "{}\n",
                event_line(
                    "session_started",
                    "2026-08-09T09:00:00Z",
                    r#""sessionId":"unnamed","agentCommand":["mock-agent"],"projectRoot":"/repo""#
                )
            ),
        )
        .unwrap();

        let records = records_for_project(&data, "k");
        let named = records.iter().find(|r| r.session_id == "named").unwrap();
        assert_eq!(
            named.title.as_deref(),
            Some("Corregir el login"),
            "the log is the source of truth, title included"
        );
        let unnamed = records.iter().find(|r| r.session_id == "unnamed").unwrap();
        assert!(
            unnamed.title.is_none(),
            "a log without a title reconstructs without one, never with a guess"
        );
    }

    #[test]
    fn rebuilds_from_logs_when_the_index_is_missing() {
        // Scenario: Índice reconstruible.
        let data = temp("rebuild");
        let dir = sessions_dir(&data, "k");
        std::fs::create_dir_all(&dir).unwrap();
        // A complete log, no index.jsonl.
        let log = format!(
            "{}\n{}\n{}\n",
            event_line(
                "session_started",
                "2026-07-11T10:00:00Z",
                r#""sessionId":"s1","agentCommand":["mock-agent"],"projectRoot":"/repo""#
            ),
            event_line(
                "turn_completed",
                "2026-07-11T10:04:00Z",
                r#""stopReason":"completed""#
            ),
            event_line(
                "session_ended",
                "2026-07-11T10:05:00Z",
                r#""reason":"completed""#
            ),
        );
        std::fs::write(dir.join("s1.jsonl"), log).unwrap();

        let records = records_for_project(&data, "k");
        assert_eq!(records.len(), 1, "the log-only session is listed");
        assert_eq!(records[0].session_id, "s1");
        assert_eq!(records[0].final_status, Some(TurnStatus::Completed));
        assert!(records[0].ended_at.is_some(), "the end was reconstructed");
        assert!(
            !records[0].resumable(),
            "reconstructed sessions are not resumable"
        );
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn a_log_without_an_end_reconstructs_as_open() {
        // Underpins "interrupted": no session_ended event → no ended_at, which
        // the list handler resolves to interrupted when not active.
        let data = temp("open");
        let dir = sessions_dir(&data, "k");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("s2.jsonl"),
            format!(
                "{}\n",
                event_line(
                    "session_started",
                    "2026-07-11T10:00:00Z",
                    r#""sessionId":"s2","agentCommand":["mock-agent"],"projectRoot":"/repo""#
                )
            ),
        )
        .unwrap();
        let records = records_for_project(&data, "k");
        assert_eq!(records.len(), 1);
        assert!(records[0].ended_at.is_none(), "no recorded end");
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn the_resolution_event_repopulates_agent_and_subscription() {
        // Scenario: Suscripción reconstruida desde el log
        let data = temp("resolution");
        let dir = sessions_dir(&data, "k");
        std::fs::create_dir_all(&dir).unwrap();
        let log = format!(
            "{}
{}
",
            event_line(
                "session_started",
                "2026-07-11T10:00:00Z",
                r#""sessionId":"s3","agentCommand":["claude"],"projectRoot":"/repo""#
            ),
            event_line(
                "agent_resolved",
                "2026-07-11T10:00:01Z",
                r#""binary":"claude","source":"profile","profile":"work","agentId":"claude-code","level":1"#
            ),
        );
        std::fs::write(dir.join("s3.jsonl"), log).unwrap();

        let records = records_for_project(&data, "k");
        assert_eq!(records[0].agent_id.as_deref(), Some("claude-code"));
        assert_eq!(records[0].profile.as_deref(), Some("work"));
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn a_record_written_before_the_resolution_fields_still_reads() {
        // Older index lines have neither agentId nor profile: the defaults keep
        // them readable instead of dropping the whole session.
        let data = temp("compat");
        let dir = sessions_dir(&data, "k");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.jsonl"),
            concat!(
                r#"{"sessionId":"s4","agentCommand":["mock-agent"],"#,
                r#""projectRoot":"/repo","level":1,"#,
                r#""startedAt":"2026-07-11T10:00:00Z"}"#,
                "
"
            ),
        )
        .unwrap();
        let records = records_for_project(&data, "k");
        assert_eq!(records.len(), 1, "the pre-field record survives");
        assert!(records[0].agent_id.is_none(), "unknown, not invented");
        assert!(records[0].profile.is_none());
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn the_fold_keeps_a_known_resolution_over_a_record_that_omits_it() {
        let data = temp("keep-resolution");
        append(
            &data,
            "k",
            &SessionRecord {
                mode: None,
                model: None,
                effort: None,
                agent_id: Some("claude-code".into()),
                profile: Some("work".into()),
                ..started("s5", "2026-07-11T10:00:00Z")
            },
        )
        .unwrap();
        // An end record from a path that does not know the resolution.
        append(
            &data,
            "k",
            &SessionRecord {
                mode: None,
                model: None,
                effort: None,
                ended_at: Some("2026-07-11T10:05:00Z".into()),
                final_status: Some(TurnStatus::Completed),
                ..started("s5", "2026-07-11T10:00:00Z")
            },
        )
        .unwrap();
        let records = records_for_project(&data, "k");
        assert_eq!(records[0].agent_id.as_deref(), Some("claude-code"));
        assert_eq!(records[0].profile.as_deref(), Some("work"));
        std::fs::remove_dir_all(&data).ok();
    }

    /// Builds one JSONL session-event line for the tests.
    fn event_line(kind: &str, ts: &str, payload: &str) -> String {
        format!(r#"{{"v":1,"ts":"{ts}","type":"{kind}","payload":{{{payload}}}}}"#)
    }
}
