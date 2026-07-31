// SPDX-License-Identifier: Apache-2.0

//! The `propose` flow (tasks 5.1-5.3).
//!
//! `propose` scaffolds a change proposal deterministically and then delegates
//! its contents to the configured agent over ACP:
//!
//! 1. validate the project root and derive a kebab-case change name (5.1);
//! 2. create `.meltemi/changes/<name>/proposal.md` from a fixed skeleton,
//!    refusing to overwrite an existing change (5.1);
//! 3. open a session, then run one ACP turn that asks the agent to fill in
//!    the proposal, with the working directory at the repository root; the
//!    agent's updates and permission requests stream to the client (5.2);
//! 4. return the change name, the proposal path and the final turn status,
//!    having streamed progress throughout (5.3).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use meltemi_proto::{ProposeParams, ProposeResult, SessionEventKind, SessionState, error_codes};

use crate::acp::{self, SessionParams};
use crate::config::Config;
use crate::paths;
use crate::rpc::RpcError;
use crate::server::DaemonState;
use crate::session_log::SessionLog;

/// Handles the `propose` request end to end.
pub async fn handle_propose(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &crate::rpc::Peer,
) -> Result<Value, RpcError> {
    let params: ProposeParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("propose: {e}")))?;

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

    let change_name = derive_change_name(&params.idea).ok_or_else(|| {
        RpcError::application(
            error_codes::INVALID_IDEA,
            "invalid idea",
            "invalid_idea",
            "no change name could be derived from the idea",
            Some("Describe the change in a few words.".into()),
        )
    })?;

    // Scaffold, refusing to overwrite an existing change (5.1).
    let change_dir = project_root
        .join(".meltemi")
        .join("changes")
        .join(&change_name);
    if change_dir.exists() {
        return Err(RpcError::application(
            error_codes::CHANGE_ALREADY_EXISTS,
            "change already exists",
            "change_already_exists",
            format!("`.meltemi/changes/{change_name}` already exists"),
            Some(format!("Try a different idea, e.g. `{change_name}-v2`.")),
        ));
    }
    std::fs::create_dir_all(&change_dir).map_err(RpcError::internal)?;
    let proposal_path = change_dir.join("proposal.md");
    std::fs::write(&proposal_path, scaffold(&change_name, &params.idea))
        .map_err(RpcError::internal)?;

    // The agent must be resolved before a session can run: the one the request
    // named — a launch profile or a catalog id, in the fleet's own order — or,
    // when it named none, the project's configured agent exactly as before that
    // parameter existed. Resolution only detects; on failure (2000/2001) nothing
    // is launched. `propose` is the interactive SDD flow, so it needs an
    // ACP-capable level (1 or 2); level 3 (headless) and level 4 (artifacts) are
    // exercised by the conformance suite, not piloted here.
    let config = Config::load(&state.config_dir, Some(&project_root));
    for diagnostic in &config.fleet_diagnostics {
        tracing::warn!(diagnostic = %diagnostic, "fleet profile hygiene");
    }
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
                "level not pilotable by propose",
                "level_not_pilotable",
                format!(
                    "the resolved agent is level {level}; `propose` needs an ACP-capable \
                     agent (level 1 or 2)"
                ),
                Some("Select a level 1/2 agent for interactive proposals.".into()),
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
    // The project registry is fed by real use only (multiproyecto D3).
    crate::projects::touch(&state.data_dir, &project_root);
    let mut log = SessionLog::create(&state.data_dir, &project_key, &session_id)
        .map_err(RpcError::internal)?
        .streaming(state.events.clone(), peer.connection_id(), &session_id);
    let _ = log.append(SessionEventKind::SessionStarted {
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: project_root.display().to_string(),
    });
    // Which binary ran and why that one, in the log itself: a reconstruction
    // from the log alone must recover the agent and the subscription. Only
    // `worktree/dispatch` and `sdd/implement` wrote this event before, so every
    // proposal and every authoring turn was mute about who wrote it
    // (lanzador-conversacional 4.2).
    let _ = log.append(SessionEventKind::AgentResolved {
        binary: agent_command.first().cloned().unwrap_or_default(),
        source: resolved.source,
        profile: resolved.profile.clone(),
        agent_id: resolved.agent_id.clone(),
        level,
    });
    let log = Arc::new(Mutex::new(log));
    // Opt the session into remote direction (control-remoto-asistido): it becomes
    // a directable target and gains the queue the multi-turn loop drains.
    let instruction_queue = state
        .sessions
        .enable_direction(&session_id, log.clone())
        .await;
    state
        .sessions
        .set_state(&session_id, SessionState::Active)
        .await;

    // Persist a start record in the session index; the matching end record is
    // appended below. A crash before the end leaves it as `interrupted`
    // (sesiones-reanudables D1).
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
            // Whatever the resolution named: the configured agent when the
            // request named none, and the profile when it named one.
            agent_id: resolved.agent_id.clone(),
            profile: resolved.profile.clone(),
        },
    );

    // Load the permission rules once for the session (proxy-permisos D1).
    let rules = Arc::new(crate::permissions::load_rules(
        &state.config_dir,
        Some(&project_root),
    ));
    for diagnostic in &rules.diagnostics {
        tracing::warn!(diagnostic, "permission rule skipped");
    }
    // MCP hygiene diagnostics (mcp-passthrough D1) surface as warnings; the
    // offending values are never carried into the diagnostic.
    for diagnostic in &config.mcp_diagnostics {
        tracing::warn!(diagnostic, "mcp hygiene");
    }
    // An invalid `[permissions]` value kept its default; say so (espera-humana).
    for diagnostic in &config.permission_diagnostics {
        tracing::warn!(diagnostic, "permissions config");
    }

    // Expand any `@` references in the idea against the repo, auditing the
    // expansions in the log (gestion-contexto-repo). A missing reference is
    // flagged inline, never aborting the turn.
    let (expanded_idea, expansions) = crate::repo_map::expand_refs(
        &project_root,
        &params.idea,
        crate::repo_map::ExpandLimits::default(),
    );
    if !expansions.is_empty() {
        append(&log, SessionEventKind::RefsExpanded { expansions }).await;
    }

    // Delegate the proposal contents to the agent (5.2).
    let edit_scope = state.edits.enter(&project_root, &session_id, log.clone());
    let outcome = acp::run_session(SessionParams {
        agent_command: agent_command.clone(),
        project_root: project_root.clone(),
        prompt: build_prompt(&expanded_idea, &proposal_path),
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel: reg.cancel,
        cancelled: reg.cancelled,
        wait: config.interactive_wait(),
        no_client_grace: config.no_client_grace(),
        clients: state.clients.clone(),
        sessions: state.sessions.clone(),
        rules,
        pending: state.pending.clone(),
        // `propose` always opens a fresh session; resume is a separate flow.
        load_session_id: None,
        mcp_servers: config.mcp_servers.clone(),
        // The profile's auth context, when a profile resolved it. Values are
        // never logged (§2).
        env: resolved.env.clone(),
        // Directable: directed instructions run as follow-up turns.
        instruction_queue,
        edit_scope: Some(edit_scope.handle()),
    })
    .await;

    // Finalize the session log and registry regardless of outcome, through the
    // shared finalizer (control-remoto-asistido) so the end record — and thus
    // resumability — matches every other single-turn run.
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
    let result = match outcome {
        Ok(session_outcome) => {
            let fin = crate::session_finalize::finalize_ok(&ctx, session_outcome).await;
            ProposeResult {
                change_name,
                // Normalize the path to the platform separator (honesty D4).
                proposal_path: normalize_path(&proposal_path),
                status: fin.status,
                // Declared honestly: how many requests the turn denied (H1).
                denied_permissions: fin.denied_permissions,
            }
        }
        Err(e) => {
            crate::session_finalize::finalize_err(&ctx, "agent_session_failed", e.to_string())
                .await;
            return Err(RpcError::application(
                error_codes::AGENT_SPAWN_FAILED,
                "agent session failed",
                "agent_session_failed",
                e.to_string(),
                Some("Check that `agent.command` runs an ACP-capable agent.".into()),
            ));
        }
    };

    Ok(serde_json::to_value(result).expect("ProposeResult serializes"))
}

