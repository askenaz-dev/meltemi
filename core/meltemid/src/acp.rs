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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{Mutex, Notify};

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest,
    PermissionOption as AcpPermissionOption, PermissionOptionKind as AcpPermissionOptionKind,
    PromptRequest, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionId, SessionNotification, StopReason, TextContent,
};
use agent_client_protocol::{AcpAgent, Agent, Client, ConnectionTo};

use meltemi_proto::{
    PermissionDecidedBy, PermissionOption, PermissionOptionKind, PermissionOutcome,
    PermissionRequestParams, PermissionRequestResult, PermissionTimeoutParams, SessionEventKind,
    SessionState, TurnStatus, methods,
};

use crate::config::WaitPolicy;
use crate::pending::{NewRequest, PendingQueue, Resolution};
use crate::permissions::{RequestFacts, RuleDecision, RuleSet};
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
    /// Set when cancellation is requested. Checked at each turn boundary so the
    /// multi-turn loop stops dispatching queued instructions even if the agent
    /// reports a non-cancelled stop reason (control-remoto-asistido).
    pub cancelled: Arc<AtomicBool>,
    /// Raised while a prompt is in flight, so `session/direct` can tell a turn
    /// that can be interrupted from one that has not started (redirigir-turno).
    pub in_flight: Arc<AtomicBool>,
    /// How long an escalated request waits for a human decision
    /// (espera-humana D2).
    pub wait: WaitPolicy,
    /// How long a pending request survives with no client connected before
    /// the constitutional deny (espera-humana D3).
    pub no_client_grace: Duration,
    /// Registry of initialized clients, observed by the wait (D3).
    pub clients: crate::clients::ClientRegistry,
    /// The session registry, so an escalated request declares the session
    /// blocked on a human decision (sesion-esperando).
    pub sessions: crate::session::SessionRegistry,
    /// The permission rules evaluated before escalation (proxy-permisos D1).
    pub rules: Arc<RuleSet>,
    /// The daemon's shared pending-permission queue (D2).
    pub pending: PendingQueue,
    /// When resuming, the agent session id to load instead of opening a new
    /// one (requires the agent to support session load; sesiones-reanudables).
    pub load_session_id: Option<String>,
    /// Declared MCP servers to inject when the agent announces support
    /// (mcp-passthrough D2).
    pub mcp_servers: Vec<crate::config::McpServerConfig>,
    /// Environment overlay applied to the agent's subprocess — the profile's
    /// auth context (flota-multiproveedor). Passed as leading `NAME=value`
    /// tokens to the ACP launch; the values are NEVER logged (§2).
    pub env: Vec<(String, String)>,
    /// When the session accepts remote direction, the FIFO the loop drains after
    /// each turn — each queued instruction becomes the next prompt of the same
    /// agent session (control-remoto-asistido). `None` for single-turn runs.
    pub instruction_queue: Option<crate::session::InstructionQueue>,
    /// How long the session waits for its next instruction before ending, and
    /// how many sessions may wait at once (sesion-que-espera design D4). Zero
    /// timeout restores the pre-change behaviour: a turn ends its session.
    pub idle_timeout: Duration,
    /// How many sessions may wait at once; reaching it closes the oldest wait.
    pub max_idle_sessions: usize,
    /// Registration of this session over its tree (edit-surface soft lock):
    /// marks turns in flight and drains the human-edit notes into the next
    /// turn's prompt (gui-tauri-paridad design D4).
    pub edit_scope: Option<crate::edits::EditScopeHandle>,
}

/// The result of one ACP turn: the mapped stop reason, how many permission
/// requests were denied (for `propose`'s honesty), and the session-resume
/// metadata (whether the agent supports load, and its session id).
#[derive(Debug, Clone)]
pub struct SessionOutcome {
    /// The mapped ACP stop reason.
    pub status: TurnStatus,
    /// Permission requests denied during the turn (rule, human, or timeout).
    pub denied_permissions: u32,
    /// Whether the agent announced session-load support in its handshake.
    pub supports_load: bool,
    /// The agent's own session id (needed to resume this session later).
    pub agent_session_id: Option<String>,
    /// Why the session ended, when it was not simply the work running out.
    /// `None` means the ordinary completion the finalizer already reports;
    /// `Some` carries a stable English identifier the surfaces translate, never
    /// a sentence (sesion-que-espera design D5).
    pub ended_reason: Option<String>,
}

