// SPDX-License-Identifier: Apache-2.0

//! The connection actor (design D5): a background task that keeps a live
//! connection to the daemon, reconnects with backoff, refreshes `status`,
//! forwards session events to the transcript, and surfaces pending permission
//! requests. It never auto-approves: interactive approval is the permission
//! tray (#9), and an unanswered request is denied by the daemon's timeout
//! (acp-session "cliente que no responde").

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::interval;

use meltemi_client::bootstrap;
use meltemi_client::rpc::{Incoming, Peer, RpcError};
use meltemi_proto::{
    ContextProjectParams, ContextProjectResult, FleetListParams, FleetListResult, InitializeParams,
    PROTOCOL_VERSION, PeerInfo, PermissionChangedParams, PermissionDecideParams,
    PermissionPendingResult, PermissionRule, SessionCancelParams, SessionListParams,
    SessionListResult, SessionLogParams, SessionLogResult, StatusResult, methods,
};

use crate::shell::live::{FleetRow, FleetSnapshot, ProjectRow, SessionRow, Update};
use crate::shell::render::ConnState;

/// A command from the UI to the connection actor.
#[derive(Debug, Clone)]
pub enum Command {
    /// Cancel a session by id (`session/cancel`).
    CancelSession(String),
    /// Shut the daemon down (`shutdown`).
    Shutdown,
    /// Force a status refresh.
    Refresh,
    /// Query the fleet catalog (`fleet/list`).
    FleetList,
    /// Query the known-project registry (`project/list`).
    ProjectList,
    /// Set the project every scoped call is made against; `None` returns to the
    /// working directory (multiproyecto-suscripciones D6).
    SetScope(Option<String>),
    /// Regenerate the projected context (`context/project`).
    ProjectContext,
    /// Fetch a historical session's log (`session/log`) for the detail view.
    FetchSessionLog {
        session_id: String,
        project_root: String,
    },
    /// Resolve a pending permission by id (`permission/decide`), optionally
    /// persisting a rule ("allow/deny always").
    DecidePermission {
        request_id: String,
        option_id: Option<String>,
        persist_rule: Option<PermissionRule>,
    },
}

const MIN_BACKOFF: Duration = Duration::from_millis(200);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
const REFRESH_EVERY: Duration = Duration::from_secs(2);

