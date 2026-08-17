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
    /// Count of initialized client connections, observed by the permission
    /// wait (espera-humana D3).
    pub clients: crate::clients::ClientRegistry,
    /// Session events published for every interested connection
    /// (eventos-para-tardios).
    pub events: crate::events::EventHub,
    /// Per-tree activity and pending human-edit notes (edit-surface soft lock,
    /// gui-tauri-paridad design D4/D5).
    pub edits: Arc<crate::edits::EditHub>,
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
            clients: crate::clients::ClientRegistry::default(),
            events: crate::events::EventHub::default(),
            edits: Arc::default(),
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
    // Counts this connection as a client once it initializes; dropping the
    // guard — however the connection ends — keeps the count honest, which is
    // what the permission wait observes (espera-humana D3).
    let mut client_guard: Option<crate::clients::ClientGuard> = None;
    // Sessions this connection asked to watch (`session/watch`). Its own
    // sessions need no entry — the hub seals every event with the connection
    // that originated it (eventos-para-tardios D1).
    let mut watched: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut session_events = state.events.subscribe();

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
            event = session_events.recv() => {
                use tokio::sync::broadcast::error::RecvError;
                match event {
                    Ok(envelope) => {
                        if initialized
                            && crate::events::delivers(&envelope, peer.connection_id(), &watched)
                        {
                            peer.notify(methods::SESSION_EVENT, envelope.params.as_ref());
                        }
                        continue;
                    }
                    // Fell behind the bounded buffer: events were missed. The
                    // complete, paginated source is `session/log` (D4); say so
                    // once rather than pretend the stream was whole.
                    Err(RecvError::Lagged(missed)) => {
                        tracing::warn!(missed, "session event stream lagged; read session/log");
                        continue;
                    }
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
                            if client_guard.is_none() {
                                client_guard = Some(state.clients.register());
                            }
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
                // `session/watch` is the one method whose state is the
                // CONNECTION's, not the daemon's, so it is answered here
                // rather than in the shared dispatcher (eventos-para-tardios).
                if method == methods::SESSION_WATCH {
                    peer.respond(id, watch_session(params, &mut watched));
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

/// `session/watch`: declare (or drop) this connection's interest in a
/// session's live event stream. Watching an unknown or finished session is not
/// an error — the client may be reconnecting around a session that just ended,
/// and a refusal would say nothing it cannot read from `session/list`.
fn watch_session(
    params: Value,
    watched: &mut std::collections::HashSet<String>,
) -> Result<Value, RpcError> {
    let params: meltemi_proto::SessionWatchParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("session/watch: {e}")))?;
    if params.watch {
        watched.insert(params.session_id.clone());
    } else {
        watched.remove(&params.session_id);
    }
    Ok(serde_json::to_value(meltemi_proto::SessionWatchResult {
        session_id: params.session_id,
        watching: params.watch,
    })
    .expect("SessionWatchResult serializes"))
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
        methods::CONTEXT_PROJECT => handle_context_project(params, state).await,
        methods::SESSION_LIST => handle_session_list(params, state).await,
        methods::SESSION_LOG => handle_session_log(params, state).await,
        methods::SESSION_DIRECT => handle_session_direct(params, state, peer).await,
        methods::SESSION_START => {
            crate::free_session::handle_session_start(params, state, peer).await
        }
        methods::REPO_MAP => handle_repo_map(params).await,
        methods::SDD_EXPLORE => crate::sdd_flow::handle_explore(params, state, peer).await,
        methods::SDD_CONSTITUTION => {
            crate::sdd_flow::handle_constitution(params, state, peer).await
        }
        methods::SDD_PROPOSE => crate::sdd_flow::handle_propose(params, state, peer).await,
        methods::SDD_PLAN => crate::sdd_flow::handle_plan(params, state, peer).await,
        methods::SDD_GATE => crate::sdd_flow::handle_gate(params, state, peer).await,
        methods::SDD_REVIEW => crate::review::handle_review(params, state).await,
        methods::SDD_REVIEW_DECIDE => {
            crate::review::handle_review_decide(params, state, peer).await
        }
        methods::PERMISSION_PENDING => handle_permission_pending(state).await,
        methods::PERMISSION_DECIDE => handle_permission_decide(params, state).await,
        methods::WORKTREE_ASSIGN => handle_worktree_assign(params).await,
        methods::WORKTREE_LIST => handle_worktree_list(params).await,
        methods::WORKTREE_REMOVE => handle_worktree_remove(params).await,
        methods::WORKTREE_DIFF => handle_worktree_diff(params, state).await,
        methods::WORKTREE_MERGE_FILE => handle_worktree_merge_file(params).await,
        methods::WORKTREE_APPLY_EDIT => handle_worktree_apply_edit(params, state).await,
        methods::WORKTREE_DISPATCH => handle_worktree_dispatch(params, state, peer).await,
        methods::CHECKPOINT_CREATE => handle_checkpoint_create(params).await,
        methods::CHECKPOINT_LIST => handle_checkpoint_list(params).await,
        methods::CHECKPOINT_REVERT => handle_checkpoint_revert(params).await,
        methods::CHECKPOINT_RECORD_OP => handle_checkpoint_record_op(params).await,
        methods::COMMIT_TASK => handle_commit_task(params).await,
        methods::SDD_VERIFY => handle_sdd_verify(params).await,
        methods::SDD_VERIFY_MARK => handle_sdd_verify_mark(params).await,
        methods::SDD_ARCHIVE => handle_sdd_archive(params).await,
        methods::SDD_IMPLEMENT => handle_sdd_implement(params, state, peer).await,
        methods::CHANGE_LIST => crate::navigate::handle_change_list(params).await,
        methods::CHANGE_SHOW => crate::navigate::handle_change_show(params).await,
        methods::CHANGE_WORKSPACE => handle_change_workspace(params).await,
        methods::SPEC_LIST => crate::navigate::handle_spec_list(params).await,
        methods::SPEC_SHOW => crate::navigate::handle_spec_show(params).await,
        methods::SDD_VALIDATE => crate::navigate::handle_sdd_validate(params).await,
        methods::PROJECT_LIST => crate::projects::handle_project_list(params, state).await,
        methods::PROJECT_REGISTER => crate::projects::handle_project_register(params, state).await,
        methods::SUBSCRIPTION_LINK => handle_subscription_link(params, state).await,
        methods::SUBSCRIPTION_UNLINK => handle_subscription_unlink(params, state).await,
        methods::PROJECT_FORGET => crate::projects::handle_project_forget(params, state).await,
        methods::ANALYTICS_USAGE => crate::analytics::handle_analytics_usage(params, state),
        other => Err(RpcError::method_not_found(other)),
    }
}

/// Default `session/log` page size when the client gives no limit.
const SESSION_LOG_PAGE: usize = 200;

/// `repo/map`: the repository tree honoring nested gitignore, with sizes and a
/// declared truncation budget (gestion-contexto-repo).
async fn handle_repo_map(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::RepoMapParams;
    let params: RepoMapParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("repo/map: {e}")))?;
    let root = std::path::PathBuf::from(&params.project_root);
    if !root.is_dir() {
        return Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ));
    }
    let result = crate::repo_map::build_map(&root, params.depth, params.limit);
    Ok(serde_json::to_value(result).expect("RepoMapResult serializes"))
}

/// Validates and returns a project root that must be an existing git repo,
/// refusing with honest degradation (4000) otherwise (orquestacion-worktrees).
fn require_git_root(project_root: &str) -> Result<PathBuf, RpcError> {
    let root = PathBuf::from(project_root);
    if !root.is_dir() {
        return Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ));
    }
    if !crate::git::is_repo(&root) {
        return Err(RpcError::application(
            error_codes::WORKTREE_UNAVAILABLE,
            "worktree orchestration unavailable",
            "worktree_unavailable",
            format!("`{}` is not a git repository", root.display()),
            Some(
                "Run `git init` (or open an existing repository) to orchestrate worktrees; \
                 simple sessions still run without isolation."
                    .into(),
            ),
        ));
    }
    Ok(root)
}

/// `change/workspace`: the change's workshop — its branch and managed worktree
/// (rama-por-change D2–D4). "Give me", not "create": idempotent by default
/// with the re-encounter declared; `branch` mounts a chosen branch (naming it
/// is consent), `unique` mints a suffixed workshop that never re-encounters.
async fn handle_change_workspace(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{ChangeWorkspaceParams, ChangeWorkspaceResult};

    let params: ChangeWorkspaceParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("change/workspace: {e}")))?;
    let root = require_git_root(&params.project_root)?;
    let workspace = crate::worktrees::workspace(
        &root,
        &params.change,
        params.branch.as_deref(),
        params.unique,
    )
    .map_err(|e| {
        RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "workshop refused",
            "worktree_refused",
            e,
            Some(
                "Name the branch explicitly to consent to it, or ask for a unique workshop.".into(),
            ),
        )
    })?;
    let result = ChangeWorkspaceResult {
        change: params.change,
        branch: workspace.worktree.branch,
        path: workspace.worktree.path,
        reencountered: workspace.reencountered,
        base_branch: workspace.base_branch,
    };
    Ok(serde_json::to_value(result).expect("ChangeWorkspaceResult serializes"))
}

