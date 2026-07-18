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
    InitializeParams, InitializeResult, PROTOCOL_VERSION, PeerInfo, PermissionChangedParams,
    PermissionDecideParams, PermissionDecideResult, PermissionPendingResult, SessionCancelParams,
    StatusResult, error_codes, methods,
};

use crate::pending::PendingQueue;
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
    /// Shared pending-permission queue (survives reconnection, multi-client).
    pub pending: PendingQueue,
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
            pending: PendingQueue::default(),
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

    // Forward pending-queue changes to this client as `permission/changed`,
    // so its tray reconciles to one snapshot without a round-trip. The
    // subscription only matters after `initialize`, but starting it now costs
    // nothing and avoids missing an early change.
    let mut queue_changed = state.pending.subscribe();

    loop {
        let message = tokio::select! {
            message = incoming.recv() => match message {
                Some(message) => message,
                None => break,
            },
            tick = queue_changed.recv() => {
                use tokio::sync::broadcast::error::RecvError;
                match tick {
                    // A tick (or a lag we recover from): push a fresh snapshot.
                    Ok(()) | Err(RecvError::Lagged(_)) => {
                        if initialized {
                            peer.notify(
                                methods::PERMISSION_CHANGED,
                                &PermissionChangedParams {
                                    pending: state.pending.snapshot().await,
                                },
                            );
                        }
                        continue;
                    }
                    // The queue is gone (daemon tearing down): stop forwarding.
                    Err(RecvError::Closed) => break,
                }
            }
        };
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
        methods::FLEET_LIST => crate::fleet::handle_fleet_list(params, state),
        methods::CONTEXT_PROJECT => handle_context_project(params).await,
        methods::SESSION_LIST => handle_session_list(params, state).await,
        methods::SESSION_LOG => handle_session_log(params, state).await,
        methods::PERMISSION_PENDING => handle_permission_pending(state).await,
        methods::PERMISSION_DECIDE => handle_permission_decide(params, state).await,
        other => Err(RpcError::method_not_found(other)),
    }
}

/// Default `session/log` page size when the client gives no limit.
const SESSION_LOG_PAGE: usize = 200;

/// `session/list`: active sessions (live state from the registry) plus the
/// historical ones from the per-project index, most recent first. A session
/// with no recorded end that is not active resolves to `interrupted` — a crash
/// leaves no ghost "active" sessions (sesiones-reanudables D1).
async fn handle_session_list(params: Value, state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    use meltemi_proto::{SessionInfo, SessionListParams, SessionListResult, SessionState};

    let params: SessionListParams = if params.is_null() {
        SessionListParams::default()
    } else {
        serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("session/list: {e}")))?
    };

    // Live state of currently-active sessions, keyed by id.
    let live: std::collections::HashMap<String, SessionState> = state
        .sessions
        .summaries()
        .await
        .into_iter()
        .map(|s| (s.session_id, s.state))
        .collect();

    let keys = match &params.project_root {
        Some(root) => vec![crate::paths::project_key(std::path::Path::new(root))],
        None => crate::session_index::all_project_keys(&state.data_dir),
    };

    let mut infos: Vec<SessionInfo> = Vec::new();
    for key in keys {
        for record in crate::session_index::records_for_project(&state.data_dir, &key) {
            let session_state = match live.get(&record.session_id) {
                Some(&live_state) => live_state,
                None if record.ended_at.is_some() => SessionState::Ended,
                None => SessionState::Interrupted,
            };
            infos.push(SessionInfo {
                session_id: record.session_id.clone(),
                agent_command: record.agent_command.clone(),
                project_root: record.project_root.clone(),
                state: session_state,
                final_status: record.final_status,
                started_at: record.started_at.clone(),
                ended_at: record.ended_at.clone(),
                resumable: record.resumable(),
            });
        }
    }

    if let Some(filter) = params.state {
        infos.retain(|i| i.state == filter);
    }
    infos.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    if let Some(limit) = params.limit {
        infos.truncate(limit as usize);
    }

    Ok(serde_json::to_value(SessionListResult { sessions: infos })
        .expect("SessionListResult serializes"))
}