/// Runs one ACP turn end to end and returns the mapped stop reason and the
/// count of denied permissions.
pub async fn run_session(params: SessionParams) -> anyhow::Result<SessionOutcome> {
    let SessionParams {
        agent_command,
        project_root,
        prompt,
        meltemi_session_id,
        peer,
        log,
        cancel,
        cancelled,
        in_flight,
        wait,
        no_client_grace,
        clients,
        sessions,
        rules,
        pending,
        load_session_id,
        mcp_servers,
        env,
        instruction_queue,
        idle_timeout,
        max_idle_sessions,
        edit_scope,
    } = params;

    // The env overlay rides as leading `NAME=value` tokens: the ACP crate parses
    // them into the subprocess environment (verified in acp_agent.rs). The
    // un-prefixed `agent_command` stays the identity used for logging — env
    // values are never logged (§2).
    let launch_argv: Vec<String> = if env.is_empty() {
        agent_command.clone()
    } else {
        let mut argv: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        argv.extend(agent_command.iter().cloned());
        argv
    };
    let agent = AcpAgent::from_args(&launch_argv)
        .map_err(|e| anyhow::anyhow!("invalid agent command: {e:?}"))?;

    let denials = Arc::new(AtomicU32::new(0));

    // Handler state (the connect_with closure keeps the originals).
    let notif = HandlerState {
        peer: peer.clone(),
        log: log.clone(),
        session_id: meltemi_session_id.clone(),
        rules: rules.clone(),
        pending: pending.clone(),
        project_root: project_root.clone(),
        denials: denials.clone(),
        wait,
        no_client_grace,
        clients: clients.clone(),
        sessions: sessions.clone(),
        cancel: Arc::clone(&cancel),
    };
    let perm = HandlerState {
        peer: peer.clone(),
        log: log.clone(),
        session_id: meltemi_session_id.clone(),
        rules: rules.clone(),
        pending: pending.clone(),
        project_root: project_root.clone(),
        denials: denials.clone(),
        wait,
        no_client_grace,
        clients: clients.clone(),
        sessions: sessions.clone(),
        cancel: Arc::clone(&cancel),
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
                let outcome = passthrough_permission(&perm, &request).await;
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, async move |connection: ConnectionTo<Agent>| {
            // Handshake: capture whether the agent supports session load, so
            // resume is only offered honestly (sesiones-reanudables D3).
            let init = connection
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let supports_load = init.agent_capabilities.load_session;

            // Open the session: resume by loading the prior agent session when
            // asked (and possible), otherwise create a new one.
            let acp_session_id = match &load_session_id {
                Some(prev) if supports_load => {
                    connection
                        .send_request(LoadSessionRequest::new(prev.clone(), project_root.clone()))
                        .block_task()
                        .await?;
                    SessionId::new(prev.clone())
                }
                _ => {
                    // Inject declared MCP servers only when the agent announced
                    // support; otherwise open the session without them and
                    // record the omission (mcp-passthrough D2/D3).
                    let mut request = NewSessionRequest::new(project_root.clone());
                    if !mcp_servers.is_empty() {
                        if crate::mcp::announces_mcp(&init.agent_capabilities.mcp_capabilities) {
                            request = request
                                .mcp_servers(mcp_servers.iter().map(crate::mcp::to_acp).collect());
                            let mut log = log.lock().await;
                            let _ = log.append(SessionEventKind::McpInjected {
                                servers: crate::mcp::names(&mcp_servers),
                            });
                        } else {
                            let mut log = log.lock().await;
                            let _ = log.append(SessionEventKind::McpNotDelivered {
                                reason: "the agent does not announce MCP support".into(),
                            });
                        }
                    }
                    connection
                        .send_request(request)
                        .block_task()
                        .await?
                        .session_id
                }
            };
            let agent_session_id = session_id_string(&acp_session_id);

            // Turns while the queue has work. The first turn runs the initial
            // prompt; each subsequent turn runs the next directed instruction on
            // the SAME live agent session (control-remoto-asistido). A session
            // that does not accept direction (no queue) runs exactly one turn —
            // identical to the prior single-turn behavior.
            let mut current_prompt = prompt;
            let mut idle_reason: Option<&'static str> = None;
            let final_status = loop {
                // Human edits since the agent's last turn ride in the prompt
                // itself (never a push outside ACP — design D4); the drain is
                // announce-exactly-once.
                if let Some(scope) = &edit_scope
                    && let Some(note) = scope.note_prefix()
                {
                    current_prompt = format!("{note}\n\n{current_prompt}");
                }
                {
                    let mut log = log.lock().await;
                    let _ = log.append(SessionEventKind::PromptSent {
                        text: current_prompt.clone(),
                    });
                }

                // Send the prompt, racing the client's cancellation. On cancel we
                // forward an ACP session/cancel and let the agent drain the turn
                // to its Cancelled stop reason.
                let prompt_request = PromptRequest::new(
                    acp_session_id.clone(),
                    vec![ContentBlock::Text(TextContent::new(current_prompt.clone()))],
                );
                let prompt_future = connection.send_request(prompt_request).block_task();
                tokio::pin!(prompt_future);
                in_flight.store(true, Ordering::SeqCst);
                if let Some(scope) = &edit_scope {
                    scope.begin_turn();
                }
                let response = tokio::select! {
                    r = &mut prompt_future => r,
                    _ = cancel.notified() => {
                        let _ = connection
                            .send_notification(CancelNotification::new(acp_session_id.clone()));
                        (&mut prompt_future).await
                    }
                };
                if let Some(scope) = &edit_scope {
                    scope.end_turn();
                }
                in_flight.store(false, Ordering::SeqCst);
                let response = response?;
                let status = map_stop_reason(response.stop_reason);

                // Turn boundary. A cancellation request stops further dispatch
                // even if the agent reported a non-cancelled stop reason, and so
                // does a cancelled turn: a directed instruction that arrives late
                // must take the resume path, never queue into a turn that will
                // not run. Otherwise dispatch the next queued instruction.
                // A stopped turn means two different things, and the queue is
                // what knows which (redirigir-turno design D2): the user
                // cancelled the SESSION, or interrupted the TURN and left a
                // relay behind. A `Cancelled` the agent reported on its own —
                // with no interruption asked for — still ends the session, and
                // that prudence is untouched: an agent that cancels itself is
                // not an invitation to keep sending it work.
                // Consumed unconditionally: a flag raised against a turn that
                // ended on its own before the signal landed must not survive to
                // govern a later one.
                let relayed = match &instruction_queue {
                    Some(queue) => queue.take_interrupted().await,
                    None => false,
                };
                if cancelled.load(Ordering::SeqCst) || matches!(status, TurnStatus::Cancelled) {
                    if !relayed {
                        if let Some(queue) = &instruction_queue {
                            queue.close().await;
                        }
                        break status;
                    }
                    // Interrupted with a relay: the session continues. The flag
                    // was consumed above, so it governs this turn and not the
                    // next one.
                    //
                    // The history says so in its own event. `TurnCompleted
                    // { cancelled }` reads identically whether the agent stopped
                    // itself or a human stopped it, and a record that cannot tell
                    // those apart misreports who decided (design D4). The
                    // instruction that relayed it travels with it, because
                    // "interrupted" without "in favour of what" is half a fact.
                    if let Some(queue) = &instruction_queue {
                        let next = queue.peek().await;
                        let mut log = log.lock().await;
                        let _ = log.append(SessionEventKind::TurnInterrupted {
                            instruction: next.unwrap_or_default(),
                        });
                    }
                }
                // The boundary. An empty queue no longer ends the session: it
                // parks here, inside `connect_with`, which is the only place the
                // agent subprocess can survive — returning from this closure IS
                // killing it (design D1).
                //
                // A session with no queue is single-turn by construction and
                // must NOT gain a wait nobody asked for: the authoring flows
                // pass `None` and expect exactly one turn.
                let Some(queue) = &instruction_queue else {
                    break status;
                };
                match queue.try_take().await {
                    Some(next) => current_prompt = next,
                    None => {
                        // Declared AFTER any human wait has ended, never before:
                        // `end_waiting` restores `Active` unconditionally and
                        // would overwrite this (design D7).
                        if let Some(evicted) = sessions
                            .begin_idle(&meltemi_session_id, max_idle_sessions)
                            .await
                        {
                            tracing::info!(
                                evicted,
                                max = max_idle_sessions,
                                "idle cap reached: closing the oldest waiting session"
                            );
                        }
                        match wait_for_instruction(
                            queue,
                            &cancel,
                            &clients,
                            no_client_grace,
                            idle_timeout,
                        )
                        .await
                        {
                            Waking::Instruction(next) => {
                                sessions.end_idle(&meltemi_session_id).await;
                                sessions
                                    .set_state(&meltemi_session_id, SessionState::Active)
                                    .await;
                                current_prompt = next;
                            }
                            Waking::Ended(reason) => {
                                sessions.end_idle(&meltemi_session_id).await;
                                queue.close().await;
                                idle_reason = Some(reason);
                                break status;
                            }
                        }
                    }
                }
            };

            Ok((final_status, supports_load, agent_session_id, idle_reason))
        })
        .await;

    let (status, supports_load, agent_session_id, idle_reason) =
        result.map_err(|e| anyhow::anyhow!("acp session failed: {e:?}"))?;
    Ok(SessionOutcome {
        status,
        denied_permissions: denials.load(Ordering::Relaxed),
        supports_load,
        agent_session_id,
        // "cancelled" is not carried here: cancelling already has its own
        // recorded ending, and reporting it twice in two vocabularies is how a
        // log starts disagreeing with itself.
        ended_reason: idle_reason
            .filter(|reason| *reason != "cancelled")
            .map(str::to_string),
    })
}

