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
    InitializeParams, PROTOCOL_VERSION, PeerInfo, PermissionOutcome, PermissionRequestResult,
    ProposeParams, ProposeResult, StatusResult, methods,
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
pub(crate) async fn connect_and_init(endpoint: &str) -> Result<(Peer, JoinHandle<()>), CliError> {
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
    let human = format!(
        "proposed `{}` [{:?}]\n{}",
        result.change_name, result.status, result.proposal_path
    );
    Ok(Outcome { human, json: value })
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
