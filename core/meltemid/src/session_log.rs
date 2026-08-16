// SPDX-License-Identifier: Apache-2.0

//! Append-only JSONL session log (design D6).
//!
//! Each session writes one JSON line per event to a file under the user data
//! directory, indexed by project key:
//! `<data_dir>/projects/<project_key>/sessions/<session_id>.jsonl`.
//! The log is inspectable after the session ends and is the audit trail the
//! `acp-session` spec requires (every permission request with its decision).
//!
//! Writing IS publishing (lanzador-conversacional D3). A log that opts into a
//! stream hands each event it persists to the session-event hub, so the live
//! stream carries the same set the file does — the prompt that opened a turn and
//! the completion that closed it included, without which no client can delimit a
//! turn. Publishing here rather than at one of the twenty-odd call sites is what
//! makes "each event exactly once" a property of the code instead of a habit:
//! there is one place that publishes, and it is the place that writes.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use meltemi_proto::{SESSION_EVENT_VERSION, SessionEvent, SessionEventKind, SessionEventParams};

use crate::clock::now_rfc3339;
use crate::events::EventHub;

/// Where a log's events are published, and on whose behalf.
struct Stream {
    events: EventHub,
    /// The connection that started this session; the hub seals every event with
    /// it, so the initiator receives its own session without declaring interest
    /// and any other connection receives it only while watching.
    origin: u64,
    session_id: String,
}

/// A per-session append-only event writer.
pub struct SessionLog {
    writer: BufWriter<File>,
    path: PathBuf,
    /// Present when the session publishes to the hub. Absent for the logs that
    /// tests and tooling open on their own, which have no connection to name.
    stream: Option<Stream>,
}

impl SessionLog {
    /// Creates (or opens for append) the JSONL log for a session.
    pub fn create(data_dir: &Path, project_key: &str, session_id: &str) -> std::io::Result<Self> {
        let dir = data_dir.join("projects").join(project_key).join("sessions");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{session_id}.jsonl"));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            path,
            stream: None,
        })
    }

    /// Publishes every event this log persists to `events`, on behalf of the
    /// connection `origin` that started the session. Called by the handlers as
    /// they open a session, so nothing downstream has to remember to publish.
    #[must_use]
    pub fn streaming(mut self, events: EventHub, origin: u64, session_id: &str) -> Self {
        self.stream = Some(Stream {
            events,
            origin,
            session_id: session_id.to_string(),
        });
        self
    }

    /// Appends an event, stamping it with the current time and the envelope
    /// version, and flushes so the record survives a crash. Then publishes it,
    /// in that order and never the other: the log is the truth and the stream is
    /// a reading of it, so an event that failed to persist is never announced as
    /// though it had.
    pub fn append(&mut self, kind: SessionEventKind) -> std::io::Result<SessionEvent> {
        let event = SessionEvent {
            v: SESSION_EVENT_VERSION,
            ts: now_rfc3339(),
            kind,
        };
        let line = serde_json::to_string(&event).expect("SessionEvent serializes");
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        if let Some(stream) = &self.stream {
            stream.events.publish(
                stream.origin,
                SessionEventParams {
                    session_id: stream.session_id.clone(),
                    event: event.clone(),
                },
            );
        }
        Ok(event)
    }

    /// The path of this log on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::TurnStatus;

    #[test]
    fn events_are_appended_in_order_and_readable() {
        let data_dir = std::env::temp_dir().join(format!("meltemi-log-{}", std::process::id()));
        let mut log = SessionLog::create(&data_dir, "projkey", "sess-1").expect("create");

        log.append(SessionEventKind::SessionStarted {
            mode: None,
            session_id: "sess-1".into(),
            agent_command: vec!["mock-agent".into()],
            project_root: "C:\\repos\\fixture".into(),
            title: Some("Corregir el login".into()),
        })
        .unwrap();
        log.append(SessionEventKind::PromptSent {
            text: "hello".into(),
        })
        .unwrap();
        log.append(SessionEventKind::TurnCompleted {
            stop_reason: TurnStatus::Completed,
        })
        .unwrap();
        let path = log.path().to_path_buf();
        drop(log);

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);

        // Every line is a valid, versioned session event, in order.
        let first: SessionEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first.v, SESSION_EVENT_VERSION);
        assert!(matches!(
            first.kind,
            SessionEventKind::SessionStarted { mode: None, .. }
        ));
        let last: SessionEvent = serde_json::from_str(lines[2]).unwrap();
        assert!(matches!(
            last.kind,
            SessionEventKind::TurnCompleted {
                stop_reason: TurnStatus::Completed
            }
        ));

        std::fs::remove_dir_all(&data_dir).ok();
    }

    #[test]
    fn reopening_appends_rather_than_truncates() {
        let data_dir =
            std::env::temp_dir().join(format!("meltemi-log-reopen-{}", std::process::id()));
        {
            let mut log = SessionLog::create(&data_dir, "k", "s").unwrap();
            log.append(SessionEventKind::PromptSent { text: "one".into() })
                .unwrap();
        }
        {
            let mut log = SessionLog::create(&data_dir, "k", "s").unwrap();
            log.append(SessionEventKind::PromptSent { text: "two".into() })
                .unwrap();
        }
        let path = data_dir
            .join("projects")
            .join("k")
            .join("sessions")
            .join("s.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().count(), 2, "append must not truncate");
        std::fs::remove_dir_all(&data_dir).ok();
    }
}
