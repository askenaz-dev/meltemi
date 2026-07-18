// SPDX-License-Identifier: Apache-2.0

//! Command execution and the command↔RPC mapping (design D5).
//!
//! Every RPC-backed subcommand reuses the daemon's on-demand bootstrap and the
//! JSON-RPC peer, sends `initialize` first, then its method. Connection failure
//! maps to [`ExitCode::Unreachable`]; a contract/RPC error maps to
//! [`ExitCode::Contract`]. The endpoint is injected so the mapping is testable
//! against an ephemeral in-process daemon.

use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use meltemi_proto::{
    CheckpointListParams, CheckpointListResult, CheckpointRevertParams, CheckpointRevertResult,
};
use meltemi_proto::{
    ContextProjectParams, ContextProjectResult, FleetListParams, FleetListResult, InitializeParams,
    PROTOCOL_VERSION, PeerInfo, PermissionOutcome, PermissionRequestResult, ProposeParams,
    ProposeResult, SessionListParams, SessionListResult, StatusResult, WorktreeAssignParams,
    WorktreeAssignResult, WorktreeDiffParams, WorktreeDiffResult, WorktreeListParams,
    WorktreeListResult, WorktreeTask, methods,
};
use meltemid::bootstrap;
use meltemid::rpc::{Incoming, Peer, RpcError};

use crate::cli::Command;
use crate::output::{CliError, Outcome};

/// Executes an RPC-backed subcommand against the daemon at `endpoint`.
pub async fn execute(command: Command, endpoint: &str) -> Result<Outcome, CliError> {
    match command {
        Command::Status => status(endpoint).await,
        Command::Propose { idea, project_root } => propose(idea, project_root, endpoint).await,
        Command::Fleet => fleet(endpoint).await,
        Command::Project { project_root } => project(project_root, endpoint).await,
        Command::Sessions { project_root } => sessions(project_root, endpoint).await,
        Command::Explore { topic } => sdd_verb(endpoint, methods::SDD_EXPLORE, topic, None).await,
        Command::Plan { change } => {
            sdd_verb(endpoint, methods::SDD_PLAN, String::new(), Some(change)).await
        }
        Command::Constitution { topic } => {
            sdd_verb(endpoint, methods::SDD_CONSTITUTION, topic, None).await
        }
        Command::Review { change } => review(change, endpoint).await,
        Command::Worktrees { project_root } => worktrees(project_root, endpoint).await,
        Command::Assign {
            change,
            task,
            agents,
            project_root,
        } => assign(change, task, agents, project_root, endpoint).await,
        Command::Race {
            change,
            task,
            project_root,
        } => race(change, task, project_root, endpoint).await,
        Command::Checkpoints { change } => checkpoints(change, endpoint).await,
        Command::Revert {
            change,
            task,
            agent,
            confirm,
        } => revert(change, task, agent, confirm, endpoint).await,
        Command::Stop => stop(endpoint).await,
        // Reserved subcommands are handled by the dispatcher before reaching
        // here; this arm keeps `execute` total.
        Command::Reserved(name) => Err(CliError::internal(format!(
            "reserved subcommand `{name}` is not executable"
        ))),
    }
}

/// Connects (starting the daemon on demand), spawns the background handler for
/// daemon-initiated traffic, and performs the mandatory `initialize`.
async fn connect_and_init(endpoint: &str) -> Result<(Peer, JoinHandle<()>), CliError> {
    let stream = bootstrap::connect_or_start(endpoint)
        .await
        .map_err(CliError::unreachable)?;
    let (peer, incoming) = Peer::start(stream);
    let background = tokio::spawn(handle_incoming(peer.clone(), incoming));

    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .map_err(CliError::contract)?;

    Ok((peer, background))
}

/// Answers daemon-initiated requests. In scriptable mode there is no
/// interactive approver yet (#6/#9), so permission requests are denied by
/// default — the safe posture. Rich permission UX arrives in a later change.
async fn handle_incoming(peer: Peer, mut incoming: UnboundedReceiver<Incoming>) {
    while let Some(message) = incoming.recv().await {
        match message {
            Incoming::Request { id, method, .. } if method == methods::PERMISSION_REQUEST => {
                eprintln!(
                    "meltemi: agent requested a permission; denied by default \
                     (interactive approval arrives in a later release)"
                );
                let result = PermissionRequestResult {
                    outcome: PermissionOutcome::Cancelled,
                };
                peer.respond(
                    id,
                    Ok(serde_json::to_value(result).expect("decision serializes")),
                );
            }
            Incoming::Request { id, method, .. } => {
                peer.respond(id, Err(RpcError::method_not_found(&method)));
            }
            Incoming::Notification { .. } => {}
        }
    }
}

