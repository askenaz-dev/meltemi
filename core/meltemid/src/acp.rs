// SPDX-License-Identifier: Apache-2.0

//! ACP layer (tasks 4.1-4.5): drive an external agent over the Agent Client
//! Protocol and bridge it to the connected Meltemi client.
//!
//! `meltemid` acts as an ACP *client*: it launches the configured agent
//! binary as a subprocess (the official crate spawns it and kills it on drop,
//! so no orphan survives the turn), runs `initialize` → `session/new` →
//! `session/prompt`, and:
//!
//! - forwards every agent `session/update` to the Meltemi client as a
//!   `session/event` notification, appending it to the session log (4.3);
//! - passes every agent `session/request_permission` through to the Meltemi
//!   client and returns its decision, denying by default when the client is
//!   gone or does not answer in time (4.4);
//! - cancels the ACP prompt when the Meltemi client cancels the session,
//!   returning the agent's `Cancelled` stop reason (4.5).
//!
//! The official binary runs with the agent's own authentication; Meltemi
//! never reads or forwards credentials (constitution §2, juego limpio).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{Mutex, Notify};

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, InitializeRequest, NewSessionRequest,
    PermissionOption as AcpPermissionOption, PermissionOptionKind as AcpPermissionOptionKind,
    PromptRequest, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, StopReason, TextContent,
};
use agent_client_protocol::{AcpAgent, Agent, Client, ConnectionTo};

use meltemi_proto::{
    PermissionDecidedBy, PermissionOption, PermissionOptionKind, PermissionOutcome,
    PermissionRequestParams, PermissionRequestResult, PermissionTimeoutParams, SessionEventKind,
    SessionEventParams, TurnStatus, methods,
};

use crate::rpc::Peer;
use crate::session_log::SessionLog;

/// Everything one ACP-driven turn needs.
pub struct SessionParams {
    /// The official agent binary and arguments (program first).
    pub agent_command: Vec<String>,
    /// Absolute repository root the agent session works in.
    pub project_root: PathBuf,
    /// The prompt to send to the agent.
    pub prompt: String,
    /// The Meltemi session id (used in events forwarded to the client).
    pub meltemi_session_id: String,
    /// Connection to the Meltemi client, for forwarding events and permissions.
    pub peer: Peer,
    /// The session's append-only log.
    pub log: Arc<Mutex<SessionLog>>,
    /// Fired by the daemon when the client cancels this session.
    pub cancel: Arc<Notify>,
    /// How long to wait for the client to answer a permission request.
    pub permission_timeout: Duration,
}

/// Runs one ACP turn end to end and returns the mapped stop reason.
pub async fn run_session(params: SessionParams) -> anyhow::Result<TurnStatus> {
    let SessionParams {
        agent_command,
        project_root,
        prompt,
        meltemi_session_id,
        peer,
        log,
        cancel,
        permission_timeout,
    } = params;

    let agent = AcpAgent::from_args(&agent_command)
        .map_err(|e| anyhow::anyhow!("invalid agent command: {e:?}"))?;

    // Handler state (the connect_with closure keeps the originals).
    let notif = HandlerState {
        peer: peer.clone(),
        log: log.clone(),
        session_id: meltemi_session_id.clone(),
    };
    let perm = HandlerState {
        peer: peer.clone(),
        log: log.clone(),
        session_id: meltemi_session_id.clone(),
    };

    let result = Client
        .builder()
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                forward_update(&notif, notification).await;
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _cx| {
                let outcome = passthrough_permission(&perm, &request, permission_timeout).await;
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, async move |connection: ConnectionTo<Agent>| {
            // Handshake and session creation.
            connection
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let new_session = connection
                .send_request(NewSessionRequest::new(project_root))
                .block_task()
                .await?;
            let acp_session_id = new_session.session_id;

            {
                let mut log = log.lock().await;
                let _ = log.append(SessionEventKind::PromptSent {
                    text: prompt.clone(),
                });
            }

            // Send the prompt, racing the client's cancellation. On cancel we
            // forward an ACP session/cancel and let the agent drain the turn
            // to its Cancelled stop reason.
            let prompt_request = PromptRequest::new(
                acp_session_id.clone(),
                vec![ContentBlock::Text(TextContent::new(prompt))],
            );
            let prompt_future = connection.send_request(prompt_request).block_task();
            tokio::pin!(prompt_future);
            let response = tokio::select! {
                r = &mut prompt_future => r?,
                _ = cancel.notified() => {
                    let _ = connection.send_notification(CancelNotification::new(acp_session_id));
                    (&mut prompt_future).await?
                }
            };

            Ok(map_stop_reason(response.stop_reason))
        })
        .await;

    result.map_err(|e| anyhow::anyhow!("acp session failed: {e:?}"))
}

/// Shared, cheaply-cloneable state the ACP callbacks need.
struct HandlerState {
    peer: Peer,
    log: Arc<Mutex<SessionLog>>,
    session_id: String,
}