/// Renders an ACP session id as a plain string (it serializes as a string).
fn session_id_string(id: &SessionId) -> Option<String> {
    serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
}

/// Shared, cheaply-cloneable state the ACP callbacks need.
struct HandlerState {
    peer: Peer,
    log: Arc<Mutex<SessionLog>>,
    session_id: String,
    /// Rules evaluated before a request is escalated.
    rules: Arc<RuleSet>,
    /// The daemon's pending-permission queue.
    pending: PendingQueue,
    /// The session's project root (for project-scoped rule creation).
    project_root: PathBuf,
    /// Count of denied permissions this turn.
    denials: Arc<AtomicU32>,
    /// The wait policy of this flow (espera-humana D2).
    wait: WaitPolicy,
    /// Reconnect grace before the constitutional no-client deny (D3).
    no_client_grace: Duration,
    /// Registry of initialized clients the wait observes (D3).
    clients: crate::clients::ClientRegistry,
    /// The session registry, to declare the session blocked while it waits.
    sessions: crate::session::SessionRegistry,
    /// The same signal the prompt races. A permission waiting on a human is the
    /// one place where a turn can be stopped and nobody would notice: the agent
    /// is blocked on our answer, so it cannot honour a cancel it never gets to
    /// read (redirigir-turno design D3).
    cancel: Arc<Notify>,
}

