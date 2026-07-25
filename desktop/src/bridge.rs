// SPDX-License-Identifier: Apache-2.0

//! The daemon bridge (gui-tauri-paridad 1.4, design D1): a background actor
//! that owns the only connection to `meltemid` — connect-or-start, mandatory
//! `initialize`, reconnection with backoff — and serves the webview through
//! two channels: commands in (RPC passthrough) and events out (connection
//! state, daemon-initiated traffic). The webview never touches a socket.
//!
//! Shape mirrors the TUI connection actor (`tui/src/shell/conn.rs`) so both
//! surfaces give the same guarantees: noisy disconnection, backoff, and
//! permission pushes held unanswered (the daemon's timeout denies; the tray
//! resolves via `permission/decide`).

use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use meltemi_client::bootstrap;
use meltemi_client::rpc::{Incoming, Peer, RpcError};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, StatusResult, methods};

const MIN_BACKOFF: Duration = Duration::from_millis(200);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
const REFRESH_EVERY: Duration = Duration::from_secs(2);

/// Connection state pushed to the webview as the `daemon:state` event.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ConnState {
    Connecting,
    #[serde(rename_all = "camelCase")]
    Connected {
        version: String,
        uptime_seconds: u64,
        sessions: usize,
        endpoint: String,
    },
    /// The daemon cannot be reached or started. Carries the endpoint so the
    /// surface can show the socket path with the diagnosis (gui-shell
    /// "Fallo de arranque diagnosticado").
    #[serde(rename_all = "camelCase")]
    Unreachable {
        detail: String,
        endpoint: String,
    },
}

/// An event pushed from the actor to the host (the Tauri layer forwards it to
/// the webview; tests consume it directly).
#[derive(Debug, Clone, Serialize)]
pub enum BridgeEvent {
    State(ConnState),
    /// A daemon-initiated notification or held permission push, forwarded
    /// verbatim: `{method, params}`.
    Incoming {
        method: String,
        params: Value,
    },
}

/// A structured error for the webview: the contract's `ErrorData` when the
/// daemon answered, or a local `unreachable` when it could not be reached.
/// Unlike the CLI (which flattens to exit codes), the surface gets the remedy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    /// JSON-RPC error code when the daemon answered; absent for local errors.
    pub code: Option<i64>,
    pub message: String,
    /// `unreachable` for local connection failures, the contract's
    /// `data.kind` when present, `contract` otherwise.
    pub kind: String,
    pub detail: Option<String>,
    pub remedy: Option<String>,
}

impl BridgeError {
    pub fn unreachable(detail: impl Into<String>) -> Self {
        Self {
            code: None,
            message: "daemon unreachable".into(),
            kind: "unreachable".into(),
            detail: Some(detail.into()),
            remedy: None,
        }
    }
}

impl From<RpcError> for BridgeError {
    fn from(error: RpcError) -> Self {
        let get = |key: &str| {
            error
                .data
                .as_ref()
                .and_then(|d| d.get(key))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        Self {
            code: Some(error.code),
            kind: get("kind").unwrap_or_else(|| "contract".into()),
            detail: get("detail"),
            remedy: get("remedy"),
            message: error.message,
        }
    }
}

/// A command from the host into the actor.
#[derive(Debug)]
pub enum BridgeCommand {
    /// Generic RPC passthrough: send `method`/`params` to the daemon and
    /// reply with its result. Requests run concurrently; a slow turn never
    /// blocks the actor.
    Request {
        method: String,
        params: Value,
        reply: oneshot::Sender<Result<Value, BridgeError>>,
    },
    /// Fire-and-forget notification (e.g. `session/cancel`). Lost while
    /// disconnected — there is nothing meaningful to notify then.
    Notify { method: String, params: Value },
}

/// Runs the bridge until the host drops its command sender.
pub async fn bridge_actor(
    endpoint: String,
    mut commands: mpsc::UnboundedReceiver<BridgeCommand>,
    events: mpsc::UnboundedSender<BridgeEvent>,
) {
    let mut backoff = MIN_BACKOFF;
    loop {
        let _ = events.send(BridgeEvent::State(ConnState::Connecting));
        let stream = match bootstrap::connect_or_start(&endpoint).await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
                    detail: error.to_string(),
                    endpoint: endpoint.clone(),
                }));
                if drain_or_stop(&mut commands, backoff, &error.to_string()).await {
                    return;
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        backoff = MIN_BACKOFF;

        match serve_connection(stream, &mut commands, &events, &endpoint).await {
            ConnExit::HostGone => return,
            ConnExit::Disconnected => continue,
        }
    }
}