async fn status(endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(methods::STATUS, &json!({})).await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let status: StatusResult = serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_status(&status),
        json: value,
    })
}

async fn propose(
    idea: String,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => root,
        None => std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string(),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::PROPOSE, &ProposeParams { idea, project_root })
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ProposeResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_propose(&result),
        json: value,
    })
}

async fn fleet(endpoint: &str) -> Result<Outcome, CliError> {
    // The current directory names the project whose config marks the
    // configured agent (same default as `propose`).
    let project_root = std::env::current_dir()
        .ok()
        .map(|root| root.display().to_string());

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::FLEET_LIST, &FleetListParams { project_root })
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: FleetListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_fleet(&result),
        json: value,
    })
}

async fn project(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => root,
        None => std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string(),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CONTEXT_PROJECT,
            &ContextProjectParams { project_root },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ContextProjectResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_project(&result),
        json: value,
    })
}

async fn sessions(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => Some(root),
        None => std::env::current_dir()
            .ok()
            .map(|root| root.display().to_string()),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SESSION_LIST,
            &SessionListParams {
                project_root,
                ..SessionListParams::default()
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SessionListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_sessions(&result),
        json: value,
    })
}

/// Drives an SDD verb (`explore`/`plan`/`constitution`) using the current
/// directory as the project root. Gates are reported, never awaited.
async fn sdd_verb(
    endpoint: &str,
    method: &str,
    topic: String,
    change: Option<String>,
) -> Result<Outcome, CliError> {
    let project_root = std::env::current_dir()
        .map_err(CliError::internal)?
        .display()
        .to_string();
    let params = match &change {
        Some(change) => json!({ "projectRoot": project_root, "changeName": change }),
        None => json!({ "projectRoot": project_root, "topic": topic }),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(method, &params).await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let phase = value["phase"].as_str().unwrap_or("done");
    let mut human = format!("sdd: {phase}");
    if let Some(artifact) = value["artifact"].as_str() {
        human.push_str(&format!(" — artifact `{artifact}`"));
    }
    if let Some(hint) = value["gateHint"].as_str() {
        human.push_str(&format!("\n{hint}"));
    }
    if let Some(diags) = value["diagnostics"].as_array() {
        for d in diags {
            if let Some(d) = d.as_str() {
                human.push_str(&format!("\n  diagnostic: {d}"));
            }
        }
    }
    Ok(Outcome { human, json: value })
}

/// Reports the review checklist of a change (scriptable; `--json` emits the
/// items and their states, never awaiting interactive input).
async fn review(change: String, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = std::env::current_dir()
        .map_err(CliError::internal)?
        .display()
        .to_string();
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_REVIEW,
            &json!({ "projectRoot": project_root, "changeName": change }),
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    use std::fmt::Write;
    let mut human = format!(
        "review `{change}` — {} pending, {}",
        value["pending"].as_u64().unwrap_or(0),
        if value["canClose"].as_bool().unwrap_or(false) {
            "closable"
        } else {
            "not closable (decide all items)"
        }
    );
    if let Some(items) = value["items"].as_array() {
        for item in items {
            let _ = write!(
                human,
                "\n  [{}] {}/{}",
                item["state"].as_str().unwrap_or("?"),
                item["capability"].as_str().unwrap_or(""),
                item["requirement"].as_str().unwrap_or("")
            );
        }
    }
    Ok(Outcome { human, json: value })
}

/// Resolves an optional project root to the current directory.
fn cwd_or(project_root: Option<String>) -> Result<String, CliError> {
    match project_root {
        Some(root) => Ok(root),
        None => Ok(std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string()),
    }
}

async fn worktrees(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::WORKTREE_LIST, &WorktreeListParams { project_root })
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_worktrees(&result),
        json: value,
    })
}

async fn assign(
    change: String,
    task: String,
    agents: Vec<String>,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &WorktreeAssignParams {
                project_root,
                tasks: vec![WorktreeTask {
                    change,
                    task,
                    agents,
                    files: Vec::new(),
                }],
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeAssignResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_assign(&result),
        json: value,
    })
}

