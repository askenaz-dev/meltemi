// SPDX-License-Identifier: Apache-2.0

//! Shared session finalization (control-remoto-asistido).
//!
//! Every single-turn session run shares the same tail: emit the turn's terminal
//! events, write the end record that carries the resume metadata (so the session
//! can later be resumed), and deregister from the live registry. `propose` and
//! the `session/direct` resume branch both use it — the latter records
//! `resumed_from` here, which is the reason this was extracted. The multi-turn
//! runners (`sdd/implement`, `worktree/dispatch`) keep their own task-loop
//! finalization; they end with `set_state(Ended)` rather than deregistering.

use std::path::Path;
use std::sync::Arc;

use tokio::sync::Mutex;

use meltemi_proto::{SessionEventKind, TurnStatus};

use crate::acp::SessionOutcome;
use crate::session::SessionRegistry;
use crate::session_index::{self, SessionRecord};
use crate::session_log::SessionLog;

/// The session facts that are fixed for its whole lifetime, borrowed for the
/// duration of finalization.
pub struct SessionContext<'a> {
    pub data_dir: &'a Path,
    pub sessions: &'a SessionRegistry,
    pub log: &'a Arc<Mutex<SessionLog>>,
    pub project_key: &'a str,
    pub session_id: &'a str,
    pub agent_command: &'a [String],
    pub project_root: &'a str,
    pub level: u8,
    pub started_at: &'a str,
    /// The original session, when this run resumed another; recorded on the end
    /// record so the resume chain stays auditable.
    pub resumed_from: Option<String>,
    /// The catalog id and profile the agent resolved to, so the record states
    /// which agent and which subscription ran (multiproyecto-suscripciones D4).
    pub agent_id: Option<String>,
    pub profile: Option<String>,
}

/// The pieces a caller reports after a finalized successful turn.
pub struct Finalized {
    pub status: TurnStatus,
    pub denied_permissions: u32,
    pub agent_session_id: Option<String>,
    pub supports_load: bool,
}

async fn append(log: &Arc<Mutex<SessionLog>>, kind: SessionEventKind) {
    let mut log = log.lock().await;
    let _ = log.append(kind);
}

/// Finalize a completed turn: emit `TurnCompleted` + `SessionEnded("completed")`,
/// write the end record carrying the resume metadata (agent session id + load
/// capability), deregister, and return the outcome pieces the caller reports.
pub async fn finalize_ok(ctx: &SessionContext<'_>, outcome: SessionOutcome) -> Finalized {
    let status = outcome.status;
    append(
        ctx.log,
        SessionEventKind::TurnCompleted {
            stop_reason: status,
        },
    )
    .await;
    append(
        ctx.log,
        SessionEventKind::SessionEnded {
            reason: "completed".into(),
        },
    )
    .await;
    let _ = session_index::append(
        ctx.data_dir,
        ctx.project_key,
        &SessionRecord {
            session_id: ctx.session_id.to_string(),
            agent_command: ctx.agent_command.to_vec(),
            project_root: ctx.project_root.to_string(),
            level: ctx.level,
            started_at: ctx.started_at.to_string(),
            ended_at: Some(crate::clock::now_rfc3339()),
            final_status: Some(status),
            agent_session_id: outcome.agent_session_id.clone(),
            supports_load: outcome.supports_load,
            resumed_from: ctx.resumed_from.clone(),
            agent_id: ctx.agent_id.clone(),
            profile: ctx.profile.clone(),
        },
    );
    ctx.sessions.deregister(ctx.session_id).await;
    Finalized {
        status,
        denied_permissions: outcome.denied_permissions,
        agent_session_id: outcome.agent_session_id,
        supports_load: outcome.supports_load,
    }
}

/// Finalize a failed turn: emit `Error` + `SessionEnded("error")`, write an end
/// record with no final status (a failed session is neither mislabeled
/// interrupted nor advertised resumable), and deregister.
pub async fn finalize_err(ctx: &SessionContext<'_>, kind: &str, detail: String) {
    append(
        ctx.log,
        SessionEventKind::Error {
            kind: kind.to_string(),
            detail,
        },
    )
    .await;
    append(
        ctx.log,
        SessionEventKind::SessionEnded {
            reason: "error".into(),
        },
    )
    .await;
    let _ = session_index::append(
        ctx.data_dir,
        ctx.project_key,
        &SessionRecord {
            session_id: ctx.session_id.to_string(),
            agent_command: ctx.agent_command.to_vec(),
            project_root: ctx.project_root.to_string(),
            level: ctx.level,
            started_at: ctx.started_at.to_string(),
            ended_at: Some(crate::clock::now_rfc3339()),
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: ctx.resumed_from.clone(),
            agent_id: ctx.agent_id.clone(),
            profile: ctx.profile.clone(),
        },
    );
    ctx.sessions.deregister(ctx.session_id).await;
}