/// Maps a domain worktree to the wire type, flagging competitors: a worktree
/// races when the daemon manages more than one for the same `(change, task)`.
fn to_proto_worktrees(
    managed: Vec<crate::worktrees::ManagedWorktree>,
) -> Vec<meltemi_proto::Worktree> {
    use std::collections::HashMap;
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for w in &managed {
        *counts
            .entry((w.change.clone(), w.task.clone()))
            .or_default() += 1;
    }
    managed
        .into_iter()
        .map(|w| {
            let competitor = counts
                .get(&(w.change.clone(), w.task.clone()))
                .copied()
                .unwrap_or(0)
                > 1;
            meltemi_proto::Worktree {
                change: w.change,
                task: w.task,
                agent: w.agent,
                path: w.path,
                branch: w.branch,
                base_rev: w.base_rev,
                competitor,
            }
        })
        .collect()
}

/// `worktree/assign`: plan the N×M assignment (parallel where declared files
/// don't overlap, serialized where they do) and create one isolated worktree
/// per agent per task from a common fixed base (orquestacion-worktrees D4/D6).
async fn handle_worktree_assign(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeAssignParams, WorktreeAssignResult, WorktreeBatch};

    let params: WorktreeAssignParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/assign: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    // The common base is fixed for the whole assignment (no drift mid-race).
    let base = crate::git::head_rev(&root).ok_or_else(|| {
        RpcError::application(
            error_codes::WORKTREE_UNAVAILABLE,
            "worktree orchestration unavailable",
            "worktree_unavailable",
            "the repository has no commit to branch worktrees from",
            Some("Make an initial commit, then assign worktrees.".into()),
        )
    })?;

    // Serialize tasks whose declared files overlap; races stay parallel.
    let task_files: Vec<crate::worktrees::TaskFiles> = params
        .tasks
        .iter()
        .map(|t| crate::worktrees::TaskFiles {
            task: t.task.clone(),
            files: t.files.clone(),
        })
        .collect();
    let batches = crate::worktrees::assignment_plan(&task_files)
        .into_iter()
        .map(|b| WorktreeBatch {
            tasks: b.tasks,
            serialized_reason: b.serialized_reason,
        })
        .collect();

    // One worktree per agent per task, all from the same base.
    let mut managed = Vec::new();
    for task in &params.tasks {
        for agent in &task.agents {
            let wt = crate::worktrees::create(&root, &task.change, &task.task, agent, &base)
                .map_err(|e| {
                    RpcError::application(
                        error_codes::WORKTREE_UNAVAILABLE,
                        "could not create worktree",
                        "worktree_unavailable",
                        e,
                        Some("Check the git version and that the base revision exists.".into()),
                    )
                })?;
            managed.push(wt);
        }
    }

    let result = WorktreeAssignResult {
        base_rev: base,
        batches,
        worktrees: to_proto_worktrees(managed),
    };
    Ok(serde_json::to_value(result).expect("WorktreeAssignResult serializes"))
}

/// `worktree/list`: the worktrees the daemon manages for a project — its own
/// registry only, never worktrees it did not create.
async fn handle_worktree_list(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeListParams, WorktreeListResult};

    let params: WorktreeListParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/list: {e}")))?;
    let root = PathBuf::from(&params.project_root);
    if !root.is_dir() {
        return Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ));
    }
    let result = WorktreeListResult {
        worktrees: to_proto_worktrees(crate::worktrees::list(&root)),
    };
    Ok(serde_json::to_value(result).expect("WorktreeListResult serializes"))
}

/// `worktree/remove`: safe cleanup of a managed worktree. Refuses a worktree
/// the daemon did not create, and a dirty one unless `force` confirms it.
async fn handle_worktree_remove(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeRemoveParams, WorktreeRemoveResult};

    let params: WorktreeRemoveParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/remove: {e}")))?;
    let root = PathBuf::from(&params.project_root);
    let path = PathBuf::from(&params.path);
    crate::worktrees::remove(&root, &path, params.force).map_err(|e| {
        RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "worktree removal refused",
            "worktree_refused",
            e,
            Some("Confirm removal of a worktree with uncommitted changes (force).".into()),
        )
    })?;
    Ok(serde_json::to_value(WorktreeRemoveResult { removed: true })
        .expect("WorktreeRemoveResult serializes"))
}

/// `worktree/diff`: every competitor of a task as a diff against the common
/// base — the side-by-side input to assisted merge (orquestacion-worktrees D5)
/// — now with each lane's own base, its commit state and the provenance of the
/// last turn dispatched over it (tablero-de-carrera design D1/D2).
///
/// The provenance is a read-only aggregation of state the daemon already
/// persists (the precedent is `navigate.rs`): the session index is joined to
/// the worktree registry by exact path equality, because both strings are
/// written by this daemon from the same value. The incoming root is
/// canonicalized on the way to the project key — `paths::project_key` does
/// that — and the STORED paths are never re-canonicalized, which is the
/// lesson the multiproject work paid for.
async fn handle_worktree_diff(params: Value, state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeCompetitorDiff, WorktreeDiffParams, WorktreeDiffResult};
    use std::collections::HashMap;

    let params: WorktreeDiffParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/diff: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    let competitors = crate::worktrees::competitors(&root, &params.change, &params.task);
    let base_rev = competitors
        .first()
        .map(|w| w.base_rev.clone())
        .unwrap_or_default();

    // The most recent session record per lane. `records_for_project` answers
    // most recent first, so the first record seen for a tree is the last turn
    // that ran there.
    let key = crate::paths::project_key(&root);
    let mut last_turn: HashMap<String, crate::session_index::SessionRecord> = HashMap::new();
    for record in crate::session_index::records_for_project(&state.data_dir, &key) {
        last_turn
            .entry(record.project_root.clone())
            .or_insert(record);
    }

    let competitors = competitors
        .into_iter()
        .map(|w| {
            let path = PathBuf::from(&w.path);
            // The lane's commit state is a fact about the tree, not about a
            // dispatch: a lane committed by hand counts too. Its head differing
            // from its own fixed base is what "committed" means here.
            let head = crate::git::head_rev(&path);
            let committed = head.as_deref().is_some_and(|head| head != w.base_rev);
            // A lane with no dispatch on record states nothing about who ran
            // it, rather than borrowing another lane's provenance.
            let turn = last_turn.get(&w.path);
            WorktreeCompetitorDiff {
                agent: w.agent,
                changed_files: crate::git::changed_files(&path, &w.base_rev),
                diff: crate::git::diff_against(&path, &w.base_rev),
                path: w.path,
                source: turn.and_then(|r| r.source),
                profile: turn.and_then(|r| r.profile.clone()),
                level: turn.map(|r| r.level),
                session_id: turn.map(|r| r.session_id.clone()),
                committed: Some(committed),
                sha: head.filter(|_| committed),
                // Each lane keeps ITS base. Two lanes of one task normally
                // share it; when an assignment was replayed against a moved
                // HEAD they do not, and folding them into one would misname
                // what every diff on the board is taken against.
                base_rev: Some(w.base_rev),
            }
        })
        .collect();
    let result = WorktreeDiffResult {
        base_rev,
        competitors,
    };
    Ok(serde_json::to_value(result).expect("WorktreeDiffResult serializes"))
}

/// `worktree/merge-file`: apply one file from a source worktree into a target
/// worktree. Nothing is applied without explicit confirmation — every
/// application is a human decision (orquestacion-worktrees D5).
async fn handle_worktree_merge_file(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeMergeFileParams, WorktreeMergeFileResult};

    let params: WorktreeMergeFileParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/merge-file: {e}")))?;
    if !params.confirm {
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "confirmation required",
            "worktree_refused",
            format!("applying `{}` is an explicit decision", params.file),
            Some("Set confirm to apply the file into the chosen base worktree.".into()),
        ));
    }
    let root = PathBuf::from(&params.project_root);
    crate::worktrees::apply_file(
        &root,
        &PathBuf::from(&params.target),
        &PathBuf::from(&params.source),
        &params.file,
    )
    .map_err(|e| {
        RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "file application refused",
            "worktree_refused",
            e,
            Some(
                "Both worktrees must be managed by Meltemi; the file must live inside the source."
                    .into(),
            ),
        )
    })?;
    Ok(
        serde_json::to_value(WorktreeMergeFileResult { applied: true })
            .expect("WorktreeMergeFileResult serializes"),
    )
}