/// What ended a session's wait for its next instruction.
enum Waking {
    /// An instruction arrived; it becomes the next turn.
    Instruction(String),
    /// The wait is over for a stated reason, and the session ends with it.
    Ended(&'static str),
}

/// Parks the session between turns until something happens, and says what.
///
/// Four ways out, and every one of them is a fact rather than a timeout in
/// disguise:
///
/// - an instruction arrived (the point of waiting at all);
/// - the session was cancelled — `session/cancel`, or the daemon's ordered
///   shutdown, which cancels every session and then waits for them to go;
/// - nobody has been connected for the grace: a session waiting for an
///   instruction with no client to send one is waiting for nobody. This reuses
///   the future the constitutional deny already runs on, rather than inventing
///   a second idea of "nobody is there" (design D4);
/// - the idle timeout elapsed.
///
/// Correctness of the first rests on the queue's ordering contract: mutate,
/// signal, and only then release the lock — so an instruction landing between
/// the check and the await leaves a permit rather than being missed.
async fn wait_for_instruction(
    queue: &crate::session::InstructionQueue,
    cancel: &Arc<Notify>,
    clients: &crate::clients::ClientRegistry,
    no_client_grace: Duration,
    idle_timeout: Duration,
) -> Waking {
    // Zero means "do not wait", which is exactly the behaviour before this
    // change existed. Kept reachable so the whole feature has an off switch
    // that is one config line, not a rebuild.
    if idle_timeout.is_zero() {
        return Waking::Ended("idle_disabled");
    }
    let deadline = tokio::time::sleep(idle_timeout);
    tokio::pin!(deadline);
    let no_clients = no_clients_sustained(clients.watch(), no_client_grace);
    tokio::pin!(no_clients);

    loop {
        // Check first, await second. The other order is the race.
        if let Some(instruction) = queue.try_take().await {
            return Waking::Instruction(instruction);
        }
        if !queue.accepting().await {
            return Waking::Ended("cancelled");
        }
        tokio::select! {
            () = queue.wait_for_work() => continue,
            () = cancel.notified() => return Waking::Ended("cancelled"),
            () = &mut no_clients => return Waking::Ended("no_client"),
            () = &mut deadline => return Waking::Ended("idle_timeout"),
        }
    }
}

/// The resolution of a permission whose turn was interrupted.
///
/// `Cancelled` is what goes back to the agent, and `Interrupted` is who is
/// recorded as having decided it. `denied: true` is not bookkeeping pedantry: an
/// unanswered request that stops counting as a denial would let a propose report
/// "no denials" for a turn a human cut short (design D3).
fn cancelled_resolution() -> Resolution {
    Resolution {
        outcome: PermissionOutcome::Cancelled,
        decided_by: PermissionDecidedBy::Interrupted,
        rule: None,
        denied: true,
    }
}

/// Logs an agent update, which is what forwards it to every interested
/// connection (4.3). The publication used to live here, and that is exactly why
/// a live client saw the agent's prose but never the prompt that opened the turn
/// nor the completion that closed it: those are written elsewhere. It now
/// belongs to the log itself (lanzador-conversacional D3), so this appends and
/// nothing more — publishing here as well would deliver every update twice.
async fn forward_update(state: &HandlerState, notification: SessionNotification) {
    let update = serde_json::to_value(&notification.update).unwrap_or(Value::Null);
    let mut log = state.log.lock().await;
    let _ = log.append(SessionEventKind::AgentUpdate { update });
}

/// Decides a permission request: consult the rules first (allow/deny without
/// bothering the human), and only escalate to the pending queue when no rule
/// applies. Every outcome is logged with its provenance (rule/human/timeout)
/// and counted toward the turn's denials (proxy-permisos D1/D2/D5).
async fn passthrough_permission(
    state: &HandlerState,
    request: &RequestPermissionRequest,
) -> RequestPermissionOutcome {
    {
        let mut log = state.log.lock().await;
        let _ = log.append(SessionEventKind::PermissionRequested {
            request: serde_json::to_value(request).unwrap_or(Value::Null),
        });
    }

    let tool_call = serde_json::to_value(&request.tool_call).unwrap_or(Value::Null);
    let facts = RequestFacts::from_tool_call(&tool_call);
    let options: Vec<PermissionOption> = request.options.iter().map(to_meltemi_option).collect();

    let resolution = match state.rules.evaluate(&facts) {
        RuleDecision::Allow(rule) => Resolution {
            outcome: allow_outcome(&options),
            decided_by: PermissionDecidedBy::Rule,
            rule: Some(rule),
            denied: false,
        },
        RuleDecision::Deny(rule) => Resolution {
            outcome: deny_outcome(&options),
            decided_by: PermissionDecidedBy::Rule,
            rule: Some(rule),
            denied: true,
        },
        RuleDecision::Ask => escalate(state, &facts, &tool_call, &options).await,
    };

    if resolution.denied {
        state.denials.fetch_add(1, Ordering::Relaxed);
    }
    {
        let mut log = state.log.lock().await;
        let _ = log.append(SessionEventKind::PermissionDecided {
            outcome: serde_json::to_value(&resolution.outcome).unwrap_or(Value::Null),
            decided_by: resolution.decided_by,
            denied: Some(resolution.denied),
            rule: resolution.rule.clone(),
        });
    }
    to_acp_outcome(resolution.outcome)
}

/// Escalates a request with no matching rule to the human via the pending
/// queue: it registers the request, fires the live `permission/request` push
/// to the initiating client (fast path), and awaits the queue — the push
/// answer or a `permission/decide` from any client, first one wins. The wait
/// is governed by the flow's policy (espera-humana): a bounded policy expires
/// audited as `timeout`; while clients are connected, `while-connected` waits
/// for the human; when the last client disconnects and the reconnect grace
/// passes, the constitutional deny fires explicit and audited (§3).
async fn escalate(
    state: &HandlerState,
    facts: &RequestFacts,
    tool_call: &Value,
    options: &[PermissionOption],
) -> Resolution {
    let tool = facts
        .tool
        .clone()
        .unwrap_or_else(|| "permission".to_string());
    let summary = tool_call
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| tool.clone());