async fn race(
    change: String,
    task: String,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_DIFF,
            &WorktreeDiffParams {
                project_root,
                change,
                task,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeDiffResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_race(&result),
        json: value,
    })
}

async fn checkpoints(change: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHECKPOINT_LIST,
            &CheckpointListParams {
                project_root,
                change,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: CheckpointListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_checkpoints(&result),
        json: value,
    })
}

async fn revert(
    change: String,
    task: String,
    agent: String,
    confirm: bool,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &CheckpointRevertParams {
                project_root,
                change,
                task,
                agent,
                confirm,
            },
        )
        .await;
    peer.close();
    background.abort();

    match response {
        Ok(value) => {
            let result: CheckpointRevertResult =
                serde_json::from_value(value.clone()).map_err(CliError::internal)?;
            Ok(Outcome {
                human: render_revert(&result),
                json: value,
            })
        }
        // Without `confirm`, the daemon refuses with the honest scope. That is
        // the preview, not a failure: render it and exit successfully.
        Err(err) if !confirm && err.code == meltemi_proto::error_codes::WORKTREE_REFUSED => {
            let detail = err
                .data
                .as_ref()
                .and_then(|d| d["detail"].as_str())
                .unwrap_or(&err.message)
                .to_string();
            Ok(Outcome {
                human: format!(
                    "preview (nothing reverted) — {detail}\nRe-run with a trailing `confirm` to revert."
                ),
                json: json!({ "preview": true, "detail": detail }),
            })
        }
        Err(err) => Err(CliError::contract(err)),
    }
}

async fn stop(endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(methods::SHUTDOWN, &json!({})).await;
    peer.close();
    background.abort();

    response.map_err(CliError::contract)?;
    Ok(Outcome {
        human: "daemon shutdown requested".into(),
        json: json!({ "shutdown": "requested" }),
    })
}

/// Human rendering of a propose result: a stable lowercase status word, the
/// normalized artifact path, and — when the turn denied any permission — a
/// visible warning that the proposal may be incomplete (honesty H1/H4/H5).
fn render_propose(result: &ProposeResult) -> String {
    let mut out = format!(
        "proposed `{}` [{}]\n{}",
        result.change_name,
        status_word(result.status),
        result.proposal_path
    );
    if result.denied_permissions > 0 {
        out.push_str(&format!(
            "\nwarning: {} permission request(s) denied — the proposal may be incomplete",
            result.denied_permissions
        ));
    }
    out
}

/// The stable, lowercase word for a turn status (never the capitalized Debug
/// form). Mirrors the contract's snake_case serialization.
fn status_word(status: meltemi_proto::TurnStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Human rendering of the session list: state word, id, agent, and a
/// `resumable` marker, most recent first.
fn render_sessions(result: &SessionListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} session(s)", result.sessions.len());
    for session in &result.sessions {
        let state = session_state_word(session.state);
        let _ = write!(
            out,
            "\n  {state:<12}  {}  {}",
            session.session_id,
            session.agent_command.join(" ")
        );
        if session.resumable {
            let _ = write!(out, "  (resumable)");
        }
    }
    out
}

/// The stable lowercase word for a session state.
fn session_state_word(state: meltemi_proto::SessionState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Human rendering of a context-projection report: which targets were written
/// vs already current, with a short fingerprint.
fn render_project(result: &ContextProjectResult) -> String {
    use std::fmt::Write;
    let written = result.targets.iter().filter(|t| t.written).count();
    let mut out = format!(
        "projected context — {} target(s), {} written",
        result.targets.len(),
        written
    );
    for target in &result.targets {
        let word = if target.written {
            "written    "
        } else {
            "up-to-date "
        };
        let short: String = target.fingerprint.chars().take(12).collect();
        let _ = write!(out, "\n  {word}  {}  {short}", target.path);
    }
    out
}

/// Human rendering of the fleet catalog: detection word, declared level,
/// id, name, then the detected path and the configured marker.
fn render_fleet(fleet: &FleetListResult) -> String {
    use std::fmt::Write;
    let detected = fleet.agents.iter().filter(|a| a.detected).count();
    let mut out = String::new();
    let _ = write!(
        out,
        "registry {} — {} agent(s), {} detected",
        fleet.registry_version,
        fleet.agents.len(),
        detected
    );
    let id_width = fleet.agents.iter().map(|a| a.id.len()).max().unwrap_or(0);
    for agent in &fleet.agents {
        let word = if agent.detected {
            "detected    "
        } else {
            "not-detected"
        };
        let verified = match agent.verified_level {
            Some(v) => format!("verified L{v}"),
            None => "unverified".to_string(),
        };
        let _ = write!(
            out,
            "\n  {word}  L{} ({verified})  {:<id_width$}  {}",
            agent.integration_level, agent.id, agent.display_name
        );
        if let Some(path) = &agent.binary_path {
            let _ = write!(out, " — {path}");
        }
        if agent.configured {
            let _ = write!(out, " (configured)");
        }
    }
    out
}

/// Human rendering of the daemon status.
fn render_status(status: &StatusResult) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = write!(
        out,
        "daemon {} — uptime {}s — {} session(s)",
        status.daemon_version,
        status.uptime_seconds,
        status.sessions.len()
    );
    for session in &status.sessions {
        let _ = write!(
            out,
            "\n  {} [{:?}] {}",
            session.session_id,
            session.state,
            session.agent_command.join(" ")
        );
    }
    out
}