/// `worktree/dispatch`: run one competitor's turn over its assignment worktree
/// with that competitor's own resolved binary and auth context (flota-
/// multiproveedor). Composes checkpoint → turn → per-task commit, and **never**
/// ticks `tasks.md` — a competitor does not own the task; the human decides via
/// assisted merge.
async fn handle_worktree_dispatch(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    use meltemi_proto::{
        DispatchResolution, SessionEventKind, WorktreeDispatchParams, WorktreeDispatchResult,
    };

    let params: WorktreeDispatchParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/dispatch: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    let config = crate::config::Config::load(&state.config_dir, Some(&root));
    for d in &config.fleet_diagnostics {
        tracing::warn!(diagnostic = %d, "fleet profile hygiene");
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let bundled = crate::fleet::bundled_dir();
    let resolved =
        crate::levels::resolve_fleet_agent(&config, &params.agent, &path_var, bundled.as_deref())?;
    let (agent_command, level) = match &resolved.launch {
        crate::levels::Launch::Acp { argv, level } => (argv.clone(), *level),
        crate::levels::Launch::Headless { level, .. }
        | crate::levels::Launch::Artifacts { level } => {
            return Err(RpcError::application(
                error_codes::AGENT_COMMAND_NOT_CONFIGURED,
                "level not pilotable by dispatch",
                "level_not_pilotable",
                format!("the resolved agent is level {level}; dispatch needs an ACP agent"),
                Some("Select a level 1/2 agent to dispatch.".into()),
            ));
        }
    };

    // The common base is fixed by the assignment; create/reuse the worktree.
    let base = crate::git::head_rev(&root).ok_or_else(|| {
        RpcError::application(
            error_codes::WORKTREE_UNAVAILABLE,
            "the repository has no commit to dispatch from",
            "worktree_unavailable",
            "make an initial commit before dispatching",
            Some("Commit something first.".into()),
        )
    })?;
    let wt = crate::worktrees::create(&root, &params.change, &params.task, &params.agent, &base)
        .map_err(|e| {
            RpcError::application(
                error_codes::WORKTREE_UNAVAILABLE,
                "could not create the dispatch worktree",
                "worktree_unavailable",
                e,
                Some("Check the git version and the base revision.".into()),
            )
        })?;
    let worktree = PathBuf::from(&wt.path);
    let _ = crate::checkpoints::create(
        &root,
        &worktree,
        &params.change,
        &params.task,
        &params.agent,
    );

    // Session setup mirroring implement, but for a single competitor turn.
    let session_id = uuid::Uuid::new_v4().to_string();
    let reg = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    let project_key = crate::paths::project_key(&root);
    // The project registry is fed by real use only (multiproyecto D3).
    crate::projects::touch(&state.data_dir, &root);
    let mut log =
        crate::session_log::SessionLog::create(&state.data_dir, &project_key, &session_id)
            .map_err(RpcError::internal)?
            .streaming(state.events.clone(), peer.connection_id(), &session_id);
    let _ = log.append(SessionEventKind::SessionStarted {
        mode: None,
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: worktree.display().to_string(),
        // A lane is named by its change and task, not by a sentence (D2).
        title: None,
    });
    let _ = log.append(SessionEventKind::AgentResolved {
        binary: agent_command.first().cloned().unwrap_or_default(),
        source: resolved.source,
        profile: resolved.profile.clone(),
        agent_id: resolved.agent_id.clone(),
        level,
    });
    let log = std::sync::Arc::new(tokio::sync::Mutex::new(log));
    let rules = std::sync::Arc::new(crate::permissions::load_rules(
        &state.config_dir,
        Some(&root),
    ));

    // The start record, so a dispatch is a first-class session in the index
    // rather than something only reconstructable from its log — reconstruction
    // cannot know the level and would list every competitor's turn at 1
    // (tablero-de-carrera design D2). The end record below completes it, and a
    // crash mid-turn correctly leaves it interrupted.
    let started_at = crate::clock::now_rfc3339();
    let dispatch_record = |ended_at: Option<String>,
                           final_status: Option<meltemi_proto::TurnStatus>,
                           agent_session_id: Option<String>,
                           supports_load: bool| {
        crate::session_index::SessionRecord {
            session_id: session_id.clone(),
            agent_command: agent_command.clone(),
            // The lane, not the repository: this is the tree the turn ran in,
            // and it is the key the board joins a lane to its session by.
            project_root: worktree.display().to_string(),
            level,
            started_at: started_at.clone(),
            ended_at,
            final_status,
            agent_session_id,
            supports_load,
            resumed_from: None,
            agent_id: resolved.agent_id.clone(),
            profile: resolved.profile.clone(),
            mode: None,
            source: Some(resolved.source),
            // A race lane is not born from a sentence (design D2).
            title: None,
        }
    };
    let _ = crate::session_index::append(
        &state.data_dir,
        &project_key,
        &dispatch_record(None, None, None, false),
    );

    // The prompt points the agent at a per-task artifact so its work is
    // committable; the task description (best-effort) titles the commit.
    let description = std::fs::read_to_string(
        root.join(".meltemi")
            .join("changes")
            .join(&params.change)
            .join("tasks.md"),
    )
    .ok()
    .and_then(|md| {
        crate::implement::parse_tasks(&md)
            .into_iter()
            .find(|t| t.id == params.task)
            .map(|t| t.description)
    })
    .unwrap_or_else(|| format!("task {}", params.task));
    let artifact = worktree.join(format!("task-{}.md", params.task.replace('.', "-")));
    let prompt = format!(
        "Implement task {} of change {} ({}): {}\nPROPOSAL_PATH: {}",
        params.task,
        params.change,
        params.agent,
        description,
        artifact.display()
    );

    let edit_scope = state.edits.enter(&worktree, &session_id, log.clone());
    let outcome = crate::acp::run_session(crate::acp::SessionParams {
        agent_command: agent_command.clone(),
        project_root: worktree.clone(),
        prompt,
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel: reg.cancel,
        cancelled: reg.cancelled,
        in_flight: reg.in_flight,
        // A dispatched task has a destination: parking it between turns
        // would hold an agent for a flow with nothing more to say.
        idle_timeout: std::time::Duration::ZERO,
        max_idle_sessions: 0,
        wait: config.autonomous_wait(),
        no_client_grace: config.no_client_grace(),
        clients: state.clients.clone(),
        sessions: state.sessions.clone(),
        rules,
        // Declared per session at start; these flows declare none.
        mode: None,
        pending: state.pending.clone(),
        load_session_id: None,
        mcp_servers: config.mcp_servers.clone(),
        env: resolved.env.clone(),
        // A competitor dispatch is a single self-managed turn, not directable.
        instruction_queue: None,
        edit_scope: Some(edit_scope.handle()),
    })
    .await;
    let status = outcome
        .as_ref()
        .map(|o| o.status)
        .unwrap_or(meltemi_proto::TurnStatus::Refused);

    // Commit with traceability, but NEVER tick tasks.md (D3).
    let message =
        crate::commit::build_message(&params.change, &params.task, &description, None, &[]);
    let cp_ref = crate::checkpoints::ref_for(&params.change, &params.task, &params.agent);
    let (committed, sha, changed_files) = match crate::commit::commit(&worktree, &message, &cp_ref)
    {
        Ok(done) => {
            append(
                &log,
                SessionEventKind::TaskCommitted {
                    change: params.change.clone(),
                    task: params.task.clone(),
                    agent: params.agent.clone(),
                    sha: done.sha.clone(),
                    requirements: Vec::new(),
                },
            )
            .await;
            (true, Some(done.sha), done.changed_files)
        }
        // Nothing to commit: compare against the worktree's OWN fixed base
        // (not repo HEAD) so a reused assignment worktree diffs correctly.
        Err(_) => (
            false,
            None,
            crate::git::changed_files(&worktree, &wt.base_rev),
        ),
    };

    append(
        &log,
        SessionEventKind::SessionEnded {
            reason: "dispatch finished".to_string(),
        },
    )
    .await;
    // The end record completes the start one: the listing shows the lane's
    // session finished, with the status it finished with, without any surface
    // having to walk the logs to find out.
    let _ = crate::session_index::append(
        &state.data_dir,
        &project_key,
        &dispatch_record(
            Some(crate::clock::now_rfc3339()),
            Some(status),
            outcome
                .as_ref()
                .ok()
                .and_then(|o| o.agent_session_id.clone()),
            outcome.as_ref().is_ok_and(|o| o.supports_load),
        ),
    );
    state
        .sessions
        .set_state(&session_id, meltemi_proto::SessionState::Ended)
        .await;

    let result = WorktreeDispatchResult {
        change: params.change,
        task: params.task,
        agent: params.agent,
        resolution: DispatchResolution {
            binary: agent_command.first().cloned().unwrap_or_default(),
            source: resolved.source,
            level,
            profile: resolved.profile,
        },
        worktree: wt.path,
        committed,
        sha,
        changed_files,
        status,
        task_ticked: false,
        session_id: Some(session_id),
    };
    Ok(serde_json::to_value(result).expect("WorktreeDispatchResult serializes"))
}

/// `worktree/apply-edit`: a traceable human edit through the daemon
/// (edit-surface; gui-tauri-paridad design D4/D5). The target tree is the
/// project root, or a managed worktree when the change/task/agent triple is
/// given. Soft lock: with a session active or a turn in flight on the tree,
/// the write demands `confirm: true` — never a hard lock. Every applied edit
/// is recorded as a `human_edit` event (the active session's JSONL, or the
/// project-scoped edits log) and noted for the agent's next turn.
async fn handle_worktree_apply_edit(
    params: Value,
    state: &Arc<DaemonState>,
) -> Result<Value, RpcError> {
    use meltemi_proto::{
        EditLogDestination, SessionEventKind, WorktreeApplyEditParams, WorktreeApplyEditResult,
    };

    let params: WorktreeApplyEditParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/apply-edit: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    let tree = match (&params.change, &params.task, &params.agent) {
        (Some(change), Some(task), Some(agent)) => resolve_worktree(&root, change, task, agent)?,
        (None, None, None) => root.clone(),
        _ => {
            return Err(RpcError::invalid_params(
                "worktree/apply-edit: change, task and agent go together",
            ));
        }
    };

    // Containment, same discipline as worktree/merge-file: relative path,
    // no parent escapes.
    if params.file.contains("..") || std::path::Path::new(&params.file).is_absolute() {
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "edit refused",
            "path_escape",
            format!("`{}` escapes the target tree", params.file),
            Some("Pass a path relative to the tree, without `..`.".into()),
        ));
    }

    // The soft-lock policy (design D4): the observed activity decides the
    // acknowledgement the save needs.
    let key = crate::edits::tree_key(&tree);
    let tree_state = state.edits.state_of(&key);
    if let Some(kind) = crate::edits::required_ack(tree_state)
        && !params.confirm
    {
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "edit requires confirmation",
            kind,
            format!(
                "a session is running on this tree ({kind}); saving may conflict \
                 with the agent's work"
            ),
            Some("Acknowledge the running session and retry with `confirm: true`.".into()),
        ));
    }

    let target = tree.join(&params.file);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(RpcError::internal)?;
    }
    std::fs::write(&target, params.content.as_bytes()).map_err(RpcError::internal)?;
    state.edits.note_edit(&key, &params.file);

    // Trace: into the active session's JSONL when one runs on the tree,
    // otherwise into the project-scoped edits log.
    let logged_to = match state.edits.active_session(&key) {
        Some((session_id, log)) => {
            let mut log = log.lock().await;
            let _ = log.append(SessionEventKind::HumanEdit {
                file: params.file.clone(),
                session_id: Some(session_id),
            });
            EditLogDestination::Session
        }
        None => {
            crate::edits::log_project_event(
                &root,
                SessionEventKind::HumanEdit {
                    file: params.file.clone(),
                    session_id: None,
                },
            );
            EditLogDestination::Project
        }
    };

    let result = WorktreeApplyEditResult {
        bytes_written: params.content.len() as u64,
        file: params.file,
        tree_state,
        logged_to,
    };
    serde_json::to_value(result).map_err(RpcError::internal)
}

