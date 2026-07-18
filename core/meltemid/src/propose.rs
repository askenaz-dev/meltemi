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
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;

use meltemi_proto::{ProposeParams, ProposeResult, SessionEventKind, SessionState, error_codes};

use crate::acp::{self, SessionParams};
use crate::config::Config;
use crate::paths;
use crate::rpc::RpcError;
use crate::server::DaemonState;
use crate::session_log::SessionLog;

/// How long a permission request waits for the client before defaulting to
/// deny. Generous for interactive use; the automated e2e answers immediately.
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(120);

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

    // The agent must be configured before a session can run: a literal
    // command, or a fleet catalog id resolved by level (D4). Resolution only
    // detects; on failure (2000/2001) nothing is launched. `propose` is the
    // interactive SDD flow, so it needs an ACP-capable level (1 or 2); level 3
    // (headless) and level 4 (artifacts) are exercised by the conformance
    // suite, not piloted here.
    let config = Config::load(&state.config_dir, Some(&project_root));
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let (agent_command, level) = match crate::levels::resolve_launch(&config, &path_var)? {
        crate::levels::Launch::Acp { argv, level } => (argv, level),
        crate::levels::Launch::Headless { level, .. }
        | crate::levels::Launch::Artifacts { level } => {
            return Err(RpcError::application(
                error_codes::AGENT_COMMAND_NOT_CONFIGURED,
                "level not pilotable by propose",
                "level_not_pilotable",
                format!(
                    "the configured agent is level {level}; `propose` needs an ACP-capable \
                     agent (level 1 or 2)"
                ),
                Some("Select a level 1/2 agent for interactive proposals.".into()),
            ));
        }
    };

    // Open the session and its log.
    let session_id = uuid::Uuid::new_v4().to_string();
    let cancel = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    let project_key = paths::project_key(&project_root);
    let mut log = SessionLog::create(&state.data_dir, &project_key, &session_id)
        .map_err(RpcError::internal)?;
    let _ = log.append(SessionEventKind::SessionStarted {
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: project_root.display().to_string(),
    });
    let log = Arc::new(Mutex::new(log));
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

    // Delegate the proposal contents to the agent (5.2).
    let outcome = acp::run_session(SessionParams {
        agent_command: agent_command.clone(),
        project_root: project_root.clone(),
        prompt: build_prompt(&params.idea, &proposal_path),
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel,
        permission_timeout: PERMISSION_TIMEOUT,
        rules,
        pending: state.pending.clone(),
        // `propose` always opens a fresh session; resume is a separate flow.
        load_session_id: None,
    })
    .await;

    // Finalize the session log and registry regardless of outcome.
    let result = match outcome {
        Ok(session_outcome) => {
            let status = session_outcome.status;
            append(
                &log,
                SessionEventKind::TurnCompleted {
                    stop_reason: status,
                },
            )
            .await;
            append(&log, ended("completed")).await;
            // The end record carries the resume metadata (agent session id and
            // load capability) so this session can be resumed later.
            let _ = crate::session_index::append(
                &state.data_dir,
                &project_key,
                &crate::session_index::SessionRecord {
                    session_id: session_id.clone(),
                    agent_command: agent_command.clone(),
                    project_root: project_root.display().to_string(),
                    level,
                    started_at: started_at.clone(),
                    ended_at: Some(crate::clock::now_rfc3339()),
                    final_status: Some(status),
                    agent_session_id: session_outcome.agent_session_id.clone(),
                    supports_load: session_outcome.supports_load,
                    resumed_from: None,
                },
            );
            state.sessions.deregister(&session_id).await;
            ProposeResult {
                change_name,
                // Normalize the path to the platform separator (honesty D4).
                proposal_path: normalize_path(&proposal_path),
                status,
                // Declared honestly: how many requests the turn denied (H1).
                denied_permissions: session_outcome.denied_permissions,
            }
        }
        Err(e) => {
            append(
                &log,
                SessionEventKind::Error {
                    kind: "agent_session_failed".into(),
                    detail: e.to_string(),
                },
            )
            .await;
            append(&log, ended("error")).await;
            // Record the end so a failed session is not mislabeled interrupted.
            let _ = crate::session_index::append(
                &state.data_dir,
                &project_key,
                &crate::session_index::SessionRecord {
                    session_id: session_id.clone(),
                    agent_command: agent_command.clone(),
                    project_root: project_root.display().to_string(),
                    level,
                    started_at: started_at.clone(),
                    ended_at: Some(crate::clock::now_rfc3339()),
                    final_status: None,
                    agent_session_id: None,
                    supports_load: false,
                    resumed_from: None,
                },
            );
            state.sessions.deregister(&session_id).await;
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

fn ended(reason: &str) -> SessionEventKind {
    SessionEventKind::SessionEnded {
        reason: reason.to_string(),
    }
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
fn derive_change_name(idea: &str) -> Option<String> {
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