    let deadline = match state.wait {
        WaitPolicy::Bounded(seconds) => Some(Instant::now() + Duration::from_secs(seconds)),
        WaitPolicy::WhileConnected => None,
    };
    let (request_id, mut rx) = state
        .pending
        .register(NewRequest {
            session_id: state.session_id.clone(),
            tool,
            summary,
            command: facts.command.clone(),
            options: options.to_vec(),
            project_root: Some(state.project_root.clone()),
            deadline,
        })
        .await;

    // From here the session is blocked on a human: say so, so `session/list`
    // distinguishes a session waiting on YOU from one hard at work
    // (sesion-esperando). Counted, so concurrent requests nest correctly.
    state.sessions.begin_waiting(&state.session_id).await;

    // Fire the live push in the background; its answer feeds the queue like a
    // decide (first-wins). Its FAILURE feeds nothing: the queue is the only
    // source of resolution (espera-humana D1).
    spawn_live_push(state, &request_id, tool_call, options);

    // The tool call the request referred to, when the agent named one: the
    // notification promises it, so it must not travel empty when it is known.
    let tool_call_id = tool_call
        .get("toolCallId")
        .and_then(Value::as_str)
        .map(str::to_string);

    let bounded = async {
        match deadline {
            Some(at) => tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(bounded);
    let no_clients = no_clients_sustained(state.clients.watch(), state.no_client_grace);
    tokio::pin!(no_clients);

    // Already signalled before this request even arrived: there is no waiter to
    // wake, so asking is the only way to find out. Without this the turn would
    // sit here until the deadline, which is precisely the wait the interruption
    // was meant to end.
    if state.sessions.is_interrupting(&state.session_id).await {
        state.pending.drop_request(&request_id).await;
        state.sessions.end_waiting(&state.session_id).await;
        return cancelled_resolution();
    }

    let resolution = tokio::select! {
        r = &mut rx => r.ok(),
        _ = state.cancel.notified() => {
            // The request leaves the tray outright — no grace, because there is
            // no longer a turn for an answer to belong to. `Cancelled` already
            // counts as a denial in the ledger, so interrupting does not open a
            // hole in the audit (design D3).
            state.pending.drop_request(&request_id).await;
            state.sessions.end_waiting(&state.session_id).await;
            return cancelled_resolution();
        }
        _ = &mut bounded => {
            state.pending.expire(&request_id).await;
            state.peer.notify(
                methods::PERMISSION_TIMEOUT,
                &PermissionTimeoutParams {
                    session_id: state.session_id.clone(),
                    tool_call_id: tool_call_id.clone(),
                },
            );
            None
        }
        _ = &mut no_clients => {
            // The constitutional deny (§3), explicit and audited. It goes
            // through the queue so a decide racing it is reconciled
            // first-wins; either way the resolution arrives on `rx`.
            default_deny_queue(&state.pending, &request_id).await;
            rx.await.ok()
        }
    };
    // However it resolved, the session is no longer blocked on this request.
    state.sessions.end_waiting(&state.session_id).await;

    resolution.unwrap_or_else(|| Resolution {
        outcome: deny_outcome(options),
        decided_by: PermissionDecidedBy::Timeout,
        rule: None,
        denied: true,
    })
}

/// Completes when the client count stays at zero for the whole grace window.
/// A client connecting during the grace restarts the vigil; a registry that
/// disappears (daemon teardown) never fires.
async fn no_clients_sustained(mut clients: tokio::sync::watch::Receiver<usize>, grace: Duration) {
    loop {
        // Wait until the count reaches zero.
        while *clients.borrow_and_update() > 0 {
            if clients.changed().await.is_err() {
                return std::future::pending().await;
            }
        }
        // Zero now: it must stay zero for the whole grace.
        let reconnected = tokio::time::timeout(grace, async {
            loop {
                if clients.changed().await.is_err() {
                    // Registry gone: never fire the deny from here.
                    return false;
                }
                if *clients.borrow_and_update() > 0 {
                    return true;
                }
            }
        })
        .await;
        match reconnected {
            // A client returned within the grace: keep waiting for the human.
            Ok(true) => continue,
            Ok(false) => return std::future::pending().await,
            // Grace elapsed with zero clients: fire.
            Err(_) => return,
        }
    }
}

/// Sends `permission/request` to the initiating client and feeds the queue
/// with its ANSWER only. A transport failure — connection gone, push never
/// answered — resolves nothing: the request stays pending for whoever
/// connects (espera-humana D1).
fn spawn_live_push(
    state: &HandlerState,
    request_id: &str,
    tool_call: &Value,
    options: &[PermissionOption],
) {
    let peer = state.peer.clone();
    let pending = state.pending.clone();
    let request_id = request_id.to_string();
    let bound = match state.wait {
        WaitPolicy::Bounded(seconds) => Some(Duration::from_secs(seconds)),
        WaitPolicy::WhileConnected => None,
    };
    let params = PermissionRequestParams {
        session_id: state.session_id.clone(),
        tool_call: tool_call.clone(),
        options: options.to_vec(),
    };
    tokio::spawn(async move {
        let push = peer.request(methods::PERMISSION_REQUEST, &params);
        let answer = match bound {
            // Bounded policy: don't leak the task past the deadline. An
            // unbounded push ends when the client answers or its connection
            // closes (the request future fails on disconnect).
            Some(bound) => match tokio::time::timeout(bound, push).await {
                Ok(answer) => answer,
                Err(_) => return,
            },
            None => push.await,
        };
        match answer {
            Ok(value) => match serde_json::from_value::<PermissionRequestResult>(value) {
                Ok(result) => {
                    let option_id = match result.outcome {
                        PermissionOutcome::Selected { option_id } => Some(option_id),
                        PermissionOutcome::Cancelled => None,
                    };
                    pending.decide(&request_id, option_id).await;
                }
                // A malformed answer is still an explicit answer: deny.
                Err(_) => default_deny_queue(&pending, &request_id).await,
            },
            // Transport failure: the request stays pending for the queue.
            Err(_) => {
                tracing::debug!(%request_id, "permission push failed; request stays pending");
            }
        }
    });
}

/// Resolves a pending request as a default-deny (no client answered).
async fn default_deny_queue(pending: &PendingQueue, request_id: &str) {
    pending
        .resolve(
            request_id,
            Resolution {
                outcome: PermissionOutcome::Cancelled,
                decided_by: PermissionDecidedBy::DefaultDeny,
                rule: None,
                denied: true,
            },
        )
        .await;
}

/// The allow outcome: select the agent's allow option, or cancel if it offered
/// none. A rule never invents an option the agent did not offer.
fn allow_outcome(options: &[PermissionOption]) -> PermissionOutcome {
    match options.iter().find(|o| {
        matches!(
            o.kind,
            PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
        )
    }) {
        Some(option) => PermissionOutcome::Selected {
            option_id: option.option_id.clone(),
        },
        None => PermissionOutcome::Cancelled,
    }
}

/// The deny outcome: select a reject option, or cancel the turn if the agent
/// offered none.
fn deny_outcome(options: &[PermissionOption]) -> PermissionOutcome {
    match options.iter().find(|o| {
        matches!(
            o.kind,
            PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
        )
    }) {
        Some(option) => PermissionOutcome::Selected {
            option_id: option.option_id.clone(),
        },
        None => PermissionOutcome::Cancelled,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A queue with a log, for the waiting tests.
    async fn wait_fixture(tag: &str) -> (crate::session::InstructionQueue, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("meltemi-acp-wait-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let registry = crate::session::SessionRegistry::default();
        registry.register("s-1", vec!["mock".into()]).await;
        let log = Arc::new(Mutex::new(
            SessionLog::create(&dir, "proj", "s-1").expect("a log to write to"),
        ));
        let queue = registry
            .enable_direction("s-1", log.clone())
            .await
            .expect("the session takes direction");
        (queue, dir)
    }

    // Scenario: Sin clientes sostenidamente, la espera termina
    #[tokio::test]
    async fn a_wait_with_nobody_connected_ends_rather_than_holding_an_agent() {
        // A session waiting for an instruction with no client to send one is
        // waiting for nobody, and what it is holding is a live agent
        // subprocess. This reuses the very future the constitutional deny runs
        // on, rather than inventing a second idea of "nobody is there".
        let (queue, dir) = wait_fixture("no-clients").await;
        let clients = crate::clients::ClientRegistry::default();
        let cancel = Arc::new(Notify::new());

        let waking = tokio::time::timeout(
            Duration::from_secs(5),
            wait_for_instruction(
                &queue,
                &cancel,
                &clients,
                Duration::from_millis(20),
                Duration::from_secs(600),
            ),
        )
        .await
        .expect("the wait ended long before the idle timeout");
        assert!(
            matches!(waking, Waking::Ended("no_client")),
            "and it says which of the four ways out it took"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_client_that_stays_keeps_the_wait_open_until_the_instruction_arrives() {
        // The other side of the same rule: with someone connected, the wait is
        // for them, and it ends when they say something.
        let (queue, dir) = wait_fixture("with-client").await;
        let clients = crate::clients::ClientRegistry::default();
        let _attending = clients.register();
        let cancel = Arc::new(Notify::new());

        let log = Arc::new(Mutex::new(
            SessionLog::create(&dir, "proj", "s-1").expect("a log"),
        ));
        let sender = queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            sender.enqueue("here you go".into(), &log).await;
        });

        let waking = tokio::time::timeout(
            Duration::from_secs(5),
            wait_for_instruction(
                &queue,
                &cancel,
                &clients,
                Duration::from_millis(20),
                Duration::from_secs(600),
            ),
        )
        .await
        .expect("the wait ended");
        match waking {
            Waking::Instruction(text) => assert_eq!(text, "here you go"),
            Waking::Ended(reason) => panic!("a connected client is somebody: got `{reason}`"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn option(id: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption {
            option_id: id.into(),
            name: id.into(),
            kind,
        }
    }

    #[test]
    fn a_rule_only_selects_options_the_agent_offered() {
        // Invariant: a rule never widens the choice — allow/deny select from
        // the offered options, and cancel when the matching kind is absent.
        let both = [
            option("y", PermissionOptionKind::AllowOnce),
            option("n", PermissionOptionKind::RejectOnce),
        ];
        assert_eq!(
            allow_outcome(&both),
            PermissionOutcome::Selected {
                option_id: "y".into()
            }
        );
        assert_eq!(
            deny_outcome(&both),
            PermissionOutcome::Selected {
                option_id: "n".into()
            }
        );

        // No allow option offered: an allow rule cannot invent one → cancel.
        let deny_only = [option("n", PermissionOptionKind::RejectOnce)];
        assert_eq!(allow_outcome(&deny_only), PermissionOutcome::Cancelled);

        // No reject option offered: a deny rule cancels the turn instead.
        let allow_only = [option("y", PermissionOptionKind::AllowOnce)];
        assert_eq!(deny_outcome(&allow_only), PermissionOutcome::Cancelled);
    }
}
