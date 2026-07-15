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

use meltemi_proto::{
    InitializeParams, PROTOCOL_VERSION, PeerInfo, SessionCancelParams, StatusResult, methods,
};
use meltemid::bootstrap;
use meltemid::rpc::{Incoming, Peer, RpcError};

use crate::shell::live::{SessionRow, Update};
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
    stream: meltemid::transport::Stream,
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

    let mut pending: usize = 0;
    let mut ticker = interval(REFRESH_EVERY);
    ticker.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            message = incoming.recv() => match message {
                None => {
                    peer.close();
                    return ConnExit::Disconnected;
                }
                Some(Incoming::Request { id, method, params }) if method == methods::PERMISSION_REQUEST => {
                    // Surface the request; do not answer (interactive approval is #9).
                    pending += 1;
                    let _ = updates.send(Update::Pending(pending));
                    let _ = updates.send(Update::Notice(permission_notice(&params)));
                    let _ = id; // held: the daemon's timeout denies if unanswered
                }
                Some(Incoming::Request { id, method, .. }) => {
                    peer.respond(id, Err(RpcError::method_not_found(&method)));
                }
                Some(Incoming::Notification { method, .. }) if method == methods::PERMISSION_TIMEOUT => {
                    pending = pending.saturating_sub(1);
                    let _ = updates.send(Update::Pending(pending));
                    let _ = updates.send(Update::Notice("permiso vencido: denegado por plazo".into()));
                }
                Some(Incoming::Notification { method, params }) if method == methods::SESSION_EVENT => {
                    if let Some(line) = summarize_event(&params) {
                        let _ = updates.send(Update::TranscriptLine(line));
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
                Some(Command::Refresh) => refresh_status(&peer, updates).await,
            },
            _ = ticker.tick() => refresh_status(&peer, updates).await,
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
                let rows = status.sessions.into_iter().map(SessionRow::from).collect();
                let _ = updates.send(Update::Sessions(rows));
            }
        }
        Err(error) => {
            let _ = updates.send(Update::Conn(ConnState::Unreachable {
                detail: error.to_string(),
            }));
        }
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