async fn append(log: &Arc<Mutex<SessionLog>>, kind: SessionEventKind) {
    let mut log = log.lock().await;
    let _ = log.append(kind);
}

/// Renders a path with a uniform platform separator (honesty D4, H4/H5). On
/// Windows both separators are valid, so forward slashes a client may have
/// sent are unified to backslashes; on Unix a backslash is a legal filename
/// character and is left untouched.
fn normalize_path(path: &Path) -> String {
    let shown = path.display().to_string();
    #[cfg(windows)]
    {
        shown.replace('/', "\\")
    }
    #[cfg(not(windows))]
    {
        shown
    }
}

/// Derives a kebab-case change name from a free-form idea: lowercase, runs of
/// non-alphanumeric characters collapse to a single `-`, trimmed, and capped
/// to a reasonable length. Returns `None` when nothing usable remains.
pub fn derive_change_name(idea: &str) -> Option<String> {
    let mut name = String::new();
    let mut prev_dash = false;
    for ch in idea.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !name.is_empty() {
            name.push('-');
            prev_dash = true;
        }
    }
    let name = name.trim_matches('-');
    if name.is_empty() {
        return None;
    }
    // Cap length on a word boundary where possible.
    let capped: String = name.chars().take(60).collect();
    Some(capped.trim_matches('-').to_string())
}

/// The deterministic proposal skeleton meltemid writes before the agent fills
/// it in.
fn scaffold(change_name: &str, idea: &str) -> String {
    format!(
        "# Propuesta: {change_name}\n\n\
         ## Why\n\n\
         <!-- Idea: {idea} -->\n\n\
         ## What Changes\n\n\
         ## Impact\n"
    )
}

/// Builds the prompt sent to the agent. The `PROPOSAL_PATH:` line lets the
/// agent (and the scripted mock) know exactly which file to fill in.
fn build_prompt(idea: &str, proposal_path: &Path) -> String {
    format!(
        "Fill in the change proposal for the following idea. Write the proposal \
         into the file at the path below.\n\n\
         IDEA: {idea}\n\
         PROPOSAL_PATH: {}\n",
        proposal_path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_names_are_kebab_case() {
        assert_eq!(
            derive_change_name("Add dark mode!").as_deref(),
            Some("add-dark-mode")
        );
        assert_eq!(
            derive_change_name("  Multiple   spaces  ").as_deref(),
            Some("multiple-spaces")
        );
        assert_eq!(
            derive_change_name("Café münchen 2").as_deref(),
            Some("caf-m-nchen-2")
        );
    }

    #[test]
    fn empty_or_symbolic_ideas_have_no_name() {
        assert_eq!(derive_change_name(""), None);
        assert_eq!(derive_change_name("!!! ???"), None);
    }

    #[test]
    fn scaffold_has_the_standard_sections() {
        let text = scaffold("add-thing", "an idea");
        assert!(text.contains("# Propuesta: add-thing"));
        assert!(text.contains("## Why"));
        assert!(text.contains("## What Changes"));
        assert!(text.contains("## Impact"));
        assert!(text.contains("an idea"));
    }
}