/// Runs the connection actor until the UI drops its command sender.
pub async fn connection_actor(
    endpoint: String,
    mut commands: UnboundedReceiver<Command>,
    updates: UnboundedSender<Update>,
) {
    let mut backoff = MIN_BACKOFF;
    loop {
        let _ = updates.send(Update::Conn(ConnState::Connecting));
        let stream = match bootstrap::connect_or_start(&endpoint).await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = updates.send(Update::Conn(ConnState::Unreachable {
                    detail: error.to_string(),
                }));
                if backoff_or_stop(&mut commands, backoff).await {
                    return;
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        backoff = MIN_BACKOFF;

        // A dropped UI sender ends the actor entirely; a dropped connection just
        // reconnects.
        match serve_connection(stream, &mut commands, &updates).await {
            ConnExit::UiGone => return,
            ConnExit::Disconnected => continue,
        }
    }
}

enum ConnExit {
    /// The UI dropped its command sender: stop the actor.
    UiGone,
    /// The connection closed: reconnect.
    Disconnected,
}

async fn serve_connection(
    stream: meltemi_client::transport::Stream,
    commands: &mut UnboundedReceiver<Command>,
    updates: &UnboundedSender<Update>,
) -> ConnExit {
    let (peer, mut incoming) = Peer::start(stream);

    if peer
        .request(methods::INITIALIZE, &init_params())
        .await
        .is_err()
    {
        let _ = updates.send(Update::Conn(ConnState::Unreachable {
            detail: "initialize failed".into(),
        }));
        peer.close();
        return ConnExit::Disconnected;
    }
    refresh_status(&peer, updates).await;
    refresh_sessions(&peer, updates).await;
    // Seed the tray from the daemon's queue so it survives reconnection
    // (the count no longer lives per-connection).
    refresh_pending(&peer, updates).await;

    let mut ticker = interval(REFRESH_EVERY);
    ticker.tick().await; // consume the immediate first tick

    // The project every scoped call is made against; `None` means the working
    // directory the surface was started in.
    let mut scope: Option<String> = None;
    loop {
        tokio::select! {
            message = incoming.recv() => match message {
                None => {
                    peer.close();
                    return ConnExit::Disconnected;
                }
                Some(Incoming::Request { id, method, params }) if method == methods::PERMISSION_REQUEST => {
                    // The request also entered the daemon's queue (permission/
                    // changed drives the tray and the counter). We hold the
                    // live push unanswered and resolve via `permission/decide`
                    // from the tray; an unanswered push is denied by timeout.
                    let _ = updates.send(Update::Notice(permission_notice(&params)));
                    let _ = id;
                }
                Some(Incoming::Request { id, method, .. }) => {
                    peer.respond(id, Err(RpcError::method_not_found(&method)));
                }
                Some(Incoming::Notification { method, params }) if method == methods::PERMISSION_CHANGED => {
                    if let Ok(changed) = serde_json::from_value::<PermissionChangedParams>(params) {
                        let _ = updates.send(Update::PermissionQueue(changed.pending));
                    }
                }
                Some(Incoming::Notification { method, .. }) if method == methods::PERMISSION_TIMEOUT => {
                    let _ = updates.send(Update::Notice("permiso vencido: denegado por plazo".into()));
                }
                Some(Incoming::Notification { method, params }) if method == methods::SESSION_EVENT => {
                    if let Some(line) = summarize_event(&params) {
                        let session_id = params
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        let _ = updates.send(Update::TranscriptLine { session_id, line });
                    }
                }
                Some(Incoming::Notification { .. }) => {}
            },
            command = commands.recv() => match command {
                None => {
                    peer.close();
                    return ConnExit::UiGone;
                }
                Some(Command::CancelSession(session_id)) => {
                    peer.notify(methods::SESSION_CANCEL, &SessionCancelParams { session_id });
                }
                Some(Command::Shutdown) => {
                    let _ = peer.request(methods::SHUTDOWN, &json!({})).await;
                }
                Some(Command::Refresh) => {
                    refresh_status(&peer, updates).await;
                    refresh_sessions(&peer, updates).await;
                }
                Some(Command::FleetList) => refresh_fleet(&peer, updates, scope.as_deref()).await,
                Some(Command::ProjectList) => refresh_projects(&peer, updates).await,
                Some(Command::SetScope(root)) => {
                    scope = root;
                    // The scope changed: re-answer what depends on it.
                    refresh_fleet(&peer, updates, scope.as_deref()).await;
                }
                Some(Command::ProjectContext) => {
                    project_context(&peer, updates, scope.as_deref()).await
                }
                Some(Command::FetchSessionLog { session_id, project_root }) => {
                    fetch_session_log(&peer, updates, &session_id, &project_root).await;
                }
                Some(Command::DecidePermission { request_id, option_id, persist_rule }) => {
                    let params = PermissionDecideParams { request_id, option_id, persist_rule };
                    // The daemon broadcasts permission/changed on resolution;
                    // refresh too so a lost broadcast still updates the tray.
                    let _ = peer.request(methods::PERMISSION_DECIDE, &params).await;
                    refresh_pending(&peer, updates).await;
                }
            },
            _ = ticker.tick() => {
                refresh_status(&peer, updates).await;
                refresh_sessions(&peer, updates).await;
            }
        }
    }
}

/// Sleeps for `backoff`, returning `true` if the UI went away meanwhile.
async fn backoff_or_stop(commands: &mut UnboundedReceiver<Command>, backoff: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(backoff) => false,
        command = commands.recv() => command.is_none(),
    }
}

async fn refresh_status(peer: &Peer, updates: &UnboundedSender<Update>) {
    match peer.request(methods::STATUS, &json!({})).await {
        Ok(value) => {
            if let Ok(status) = serde_json::from_value::<StatusResult>(value) {
                let _ = updates.send(Update::Conn(ConnState::Connected {
                    version: status.daemon_version,
                    uptime_s: status.uptime_seconds,
                    sessions: status.sessions.len(),
                }));
            }
        }
        Err(error) => {
            let _ = updates.send(Update::Conn(ConnState::Unreachable {
                detail: error.to_string(),
            }));
        }
    }
}

/// Populates the Sessions table from `session/list` (active and historical) for
/// the current project, so the table shows history and survives reconnection.
async fn refresh_sessions(peer: &Peer, updates: &UnboundedSender<Update>) {
    // Unfiltered on purpose (multiproyecto-suscripciones D7): one query brings
    // every session with its own root, and the shell groups by project.
    let params = SessionListParams::default();
    if let Ok(value) = peer.request(methods::SESSION_LIST, &params).await
        && let Ok(result) = serde_json::from_value::<SessionListResult>(value)
    {
        let rows = result.sessions.into_iter().map(SessionRow::from).collect();
        let _ = updates.send(Update::Sessions(rows));
    }
}

