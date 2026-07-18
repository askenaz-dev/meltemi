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
    ContextProjectParams, ContextProjectResult, FleetListParams, FleetListResult, InitializeParams,
    PROTOCOL_VERSION, PeerInfo, PermissionOutcome, PermissionRequestResult, ProposeParams,
    ProposeResult, SessionListParams, SessionListResult, StatusResult, methods,
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
        let _ = write!(
            out,
            "\n  {word}  L{}  {:<id_width$}  {}",
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