/// Logs an agent update and forwards it to the Meltemi client (4.3).
async fn forward_update(state: &HandlerState, notification: SessionNotification) {
    let update = serde_json::to_value(&notification.update).unwrap_or(Value::Null);
    let event = {
        let mut log = state.log.lock().await;
        log.append(SessionEventKind::AgentUpdate { update }).ok()
    };
    if let Some(event) = event {
        state.peer.notify(
            methods::SESSION_EVENT,
            &SessionEventParams {
                session_id: state.session_id.clone(),
                event,
            },
        );
    }
}

/// Forwards a permission request to the client and returns the ACP outcome,
/// denying by default if the client is gone or does not answer in time (4.4).
async fn passthrough_permission(
    state: &HandlerState,
    request: &RequestPermissionRequest,
    timeout: Duration,
) -> RequestPermissionOutcome {
    {
        let mut log = state.log.lock().await;
        let _ = log.append(SessionEventKind::PermissionRequested {
            request: serde_json::to_value(request).unwrap_or(Value::Null),
        });
    }

    let params = PermissionRequestParams {
        session_id: state.session_id.clone(),
        tool_call: serde_json::to_value(&request.tool_call).unwrap_or(Value::Null),
        options: request.options.iter().map(to_meltemi_option).collect(),
    };

    let (outcome, decided_by) = match tokio::time::timeout(
        timeout,
        state.peer.request(methods::PERMISSION_REQUEST, &params),
    )
    .await
    {
        Ok(Ok(value)) => match serde_json::from_value::<PermissionRequestResult>(value) {
            Ok(result) => (to_acp_outcome(result.outcome), PermissionDecidedBy::Client),
            Err(_) => (
                default_deny(&request.options),
                PermissionDecidedBy::DefaultDeny,
            ),
        },
        // The client connection is gone: deny by default.
        Ok(Err(_)) => (
            default_deny(&request.options),
            PermissionDecidedBy::DefaultDeny,
        ),
        // The client did not answer in time: notify it and deny.
        Err(_) => {
            state.peer.notify(
                methods::PERMISSION_TIMEOUT,
                &PermissionTimeoutParams {
                    session_id: state.session_id.clone(),
                    tool_call_id: None,
                },
            );
            (default_deny(&request.options), PermissionDecidedBy::Timeout)
        }
    };

    {
        let mut log = state.log.lock().await;
        let _ = log.append(SessionEventKind::PermissionDecided {
            outcome: serde_json::to_value(&outcome).unwrap_or(Value::Null),
            decided_by,
        });
    }
    outcome
}

/// Picks a rejecting option to deny with, falling back to cancelling the turn
/// when the agent offered no reject option.
fn default_deny(options: &[AcpPermissionOption]) -> RequestPermissionOutcome {
    for option in options {
        if is_reject(option.kind) {
            return RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                option.option_id.clone(),
            ));
        }
    }
    RequestPermissionOutcome::Cancelled
}

fn is_reject(kind: AcpPermissionOptionKind) -> bool {
    matches!(
        kind,
        AcpPermissionOptionKind::RejectOnce | AcpPermissionOptionKind::RejectAlways
    )
}

/// Maps the client's decision back to an ACP outcome.
fn to_acp_outcome(outcome: PermissionOutcome) -> RequestPermissionOutcome {
    match outcome {
        PermissionOutcome::Cancelled => RequestPermissionOutcome::Cancelled,
        PermissionOutcome::Selected { option_id } => {
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id))
        }
    }
}

/// Maps an ACP permission option into the Meltemi contract shape.
fn to_meltemi_option(option: &AcpPermissionOption) -> PermissionOption {
    PermissionOption {
        option_id: option.option_id.to_string(),
        name: option.name.clone(),
        kind: to_meltemi_kind(option.kind),
    }
}

fn to_meltemi_kind(kind: AcpPermissionOptionKind) -> PermissionOptionKind {
    match kind {
        AcpPermissionOptionKind::AllowOnce => PermissionOptionKind::AllowOnce,
        AcpPermissionOptionKind::AllowAlways => PermissionOptionKind::AllowAlways,
        AcpPermissionOptionKind::RejectOnce => PermissionOptionKind::RejectOnce,
        // Unknown future kinds are treated as a one-time rejection.
        _ => PermissionOptionKind::RejectAlways,
    }
}

fn map_stop_reason(reason: StopReason) -> TurnStatus {
    match reason {
        StopReason::EndTurn => TurnStatus::Completed,
        StopReason::Cancelled => TurnStatus::Cancelled,
        StopReason::Refusal => TurnStatus::Refused,
        StopReason::MaxTokens => TurnStatus::MaxTokens,
        StopReason::MaxTurnRequests => TurnStatus::MaxTurnRequests,
        _ => TurnStatus::Completed,
    }
}
