// SPDX-License-Identifier: Apache-2.0

//! Connection handling and method dispatch for `meltemid`.
//!
//! Every connection must start with `initialize` (contract version
//! negotiation). An unsupported version is answered with application error
//! 1000 carrying both versions, and the connection is closed in an orderly
//! fashion; any other method before `initialize` is answered with 1001.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::mpsc;

use meltemi_proto::{
    InitializeParams, InitializeResult, PROTOCOL_VERSION, PeerInfo, SessionCancelParams,
    StatusResult, error_codes, methods,
};

use crate::rpc::{Incoming, Peer, RpcError};
use crate::session::SessionRegistry;
use crate::transport::Listener;

/// How long `shutdown` waits for agent sessions to terminate before exiting.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// Shared daemon state.
pub struct DaemonState {
    /// Daemon semantic version.
    pub version: &'static str,
    /// Start instant, for uptime reporting.
    pub started: Instant,
    /// User data directory (session logs live here).
    pub data_dir: PathBuf,
    /// User config directory (for the user-level config file).
    pub config_dir: PathBuf,
    /// Registry of active agent sessions.
    pub sessions: SessionRegistry,
    /// Signals the accept loop to begin the orderly shutdown.
    shutdown: mpsc::Sender<()>,
}

impl DaemonState {
    /// Builds shared state. `data_dir`/`config_dir` come from [`crate::paths`].
    pub fn new(data_dir: PathBuf, config_dir: PathBuf, shutdown: mpsc::Sender<()>) -> Arc<Self> {
        Arc::new(Self {
            version: env!("CARGO_PKG_VERSION"),
            started: Instant::now(),
            data_dir,
            config_dir,
            sessions: SessionRegistry::default(),
            shutdown,
        })
    }

    /// Convenience constructor for tests and tooling: data/config isolated
    /// under a unique temp subdirectory, so a test daemon never touches the
    /// user's real data.
    pub fn for_test(tag: &str, shutdown: mpsc::Sender<()>) -> Arc<Self> {
        let base =
            std::env::temp_dir().join(format!("meltemid-state-{}-{tag}", std::process::id()));
        Self::new(base.join("data"), base.join("config"), shutdown)
    }
}

/// Serves connections until `shutdown_rx` fires; then returns so the caller
/// can run the orderly cleanup.
pub async fn serve_until_shutdown(
    mut listener: Listener,
    state: Arc<DaemonState>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("shutdown requested; leaving accept loop");
                break;
            }
            accepted = listener.accept() => match accepted {
                Ok(stream) => {
                    let state = Arc::clone(&state);
                    tokio::spawn(async move {
                        handle_connection(stream, state).await;
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "accept failed");
                }
            }
        }
    }
}

async fn handle_connection(stream: crate::transport::Stream, state: Arc<DaemonState>) {
    let (peer, mut incoming) = Peer::start(stream);
    let mut initialized = false;

    while let Some(message) = incoming.recv().await {
        match message {
            Incoming::Request { id, method, params } => {
                if method == methods::INITIALIZE {
                    match handle_initialize(&params, &state) {
                        Ok(result) => {
                            initialized = true;
                            peer.respond(id, Ok(result));
                        }
                        Err(err) => {
                            let close = err.code == error_codes::PROTOCOL_VERSION_UNSUPPORTED;
                            peer.respond(id, Err(err));
                            if close {
                                // Orderly close: flush the error response, then
                                // drop the write half so the client sees EOF.
                                peer.close();
                                break;
                            }
                        }
                    }
                    continue;
                }
                if !initialized {
                    peer.respond(
                        id,
                        Err(RpcError::application(
                            error_codes::NOT_INITIALIZED,
                            "not initialized",
                            "not_initialized",
                            format!("`{method}` was called before a successful `initialize`"),
                            Some(
                                "Send `initialize` as the first request of the connection.".into(),
                            ),
                        )),
                    );
                    continue;
                }
                let state = Arc::clone(&state);
                let peer_for_task = peer.clone();
                tokio::spawn(async move {
                    let result = dispatch_request(&method, params, &state, &peer_for_task).await;
                    peer_for_task.respond(id, result);
                });
            }
            Incoming::Notification { method, params } => {
                if initialized {
                    dispatch_notification(&method, params, &state).await;
                } else {
                    tracing::warn!(method, "notification before initialize; ignored");
                }
            }
        }
    }
    tracing::debug!("connection closed");
}

fn handle_initialize(params: &Value, state: &DaemonState) -> Result<Value, RpcError> {
    let params: InitializeParams = serde_json::from_value(params.clone())
        .map_err(|e| RpcError::invalid_params(format!("initialize: {e}")))?;
    if params.protocol_version != PROTOCOL_VERSION {
        return Err(RpcError::application(
            error_codes::PROTOCOL_VERSION_UNSUPPORTED,
            "unsupported protocol version",
            "protocol_version_unsupported",
            format!(
                "client declared protocol version {}; this daemon supports [{}]",
                params.protocol_version, PROTOCOL_VERSION
            ),
            Some(format!(
                "Use a client that speaks protocol version {PROTOCOL_VERSION}."
            )),
        ));
    }
    tracing::info!(
        client = %params.client.name,
        client_version = %params.client.version,
        "client initialized"
    );
    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION,
        daemon: PeerInfo {
            name: "meltemid".into(),
            version: state.version.into(),
        },
    };
    Ok(serde_json::to_value(result).expect("InitializeResult serializes"))
}

async fn dispatch_request(
    method: &str,
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    match method {
        methods::STATUS => handle_status(state).await,
        methods::SHUTDOWN => handle_shutdown(state).await,
        methods::PROPOSE => crate::propose::handle_propose(params, state, peer).await,
        other => Err(RpcError::method_not_found(other)),
    }
}

async fn handle_status(state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    let result = StatusResult {
        daemon_version: state.version.to_string(),
        uptime_seconds: state.started.elapsed().as_secs(),
        sessions: state.sessions.summaries().await,
    };
    Ok(serde_json::to_value(result).expect("StatusResult serializes"))
}

async fn handle_shutdown(state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    tracing::info!("shutdown: terminating agent sessions");
    // Ask every session to cancel and terminate its agent subprocess.
    state.sessions.cancel_all().await;

    // Wait (bounded) for the ACP tasks to tear down and deregister, so no
    // orphan processes remain and each session log is closed and complete.
    let deadline = Instant::now() + SHUTDOWN_GRACE;
    while !state.sessions.is_empty().await {
        if Instant::now() >= deadline {
            tracing::warn!("shutdown grace elapsed with sessions still active");
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Break the accept loop; the process returns from `run` and exits after
    // this response is flushed.
    let _ = state.shutdown.send(()).await;
    Ok(serde_json::json!({}))
}

async fn dispatch_notification(method: &str, params: Value, state: &Arc<DaemonState>) {
    match method {
        methods::SESSION_CANCEL => match serde_json::from_value::<SessionCancelParams>(params) {
            Ok(cancel) => {
                let existed = state.sessions.cancel(&cancel.session_id).await;
                if !existed {
                    tracing::warn!(session_id = %cancel.session_id, "cancel for unknown session");
                }
            }
            Err(e) => tracing::warn!(error = %e, "malformed session/cancel params"),
        },
        other => tracing::warn!(method = other, "unknown notification; ignored"),
    }
}