/// Resolves the managed worktree path for a `(change, task, agent)`.
fn resolve_worktree(
    root: &std::path::Path,
    change: &str,
    task: &str,
    agent: &str,
) -> Result<PathBuf, RpcError> {
    crate::worktrees::list(root)
        .into_iter()
        .find(|w| w.change == change && w.task == task && w.agent == agent)
        .map(|w| PathBuf::from(w.path))
        .ok_or_else(|| {
            RpcError::application(
                error_codes::WORKTREE_UNAVAILABLE,
                "no managed worktree for this task",
                "worktree_unavailable",
                format!("no worktree is assigned for {change}/{task}-{agent}"),
                Some("Assign the task first (`worktree/assign`).".into()),
            )
        })
}

/// `checkpoint/create`: snapshot a task's worktree into a technical ref before
/// it runs (checkpoints-rollback D1), recording the lifecycle event.
async fn handle_checkpoint_create(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{Checkpoint, CheckpointCreateParams, CheckpointCreateResult};

    let params: CheckpointCreateParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("checkpoint/create: {e}")))?;
    let root = require_git_root(&params.project_root)?;
    let worktree = resolve_worktree(&root, &params.change, &params.task, &params.agent)?;

    let record = crate::checkpoints::create(
        &root,
        &worktree,
        &params.change,
        &params.task,
        &params.agent,
    )
    .map_err(|e| {
        RpcError::application(
            error_codes::WORKTREE_UNAVAILABLE,
            "could not create checkpoint",
            "worktree_unavailable",
            e,
            Some("Check the git version and that the worktree has a commit.".into()),
        )
    })?;

    crate::checkpoints::log_event(
        &root,
        meltemi_proto::SessionEventKind::CheckpointCreated {
            git_ref: record.git_ref.clone(),
            change: record.change.clone(),
            task: record.task.clone(),
            agent: record.agent.clone(),
        },
    );

    let checkpoint = Checkpoint {
        change: record.change,
        task: record.task,
        agent: record.agent,
        git_ref: record.git_ref,
        worktree: record.worktree,
        created_at: record.created_at,
        irreversible: Vec::new(),
    };
    Ok(serde_json::to_value(CheckpointCreateResult { checkpoint })
        .expect("CheckpointCreateResult serializes"))
}

/// `checkpoint/list`: the checkpoints the daemon recorded for a project, each
/// with its accumulated irreversible operations.
async fn handle_checkpoint_list(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{Checkpoint, CheckpointListParams, CheckpointListResult};

    let params: CheckpointListParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("checkpoint/list: {e}")))?;
    let root = PathBuf::from(&params.project_root);
    if !root.is_dir() {
        return Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ));
    }
    let checkpoints = crate::checkpoints::list(&root, params.change.as_deref())
        .into_iter()
        .map(|r| {
            let irreversible =
                crate::checkpoints::irreversibles_for(&root, &r.change, &r.task, &r.agent);
            Checkpoint {
                change: r.change,
                task: r.task,
                agent: r.agent,
                git_ref: r.git_ref,
                worktree: r.worktree,
                created_at: r.created_at,
                irreversible,
            }
        })
        .collect();
    Ok(serde_json::to_value(CheckpointListResult { checkpoints })
        .expect("CheckpointListResult serializes"))
}

/// `checkpoint/revert`: restore a task's worktree to its checkpoint with an
/// honest scope. Requires explicit confirmation; the reversion is never
/// presented as total when out-of-tree operations remain (D2/D3).
async fn handle_checkpoint_revert(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{CheckpointRevertParams, CheckpointRevertResult, RevertScope};

    let params: CheckpointRevertParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("checkpoint/revert: {e}")))?;

    // Before anything else, including the confirmation prompt: reverting means
    // `git reset --hard` plus `git clean -fd` over the tree the record names,
    // and until the free session existed every recorded tree was a worktree
    // Meltemi made. A free session's restore point is over the USER's own tree
    // — uncommitted human work and untracked files included — so this verb, the
    // CLI's `revert`, the palette entry and the GUI registry entry all point at
    // a loaded gun. The refusal comes first so no surface can even offer the
    // control: an unconfirmed call is how a surface asks what would happen, and
    // the answer is "nothing, and here is why" (lanzador-conversacional D2).
    if let Some(record) =
        crate::checkpoints::list(&PathBuf::from(&params.project_root), Some(&params.change))
            .into_iter()
            .find(|r| r.task == params.task && r.agent == params.agent)
        && !crate::worktrees::is_managed(
            &PathBuf::from(&params.project_root),
            std::path::Path::new(&record.worktree),
        )
    {
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "reversion refused",
            "worktree_refused",
            format!(
                "the checkpoint `{}` was taken over `{}`, which is not a worktree Meltemi \
                 manages; reverting it would reset that tree and delete its untracked files",
                record.git_ref, record.worktree
            ),
            Some(format!(
                "Restore what you want from the checkpoint yourself, with git: \
                 `git restore --source {} -- <path>`.",
                record.git_ref
            )),
        ));
    }

    if !params.confirm {
        // Report the scope so the surface can show what will (and won't) revert
        // before the user confirms.
        let root = PathBuf::from(&params.project_root);
        let irreversible = crate::checkpoints::irreversibles_for(
            &root,
            &params.change,
            &params.task,
            &params.agent,
        );
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "confirmation required",
            "worktree_refused",
            if irreversible.is_empty() {
                format!(
                    "reverting {}/{}-{} restores the worktree; confirm to proceed",
                    params.change, params.task, params.agent
                )
            } else {
                format!(
                    "reverting {}/{}-{} cannot undo {} out-of-tree operation(s): {}",
                    params.change,
                    params.task,
                    params.agent,
                    irreversible.len(),
                    irreversible.join("; ")
                )
            },
            Some("Set confirm to revert the worktree to its checkpoint.".into()),
        ));
    }

    let root = require_git_root(&params.project_root)?;
    let reverted = crate::checkpoints::revert(&root, &params.change, &params.task, &params.agent)
        .map_err(|e| {
        RpcError::application(
            error_codes::CHECKPOINT_NOT_FOUND,
            "checkpoint not found",
            "checkpoint_not_found",
            e,
            Some("List checkpoints to see what is available (`checkpoint/list`).".into()),
        )
    })?;

    crate::checkpoints::log_event(
        &root,
        meltemi_proto::SessionEventKind::CheckpointRestored {
            git_ref: reverted.git_ref,
            change: params.change.clone(),
            task: params.task.clone(),
            agent: params.agent.clone(),
            irreversible: reverted.irreversible.clone(),
        },
    );

    let complete = reverted.irreversible.is_empty();
    let result = CheckpointRevertResult {
        reverted: true,
        scope: RevertScope {
            worktree_restored: true,
            complete,
            irreversible: reverted.irreversible,
        },
    };
    Ok(serde_json::to_value(result).expect("CheckpointRevertResult serializes"))
}

