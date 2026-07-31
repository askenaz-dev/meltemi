// SPDX-License-Identifier: Apache-2.0

//! The free session (lanzador-conversacional D1/D2): the door the method's
//! verbs never had.
//!
//! `session/direct` steers a session that already exists, `propose` and the
//! `sdd/*` verbs are verbs of the method, and `worktree/dispatch` demands a
//! change and a task. None of them starts plain work on a repository. This one
//! does — and relaxes nothing to do it: the agent is resolved from the fleet
//! and the resolution recorded, every permission request goes through the same
//! deny-by-default proxy, the JSONL log is append-only, the project enters the
//! registry, and the session is directable so the next instruction runs as the
//! next turn of the same agent session.
//!
//! What it does NOT do is invent a worktree. The free session runs on the
//! project root like every other human-attended path (design D2): the human is
//! at the composer, sees each permission card and decides. In exchange the
//! daemon takes a restore point before the first turn and declares it — or
//! declares why there is none.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use meltemi_proto::{
    SessionEventKind, SessionStartParams, SessionStartResult, SessionState, error_codes,
};

use crate::acp::{self, SessionParams};
use crate::config::Config;
use crate::levels::ResolvedAgent;
use crate::paths;
use crate::rpc::{Peer, RpcError};
use crate::server::DaemonState;
use crate::session_log::SessionLog;

/// The change name reserved for the restore point of a free session. A free
/// session has no change, so its checkpoint triple is `(free, <session-id>,
/// <agent>)`: synthetic, and declared as such. The cost is one pseudo-change in
/// `checkpoint/list`, which filters by change and therefore keeps it apart; the
/// competitor model is untouched, because there is no branch, no worktree and
/// nobody to compete with (design D2, Risks).
const FREE_CHANGE: &str = "free";