/// Human rendering of the managed worktree list: branch, agent and path, with a
/// `race` marker when more than one competes on the same task.
fn render_worktrees(result: &WorktreeListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} managed worktree(s)", result.worktrees.len());
    for w in &result.worktrees {
        let _ = write!(
            out,
            "\n  {}  {}/{}  {}  {}",
            if w.competitor { "race " } else { "solo " },
            w.change,
            w.task,
            w.agent,
            w.path
        );
    }
    out
}

/// Human rendering of an assignment: the fixed base, then each parallel batch
/// (serialized batches show why), and every worktree created.
fn render_assign(result: &WorktreeAssignResult) -> String {
    use std::fmt::Write;
    let short: String = result.base_rev.chars().take(12).collect();
    let mut out = format!(
        "assigned {} worktree(s) from base {short} — {} batch(es)",
        result.worktrees.len(),
        result.batches.len()
    );
    for (i, batch) in result.batches.iter().enumerate() {
        let _ = write!(out, "\n  batch {}: {}", i + 1, batch.tasks.join(", "));
        if let Some(reason) = &batch.serialized_reason {
            let _ = write!(out, "  [{reason}]");
        }
    }
    for w in &result.worktrees {
        let marker = if w.competitor { "race " } else { "solo " };
        let _ = write!(out, "\n  {marker} {}  {}  {}", w.agent, w.branch, w.path);
    }
    out
}

/// Human rendering of the checkpoint list: change/task/agent, short ref, moment,
/// and a count of irreversible out-of-tree operations when present.
fn render_checkpoints(result: &CheckpointListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} checkpoint(s)", result.checkpoints.len());
    for c in &result.checkpoints {
        let short = c.git_ref.rsplit('/').next().unwrap_or(&c.git_ref);
        let _ = write!(
            out,
            "\n  {}/{}-{}  {short}  {}",
            c.change, c.task, c.agent, c.created_at
        );
        if !c.irreversible.is_empty() {
            let _ = write!(out, "  ({} irreversible)", c.irreversible.len());
        }
    }
    out
}

/// Human rendering of a reversion result with its honest scope: complete vs
/// partial, and the out-of-tree operations that remain in effect.
fn render_revert(result: &CheckpointRevertResult) -> String {
    use std::fmt::Write;
    let mut out = if result.scope.complete {
        "reverted — worktree restored completely".to_string()
    } else {
        "reverted — worktree restored, but NOT a total reversion".to_string()
    };
    for op in &result.scope.irreversible {
        let _ = write!(out, "\n  irreversible: {op}");
    }
    out
}

/// Human rendering of a race: each competitor's changed-file count and the
/// full diff against the common base, ready for an assisted merge decision.
fn render_race(result: &WorktreeDiffResult) -> String {
    use std::fmt::Write;
    let short: String = result.base_rev.chars().take(12).collect();
    let mut out = format!(
        "{} competitor(s) against base {short}",
        result.competitors.len()
    );
    for c in &result.competitors {
        let _ = write!(
            out,
            "\n\n=== {} ({} file(s) changed) — {}\n{}",
            c.agent,
            c.changed_files.len(),
            c.path,
            if c.diff.is_empty() {
                "(no changes)"
            } else {
                c.diff.trim_end()
            }
        );
    }
    out
}