/// `checkpoint/record-op`: record an approved out-of-tree operation against a
/// task, so its reversion declares the operation irreversible (D3). The proxy
/// classifies operations; this ledger records what it already classified.
async fn handle_checkpoint_record_op(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{CheckpointRecordOpParams, CheckpointRecordOpResult};

    let params: CheckpointRecordOpParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("checkpoint/record-op: {e}")))?;
    let root = PathBuf::from(&params.project_root);
    crate::checkpoints::record_irreversible(
        &root,
        &params.change,
        &params.task,
        &params.agent,
        &params.operation,
    )
    .map_err(RpcError::internal)?;
    Ok(
        serde_json::to_value(CheckpointRecordOpResult { recorded: true })
            .expect("CheckpointRecordOpResult serializes"),
    )
}

/// `commit/task`: propose (preview) or apply the atomic per-task commit with
/// traceability trailers (git-commit-por-tarea). Supervised previews with
/// `confirm` false; autonomous/approved applies with `confirm` true. Hooks are
/// honored and never bypassed; deviations from the declared scope are reported.
async fn handle_commit_task(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{CommitTaskParams, CommitTaskResult};

    let params: CommitTaskParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("commit/task: {e}")))?;
    let root = require_git_root(&params.project_root)?;
    let worktree = resolve_worktree(&root, &params.change, &params.task, &params.agent)?;

    let requirements: Vec<crate::commit::Requirement> = params
        .requirements
        .iter()
        .map(|r| crate::commit::Requirement {
            capability: r.capability.clone(),
            requirement: r.requirement.clone(),
        })
        .collect();
    let message = crate::commit::build_message(
        &params.change,
        &params.task,
        &params.title,
        params.body.as_deref(),
        &requirements,
    );

    // The task's checkpoint is the base for scope comparison; without one, fall
    // back to the current HEAD *resolved to a sha now* — after the commit,
    // `HEAD` would name the new commit and the diff would be empty.
    let base = {
        let cp_ref = crate::checkpoints::ref_for(&params.change, &params.task, &params.agent);
        if crate::git::run(&worktree, &["rev-parse", "--verify", "--quiet", &cp_ref]).is_ok() {
            cp_ref
        } else {
            crate::git::head_rev(&worktree).unwrap_or_else(|| "HEAD".to_string())
        }
    };

    if !params.confirm {
        // Supervised preview: show the message and what would be committed,
        // predicting deviations — nothing is applied.
        let changed = crate::commit::pending_files(&worktree);
        let deviations = crate::commit::scope_deviations(&changed, &params.declared_files);
        let result = CommitTaskResult {
            committed: false,
            message,
            sha: None,
            changed_files: changed,
            deviations,
            tree_clean: !crate::git::is_dirty(&worktree),
        };
        return Ok(serde_json::to_value(result).expect("CommitTaskResult serializes"));
    }

    // Apply. A hook rejection (or any git failure) surfaces verbatim; the task
    // stays completed-without-commit.
    let committed = crate::commit::commit(&worktree, &message, &base).map_err(|detail| {
        RpcError::application(
            error_codes::GIT_COMMIT_FAILED,
            "the per-task commit failed",
            "git_commit_failed",
            detail,
            Some(
                "Resolve what your git hook reported, then retry; Meltemi never bypasses hooks."
                    .into(),
            ),
        )
    })?;

    let deviations =
        crate::commit::scope_deviations(&committed.changed_files, &params.declared_files);

    crate::checkpoints::log_event(
        &root,
        meltemi_proto::SessionEventKind::TaskCommitted {
            change: params.change.clone(),
            task: params.task.clone(),
            agent: params.agent.clone(),
            sha: committed.sha.clone(),
            requirements: crate::commit::requirement_refs(&requirements),
        },
    );

    let result = CommitTaskResult {
        committed: true,
        message,
        sha: Some(committed.sha),
        changed_files: committed.changed_files,
        deviations,
        tree_clean: !crate::git::is_dirty(&worktree),
    };
    Ok(serde_json::to_value(result).expect("CommitTaskResult serializes"))
}

/// Validates a project root exists as a directory.
fn require_project_dir(project_root: &str) -> Result<PathBuf, RpcError> {
    let root = PathBuf::from(project_root);
    if root.is_dir() {
        Ok(root)
    } else {
        Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing repository root.".into()),
        ))
    }
}

/// `sdd/verify`: the per-requirement verification checklist of a change — each
/// scenario linked to a test, manually marked, or unverified (verify-archive).
async fn handle_sdd_verify(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SddVerifyParams, SddVerifyResult, VerifyScenario};

    let params: SddVerifyParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/verify: {e}")))?;
    let root = require_project_dir(&params.project_root)?;

    let verifications = crate::verify::verify_change(&root, &params.change);
    let total = verifications.len() as u32;
    let verified = verifications
        .iter()
        .filter(|v| v.status != crate::verify::ScenarioStatus::Unverified)
        .count() as u32;
    let scenarios = verifications
        .into_iter()
        .map(|v| VerifyScenario {
            capability: v.capability,
            requirement: v.requirement,
            scenario: v.scenario,
            status: v.status.as_str().to_string(),
            note: v.note,
        })
        .collect();
    let result = SddVerifyResult {
        scenarios,
        verified,
        total,
        complete: verified == total,
    };
    Ok(serde_json::to_value(result).expect("SddVerifyResult serializes"))
}

/// `sdd/verify-mark`: record a manual verification of a scenario with a note.
async fn handle_sdd_verify_mark(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SddVerifyMarkParams, SddVerifyMarkResult};

    let params: SddVerifyMarkParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/verify-mark: {e}")))?;
    let root = require_project_dir(&params.project_root)?;
    crate::verify::mark_manual(&root, &params.change, &params.scenario, &params.note).map_err(
        |e| {
            RpcError::application(
                error_codes::PROJECT_ROOT_INVALID,
                "could not record the manual verification",
                "project_root_invalid",
                e,
                Some("Check the change exists under `.meltemi/changes/`.".into()),
            )
        },
    )?;
    Ok(serde_json::to_value(SddVerifyMarkResult { marked: true })
        .expect("SddVerifyMarkResult serializes"))
}

/// `sdd/archive`: fold a change's deltas into the living truth atomically,
/// gated by complete verification (or recorded exceptions), warning on a dirty
/// specs tree, then preserve the change in the dated history (verify-archive).
async fn handle_sdd_archive(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{SddArchiveParams, SddArchiveResult};

    let params: SddArchiveParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/archive: {e}")))?;
    let root = require_project_dir(&params.project_root)?;

    // 1. Verification gate: every scenario verified, or explicitly excepted.
    let excepted: std::collections::HashSet<&str> = params
        .exceptions
        .iter()
        .map(|e| e.scenario.as_str())
        .collect();
    let verifications = crate::verify::verify_change(&root, &params.change);
    let blocking: Vec<String> = verifications
        .iter()
        .filter(|v| {
            v.status == crate::verify::ScenarioStatus::Unverified
                && !excepted.contains(v.scenario.as_str())
        })
        .map(|v| format!("{}/{}", v.requirement, v.scenario))
        .collect();
    if !blocking.is_empty() {
        return Err(RpcError::application(
            error_codes::VERIFY_INCOMPLETE,
            "archiving blocked: verification incomplete",
            "verify_incomplete",
            format!(
                "{} unverified requirement(s): {}",
                blocking.len(),
                blocking.join("; ")
            ),
            Some("Verify or except each requirement before archiving.".into()),
        ));
    }

    // 2. Merge validation (dry run): any conflict blocks the fold entirely.
    let diagnostics = crate::archive::dry_run_diagnostics(&root, &params.change);
    if !diagnostics.is_empty() {
        return Err(RpcError::application(
            error_codes::SPEC_MERGE_CONFLICT,
            "archiving blocked: spec merge conflict",
            "spec_merge_conflict",
            diagnostics.join("; "),
            Some("Resolve the delta against the living truth, then archive.".into()),
        ));
    }

    // 3. A dirty living-specs tree needs explicit confirmation before folding.
    if !params.confirm && crate::archive::living_specs_dirty(&root) {
        return Err(RpcError::application(
            error_codes::WORKTREE_REFUSED,
            "the living specs tree has uncommitted changes",
            "worktree_refused",
            "archiving would fold into specs with local changes; confirm to proceed".to_string(),
            Some("Commit or stash the specs changes, or set confirm to proceed.".into()),
        ));
    }

    // 4. Fold atomically, preserve in history, regenerate projection.
    let date = crate::clock::now_rfc3339();
    let date = date.get(..10).unwrap_or(&date);
    let report = crate::archive::archive_change(&root, &params.change, date).map_err(|e| {
        RpcError::application(
            error_codes::SPEC_MERGE_CONFLICT,
            "archiving failed",
            "spec_merge_conflict",
            e,
            Some("No specs were changed; inspect the change and retry.".into()),
        )
    })?;

    let result = SddArchiveResult {
        capabilities: report.capabilities,
        archived_to: report.archived_to,
        projection_regenerated: report.projection_regenerated,
        excepted: params.exceptions.into_iter().map(|e| e.scenario).collect(),
    };
    Ok(serde_json::to_value(result).expect("SddArchiveResult serializes"))
}