/// Handles `session/start` end to end: the recipe of `propose` without the
/// scaffolding, closing through the shared finalizer so a completed free
/// session never lists as interrupted.
pub async fn handle_session_start(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SessionStartParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("session/start: {e}")))?;

    let project_root = PathBuf::from(&params.project_root);
    if !project_root.is_dir() {
        return Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", project_root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ));
    }
    // An instruction is what a free session is made of, and there is no default
    // to fall back on: the contract refuses an empty one, and so does this.
    if params.instruction.trim().is_empty() {
        return Err(RpcError::invalid_params(
            "session/start: the instruction is empty; a free session has no default prompt",
        ));
    }

    let config = Config::load(&state.config_dir, Some(&project_root));
    for diagnostic in &config.fleet_diagnostics {
        tracing::warn!(diagnostic = %diagnostic, "fleet profile hygiene");
    }
    for diagnostic in &config.mcp_diagnostics {
        tracing::warn!(diagnostic, "mcp hygiene");
    }
    for diagnostic in &config.permission_diagnostics {
        tracing::warn!(diagnostic, "permissions config");
    }

    // The fleet's own resolution order, so naming an agent here means exactly
    // what it means everywhere else — and naming one that is not detected
    // refuses instead of quietly running somebody else's binary.
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let bundled = crate::fleet::bundled_dir();
    let resolved = crate::levels::resolve_session_agent(
        &config,
        params.agent.as_deref(),
        &path_var,
        bundled.as_deref(),
    )?;
    let (agent_command, level) = match &resolved.launch {
        crate::levels::Launch::Acp { argv, level } => (argv.clone(), *level),
        crate::levels::Launch::Headless { level, .. }
        | crate::levels::Launch::Artifacts { level } => {
            return Err(RpcError::application(
                error_codes::AGENT_COMMAND_NOT_CONFIGURED,
                "level not pilotable by a free session",
                "level_not_pilotable",
                format!(
                    "the resolved agent is level {level}; a free session is a conversation and \
                     needs an ACP-capable agent (level 1 or 2)"
                ),
                Some("Select a level 1/2 agent for free sessions.".into()),
            ));
        }
    };

    // Open the session and its log.
    let session_id = uuid::Uuid::new_v4().to_string();
    let reg = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    let project_key = paths::project_key(&project_root);
    // Starting work on a root IS pointing Meltemi at it: the registry is fed by
    // real use, and a free session is as real as it gets (multiproyecto D3).
    crate::projects::touch(&state.data_dir, &project_root);
    let mut log = SessionLog::create(&state.data_dir, &project_key, &session_id)
        .map_err(RpcError::internal)?;
    let _ = log.append(SessionEventKind::SessionStarted {
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: project_root.display().to_string(),
    });
    // Which binary ran, and why that one: a reconstruction from the log alone
    // must recover the agent and its subscription, so the free session records
    // its resolution like a dispatch does.
    let _ = log.append(SessionEventKind::AgentResolved {
        binary: agent_command.first().cloned().unwrap_or_default(),
        source: resolved.source,
        profile: resolved.profile.clone(),
        agent_id: resolved.agent_id.clone(),
        level,
    });
    let log = Arc::new(Mutex::new(log));

    // Directable from the start: this is what makes it a conversation rather
    // than one shot, and the composer's next instruction becomes the next turn
    // of the same agent session (control-remoto-asistido).
    let instruction_queue = state
        .sessions
        .enable_direction(&session_id, log.clone())
        .await;
    state
        .sessions
        .set_state(&session_id, SessionState::Active)
        .await;

    // The start record; the matching end record is written by the finalizer
    // below. A crash in between leaves it `interrupted`, which is what the word
    // means (sesiones-reanudables D1).
    let started_at = crate::clock::now_rfc3339();
    let _ = crate::session_index::append(
        &state.data_dir,
        &project_key,
        &crate::session_index::SessionRecord {
            session_id: session_id.clone(),
            agent_command: agent_command.clone(),
            project_root: project_root.display().to_string(),
            level,
            started_at: started_at.clone(),
            ended_at: None,
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: resolved.agent_id.clone(),
            profile: resolved.profile.clone(),
        },
    );

    // The same rule set every other session loads. Nothing is relaxed for
    // lacking a specification (constitution §3).
    let rules = Arc::new(crate::permissions::load_rules(
        &state.config_dir,
        Some(&project_root),
    ));
    for diagnostic in &rules.diagnostics {
        tracing::warn!(diagnostic, "permission rule skipped");
    }

    // What is offered in place of isolation: a restore point, taken before the
    // first turn. `checkpoints::create` snapshots into a scratch index
    // (`GIT_INDEX_FILE`), so it touches neither the user's index nor any branch
    // of theirs — it writes one technical ref under `refs/meltemi/checkpoints/`
    // and one registry line (design D2).
    let agent_label = agent_label(&resolved, &agent_command);
    let checkpoint = crate::checkpoints::create(
        &project_root,
        &project_root,
        FREE_CHANGE,
        &session_id,
        &agent_label,
    );
    let checkpoint_ref = match &checkpoint {
        Ok(record) => {
            append(
                &log,
                SessionEventKind::CheckpointCreated {
                    git_ref: record.git_ref.clone(),
                    change: record.change.clone(),
                    task: record.task.clone(),
                    agent: record.agent.clone(),
                },
            )
            .await;
            Some(record.git_ref.clone())
        }
        Err(error) => {
            // The session starts regardless: promising a restore point and not
            // creating one would be worse than not promising it, and refusing
            // the start over it would be worse still. Task 2.4 gives the two
            // nameable causes their remedy.
            tracing::warn!(error = %error, "no restore point for the free session");
            None
        }
    };

    // `@` references in the instruction are expanded against the repo and the
    // expansion audited, exactly as `propose` does with an idea.
    let (prompt, expansions) = crate::repo_map::expand_refs(
        &project_root,
        &params.instruction,
        crate::repo_map::ExpandLimits::default(),
    );
    if !expansions.is_empty() {
        append(&log, SessionEventKind::RefsExpanded { expansions }).await;
    }

    let edit_scope = state.edits.enter(&project_root, &session_id, log.clone());
    let outcome = acp::run_session(SessionParams {
        agent_command: agent_command.clone(),
        project_root: project_root.clone(),
        prompt,
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel: reg.cancel,
        cancelled: reg.cancelled,
        // A human is at the composer: the interactive wait, not the autonomous
        // one.
        wait: config.interactive_wait(),
        no_client_grace: config.no_client_grace(),
        clients: state.clients.clone(),
        sessions: state.sessions.clone(),
        events: state.events.clone(),
        rules,
        pending: state.pending.clone(),
        // A free session always opens fresh; resuming one is `session/direct`.
        load_session_id: None,
        mcp_servers: config.mcp_servers.clone(),
        env: resolved.env.clone(),
        instruction_queue,
        edit_scope: Some(edit_scope.handle()),
    })
    .await;

    // Closing through `session_finalize` is not decoration: skipping it is
    // exactly the defect `gui-acabado-y-cierre-sdd` D3 fixed — the index never
    // received `ended_at`, so every completed session listed as interrupted
    // forever and its active time counted as nothing.
    let project_root_str = project_root.display().to_string();
    let ctx = crate::session_finalize::SessionContext {
        data_dir: &state.data_dir,
        sessions: &state.sessions,
        log: &log,
        project_key: &project_key,
        session_id: &session_id,
        agent_command: &agent_command,
        project_root: &project_root_str,
        level,
        started_at: &started_at,
        resumed_from: None,
        agent_id: resolved.agent_id.clone(),
        profile: resolved.profile.clone(),
    };
    match outcome {
        Ok(session_outcome) => {
            let fin = crate::session_finalize::finalize_ok(&ctx, session_outcome).await;
            let result = SessionStartResult {
                session_id,
                agent_command,
                status: fin.status,
                denied_permissions: fin.denied_permissions,
                checkpoint_ref,
                checkpoint_unavailable: None,
                checkpoint_remedy: None,
            };
            Ok(serde_json::to_value(result).expect("SessionStartResult serializes"))
        }
        Err(e) => {
            crate::session_finalize::finalize_err(&ctx, "agent_session_failed", e.to_string())
                .await;
            Err(RpcError::application(
                error_codes::AGENT_SPAWN_FAILED,
                "agent session failed",
                "agent_session_failed",
                e.to_string(),
                Some("Check that the resolved agent runs and speaks ACP.".into()),
            ))
        }
    }
}

async fn append(log: &Arc<Mutex<SessionLog>>, kind: SessionEventKind) {
    let mut log = log.lock().await;
    let _ = log.append(kind);
}

/// The agent slot of the restore point's triple: the catalog id the resolution
/// named, or — when the project pins a bare `agent.command` and there is no id —
/// the program's own file name, so the ref still says which binary took it.
fn agent_label(resolved: &ResolvedAgent, agent_command: &[String]) -> String {
    if let Some(id) = &resolved.agent_id {
        return id.clone();
    }
    agent_command
        .first()
        .map(|program| {
            std::path::Path::new(program).file_stem().map_or_else(
                || program.clone(),
                |stem| stem.to_string_lossy().into_owned(),
            )
        })
        .unwrap_or_else(|| "agent".to_string())
}