/// Queries `project/list` and pushes the known-project registry, so the
/// Sessions view can group by project and mark a root that vanished.
async fn refresh_projects(peer: &Peer, updates: &UnboundedSender<Update>) {
    let params = meltemi_proto::ProjectListParams::default();
    match peer.request(methods::PROJECT_LIST, &params).await {
        Ok(value) => {
            if let Ok(result) = serde_json::from_value::<meltemi_proto::ProjectListResult>(value) {
                let rows = result.projects.into_iter().map(ProjectRow::from).collect();
                let _ = updates.send(Update::Projects(rows));
            }
        }
        Err(error) => {
            let _ = updates.send(Update::Notice(format!("project/list: {error}")));
        }
    }
}

/// Fetches a historical session's log (tail page) and forwards a summarized
/// transcript for the detail view.
async fn fetch_session_log(
    peer: &Peer,
    updates: &UnboundedSender<Update>,
    session_id: &str,
    project_root: &str,
) {
    let params = SessionLogParams {
        project_root: project_root.to_string(),
        session_id: session_id.to_string(),
        offset: None,
        limit: None,
    };
    if let Ok(value) = peer.request(methods::SESSION_LOG, &params).await
        && let Ok(result) = serde_json::from_value::<SessionLogResult>(value)
    {
        let lines = result.lines.iter().map(|l| summarize_log_line(l)).collect();
        let _ = updates.send(Update::SessionLog {
            session_id: result.session_id,
            lines,
        });
    }
}

/// Summarizes one raw JSONL session-event line into a transcript row.
fn summarize_log_line(line: &str) -> String {
    match serde_json::from_str::<Value>(line) {
        Ok(event) => {
            let kind = event.get("type").and_then(Value::as_str).unwrap_or("event");
            let ts = event.get("ts").and_then(Value::as_str).unwrap_or("");
            format!("{ts}  {kind}")
        }
        Err(_) => line.to_string(),
    }
}

/// Queries `fleet/list` and pushes the snapshot. The current directory names
/// the project whose config marks the configured agent.
async fn refresh_fleet(peer: &Peer, updates: &UnboundedSender<Update>, scope: Option<&str>) {
    let params = FleetListParams {
        project_root: scope.map(str::to_string).or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|root| root.display().to_string())
        }),
    };
    match peer.request(methods::FLEET_LIST, &params).await {
        Ok(value) => {
            if let Ok(result) = serde_json::from_value::<FleetListResult>(value) {
                let _ = updates.send(Update::Fleet(FleetSnapshot {
                    registry_version: result.registry_version,
                    rows: result.agents.into_iter().map(FleetRow::from).collect(),
                }));
            }
        }
        Err(error) => {
            let _ = updates.send(Update::Notice(format!("fleet/list: {error}")));
        }
    }
}

/// Regenerates the projected context and reports the result as a notice.
async fn project_context(peer: &Peer, updates: &UnboundedSender<Update>, scope: Option<&str>) {
    let root = match scope {
        Some(root) => root.to_string(),
        None => match std::env::current_dir() {
            Ok(cwd) => cwd.display().to_string(),
            Err(_) => return,
        },
    };
    let params = ContextProjectParams { project_root: root };
    match peer.request(methods::CONTEXT_PROJECT, &params).await {
        Ok(value) => {
            if let Ok(result) = serde_json::from_value::<ContextProjectResult>(value) {
                let written = result.targets.iter().filter(|t| t.written).count();
                let _ = updates.send(Update::Notice(format!(
                    "proyección: {} destino(s), {written} escritos",
                    result.targets.len()
                )));
            }
        }
        Err(error) => {
            let _ = updates.send(Update::Notice(format!("context/project: {error}")));
        }
    }
}

/// Queries `permission/pending` and pushes the tray snapshot. Used on connect
/// (seed) and after a decide (belt-and-suspenders vs the broadcast).
async fn refresh_pending(peer: &Peer, updates: &UnboundedSender<Update>) {
    if let Ok(value) = peer.request(methods::PERMISSION_PENDING, &json!({})).await
        && let Ok(result) = serde_json::from_value::<PermissionPendingResult>(value)
    {
        let _ = updates.send(Update::PermissionQueue(result.pending));
    }
}

fn init_params() -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: "meltemi".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    }
}

/// A one-line summary of a session event for the transcript.
fn summarize_event(params: &Value) -> Option<String> {
    let session = params
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let kind = params
        .get("event")
        .and_then(|e| e.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("event");
    Some(format!("[{session}] {kind}"))
}

/// A labeled notice for an incoming permission request.
fn permission_notice(params: &Value) -> String {
    let session = params
        .get("sessionId")
        .and_then(Value::as_str)
        .unwrap_or("?");
    format!("[{session}] permiso solicitado — aprobación interactiva llega en la bandeja (#9)")
}
