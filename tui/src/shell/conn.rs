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
    PermissionPendingResult, PermissionRule, SessionCancelParams, SessionDirectParams,
    SessionListParams, SessionListResult, SessionLogParams, SessionLogResult, StatusResult,
    WorktreeDiffResult, methods,
};

use crate::shell::live::{
    FleetRow, FleetSnapshot, ProjectRow, RaceBoard, RaceLane, SessionRow, Update,
};
use crate::shell::messages::{Lang, Msg, text};
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
    /// Steer a session with an instruction (`session/direct`): queued as the
    /// next turn of a live session, or resumed when it has ended and can be.
    Direct {
        session_id: String,
        project_root: String,
        instruction: String,
    },
    /// Read the lanes of a task for the race board (`worktree/diff`).
    RaceDiff { change: String, task: String },
    /// Run a competitor's turn over its worktree (`worktree/dispatch`).
    RaceDispatch {
        change: String,
        task: String,
        agent: String,
    },
    /// Add a root to the project registry (`project/register`).
    RegisterProject(String),
    /// Drop a project from the registry's listing (`project/forget`).
    ForgetProject(String),
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

/// Runs the connection actor until the UI drops its command sender. `lang` is
/// carried because some answers are reported as notices and the words are the
/// surface's, never the daemon's (constitution §11).
pub async fn connection_actor(
    endpoint: String,
    lang: Lang,
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
        match serve_connection(stream, lang, &mut commands, &updates).await {
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
    lang: Lang,
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
                Some(Command::RaceDiff { change, task }) => {
                    refresh_race(&peer, updates, scope.as_deref(), &change, &task).await;
                }
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
                Some(Command::Direct { session_id, project_root, instruction }) => {
                    // Not awaited here, and the reason is the resume branch:
                    // directing an ended session runs a whole turn before it
                    // answers, and this loop is what keeps the shell breathing —
                    // status, events, the permission tray. A cloned peer answers
                    // on its own task and reports back as a notice.
                    let peer = peer.clone();
                    let updates = updates.clone();
                    tokio::spawn(async move {
                        let params = SessionDirectParams {
                            session_id,
                            instruction,
                            project_root: Some(project_root),
                        };
                        let notice = match peer.request(methods::SESSION_DIRECT, &params).await {
                            Ok(value) => direct_notice(&value, lang),
                            Err(error) => refusal_notice(&error, lang),
                        };
                        let _ = updates.send(Update::Notice(notice));
                    });
                }
                Some(Command::RaceDispatch { change, task, agent }) => {
                    // A dispatch answers only when the competitor's whole turn
                    // is over — minutes, not milliseconds. Awaiting it here
                    // would stop the status tick, the event stream and the
                    // permission tray for exactly as long, which is the one
                    // thing the shell must never do. Same shape as `direct`.
                    let peer = peer.clone();
                    let updates = updates.clone();
                    let scope = scope.clone();
                    tokio::spawn(async move {
                        let root = match scope {
                            Some(root) => root,
                            None => std::env::current_dir()
                                .map(|d| d.display().to_string())
                                .unwrap_or_default(),
                        };
                        let params = serde_json::json!({
                            "projectRoot": root,
                            "change": change,
                            "task": task,
                            "agent": agent,
                        });
                        let notice = match peer.request(methods::WORKTREE_DISPATCH, &params).await {
                            Ok(value) => dispatch_notice(&value, &agent, lang),
                            Err(error) => format!("worktree/dispatch: {error}"),
                        };
                        let _ = updates.send(Update::Notice(notice));
                        // The turn is over: the lane has a new diff and a new
                        // commit state, so the board re-reads itself.
                        refresh_race(&peer, &updates, Some(root.as_str()), &change, &task).await;
                    });
                }
                Some(Command::RegisterProject(root)) => {
                    register_project(&peer, updates, lang, root).await;
                }
                Some(Command::ForgetProject(root)) => {
                    forget_project(&peer, updates, lang, root).await;
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

/// Registers a project root and re-queries the registry, so the view shows the
/// new entry without the shell being restarted. A refusal — the path does not
/// exist, or is not a directory — arrives with the daemon's remedy attached.
async fn register_project(
    peer: &Peer,
    updates: &UnboundedSender<Update>,
    lang: Lang,
    root: String,
) {
    let params = meltemi_proto::ProjectRegisterParams { root };
    match peer.request(methods::PROJECT_REGISTER, &params).await {
        Ok(value) => {
            let registered = value["project"]["root"].as_str().unwrap_or_default();
            let _ = updates.send(Update::Notice(format!(
                "{}: {registered}",
                text(Msg::ProjectRegistered, lang)
            )));
            refresh_projects(peer, updates).await;
        }
        Err(error) => {
            let _ = updates.send(Update::Notice(refusal_line(&error)));
        }
    }
}

/// Forgets a project and re-queries the registry. The notice says what was NOT
/// done, because "forget" is one keystroke away from being read as "delete".
async fn forget_project(peer: &Peer, updates: &UnboundedSender<Update>, lang: Lang, root: String) {
    let params = meltemi_proto::ProjectForgetParams { root: root.clone() };
    match peer.request(methods::PROJECT_FORGET, &params).await {
        Ok(value) => {
            let key = if value["forgotten"].as_bool().unwrap_or(false) {
                Msg::ProjectForgotten
            } else {
                Msg::ProjectNotListed
            };
            let _ = updates.send(Update::Notice(format!("{root}: {}", text(key, lang))));
            refresh_projects(peer, updates).await;
        }
        Err(error) => {
            let _ = updates.send(Update::Notice(refusal_line(&error)));
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

/// Reads the lanes of one task for the race board (`worktree/diff`). The
/// provenance rides on the same answer, so the board is one round trip and the
/// shell never joins sessions to worktrees itself (design D1).
async fn refresh_race(
    peer: &Peer,
    updates: &UnboundedSender<Update>,
    scope: Option<&str>,
    change: &str,
    task: &str,
) {
    let root = match scope {
        Some(root) => root.to_string(),
        None => match std::env::current_dir() {
            Ok(dir) => dir.display().to_string(),
            Err(e) => {
                let _ = updates.send(Update::Notice(format!("worktree/diff: {e}")));
                return;
            }
        },
    };
    let params = serde_json::json!({ "projectRoot": root, "change": change, "task": task });
    match peer.request(methods::WORKTREE_DIFF, &params).await {
        Ok(value) => match serde_json::from_value::<WorktreeDiffResult>(value) {
            Ok(result) => {
                let _ = updates.send(Update::Race(RaceBoard {
                    change: change.to_string(),
                    task: task.to_string(),
                    base_rev: result.base_rev,
                    lanes: result
                        .competitors
                        .into_iter()
                        .map(|c| RaceLane {
                            agent: c.agent,
                            path: c.path,
                            changed_files: c.changed_files.len(),
                            diff: c.diff,
                            source: c
                                .source
                                .and_then(|s| serde_json::to_value(s).ok())
                                .and_then(|v| v.as_str().map(str::to_string)),
                            profile: c.profile,
                            level: c.level,
                            session_id: c.session_id,
                            committed: c.committed,
                            sha: c.sha,
                            base_rev: c.base_rev,
                        })
                        .collect(),
                    selected: 0,
                }));
            }
            Err(e) => {
                let _ = updates.send(Update::Notice(format!("worktree/diff: {e}")));
            }
        },
        Err(error) => {
            let _ = updates.send(Update::Notice(format!("worktree/diff: {error}")));
        }
    }
}

/// What a finished dispatch says in one line: the lane, how the turn ended and
/// whether it left a commit. Never a bare "ok": the two facts differ.
fn dispatch_notice(value: &serde_json::Value, agent: &str, lang: Lang) -> String {
    let status = value["status"].as_str().unwrap_or("?");
    let committed = value["committed"].as_bool().unwrap_or(false);
    let sha = value["sha"].as_str().unwrap_or("");
    let short: String = sha.chars().take(12).collect();
    match (lang, committed) {
        (Lang::Es, true) => format!("{agent}: turno {status}, commit {short}"),
        (Lang::Es, false) => format!("{agent}: turno {status}, sin commit"),
        (Lang::En, true) => format!("{agent}: turn {status}, commit {short}"),
        (Lang::En, false) => format!("{agent}: turn {status}, no commit"),
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

/// What became of a directed instruction, in the surface's own words. Both
/// dispositions are told apart out loud: a queued instruction has NOT been
/// attended yet, and saying otherwise would be the shell inventing progress.
fn direct_notice(value: &Value, lang: Lang) -> String {
    use meltemi_proto::{DirectDisposition, SessionDirectResult};
    let Ok(result) = serde_json::from_value::<SessionDirectResult>(value.clone()) else {
        return format!("session/direct: {value}");
    };
    match result.disposition {
        DirectDisposition::Queued => format!(
            "{} #{} — {}",
            text(Msg::DirectQueued, lang),
            result.queue_position.unwrap_or(0),
            result.session_id
        ),
        DirectDisposition::Resumed => {
            let mut notice = format!("{} — {}", text(Msg::DirectResumed, lang), result.session_id);
            if result.denied_permissions > 0 {
                notice.push_str(&format!(" ({} denegado/s)", result.denied_permissions));
            }
            notice
        }
    }
}

/// The daemon's own diagnosis with its remedy, kept together. The sentence that
/// tells the user what to do next is the contract's, and dropping it would leave
/// a dead end where an action used to be.
fn refusal_line(error: &RpcError) -> String {
    let detail = error
        .data
        .as_ref()
        .and_then(|data| data["detail"].as_str())
        .unwrap_or(&error.message);
    match error.data.as_ref().and_then(|data| data["remedy"].as_str()) {
        Some(remedy) => format!("{detail} — {remedy}"),
        None => detail.to_string(),
    }
}

/// A refused direction, behind the surface's own label.
fn refusal_notice(error: &RpcError, lang: Lang) -> String {
    format!(
        "{}: {}",
        text(Msg::DirectRefused, lang),
        refusal_line(error)
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::{DirectDisposition, SessionDirectResult, TurnStatus};

    // Scenario: Instrucción dirigida desde el drill-in
    #[test]
    fn a_queued_instruction_is_reported_queued_with_its_position() {
        let queued = serde_json::to_value(SessionDirectResult {
            disposition: DirectDisposition::Queued,
            session_id: "s-1".into(),
            resumed_from: None,
            queue_position: Some(2),
            status: None,
            denied_permissions: 0,
        })
        .expect("serializes");
        let notice = direct_notice(&queued, Lang::Es);
        assert!(
            notice.contains("encolada") && notice.contains("#2") && notice.contains("s-1"),
            "queued says queued, with its position: {notice}"
        );
        // The turn in flight was not interrupted and the instruction has not
        // been attended: the notice must not suggest either.
        assert!(
            !notice.contains("enviada") && !notice.contains("atendida"),
            "a queued instruction is not an attended one: {notice}"
        );

        let resumed = serde_json::to_value(SessionDirectResult {
            disposition: DirectDisposition::Resumed,
            session_id: "s-2".into(),
            resumed_from: Some("s-1".into()),
            queue_position: None,
            status: Some(TurnStatus::Completed),
            denied_permissions: 0,
        })
        .expect("serializes");
        let notice = direct_notice(&resumed, Lang::En);
        assert!(
            notice.contains("resumed") && notice.contains("s-2"),
            "a resume says so, and names the session that continues: {notice}"
        );
    }

    #[test]
    fn a_refusal_keeps_the_daemons_diagnosis_and_its_remedy() {
        let error = RpcError::application(
            meltemi_proto::error_codes::SESSION_NOT_FOUND,
            "session not directable",
            "session_not_found",
            "no session `s-9` is active or resumable",
            Some("List sessions with `meltemi sessions`.".into()),
        );
        let notice = refusal_notice(&error, Lang::Es);
        assert!(notice.contains("no admite"), "{notice}");
        assert!(
            notice.contains("s-9") && notice.contains("meltemi sessions"),
            "the diagnosis and the remedy are the daemon's, and both survive: {notice}"
        );
    }
}
