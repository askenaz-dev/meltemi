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
        methods::CONTEXT_PROJECT => handle_context_project(params, state).await,
        methods::SESSION_LIST => handle_session_list(params, state).await,
        methods::SESSION_LOG => handle_session_log(params, state).await,
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
        methods::WORKTREE_DIFF => handle_worktree_diff(params).await,
        methods::WORKTREE_MERGE_FILE => handle_worktree_merge_file(params).await,
        methods::CHECKPOINT_CREATE => handle_checkpoint_create(params).await,
        methods::CHECKPOINT_LIST => handle_checkpoint_list(params).await,
        methods::CHECKPOINT_REVERT => handle_checkpoint_revert(params).await,
        methods::CHECKPOINT_RECORD_OP => handle_checkpoint_record_op(params).await,
        methods::COMMIT_TASK => handle_commit_task(params).await,
        methods::SDD_VERIFY => handle_sdd_verify(params).await,
        methods::SDD_VERIFY_MARK => handle_sdd_verify_mark(params).await,
        methods::SDD_ARCHIVE => handle_sdd_archive(params).await,
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
/// base — the side-by-side input to assisted merge (orquestacion-worktrees D5).
async fn handle_worktree_diff(params: Value) -> Result<Value, RpcError> {
    use meltemi_proto::{WorktreeCompetitorDiff, WorktreeDiffParams, WorktreeDiffResult};

    let params: WorktreeDiffParams = serde_json::from_value(params)
        .map_err(|e| RpcError::invalid_params(format!("worktree/diff: {e}")))?;
    let root = require_git_root(&params.project_root)?;

    let competitors = crate::worktrees::competitors(&root, &params.change, &params.task);
    let base_rev = competitors
        .first()
        .map(|w| w.base_rev.clone())
        .unwrap_or_default();
    let competitors = competitors
        .into_iter()
        .map(|w| {
            let path = PathBuf::from(&w.path);
            WorktreeCompetitorDiff {
                agent: w.agent,
                changed_files: crate::git::changed_files(&path, &w.base_rev),
                diff: crate::git::diff_against(&path, &w.base_rev),
                path: w.path,
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