/// Appends an event to a session log guarded by a mutex (implement progress).
async fn append(
    log: &std::sync::Arc<tokio::sync::Mutex<crate::session_log::SessionLog>>,
    kind: meltemi_proto::SessionEventKind,
) {
    let _ = log.lock().await.append(kind);
}

/// `sdd/implement`: deploy an agent over a change's `tasks.md`, task by task,
/// composing checkpoint (#17) → turn in the worktree (#16) → per-task commit
/// (#18) → tick. Progress is the ticked `tasks.md` (a restart resumes at the
/// first undone task). Plan mode returns the sequence without acting; autonomy
/// needs permission rules or it degrades to supervised with a notice (D3).
async fn handle_sdd_implement(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    use meltemi_proto::{ImplementTask, SddImplementParams, SddImplementResult, SessionEventKind};

    let params: SddImplementParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("sdd/implement: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    // Load the change's task list.
    let tasks_path = root
        .join(".meltemi")
        .join("changes")
        .join(&params.change)
        .join("tasks.md");
    let tasks_md = std::fs::read_to_string(&tasks_path).map_err(|_| {
        RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "no tasks.md for the change",
            "project_root_invalid",
            format!("`{}` does not exist", tasks_path.display()),
            Some("Plan the change first so its tasks.md exists.".into()),
        )
    })?;
    let tasks = crate::implement::parse_tasks(&tasks_md);

    // Autonomy needs applicable permission rules; without them it degrades to
    // supervised with a visible notice — never autonomy by accident (D3).
    let rules = crate::permissions::load_rules(&state.config_dir, Some(&root));
    let (autonomous, degraded) = if params.autonomous && rules.rules.is_empty() {
        (
            false,
            Some("no permission rules apply to this project; running supervised".to_string()),
        )
    } else {
        (params.autonomous, None)
    };

    // Plan mode: report the eligible sequence, touching nothing (D2 gate).
    if params.plan_only {
        let planned = tasks
            .into_iter()
            .map(|t| ImplementTask {
                id: t.id,
                description: t.description,
                status: if t.done { "already-done" } else { "planned" }.to_string(),
                sha: None,
            })
            .collect();
        let result = SddImplementResult {
            mode: "plan".to_string(),
            autonomous,
            degraded,
            tasks: planned,
            committed: Vec::new(),
        };
        return Ok(serde_json::to_value(result).expect("SddImplementResult serializes"));
    }

    // Act mode. One deployment session log carries the per-task progress.
    let config = crate::config::Config::load(&state.config_dir, Some(&root));
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    // Resolve the deployed agent from the fleet (profile > catalog id >
    // configured); a free label like `claude` falls through to the configured
    // agent (flota-multiproveedor). Surface any profile hygiene diagnostics.
    for d in &config.fleet_diagnostics {
        tracing::warn!(diagnostic = %d, "fleet profile hygiene");
    }
    let bundled = crate::fleet::bundled_dir();
    let resolved =
        crate::levels::resolve_fleet_agent(&config, &params.agent, &path_var, bundled.as_deref())?;
    let (agent_command, level) = match &resolved.launch {
        crate::levels::Launch::Acp { argv, level } => (argv.clone(), *level),
        crate::levels::Launch::Headless { level, .. }
        | crate::levels::Launch::Artifacts { level } => {
            return Err(RpcError::application(
                error_codes::AGENT_COMMAND_NOT_CONFIGURED,
                "level not pilotable by implement",
                "level_not_pilotable",
                format!("the resolved agent is level {level}; implement needs an ACP agent"),
                Some("Select a level 1/2 agent to deploy tasks.".into()),
            ));
        }
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    let reg = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    let project_key = crate::paths::project_key(&root);
    // The project registry is fed by real use only (multiproyecto D3).
    crate::projects::touch(&state.data_dir, &root);
    let mut log =
        crate::session_log::SessionLog::create(&state.data_dir, &project_key, &session_id)
            .map_err(RpcError::internal)?
            .streaming(state.events.clone(), peer.connection_id(), &session_id);
    let _ = log.append(SessionEventKind::SessionStarted {
        mode: None,
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: root.display().to_string(),
        // `sdd/implement` deploys over a change's tasks; no sentence opened it,
        // and the change and task are what name its work (design D2).
        title: None,
    });
    // Record which binary/source ran — never the env values (§2).
    let _ = log.append(SessionEventKind::AgentResolved {
        binary: agent_command.first().cloned().unwrap_or_default(),
        source: resolved.source,
        profile: resolved.profile.clone(),
        agent_id: resolved.agent_id.clone(),
        level,
    });
    let log = std::sync::Arc::new(tokio::sync::Mutex::new(log));
    let rules = std::sync::Arc::new(rules);

    let base = crate::git::head_rev(&root).ok_or_else(|| {
        RpcError::application(
            error_codes::WORKTREE_UNAVAILABLE,
            "the repository has no commit to deploy from",
            "worktree_unavailable",
            "make an initial commit before deploying tasks",
            Some("Commit something first.".into()),
        )
    })?;

    let mut current_md = tasks_md.clone();
    let mut result_tasks: Vec<ImplementTask> = Vec::new();
    let mut committed: Vec<String> = Vec::new();
    let mut interrupted = false;

    for task in &tasks {
        if task.done {
            result_tasks.push(ImplementTask {
                id: task.id.clone(),
                description: task.description.clone(),
                status: "already-done".to_string(),
                sha: None,
            });
            continue;
        }
        // Interruption leaves a consistent state: completed tasks are already
        // committed and ticked; once a turn is cancelled (its commit produces
        // nothing) no further task starts (D4).
        if interrupted {
            result_tasks.push(ImplementTask {
                id: task.id.clone(),
                description: task.description.clone(),
                status: "pending".to_string(),
                sha: None,
            });
            continue;
        }

        // checkpoint → turn → commit → tick.
        let wt = crate::worktrees::create(&root, &params.change, &task.id, &params.agent, &base)
            .map_err(|e| {
                RpcError::application(
                    error_codes::WORKTREE_UNAVAILABLE,
                    "could not create the task worktree",
                    "worktree_unavailable",
                    e,
                    Some("Check the git version and the base revision.".into()),
                )
            })?;
        let worktree = PathBuf::from(&wt.path);
        let _ =
            crate::checkpoints::create(&root, &worktree, &params.change, &task.id, &params.agent);

        append(
            &log,
            SessionEventKind::TaskStarted {
                change: params.change.clone(),
                task: task.id.clone(),
                agent: params.agent.clone(),
            },
        )
        .await;

        // The turn: the agent works in the worktree. The prompt points it at a
        // per-task artifact so the mock (and a real agent) leave committable work.
        let artifact = worktree.join(format!("task-{}.md", task.id.replace('.', "-")));
        let prompt = format!(
            "Implement task {} of change {}: {}\nPROPOSAL_PATH: {}",
            task.id,
            params.change,
            task.description,
            artifact.display()
        );
        let task_edit_scope = state.edits.enter(&worktree, &session_id, log.clone());
        let _ = crate::acp::run_session(crate::acp::SessionParams {
            agent_command: agent_command.clone(),
            project_root: worktree.clone(),
            prompt,
            meltemi_session_id: session_id.clone(),
            peer: peer.clone(),
            log: log.clone(),
            cancel: reg.cancel.clone(),
            cancelled: reg.cancelled.clone(),
            in_flight: reg.in_flight.clone(),
            idle_timeout: std::time::Duration::ZERO,
            max_idle_sessions: 0,
            wait: config.autonomous_wait(),
            no_client_grace: config.no_client_grace(),
            clients: state.clients.clone(),
            sessions: state.sessions.clone(),
            rules: rules.clone(),
            // Declared per session at start; these flows declare none.
            mode: None,
            pending: state.pending.clone(),
            load_session_id: None,
            mcp_servers: config.mcp_servers.clone(),
            env: resolved.env.clone(),
            // `implement` drives its own per-task loop; each turn is not directable.
            instruction_queue: None,
            edit_scope: Some(task_edit_scope.handle()),
        })
        .await;

        // Commit the task with traceability. If nothing was produced (e.g. the
        // turn was denied), the commit fails and the task is left pending.
        let message =
            crate::commit::build_message(&params.change, &task.id, &task.description, None, &[]);
        let cp_ref = crate::checkpoints::ref_for(&params.change, &task.id, &params.agent);
        match crate::commit::commit(&worktree, &message, &cp_ref) {
            Ok(done) => {
                current_md = crate::implement::tick_task(&current_md, &task.id);
                let _ = std::fs::write(&tasks_path, &current_md);
                append(
                    &log,
                    SessionEventKind::TaskCommitted {
                        change: params.change.clone(),
                        task: task.id.clone(),
                        agent: params.agent.clone(),
                        sha: done.sha.clone(),
                        requirements: Vec::new(),
                    },
                )
                .await;
                committed.push(task.id.clone());
                result_tasks.push(ImplementTask {
                    id: task.id.clone(),
                    description: task.description.clone(),
                    status: "committed".to_string(),
                    sha: Some(done.sha),
                });
            }
            Err(_) => {
                // The task produced nothing to commit; stop here, leaving it and
                // the rest pending — a visible, consistent state.
                interrupted = true;
                result_tasks.push(ImplementTask {
                    id: task.id.clone(),
                    description: task.description.clone(),
                    status: "pending".to_string(),
                    sha: None,
                });
            }
        }
    }

    append(
        &log,
        SessionEventKind::SessionEnded {
            reason: "implement finished".to_string(),
        },
    )
    .await;
    state
        .sessions
        .set_state(&session_id, meltemi_proto::SessionState::Ended)
        .await;

    let result = SddImplementResult {
        mode: "act".to_string(),
        autonomous,
        degraded,
        tasks: result_tasks,
        committed,
    };
    Ok(serde_json::to_value(result).expect("SddImplementResult serializes"))
}

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
                level: record.level,
                final_status: record.final_status,
                started_at: record.started_at.clone(),
                ended_at: record.ended_at.clone(),
                resumable: record.resumable(),
                agent_id: record.agent_id.clone(),
                profile: record.profile.clone(),
                title: record.title.clone(),
                mode: record.mode,
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

/// `session/direct`: steer an existing session (control-remoto-asistido, the
/// third remote verb). An active session queues the instruction as its next
/// turn; a terminated-but-resumable one is resumed with it; anything else is a
/// `SESSION_NOT_FOUND` error with a remedy. Every instruction is audited in the
/// session log (queued, then sent).
async fn handle_session_direct(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    use meltemi_proto::{DirectDisposition, SessionDirectParams, SessionDirectResult};

    let params: SessionDirectParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("session/direct: {e}")))?;
    if !is_safe_session_id(&params.session_id) {
        return Err(RpcError::invalid_params(
            "session/direct: invalid session id",
        ));
    }
    if params.instruction.trim().is_empty() {
        return Err(RpcError::invalid_params(
            "session/direct: the instruction is empty",
        ));
    }

    // Active branch: enqueue as the next turn of the live session. The enqueue
    // records the instruction before it becomes visible to the draining loop
    // (log-before-enqueue), so the audit trail is always queued-then-sent.
    match state.sessions.direct_target(&params.session_id).await {
        Some((queue, log)) => {
            // Asking to interrupt enqueues the relay and signals in one lock,
            // in that order: the boundary must never see the queue empty
            // between the halves (redirigir-turno design D1). Then the cancel
            // is signalled, which is what makes the turn drain.
            // Nothing in flight means nothing to interrupt, and saying
            // "relayed" would report an interruption that did not happen. It
            // queues, and it says so.
            let interrupting =
                params.interrupt && state.sessions.turn_in_flight(&params.session_id).await;
            let queued = if interrupting {
                let position = queue.interrupt_with(params.instruction.clone(), &log).await;
                if position.is_some() {
                    state.sessions.interrupt(&params.session_id).await;
                }
                position
            } else {
                queue.enqueue(params.instruction.clone(), &log).await
            };
            if let Some(position) = queued {
                let result = SessionDirectResult {
                    disposition: if interrupting {
                        DirectDisposition::Relayed
                    } else {
                        DirectDisposition::Queued
                    },
                    session_id: params.session_id.clone(),
                    resumed_from: None,
                    queue_position: Some(position as u32),
                    status: None,
                    denied_permissions: 0,
                };
                return Ok(serde_json::to_value(result).expect("SessionDirectResult serializes"));
            }
            // Directable but no longer accepting (the session is ending): fall
            // through to the resume path below.
        }
        None => {
            // Not directable. If it is nonetheless still LIVE, it is an active
            // session that does not accept direction (implement / dispatch drive
            // their own turn loop) — refuse honestly, with a distinct message, so
            // a running session is never misreported as ended-and-not-resumable.
            if state.sessions.contains(&params.session_id).await {
                return Err(RpcError::application(
                    error_codes::SESSION_NOT_FOUND,
                    "session does not accept direction",
                    "session_not_directable",
                    format!(
                        "session `{}` is active but does not accept direction; only \
                         interactive sessions can be steered",
                        params.session_id
                    ),
                    Some("Wait for this session to finish, or steer an interactive one.".into()),
                ));
            }
        }
    }
    // Ended (or a directable session that just stopped accepting turns): resume
    // if the record says it can, otherwise refuse honestly.

    let Some(record) = find_session_record(
        &state.data_dir,
        params.project_root.as_deref(),
        &params.session_id,
    ) else {
        return Err(not_directable(
            &params.session_id,
            format!("no session `{}` is active or resumable", params.session_id),
        ));
    };
    if !record.resumable() {
        return Err(not_directable(
            &params.session_id,
            format!(
                "session `{}` has ended and its agent does not support resuming",
                params.session_id
            ),
        ));
    }
    resume_with_instruction(state, peer, &record, &params.instruction, params.detach).await
}