enum ConnExit {
    HostGone,
    Disconnected,
}

async fn serve_connection(
    stream: meltemi_client::transport::Stream,
    commands: &mut mpsc::UnboundedReceiver<BridgeCommand>,
    events: &mpsc::UnboundedSender<BridgeEvent>,
    endpoint: &str,
) -> ConnExit {
    let (peer, mut incoming) = Peer::start(stream);

    if let Err(error) = peer.request(methods::INITIALIZE, &init_params()).await {
        let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
            detail: format!("initialize failed: {error}"),
            endpoint: endpoint.to_string(),
        }));
        peer.close();
        return ConnExit::Disconnected;
    }
    push_status(&peer, events, endpoint).await;

    let mut ticker = tokio::time::interval(REFRESH_EVERY);
    ticker.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            message = incoming.recv() => match message {
                None => {
                    peer.close();
                    return ConnExit::Disconnected;
                }
                Some(Incoming::Request { id, method, params }) if method == methods::PERMISSION_REQUEST => {
                    // Held unanswered on purpose: the daemon's queue drives the
                    // tray (permission/changed) and its timeout denies; the tray
                    // resolves via permission/decide. Forward for visibility.
                    let _ = events.send(BridgeEvent::Incoming { method, params });
                    let _ = id;
                }
                Some(Incoming::Request { id, method, .. }) => {
                    peer.respond(id, Err(RpcError::method_not_found(&method)));
                }
                Some(Incoming::Notification { method, params }) => {
                    let _ = events.send(BridgeEvent::Incoming { method, params });
                }
            },
            command = commands.recv() => match command {
                None => {
                    peer.close();
                    return ConnExit::HostGone;
                }
                Some(BridgeCommand::Request { method, params, reply }) => {
                    // Concurrent by design: a long-running turn must not block
                    // the ticker or other requests.
                    let peer = peer.clone();
                    tokio::spawn(async move {
                        let result = peer
                            .request(&method, &params)
                            .await
                            .map_err(BridgeError::from);
                        let _ = reply.send(result);
                    });
                }
                Some(BridgeCommand::Notify { method, params }) => {
                    peer.notify(&method, &params);
                }
            },
            _ = ticker.tick() => push_status(&peer, events, endpoint).await,
        }
    }
}

/// Sleeps for `backoff`; requests arriving meanwhile fail fast as
/// unreachable. Returns `true` when the host dropped its sender.
async fn drain_or_stop(
    commands: &mut mpsc::UnboundedReceiver<BridgeCommand>,
    backoff: Duration,
    detail: &str,
) -> bool {
    let sleep = tokio::time::sleep(backoff);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => return false,
            command = commands.recv() => match command {
                None => return true,
                Some(BridgeCommand::Request { reply, .. }) => {
                    let _ = reply.send(Err(BridgeError::unreachable(detail)));
                }
                Some(BridgeCommand::Notify { .. }) => {}
            },
        }
    }
}

async fn push_status(peer: &Peer, events: &mpsc::UnboundedSender<BridgeEvent>, endpoint: &str) {
    match peer.request(methods::STATUS, &json!({})).await {
        Ok(value) => {
            if let Ok(status) = serde_json::from_value::<StatusResult>(value) {
                let _ = events.send(BridgeEvent::State(ConnState::Connected {
                    version: status.daemon_version,
                    uptime_seconds: status.uptime_seconds,
                    sessions: status.sessions.len(),
                    endpoint: endpoint.to_string(),
                }));
            }
        }
        Err(error) => {
            let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
                detail: error.to_string(),
                endpoint: endpoint.to_string(),
            }));
        }
    }
}

fn init_params() -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: "meltemi-desktop".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    }
}