/// `session/log`: a paginated slice of a session's JSONL log, so a thin client
/// never reads the daemon's disk. The session id must be a safe filename.
async fn handle_session_log(params: Value, state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    use meltemi_proto::{SessionLogParams, SessionLogResult};

    let params: SessionLogParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("session/log: {e}")))?;

    if !is_safe_session_id(&params.session_id) {
        return Err(RpcError::invalid_params("session/log: invalid session id"));
    }
    let key = crate::paths::project_key(std::path::Path::new(&params.project_root));
    let path = crate::session_index::sessions_dir(&state.data_dir, &key)
        .join(format!("{}.jsonl", params.session_id));

    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    let all: Vec<&str> = contents.lines().collect();
    let total = all.len();
    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.map(|l| l as usize).unwrap_or(SESSION_LOG_PAGE);
    let (start, end) = page_bounds(total, offset, limit);
    let lines: Vec<String> = all[start..end].iter().map(|s| (*s).to_string()).collect();

    let result = SessionLogResult {
        session_id: params.session_id,
        total: total as u32,
        offset: start as u32,
        lines,
    };
    Ok(serde_json::to_value(result).expect("SessionLogResult serializes"))
}

/// The clamped `[start, end)` line range for a `session/log` page: `offset` is
/// clamped into the log, and the window never runs past the end.
fn page_bounds(total: usize, offset: usize, limit: usize) -> (usize, usize) {
    let start = offset.min(total);
    let end = start.saturating_add(limit).min(total);
    (start, end)
}

/// Whether a session id is a safe filename component (no traversal): the id
/// the daemon mints is a UUID, so restrict to that alphabet.
fn is_safe_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `context/project`: compile the project's artifacts and write every declared
/// target inside its managed block, reporting each destination and fingerprint.
async fn handle_context_project(params: Value) -> Result<Value, RpcError> {
    let params: meltemi_proto::ContextProjectParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("context/project: {e}")))?;
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
    let written = crate::context::project_and_write(&project_root).map_err(RpcError::internal)?;
    let result = meltemi_proto::ContextProjectResult {
        targets: written.into_iter().map(Into::into).collect(),
    };
    Ok(serde_json::to_value(result).expect("ContextProjectResult serializes"))
}

/// `permission/pending`: the current queue snapshot (survives reconnection).
async fn handle_permission_pending(state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    let result = PermissionPendingResult {
        pending: state.pending.snapshot().await,
    };
    Ok(serde_json::to_value(result).expect("PermissionPendingResult serializes"))
}

/// `permission/decide`: resolve a pending request by id (first-wins), and
/// persist an accompanying rule when the client asked to remember the choice.
async fn handle_permission_decide(
    params: Value,
    state: &Arc<DaemonState>,
) -> Result<Value, RpcError> {
    let params: PermissionDecideParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("permission/decide: {e}")))?;

    // Persist the rule before resolving, so a crash cannot resolve-without-
    // remembering; the project root comes from the pending entry's session.
    if let Some(rule) = &params.persist_rule {
        let project_root = state.pending.project_root_of(&params.request_id).await;
        if let Err(e) =
            crate::permissions::persist_rule(&state.config_dir, project_root.as_deref(), rule)
        {
            tracing::warn!(error = %e, "failed to persist permission rule");
        }
    }

    // A losing race is not an error: the caller learns it via `status`.
    let status = state
        .pending
        .decide(&params.request_id, params.option_id)
        .await;
    let result = PermissionDecideResult { status };
    Ok(serde_json::to_value(result).expect("PermissionDecideResult serializes"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_log_paging_clamps_within_bounds() {
        // A page inside the log.
        assert_eq!(page_bounds(10, 2, 3), (2, 5));
        // The last page returns only the tail.
        assert_eq!(page_bounds(10, 9, 100), (9, 10));
        // An offset past the end yields an empty range, never a panic.
        assert_eq!(page_bounds(10, 50, 5), (10, 10));
        // A zero-length log pages to nothing.
        assert_eq!(page_bounds(0, 0, 200), (0, 0));
    }

    #[test]
    fn session_ids_reject_path_traversal() {
        assert!(is_safe_session_id("a1b2-c3d4-uuid"));
        assert!(is_safe_session_id("mock_session"));
        assert!(!is_safe_session_id(""));
        assert!(!is_safe_session_id("../secret"));
        assert!(!is_safe_session_id("a/b"));
        assert!(!is_safe_session_id("a\\b"));
        assert!(!is_safe_session_id("a.jsonl"));
    }
}