/// Finds a session's metadata record by id, within one project when a root is
/// given, or across every project otherwise (session ids are UUIDs).
fn find_session_record(
    data_dir: &std::path::Path,
    project_root: Option<&str>,
    session_id: &str,
) -> Option<crate::session_index::SessionRecord> {
    let keys = match project_root {
        Some(root) => vec![crate::paths::project_key(std::path::Path::new(root))],
        None => crate::session_index::all_project_keys(data_dir),
    };
    for key in keys {
        if let Some(record) = crate::session_index::records_for_project(data_dir, &key)
            .into_iter()
            .find(|r| r.session_id == session_id)
        {
            return Some(record);
        }
    }
    None
}

/// A `SESSION_NOT_FOUND` refusal with the standing remedy (list the sessions).
fn not_directable(_session_id: &str, detail: String) -> RpcError {
    RpcError::application(
        error_codes::SESSION_NOT_FOUND,
        "session not directable",
        "session_not_found",
        detail,
        Some("List sessions with `meltemi sessions` and direct an active or resumable one.".into()),
    )
}

/// Resumes a terminated-but-resumable session with `instruction` as its prompt,
/// as a new session linked to the original (`resumed_from`). Mirrors `propose`'s
/// session setup and shares the finalizer, so the resumed session is itself
/// resumable and directable.
async fn resume_with_instruction(
    state: &Arc<DaemonState>,
    peer: &Peer,
    record: &crate::session_index::SessionRecord,
    instruction: &str,
    detach: bool,
) -> Result<Value, RpcError> {
    use meltemi_proto::{DirectDisposition, SessionDirectResult, SessionState};

    let project_root = std::path::PathBuf::from(&record.project_root);
    let agent_command = record.agent_command.clone();
    let project_key = crate::paths::project_key(&project_root);
    crate::projects::touch(&state.data_dir, &project_root);

    let session_id = uuid::Uuid::new_v4().to_string();
    let reg = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    let mut log =
        crate::session_log::SessionLog::create(&state.data_dir, &project_key, &session_id)
            .map_err(RpcError::internal)?
            .streaming(state.events.clone(), peer.connection_id(), &session_id);
    let _ = log.append(meltemi_proto::SessionEventKind::SessionStarted {
        mode: None,
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: record.project_root.clone(),
        // The same conversation continues, so it keeps its name (design D5).
        title: record.title.clone(),
    });
    let log = Arc::new(tokio::sync::Mutex::new(log));
    let instruction_queue = state
        .sessions
        .enable_direction(&session_id, log.clone())
        .await;
    state
        .sessions
        .set_state(&session_id, SessionState::Active)
        .await;

    // A start record so a crash mid-resume leaves the session interrupted, and
    // the resume linkage is visible even before the turn completes.
    let started_at = crate::clock::now_rfc3339();
    let _ = crate::session_index::append(
        &state.data_dir,
        &project_key,
        &crate::session_index::SessionRecord {
            session_id: session_id.clone(),
            agent_command: agent_command.clone(),
            project_root: record.project_root.clone(),
            level: record.level,
            started_at: started_at.clone(),
            ended_at: None,
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: Some(record.session_id.clone()),
            // A resume runs the same agent and subscription it resumed.
            agent_id: record.agent_id.clone(),
            profile: record.profile.clone(),
            mode: None,
            source: record.source,
            // And continues the same conversation, so it keeps its name rather
            // than deriving a second one from the continuation (design D5).
            title: record.title.clone(),
        },
    );

    let rules = Arc::new(crate::permissions::load_rules(
        &state.config_dir,
        Some(&project_root),
    ));
    // Only the wait policy is read here: a resumed session deliberately
    // carries its prior MCP set, not a fresh config.
    let config = crate::config::Config::load(&state.config_dir, Some(&project_root));

    let edit_scope = state.edits.enter(&project_root, &session_id, log.clone());
    let outcome = crate::acp::run_session(crate::acp::SessionParams {
        agent_command: agent_command.clone(),
        project_root: project_root.clone(),
        prompt: instruction.to_string(),
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel: reg.cancel,
        cancelled: reg.cancelled,
        in_flight: reg.in_flight,
        // A resumed session IS the conversation continuing — but only for a
        // caller who is staying. This branch runs the turn INSIDE the request,
        // so parking it for a caller that is waiting would hang the very call
        // that asked (sesion-que-espera design D2).
        idle_timeout: if detach {
            config.idle_timeout()
        } else {
            std::time::Duration::ZERO
        },
        max_idle_sessions: config.max_idle_sessions(),
        wait: config.interactive_wait(),
        no_client_grace: config.no_client_grace(),
        clients: state.clients.clone(),
        sessions: state.sessions.clone(),
        rules,
        // Declared per session at start; these flows declare none.
        mode: None,
        pending: state.pending.clone(),
        // The heart of resume: load the agent's prior session instead of a new
        // one (only honored if the agent re-announces load support).
        load_session_id: record.agent_session_id.clone(),
        // MCP is injected on new-session only; a load carries the prior server set.
        mcp_servers: Vec::new(),
        env: Vec::new(),
        // The resumed session is itself directable.
        instruction_queue,
        edit_scope: Some(edit_scope.handle()),
    })
    .await;

    let ctx = crate::session_finalize::SessionContext {
        mode: None,
        data_dir: &state.data_dir,
        sessions: &state.sessions,
        log: &log,
        project_key: &project_key,
        session_id: &session_id,
        agent_command: &agent_command,
        project_root: &record.project_root,
        level: record.level,
        started_at: &started_at,
        resumed_from: Some(record.session_id.clone()),
        agent_id: record.agent_id.clone(),
        profile: record.profile.clone(),
        source: record.source,
    };
    match outcome {
        Ok(session_outcome) => {
            let fin = crate::session_finalize::finalize_ok(&ctx, session_outcome).await;
            let result = SessionDirectResult {
                disposition: DirectDisposition::Resumed,
                session_id: session_id.clone(),
                resumed_from: Some(record.session_id.clone()),
                queue_position: None,
                status: Some(fin.status),
                denied_permissions: fin.denied_permissions,
            };
            Ok(serde_json::to_value(result).expect("SessionDirectResult serializes"))
        }
        Err(e) => {
            crate::session_finalize::finalize_err(&ctx, "agent_session_failed", e.to_string())
                .await;
            Err(RpcError::application(
                error_codes::AGENT_SPAWN_FAILED,
                "resume failed",
                "agent_session_failed",
                e.to_string(),
                Some("Check that the agent still runs and supports session load.".into()),
            ))
        }
    }
}

/// A `SUBSCRIPTION_REFUSED` (2005) with its remedy: not-linkable is a
/// different fact from not-detected, and the remedy differs with the cause.
fn subscription_refused(detail: String, remedy: &str) -> RpcError {
    RpcError::application(
        error_codes::SUBSCRIPTION_REFUSED,
        "subscription refused",
        "subscription_refused",
        detail,
        Some(remedy.to_string()),
    )
}

/// `subscription/link`: link a named subscription of a catalog agent
/// (vincular-suscripciones D1/D3/D4/D5). Writes a launch profile whose env
/// pins the entry's declared auth-context variable to a fresh directory, and
/// answers with the composed login gesture Meltemi never runs. The context
/// directory is created empty and never read afterwards (fair play); relinking
/// a name whose directory survived an unlink reuses it untouched.
async fn handle_subscription_link(
    params: Value,
    state: &Arc<DaemonState>,
) -> Result<Value, RpcError> {
    use meltemi_proto::{LoginGesture, SubscriptionLinkParams, SubscriptionLinkResult};

    let params: SubscriptionLinkParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("subscription/link: {e}")))?;

    // The name becomes a directory: refuse before anything exists.
    if !crate::subscriptions::is_valid_name(&params.name) {
        return Err(subscription_refused(
            format!("`{}` is not a valid link name", params.name),
            "Use kebab-case: lowercase letters, digits and single hyphens (e.g. `work`, `personal-2`).",
        ));
    }

    // The entry must exist and declare its auth-context variable — that
    // knowledge is registry data, never code (D3).
    let config = crate::config::Config::load(&state.config_dir, None);
    let catalog = crate::fleet::build_catalog(&config);
    let Some(entry) = catalog.entries.iter().find(|e| e.id == params.agent) else {
        return Err(subscription_refused(
            format!("`{}` is not in the fleet catalog", params.agent),
            "List the catalog with `meltemi fleet` and link one of its ids.",
        ));
    };
    let Some(var) = entry.auth_context_var.clone() else {
        return Err(subscription_refused(
            format!(
                "`{}` declares no authentication-context variable in the registry",
                params.agent
            ),
            "Write the profile by hand in config.toml ([[fleet.profile]]) — see docs/agentes.md.",
        ));
    };

    // The context directory: created empty, owned by the provider's binary
    // from here on. A directory that already exists (a relinked name) is
    // reused untouched — whatever the provider stored there is not ours.
    let context_dir = state.data_dir.join("subscriptions").join(&params.name);
    std::fs::create_dir_all(&context_dir).map_err(RpcError::internal)?;
    let value = context_dir.display().to_string();

    // Persist the profile. A taken name refuses; the store never overwrites.
    let profile = crate::config::FleetProfile {
        name: params.name.clone(),
        agent: params.agent.clone(),
        env: vec![(var.clone(), value.clone())],
    };
    match crate::subscriptions::add(&state.config_dir, profile) {
        Ok(()) => {}
        Err(crate::subscriptions::AddError::AlreadyLinked) => {
            return Err(subscription_refused(
                format!("`{}` is already linked", params.name),
                "Unlink it first (`meltemi unlink <name>`) or pick another name.",
            ));
        }
        Err(crate::subscriptions::AddError::InvalidName) => {
            return Err(subscription_refused(
                format!("`{}` is not a valid link name", params.name),
                "Use kebab-case: lowercase letters, digits and single hyphens.",
            ));
        }
    }

    // The composed login gesture: data for the human to run. The daemon never
    // runs it and never checks whether it was run (fair play).
    let hint = entry
        .login_hint
        .clone()
        .unwrap_or_else(|| "see the provider's login documentation".to_string());
    let gesture = LoginGesture {
        var: var.clone(),
        value: value.clone(),
        hint: hint.clone(),
        posix: format!("{var}='{value}' {hint}"),
        powershell: format!("$env:{var} = \"{value}\"; {hint}"),
    };
    let result = SubscriptionLinkResult {
        profile: params.name,
        agent: params.agent,
        gesture,
    };
    Ok(serde_json::to_value(result).expect("SubscriptionLinkResult serializes"))
}

/// `subscription/unlink`: undo a linked subscription (vincular-suscripciones
/// D2/D4). Removes the profile from the daemon-owned store only — a
/// hand-written profile refuses with its remedy — and NEVER deletes the
/// context directory: whatever the provider stored there is not Meltemi's to
/// destroy. The response names the path left behind.
async fn handle_subscription_unlink(
    params: Value,
    state: &Arc<DaemonState>,
) -> Result<Value, RpcError> {
    use meltemi_proto::{SubscriptionUnlinkParams, SubscriptionUnlinkResult};

    let params: SubscriptionUnlinkParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("subscription/unlink: {e}")))?;

    match crate::subscriptions::remove(&state.config_dir, &params.name) {
        Some(removed) => {
            let context_dir = removed
                .env
                .first()
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let result = SubscriptionUnlinkResult {
                profile: removed.name,
                context_dir,
            };
            Ok(serde_json::to_value(result).expect("SubscriptionUnlinkResult serializes"))
        }
        None => {
            // Not in the store. A hand-written homonym is the user's, not ours.
            let config = crate::config::Config::load(&state.config_dir, None);
            if config.fleet_profiles.iter().any(|p| p.name == params.name) {
                Err(subscription_refused(
                    format!(
                        "`{}` lives in hand-written configuration, not in the linked store",
                        params.name
                    ),
                    "Edit it in your config.toml ([[fleet.profile]]); unlink only removes links.",
                ))
            } else {
                Err(subscription_refused(
                    format!("no link named `{}`", params.name),
                    "List the fleet with `meltemi fleet` to see the linked profiles.",
                ))
            }
        }
    }
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
async fn handle_context_project(
    params: Value,
    state: &Arc<DaemonState>,
) -> Result<Value, RpcError> {
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
    // Resolving a project's context by contract is real use of that project, and
    // the requirement names it as a registration moment alongside a session
    // start (multiproyecto-suscripciones D3). Still no disk walk: only what the
    // caller pointed at.
    crate::projects::touch(&state.data_dir, &project_root);
    // A configured level-4 agent contributes its instruction file as a target
    // (projection is its only integration channel).
    let config = crate::config::Config::load(&state.config_dir, Some(&project_root));
    let l4 = crate::levels::l4_target_for(&config);
    let written = crate::context::project_and_write_with(&project_root, l4.as_deref())
        .map_err(RpcError::internal)?;
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
