// SPDX-License-Identifier: Apache-2.0

//! Session registry: the daemon's view of active agent sessions.
//!
//! The ACP layer registers a session when it spawns an agent and deregisters
//! it when the turn ends. `status` reads this registry; `shutdown` and
//! `session/cancel` use the per-session cancel signal to terminate agents
//! without leaving orphan processes.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Mutex, Notify};

use agent_client_protocol::{Agent, ConnectionTo};
use meltemi_proto::{SessionConfigOption, SessionEventKind, SessionState, SessionSummary};

use crate::session_log::SessionLog;

/// What a live session needs to change an announced configuration option
/// without relaunching anything: the agent's own connection, the ACP session id
/// it answers to, and exactly what it announced.
///
/// The connection is cloned out of the ACP task rather than reached through a
/// command channel because ACP is full-duplex: `session/set_config_option` is a
/// request like any other and does not have to wait for a turn boundary. The
/// clone is cheap (`ConnectionTo` is a handle over the same transport), and
/// awaiting a response from the server's task is the documented safe case —
/// blocking is only a deadlock inside a connection handler, which this is not.
#[derive(Clone)]
pub struct LiveConfig {
    /// The session id the AGENT gave us, which is what its own protocol
    /// answers to — never Meltemi's session id.
    pub acp_session_id: String,
    /// The live connection to the agent.
    pub connection: ConnectionTo<Agent>,
    /// Exactly what the agent announced. Empty means it announced nothing, and
    /// no surface may offer a live change (modelo-y-esfuerzo design D2).
    pub options: Vec<SessionConfigOption>,
    /// Whether the mode option in `options` was synthesised from the
    /// protocol's older modes field rather than announced as a configuration
    /// option (apagado-entero-y-modos design D6).
    ///
    /// It decides which verb sets it, and nothing else. It cannot be read off
    /// the option: an agent is free to announce its own option with the same
    /// id and category, and that one is set through the verb it was announced
    /// with. So the derivation's answer is remembered here, per session,
    /// internal, and never travels in the contract.
    pub mode_is_inherited: bool,
    /// The session's append-only log, so a change lands where the session's
    /// history is read. Without it `agent_resolved` would name one model while
    /// the session finished under another, and nothing would say when it
    /// changed (§8; modelo-y-esfuerzo design D5).
    pub log: Arc<Mutex<SessionLog>>,
}

/// A per-session FIFO of directed instructions (control-remoto-asistido).
///
/// While the session is `accepting`, `session/direct` enqueues an instruction
/// and the multi-turn loop dispatches it as the next turn. An empty queue at a
/// turn boundary no longer ends the session: the loop WAITS on this queue's
/// signal, keeping the ACP connection and the agent subprocess alive, until an
/// instruction arrives or the session is ended for a stated reason
/// (sesion-que-espera design D1). Only then does it stop accepting, so a later
/// instruction takes the resume path instead of queueing into a session that
/// will never drain it.
#[derive(Clone)]
pub struct InstructionQueue {
    inner: Arc<Mutex<QueueInner>>,
    /// Wakes the turn boundary when work arrives.
    ///
    /// `notify_one`, never `notify_waiters`: the first STORES a permit when
    /// nobody is waiting yet, and the second drops it on the floor. That
    /// difference is the whole race. The boundary checks the queue and only then
    /// awaits, so an instruction landing in between leaves a permit that the
    /// await consumes immediately (sesion-que-espera design D1).
    ///
    /// `redirigir-turno` learned the other half of this the hard way: it uses
    /// `notify_waiters` for interruption, and an interruption asked for before
    /// the turn started woke nobody. Here a lost wake-up would park a session
    /// with work in its queue, which is worse, so the primitive is the one with
    /// a memory.
    signal: Arc<Notify>,
}

struct QueueInner {
    items: VecDeque<String>,
    accepting: bool,
    /// Set when the session is cancelled. Consulted by `take_or_close` under the
    /// same lock, so a cancel and a dequeue can never both win: once cancelled,
    /// no queued instruction is dispatched, however the turn's stop reason came
    /// back.
    cancelled: bool,
    /// Set when the user interrupted the turn AND left a relay behind. Distinct
    /// from `cancelled` on purpose: a cancellation ends the session, an
    /// interruption ends the TURN and the session continues with what was
    /// enqueued. Both live under this lock, so the boundary reads one truth
    /// (redirigir-turno design D1, D2).
    interrupted: bool,
}

impl InstructionQueue {
    fn new() -> Self {
        Self {
            signal: Arc::new(Notify::new()),
            inner: Arc::new(Mutex::new(QueueInner {
                items: VecDeque::new(),
                accepting: true,
                cancelled: false,
                interrupted: false,
            })),
        }
    }

    /// Enqueue an instruction as the session's next turn, recording it in the
    /// log *before* it becomes visible to the draining loop (log-before-enqueue,
    /// so the audit trail always shows `instruction_queued` before the matching
    /// `prompt_sent`). Returns the 1-based queue position, or `None` if the
    /// session no longer accepts turns.
    pub async fn enqueue(
        &self,
        instruction: String,
        log: &Arc<Mutex<SessionLog>>,
    ) -> Option<usize> {
        let mut inner = self.inner.lock().await;
        if !inner.accepting {
            return None;
        }
        // Written while the queue lock is held, so the loop (which locks the
        // same queue to pop) cannot dispatch this instruction before it is
        // recorded. The queue lock is never acquired while holding the log lock,
        // so there is no lock-order inversion.
        {
            let mut log = log.lock().await;
            let _ = log.append(SessionEventKind::InstructionQueued {
                instruction: instruction.clone(),
            });
        }
        inner.items.push_back(instruction);
        let position = inner.items.len();
        // Mutated first, signalled after, and both before the lock is released:
        // a boundary that wakes on this permit always finds the item already
        // there.
        self.signal.notify_one();
        Some(position)
    }

    /// Loop side: take the next queued instruction, or atomically stop accepting
    /// and return `None` when the queue is empty or the session was cancelled.
    /// The cancel check is under the same lock as [`Self::mark_cancelled`], so a
    /// cancel that has taken effect is never overtaken by a dequeue — no queued
    /// turn runs after cancellation. (A cancel racing the dequeue at the instant
    /// of the boundary may still let the one instruction being dequeued run; that
    /// residual coincidence is inherent and bounded to a single turn.)
    pub async fn take_or_close(&self) -> Option<String> {
        let mut inner = self.inner.lock().await;
        if inner.cancelled {
            inner.accepting = false;
            return None;
        }
        match inner.items.pop_front() {
            Some(instruction) => Some(instruction),
            None => {
                inner.accepting = false;
                None
            }
        }
    }

    /// Loop side: take the next queued instruction if there is one, WITHOUT
    /// closing the queue when there is not.
    ///
    /// This is the half of `take_or_close` that survives. The other half — stop
    /// accepting on an empty queue — was the atomic that shut the "enqueue while
    /// the loop exits" race, and it is gone because an empty queue now means
    /// wait, not end. What replaces the atomic is the signal's memory: see
    /// [`Self::wait_for_work`].
    ///
    /// A cancelled session still closes here, unchanged: cancelling ends a
    /// session, and no queued turn runs after it.
    pub async fn try_take(&self) -> Option<String> {
        let mut inner = self.inner.lock().await;
        if inner.cancelled {
            inner.accepting = false;
            return None;
        }
        inner.items.pop_front()
    }

    /// Loop side: park until something changes — an instruction arrives, the
    /// session is cancelled, or the queue is closed.
    ///
    /// Correctness rests on ONE ordering, and it is the caller's: check the
    /// queue first, await second. Every mutator signals after mutating and
    /// before releasing the lock, and the signal stores a permit when no one is
    /// waiting, so an instruction that lands between the check and the await is
    /// not lost — the await returns at once and the caller checks again. There
    /// is no window, and there is no busy-wait.
    ///
    /// Waking is not a promise that there IS work: spurious wake-ups are
    /// expected and the caller re-checks.
    pub async fn wait_for_work(&self) {
        self.signal.notified().await;
    }

    /// Whether this queue still accepts turns.
    pub async fn accepting(&self) -> bool {
        self.inner.lock().await.accepting
    }

    /// Enqueue a relay AND signal the interruption, under one lock and in that
    /// order.
    ///
    /// The order is the whole point. `cancel` signals first and marks after,
    /// which is fine because a cancellation leaves nothing to dispatch. An
    /// interruption does leave something, and signalling before enqueuing is
    /// exactly the window the turn boundary would use to break instead of
    /// continuing (design D1).
    ///
    /// Returns the 1-based position, or `None` when the session no longer
    /// accepts turns — in which case nothing was signalled either: an
    /// interruption without a relay is just a cancellation someone did not ask
    /// for.
    pub async fn interrupt_with(
        &self,
        instruction: String,
        log: &Arc<Mutex<SessionLog>>,
    ) -> Option<usize> {
        let mut inner = self.inner.lock().await;
        if !inner.accepting || inner.cancelled {
            return None;
        }
        {
            let mut log = log.lock().await;
            let _ = log.append(SessionEventKind::InstructionQueued {
                instruction: instruction.clone(),
            });
        }
        inner.items.push_back(instruction);
        inner.interrupted = true;
        let position = inner.items.len();
        self.signal.notify_one();
        Some(position)
    }

    /// The instruction at the head of the queue, without removing it. The
    /// record of an interruption names what relayed it, and naming it must not
    /// be what consumes it.
    pub async fn peek(&self) -> Option<String> {
        self.inner.lock().await.items.front().cloned()
    }

    /// Whether an interruption is in flight, WITHOUT consuming it.
    ///
    /// The boundary is the one that consumes; this is for the code that has to
    /// decide what to do *during* the turn — a permission still waiting on a
    /// human when the turn it belongs to is being relayed away.
    pub async fn is_interrupted(&self) -> bool {
        self.inner.lock().await.interrupted
    }

    /// Loop side: whether the turn that just drained was interrupted by the user
    /// with a relay waiting. Clears the flag, so it governs one turn and not the
    /// next.
    pub async fn take_interrupted(&self) -> bool {
        let mut inner = self.inner.lock().await;
        std::mem::take(&mut inner.interrupted)
    }

    /// Mark the session cancelled: no queued instruction is dispatched afterward,
    /// even one already sitting in the queue. Set under the queue lock so it is
    /// atomic with `take_or_close`'s decision to pop.
    pub async fn mark_cancelled(&self) {
        let mut inner = self.inner.lock().await;
        inner.cancelled = true;
        inner.accepting = false;
        // Cancel wins over a concurrent interrupt: it is the stronger gesture
        // and the only irreversible one (design D1, "Cancelar gana").
        inner.interrupted = false;
        // A session parked at the boundary waits on this signal and nothing
        // else. Without this, `session/cancel` would stop meaning anything for a
        // session that is not running a turn — which, after sesion-que-espera,
        // is most of the time.
        self.signal.notify_one();
    }

    /// Stop accepting further turns without draining (used when the loop ends a
    /// session): a late instruction then takes the resume path rather than
    /// queueing into a turn that will never run.
    pub async fn close(&self) {
        self.inner.lock().await.accepting = false;
        self.signal.notify_one();
    }
}

/// What a session opts into when it accepts remote direction: the queue the
/// loop drains and the log handle `session/direct` audits through.
struct Direction {
    queue: InstructionQueue,
    log: Arc<Mutex<SessionLog>>,
}

/// Handle the registry keeps for one live session.
struct Entry {
    agent_command: Vec<String>,
    /// The repository root this session runs in.
    ///
    /// The registry did not hold it before: `session/list` reads the root from
    /// the session index on disk and only borrows the live state from here. The
    /// live-state notification has to carry it, and looking it up on disk on
    /// every transition would read a file to answer a question the registry
    /// already had in its hand at registration (estado-en-vivo 2.1).
    project_root: String,
    state: SessionState,
    /// When this session entered `state` (RFC 3339, UTC).
    ///
    /// Set on entry and only on entry: a mutation that sets the state it
    /// already had leaves this alone, because a clock that restarted every time
    /// something re-asserted the same state would show "working for 0 s" on a
    /// session that has been working for a minute (design D6).
    state_since: String,
    /// Fired to ask the owning ACP task to cancel and terminate the agent.
    cancel: Arc<Notify>,
    /// When this session started waiting for its next instruction, so the
    /// oldest wait is the one that gives way when the cap is reached.
    idle_since: Option<std::time::Instant>,
    /// Whether a prompt is in flight right now.
    ///
    /// `Notify::notify_waiters` wakes only waiters already registered, so an
    /// interruption signalled before the turn starts wakes nobody and is lost —
    /// and answering `relayed` for it would report an interruption that never
    /// happened. Asking this first is what makes the answer true
    /// (redirigir-turno).
    in_flight: Arc<AtomicBool>,
    /// Set when cancellation is requested, so the multi-turn loop stops
    /// dispatching queued instructions at the next turn boundary even if the
    /// agent reports a non-cancelled stop reason.
    cancelled: Arc<AtomicBool>,
    /// Present once the session opts into remote direction (`propose` and the
    /// `session/direct` resume branch).
    direction: Option<Direction>,
    /// How many permission requests of this session are awaiting a human
    /// decision. Counted, not toggled: with two requests in flight, resolving
    /// one must not declare the session unblocked (sesion-esperando D1).
    waiting: u32,
    /// Present from the moment the handshake completes until the session ends.
    /// `None` — a session still starting, or one already over — is a refusal,
    /// not a wait: there is no live connection to carry the change.
    config: Option<LiveConfig>,
}

impl Entry {
    /// Moves the session into `state`, stamping when it got there.
    ///
    /// A no-op when it is already in that state, and that is the whole reason
    /// this is a method instead of two assignments at five call sites: several
    /// of those sites re-assert `Active` on a session that is already active,
    /// and a `since` that restarted on each of them would reset every clock a
    /// surface shows (design D6).
    fn enter(&mut self, state: SessionState) {
        if self.state == state {
            return;
        }
        self.state = state;
        self.state_since = crate::clock::now_rfc3339();
    }

    /// What this session's standing is, as the contract states it.
    fn affirmation(&self, session_id: &str) -> meltemi_proto::SessionStateParams {
        meltemi_proto::SessionStateParams {
            session_id: session_id.to_string(),
            project_root: self.project_root.clone(),
            state: self.state,
            since: self.state_since.clone(),
            pending_permissions: self.waiting,
        }
    }
}

/// What [`SessionRegistry::register`] hands back to the ACP task that runs the
/// session: the cancel signal it awaits and the cancellation flag it checks at
/// each turn boundary.
pub struct Registration {
    /// Await this to cancel and terminate the agent mid-turn.
    pub cancel: Arc<Notify>,
    /// Observed at turn boundaries by the multi-turn loop.
    pub cancelled: Arc<AtomicBool>,
    /// Raised by the loop while a prompt is in flight, so an interruption can
    /// tell "there is a turn to stop" from "there is not".
    pub in_flight: Arc<AtomicBool>,
}

/// Thread-safe registry of active sessions.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
    /// Where a state change is announced.
    ///
    /// The registry announces because the registry is where state changes.
    /// There are a dozen call sites that move a session, and asking each of
    /// them to remember to announce afterwards is asking for the one that
    /// forgets — and a state that moved without being announced is exactly the
    /// silent lie this change exists to end (estado-en-vivo design D2).
    ///
    /// Defaulted for tests, which publish into a hub nobody listens to and are
    /// right not to care. The daemon wires the shared one at construction.
    events: crate::events::EventHub,
}

impl SessionRegistry {
    /// The registry the daemon runs, announcing on the hub every connection
    /// listens to.
    #[must_use]
    pub fn with_hub(events: crate::events::EventHub) -> Self {
        Self {
            inner: Arc::default(),
            events,
        }
    }

    /// Registers a new session in the `Starting` state, returning the cancel
    /// signal the ACP task should await and the cancellation flag it checks at
    /// each turn boundary.
    pub async fn register(
        &self,
        session_id: &str,
        agent_command: Vec<String>,
        project_root: String,
    ) -> Registration {
        let cancel = Arc::new(Notify::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        let in_flight = Arc::new(AtomicBool::new(false));
        self.inner.lock().await.insert(
            session_id.to_string(),
            Entry {
                agent_command,
                project_root,
                state: SessionState::Starting,
                state_since: crate::clock::now_rfc3339(),
                cancel: Arc::clone(&cancel),
                cancelled: Arc::clone(&cancelled),
                idle_since: None,
                in_flight: Arc::clone(&in_flight),
                direction: None,
                waiting: 0,
                config: None,
            },
        );
        self.announce(session_id).await;
        Registration {
            cancel,
            cancelled,
            in_flight,
        }
    }

    /// Opt a session into remote direction: create its instruction queue and
    /// record the log handle so `session/direct` can enqueue (and audit) turns.
    /// Returns the queue the session loop drains, or `None` if the session is
    /// no longer registered.
    pub async fn enable_direction(
        &self,
        session_id: &str,
        log: Arc<Mutex<SessionLog>>,
    ) -> Option<InstructionQueue> {
        let mut guard = self.inner.lock().await;
        let entry = guard.get_mut(session_id)?;
        let queue = InstructionQueue::new();
        entry.direction = Some(Direction {
            queue: queue.clone(),
            log,
        });
        Some(queue)
    }

    /// The queue and log of a session that accepts direction, when it is still
    /// active. `session/direct` uses this to enqueue the next turn.
    pub async fn direct_target(
        &self,
        session_id: &str,
    ) -> Option<(InstructionQueue, Arc<Mutex<SessionLog>>)> {
        let guard = self.inner.lock().await;
        let direction = guard.get(session_id)?.direction.as_ref()?;
        Some((direction.queue.clone(), direction.log.clone()))
    }

    /// Records the live ACP connection and whatever configuration options the
    /// agent announced when the session opened, so a later change can be sent
    /// over the same connection instead of relaunching the agent.
    ///
    /// Called once per session, right after the handshake. An agent that
    /// announced nothing is still recorded — with an empty list — because
    /// "connected and announced nothing" and "not connected yet" must not be
    /// answered with the same refusal.
    pub async fn set_live_config(
        &self,
        session_id: &str,
        acp_session_id: String,
        connection: ConnectionTo<Agent>,
        announced: crate::session_config::Announced,
        log: Arc<Mutex<SessionLog>>,
    ) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.config = Some(LiveConfig {
                acp_session_id,
                connection,
                options: announced.options,
                mode_is_inherited: announced.mode_is_inherited,
                log,
            });
        }
    }

    /// The live connection and announced options of a session, when it has
    /// completed its handshake and is still registered.
    pub async fn live_config(&self, session_id: &str) -> Option<LiveConfig> {
        self.inner.lock().await.get(session_id)?.config.clone()
    }

    /// Replaces the announced options with what the agent reported after a
    /// change. The agent's answer is the record, not what we asked for: one
    /// that clamps or reorders must be believed over our request.
    pub async fn update_config_options(&self, session_id: &str, options: Vec<SessionConfigOption>) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id)
            && let Some(config) = entry.config.as_mut()
        {
            config.options = options;
            // A list that replaces the announcement replaces where the mode
            // came from too: whatever mode option the new list carries is one
            // the agent announced as a configuration option, and is set with
            // the verb it was announced with. A list with none leaves the flag
            // with nothing to describe (apagado-entero-y-modos design D7).
            //
            // The consequence, said rather than hidden: an agent that
            // announces its modes by the older field AND configuration options
            // by the newer one loses its mode selector when any other option
            // is changed, because the replacing list has no mode in it. It is
            // what "replace whole, never merge" costs, and re-deriving the
            // synthesised option here would be the merge that rule forbids.
            config.mode_is_inherited = false;
        }
    }

    /// Moves the session's mode option to `mode_id` and hands back the
    /// announcement as it now stands.
    ///
    /// `None` when this session has no live configuration or no mode option to
    /// move — an agent may have replaced its announcement with one that has
    /// none, and a notification about something nobody announces changes
    /// nothing.
    pub async fn set_current_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Option<Vec<SessionConfigOption>> {
        let mut inner = self.inner.lock().await;
        let config = inner.get_mut(session_id)?.config.as_mut()?;
        crate::session_config::set_current_mode(&mut config.options, mode_id)
            .then(|| config.options.clone())
    }

    /// Updates the lifecycle state of a session, if still present.
    ///
    /// Re-asserting the state a session already has leaves its instant alone:
    /// `since` means "entered this state", and several call sites set `Active`
    /// on a session that is already active (design D6).
    pub async fn set_state(&self, session_id: &str, state: SessionState) {
        {
            let mut guard = self.inner.lock().await;
            let Some(entry) = guard.get_mut(session_id) else {
                return;
            };
            entry.enter(state);
        }
        self.announce(session_id).await;
    }

    /// A permission request of this session began waiting for a human: the
    /// first one blocks the session (`waiting_permission`); further concurrent
    /// ones only deepen the count (sesion-esperando D1).
    pub async fn begin_waiting(&self, session_id: &str) {
        {
            let mut guard = self.inner.lock().await;
            let Some(entry) = guard.get_mut(session_id) else {
                return;
            };
            entry.waiting += 1;
            if entry.waiting == 1 {
                entry.enter(SessionState::WaitingPermission);
            }
        }
        // Announced even when the state did not move: the count is part of the
        // affirmation, and a second request waiting is news a surface shows.
        self.announce(session_id).await;
    }

    /// A waiting request resolved (any way). The session goes back to `active`
    /// only when none is left waiting; a session already deregistered is left
    /// alone, so a turn that ended mid-wait never resurfaces as active (D2).
    pub async fn end_waiting(&self, session_id: &str) {
        {
            let mut guard = self.inner.lock().await;
            let Some(entry) = guard.get_mut(session_id) else {
                return;
            };
            entry.waiting = entry.waiting.saturating_sub(1);
            if entry.waiting == 0 {
                entry.enter(SessionState::Active);
            }
        }
        self.announce(session_id).await;
    }

    /// Removes a session from the registry (turn ended), affirming that it
    /// ended on the way out.
    ///
    /// The affirmation is not decoration. Not every flow ends with
    /// `set_state(Ended)`: the single-turn runners finalize by deregistering
    /// (`session_finalize`), so without this a session that finished — or was
    /// cancelled while parked — would leave the registry with its last live
    /// state still standing in every surface, forever. That is the exact defect
    /// this change exists to close, and the end-to-end test with two clients is
    /// what found it: the second client was told the session was waiting and
    /// then never told anything again.
    pub async fn deregister(&self, session_id: &str) {
        let affirmation = {
            let mut guard = self.inner.lock().await;
            guard.remove(session_id).map(|mut entry| {
                // A session leaving the registry is a session that is over. The
                // turn's own verdict — completed, refused, failed — travels in
                // the log; the state says only that it is no longer running.
                entry.enter(SessionState::Ended);
                entry.affirmation(session_id)
            })
        };
        if let Some(affirmation) = affirmation {
            self.events.publish_state(affirmation);
        }
    }

    /// Whether a session id is currently registered.
    pub async fn contains(&self, session_id: &str) -> bool {
        self.inner.lock().await.contains_key(session_id)
    }

    /// Whether the session is registered AND still running. The multi-turn
    /// runners (`sdd/implement`, `worktree/dispatch`) end with
    /// `set_state(Ended)` and stay registered, so being registered is not being
    /// live (registro-de-la-sesion-de-calle D4).
    pub async fn is_running(&self, session_id: &str) -> bool {
        self.inner
            .lock()
            .await
            .get(session_id)
            .is_some_and(|entry| entry.state != SessionState::Ended)
    }

    /// Signals cancellation for one session. Returns whether it existed. Sets the
    /// cancellation flag and notifies the in-turn waiter, then marks the queue
    /// cancelled so a directed instruction sitting at the turn boundary is not
    /// dequeued — closing the race where a cancel lands between a turn ending and
    /// the next instruction being taken.
    pub async fn cancel(&self, session_id: &str) -> bool {
        let queue = {
            let guard = self.inner.lock().await;
            match guard.get(session_id) {
                Some(entry) => {
                    entry.cancelled.store(true, Ordering::SeqCst);
                    entry.cancel.notify_waiters();
                    entry.direction.as_ref().map(|d| d.queue.clone())
                }
                None => return false,
            }
        };
        // Marked outside the registry lock (the queue has its own lock), so there
        // is no cross-lock hold between the registry and a queue.
        if let Some(queue) = queue {
            queue.mark_cancelled().await;
        }
        true
    }

    /// Signals an interruption of the turn in flight, WITHOUT ending the
    /// session.
    ///
    /// The relay is already in the queue and the interruption flag already set
    /// — `interrupt_with` did both under one lock before this is called. What
    /// remains is waking the prompt's `select!` so the turn drains, and the
    /// session-wide `cancelled` flag is deliberately NOT set: that flag is what
    /// ends a session, and this ends a turn (redirigir-turno design D1, D2).
    pub async fn interrupt(&self, session_id: &str) -> bool {
        let guard = self.inner.lock().await;
        match guard.get(session_id) {
            Some(entry) => {
                entry.cancel.notify_waiters();
                true
            }
            None => false,
        }
    }

    /// Declares this session waiting for its next instruction and enforces the
    /// cap, returning the id of the session evicted to make room, if any.
    ///
    /// The cap closes the OLDEST wait rather than refusing the new one. Refusing
    /// would punish the user for sessions they have already stopped looking at,
    /// and the thing being rationed — a live agent subprocess — is most wasted
    /// exactly where it has been idle longest (sesion-que-espera design D4).
    ///
    /// A cap of zero means no session waits at all, which is the behaviour
    /// before this change.
    pub async fn begin_idle(&self, session_id: &str, max: usize) -> Option<String> {
        {
            let mut guard = self.inner.lock().await;
            let now = std::time::Instant::now();
            let entry = guard.get_mut(session_id)?;
            entry.idle_since = Some(now);
            entry.enter(SessionState::WaitingInstruction);
        }
        // Announced before the cap is enforced: this session is waiting now,
        // whatever the cap goes on to decide about an older one. The eviction
        // announces itself through the cancellation it causes.
        self.announce(session_id).await;

        let guard = self.inner.lock().await;
        let mut idle: Vec<(std::time::Instant, String)> = guard
            .iter()
            .filter_map(|(id, entry)| entry.idle_since.map(|since| (since, id.clone())))
            .collect();
        if idle.len() <= max {
            return None;
        }
        idle.sort_by_key(|(since, _)| *since);
        let (_, evicted) = idle.first()?.clone();
        // Cancelled, not merely un-marked: what is being reclaimed is the agent
        // subprocess, and only cancellation reaches the loop that holds it.
        if let Some(entry) = guard.get(&evicted) {
            entry.cancelled.store(true, Ordering::SeqCst);
            entry.cancel.notify_waiters();
            let queue = entry.direction.as_ref().map(|d| d.queue.clone());
            drop(guard);
            if let Some(queue) = queue {
                queue.mark_cancelled().await;
            }
        }
        Some(evicted)
    }

    /// Ends this session's wait (an instruction arrived, or the session is
    /// ending), so it stops counting against the cap.
    pub async fn end_idle(&self, session_id: &str) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.idle_since = None;
        }
    }

    /// Whether a prompt is in flight for this session — that is, whether there
    /// is a turn an interruption could actually stop.
    pub async fn turn_in_flight(&self, session_id: &str) -> bool {
        self.inner
            .lock()
            .await
            .get(session_id)
            .is_some_and(|e| e.in_flight.load(Ordering::SeqCst))
    }

    /// Whether this session has an interruption in flight. Answered without
    /// consuming it: the turn boundary is what consumes, and asking must never
    /// be what makes the answer disappear.
    pub async fn is_interrupting(&self, session_id: &str) -> bool {
        let queue = {
            let guard = self.inner.lock().await;
            match guard.get(session_id).and_then(|e| e.direction.as_ref()) {
                Some(direction) => direction.queue.clone(),
                None => return false,
            }
        };
        queue.is_interrupted().await
    }

    /// Signals cancellation for every session (used by shutdown).
    pub async fn cancel_all(&self) {
        for entry in self.inner.lock().await.values() {
            entry.cancelled.store(true, Ordering::SeqCst);
            entry.cancel.notify_waiters();
        }
    }

    /// Number of registered sessions.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Whether there are no registered sessions.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    /// Announces where one session stands to every connection.
    ///
    /// Reads under the lock and publishes after releasing it: the send never
    /// blocks, but holding a mutex across a fan-out is a habit worth not
    /// forming. A session that is no longer registered announces nothing —
    /// there is nothing to affirm about a session that left.
    async fn announce(&self, session_id: &str) {
        let affirmation = self.affirmation(session_id).await;
        if let Some(affirmation) = affirmation {
            self.events.publish_state(affirmation);
        }
    }

    /// How one session stands, as the contract states it.
    ///
    /// `None` for a session the registry does not hold: there is nothing to
    /// affirm about a session that ended and left.
    pub async fn affirmation(&self, session_id: &str) -> Option<meltemi_proto::SessionStateParams> {
        self.inner
            .lock()
            .await
            .get(session_id)
            .map(|entry| entry.affirmation(session_id))
    }

    /// How every session the registry holds stands right now.
    ///
    /// This is the snapshot a connection receives when it initializes, and the
    /// one it receives again if it ever falls behind the channel (design D3,
    /// D4).
    pub async fn affirmations(&self) -> Vec<meltemi_proto::SessionStateParams> {
        self.inner
            .lock()
            .await
            .iter()
            .map(|(session_id, entry)| entry.affirmation(session_id))
            .collect()
    }

    /// Snapshot of active sessions for the `status` method.
    pub async fn summaries(&self) -> Vec<SessionSummary> {
        self.inner
            .lock()
            .await
            .iter()
            .map(|(session_id, entry)| SessionSummary {
                session_id: session_id.clone(),
                agent_command: entry.agent_command.clone(),
                state: entry.state,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The state the registry reports to `session/list`, for the one session.
    async fn listed(registry: &SessionRegistry, session_id: &str) -> Option<SessionState> {
        registry
            .summaries()
            .await
            .into_iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.state)
    }

    // Scenario: La notificación afirma el estado entero y es idempotente
    #[tokio::test]
    async fn every_transition_is_announced_and_says_what_the_list_would_say() {
        // One affirmation per transition, and each one agreeing with the answer
        // `session/list` would give in that instant. If the two could disagree,
        // a surface fed by the channel and a surface fed by the list would show
        // different things about the same session — which is the whole class of
        // bug this change closes.
        let hub = crate::events::EventHub::default();
        let mut feed = hub.subscribe_state();
        let registry = SessionRegistry::with_hub(hub);

        let mut seen = Vec::new();
        let take = |feed: &mut tokio::sync::broadcast::Receiver<
            std::sync::Arc<meltemi_proto::SessionStateParams>,
        >| { feed.try_recv().expect("a transition was announced") };

        registry
            .register("s-1", vec!["mock".into()], "/repo".into())
            .await;
        let a = take(&mut feed);
        assert_eq!(a.state, SessionState::Starting);
        assert_eq!(a.project_root, "/repo");
        assert_eq!(listed(&registry, "s-1").await, Some(a.state));
        seen.push(a.state);

        registry.set_state("s-1", SessionState::Active).await;
        let b = take(&mut feed);
        assert_eq!(b.state, SessionState::Active);
        assert_eq!(listed(&registry, "s-1").await, Some(b.state));
        seen.push(b.state);

        registry.begin_waiting("s-1").await;
        let c = take(&mut feed);
        assert_eq!(c.state, SessionState::WaitingPermission);
        assert_eq!(c.pending_permissions, 1);
        assert_eq!(listed(&registry, "s-1").await, Some(c.state));
        seen.push(c.state);

        // A second request waiting does not move the state, and is announced
        // anyway: the count is part of what the affirmation states.
        registry.begin_waiting("s-1").await;
        let d = take(&mut feed);
        assert_eq!(d.state, SessionState::WaitingPermission);
        assert_eq!(d.pending_permissions, 2);

        registry.end_waiting("s-1").await;
        let _ = take(&mut feed);
        registry.end_waiting("s-1").await;
        let e = take(&mut feed);
        assert_eq!(e.state, SessionState::Active);
        assert_eq!(e.pending_permissions, 0);
        assert_eq!(listed(&registry, "s-1").await, Some(e.state));
        seen.push(e.state);

        registry.begin_idle("s-1", 8).await;
        let f = take(&mut feed);
        assert_eq!(f.state, SessionState::WaitingInstruction);
        assert_eq!(listed(&registry, "s-1").await, Some(f.state));
        seen.push(f.state);

        registry.set_state("s-1", SessionState::Ended).await;
        let g = take(&mut feed);
        assert_eq!(g.state, SessionState::Ended);
        assert_eq!(listed(&registry, "s-1").await, Some(g.state));
        seen.push(g.state);

        assert_eq!(
            seen,
            [
                SessionState::Starting,
                SessionState::Active,
                SessionState::WaitingPermission,
                SessionState::Active,
                SessionState::WaitingInstruction,
                SessionState::Ended,
            ],
            "the whole lifecycle was announced, in order"
        );

        // Nothing else was published: an affirmation per transition, not per
        // call.
        assert!(feed.try_recv().is_err());
    }

    #[tokio::test]
    async fn leaving_the_registry_is_announced_as_having_ended() {
        // Not every flow ends with an explicit `Ended`: the single-turn runners
        // finalize by deregistering. A silent exit would leave every surface
        // showing the last live state of a session that is over.
        let hub = crate::events::EventHub::default();
        let mut feed = hub.subscribe_state();
        let registry = SessionRegistry::with_hub(hub);

        registry
            .register("s-1", vec!["mock".into()], "/repo".into())
            .await;
        registry.begin_idle("s-1", 8).await;
        while feed.try_recv().is_ok() {}

        registry.deregister("s-1").await;
        let last = feed.try_recv().expect("leaving is announced");
        assert_eq!(last.state, SessionState::Ended);
        assert_eq!(last.session_id, "s-1");
        assert_eq!(last.project_root, "/repo");

        // And it really is gone: nothing more to affirm about it.
        assert!(registry.affirmation("s-1").await.is_none());
    }

    #[tokio::test]
    async fn a_session_that_is_not_registered_announces_nothing() {
        // Every mutator takes an id that may name nothing — a session that
        // ended and left. Announcing there would affirm a session that does not
        // exist.
        let hub = crate::events::EventHub::default();
        let mut feed = hub.subscribe_state();
        let registry = SessionRegistry::with_hub(hub);

        registry.set_state("ghost", SessionState::Active).await;
        registry.begin_waiting("ghost").await;
        registry.end_waiting("ghost").await;
        assert_eq!(registry.begin_idle("ghost", 8).await, None);

        assert!(
            feed.try_recv().is_err(),
            "nothing was affirmed about a session the registry does not hold"
        );
    }

    #[tokio::test]
    async fn the_instant_marks_entering_a_state_and_survives_re_asserting_it() {
        // The whole point of `since`: a surface subtracts it to say how long a
        // session has been working. Several call sites set `Active` on a session
        // that is already active, so an instant that moved on every assignment
        // would show a clock that keeps restarting.
        //
        // This test sleeps past a second because the stamp has second
        // resolution: without crossing that boundary, "unchanged" and "rewritten
        // to the same value" are indistinguishable, and the test would pass
        // either way.
        let registry = SessionRegistry::default();
        registry
            .register("s-1", vec!["mock".into()], "/repo".into())
            .await;

        let first = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(first.state, SessionState::Starting);
        assert_eq!(first.project_root, "/repo");
        assert!(!first.since.is_empty(), "registering stamps the instant");

        tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;

        registry.set_state("s-1", SessionState::Starting).await;
        let same = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(
            same.since, first.since,
            "re-asserting the state it already had must not move the instant"
        );

        registry.set_state("s-1", SessionState::Active).await;
        let moved = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(moved.state, SessionState::Active);
        assert_ne!(
            moved.since, first.since,
            "entering a different state stamps a new instant"
        );
    }

    #[tokio::test]
    async fn an_affirmation_carries_the_waiting_count_and_the_wait_moves_the_state() {
        let registry = SessionRegistry::default();
        registry
            .register("s-1", vec!["mock".into()], "/repo".into())
            .await;
        registry.set_state("s-1", SessionState::Active).await;

        registry.begin_waiting("s-1").await;
        registry.begin_waiting("s-1").await;
        let waiting = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(waiting.state, SessionState::WaitingPermission);
        assert_eq!(
            waiting.pending_permissions, 2,
            "counted, not toggled: two requests wait"
        );

        // Resolving one leaves the session waiting, which is the rule the
        // counter exists for.
        registry.end_waiting("s-1").await;
        let still = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(still.state, SessionState::WaitingPermission);
        assert_eq!(still.pending_permissions, 1);

        registry.end_waiting("s-1").await;
        let free = registry.affirmation("s-1").await.expect("registered");
        assert_eq!(free.state, SessionState::Active);
        assert_eq!(free.pending_permissions, 0);
    }

    #[tokio::test]
    async fn affirmations_cover_every_session_and_nothing_that_left() {
        let registry = SessionRegistry::default();
        registry
            .register("s-1", vec!["mock".into()], "/repo-a".into())
            .await;
        registry
            .register("s-2", vec!["mock".into()], "/repo-b".into())
            .await;

        let mut roots: Vec<String> = registry
            .affirmations()
            .await
            .into_iter()
            .map(|a| a.project_root)
            .collect();
        roots.sort();
        assert_eq!(roots, ["/repo-a", "/repo-b"]);

        // A session that left has nothing to affirm, and saying so is not an
        // error: the snapshot is of what the registry holds.
        registry.deregister("s-1").await;
        assert!(registry.affirmation("s-1").await.is_none());
        assert_eq!(registry.affirmations().await.len(), 1);
    }

    // Scenario: El tope de esperas cierra la más antigua
    #[tokio::test]
    async fn the_idle_cap_closes_the_oldest_wait_and_never_refuses_a_new_one() {
        let registry = SessionRegistry::default();
        for id in ["first", "second", "third"] {
            registry
                .register(id, vec!["mock".into()], "/repo".into())
                .await;
        }

        // Entered in order, so "oldest" is unambiguous. A cap of two.
        assert_eq!(registry.begin_idle("first", 2).await, None, "under the cap");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        assert_eq!(registry.begin_idle("second", 2).await, None, "at the cap");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        // Over it. What gives way is the one that has been idle longest — not
        // the arriving session, because refusing would punish the user for
        // sessions they have already stopped looking at.
        assert_eq!(
            registry.begin_idle("third", 2).await.as_deref(),
            Some("first"),
            "the oldest wait is what is reclaimed"
        );
        let states: std::collections::HashMap<String, SessionState> = registry
            .summaries()
            .await
            .into_iter()
            .map(|s| (s.session_id, s.state))
            .collect();
        assert_eq!(
            states.get("third"),
            Some(&SessionState::WaitingInstruction),
            "and the new one is waiting, not refused"
        );
        assert_eq!(
            states.get("second"),
            Some(&SessionState::WaitingInstruction),
            "the one still within the cap is untouched"
        );

        // Reclaimed means CANCELLED: what is being freed is an agent
        // subprocess, and only cancellation reaches the loop that holds it.
        let cancelled = registry.inner.lock().await["first"]
            .cancelled
            .load(Ordering::SeqCst);
        assert!(
            cancelled,
            "un-marking it would leave the subprocess alive, which is the whole resource"
        );
    }

    #[tokio::test]
    async fn ending_a_wait_takes_it_out_of_the_reckoning() {
        // A session that got its instruction is no longer idle, so it must stop
        // competing for the cap — otherwise the busiest session in the fleet
        // would be evicted for having started waiting earliest.
        let registry = SessionRegistry::default();
        for id in ["a", "b"] {
            registry
                .register(id, vec!["mock".into()], "/repo".into())
                .await;
        }
        registry.begin_idle("a", 1).await;
        registry.end_idle("a").await;
        assert_eq!(
            registry.begin_idle("b", 1).await,
            None,
            "with `a` back at work there is room, and nothing was evicted"
        );
    }

    #[tokio::test]
    async fn register_report_and_deregister() {
        let registry = SessionRegistry::default();
        assert!(registry.is_empty().await);

        let _reg = registry
            .register("s1", vec!["mock-agent".into()], "/repo".into())
            .await;
        registry.set_state("s1", SessionState::Active).await;

        let summaries = registry.summaries().await;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, "s1");
        assert_eq!(summaries[0].state, SessionState::Active);
        assert!(registry.contains("s1").await);

        registry.deregister("s1").await;
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn an_ended_session_still_registered_is_not_running() {
        // The multi-turn runners end with `set_state(Ended)` and stay
        // registered: registered is not running (registro-de-la-sesion-de-calle).
        let registry = SessionRegistry::default();
        let _reg = registry
            .register("lane", vec!["mock-agent".into()], "/repo".into())
            .await;
        assert!(registry.is_running("lane").await, "starting is running");
        registry.set_state("lane", SessionState::Active).await;
        assert!(registry.is_running("lane").await);
        registry.set_state("lane", SessionState::Ended).await;
        assert!(registry.contains("lane").await);
        assert!(!registry.is_running("lane").await, "ended is over");
        assert!(!registry.is_running("never").await);
    }

    #[tokio::test]
    async fn waiting_is_counted_not_toggled() {
        // Scenario: Sesión esperando se declara esperando
        // Scenario: Resuelta la petición, la sesión vuelve a activa
        // Scenario: Varias esperas simultáneas
        let registry = SessionRegistry::default();
        let _reg = registry
            .register("s1", vec!["mock-agent".into()], "/repo".into())
            .await;
        registry.set_state("s1", SessionState::Active).await;

        registry.begin_waiting("s1").await;
        assert_eq!(
            registry.summaries().await[0].state,
            SessionState::WaitingPermission,
            "an escalated request blocks the session"
        );

        // A second concurrent request deepens the wait.
        registry.begin_waiting("s1").await;
        registry.end_waiting("s1").await;
        assert_eq!(
            registry.summaries().await[0].state,
            SessionState::WaitingPermission,
            "one resolved, another still waits"
        );

        registry.end_waiting("s1").await;
        assert_eq!(
            registry.summaries().await[0].state,
            SessionState::Active,
            "nothing left waiting"
        );

        // An unbalanced end neither underflows nor resurrects a gone session.
        registry.end_waiting("s1").await;
        assert_eq!(registry.summaries().await[0].state, SessionState::Active);
        registry.deregister("s1").await;
        registry.end_waiting("s1").await;
        assert!(registry.is_empty().await, "a gone session stays gone");
    }

    #[tokio::test]
    async fn cancel_signals_the_waiter() {
        let registry = SessionRegistry::default();
        let cancel = registry
            .register("s1", vec!["a".into()], "/repo".into())
            .await
            .cancel;

        let waiter = tokio::spawn(async move {
            cancel.notified().await;
            "cancelled"
        });
        // Give the task a moment to start awaiting.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(registry.cancel("s1").await);
        assert_eq!(waiter.await.unwrap(), "cancelled");

        assert!(!registry.cancel("missing").await);
    }

    #[tokio::test]
    async fn instruction_queue_is_fifo_and_closes_when_empty() {
        // Scenario: Instrucción a una sesión activa se despacha como siguiente turno
        // The queue drains in order; once empty it stops accepting, so a later
        // instruction is refused (it must take the resume path instead).
        let dir = std::env::temp_dir().join(format!("meltemi-queue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let log = SessionLog::create(&dir, "proj", "sess").expect("log");
        let log = Arc::new(Mutex::new(log));

        let queue = InstructionQueue::new();
        assert_eq!(queue.enqueue("first".into(), &log).await, Some(1));
        assert_eq!(queue.enqueue("second".into(), &log).await, Some(2));

        // FIFO order.
        assert_eq!(queue.take_or_close().await.as_deref(), Some("first"));
        assert_eq!(queue.take_or_close().await.as_deref(), Some("second"));
        // Empty: the loop closes the queue and stops.
        assert_eq!(queue.take_or_close().await, None);
        // A late instruction is refused once closed.
        assert_eq!(queue.enqueue("late".into(), &log).await, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_cancelled_queue_dispatches_nothing_even_with_items() {
        // Scenario: Dirigir no interrumpe ni cancela
        // A cancel that takes effect stops dispatch: an instruction already in
        // the queue is never handed to the loop, and later enqueues are refused.
        let dir = std::env::temp_dir().join(format!("meltemi-queue-cx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let log = SessionLog::create(&dir, "proj", "sess").expect("log");
        let log = Arc::new(Mutex::new(log));

        let queue = InstructionQueue::new();
        assert_eq!(queue.enqueue("queued".into(), &log).await, Some(1));
        queue.mark_cancelled().await;
        // The queued instruction is NOT dispatched after cancellation.
        assert_eq!(queue.take_or_close().await, None);
        // And nothing new can be queued.
        assert_eq!(queue.enqueue("late".into(), &log).await, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A queue with its log, in a temp dir the caller does not have to name.
    async fn queue_fixture(
        tag: &str,
    ) -> (InstructionQueue, Arc<Mutex<SessionLog>>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("meltemi-queue-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let log = SessionLog::create(&dir, "proj", "sess").expect("log");
        (InstructionQueue::new(), Arc::new(Mutex::new(log)), dir)
    }

    // Scenario: La instrucción releva al turno interrumpido
    #[tokio::test]
    async fn an_interrupted_queue_dispatches_the_relay_it_enqueued() {
        // The twin of the test above, written beside it so the difference is
        // visible: a CANCELLED queue dispatches nothing, an INTERRUPTED one
        // dispatches the relay the interruption left, and only that one
        // (redirigir-turno design D5).
        let (queue, log, dir) = queue_fixture("interrupted").await;

        let position = queue
            .interrupt_with("seguir por aqui".into(), &log)
            .await
            .expect("the session accepts the relay");
        assert_eq!(position, 1);

        // The boundary asks whether the turn it drained was interrupted…
        assert!(
            queue.take_interrupted().await,
            "the interruption is reported"
        );
        // …and the flag governs one turn, not the next.
        assert!(
            !queue.take_interrupted().await,
            "the flag is cleared once read"
        );
        // …and the relay is there to run.
        assert_eq!(
            queue.take_or_close().await.as_deref(),
            Some("seguir por aqui"),
            "the relay runs as the next turn"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: Encolar y despertar no dejan ventana
    #[tokio::test]
    async fn an_instruction_that_lands_before_the_wait_starts_is_not_lost() {
        // The window this closes: the boundary checks the queue, finds it empty,
        // and an instruction arrives BEFORE it starts waiting. With a signal
        // that only wakes current waiters, that session parks forever with work
        // in its queue. This test is deterministic — it does not race — because
        // it reproduces the ordering directly: signal first, wait after.
        let (queue, log, dir) = queue_fixture("no-window").await;
        assert_eq!(queue.try_take().await, None, "nothing yet");

        queue
            .enqueue("llegue justo entonces".into(), &log)
            .await
            .expect("enqueued while nobody waits");

        // No waiter existed when this was enqueued. The wait must still return.
        tokio::time::timeout(std::time::Duration::from_millis(500), queue.wait_for_work())
            .await
            .expect("the permit survived the gap: no wake-up was lost");
        assert_eq!(
            queue.try_take().await.as_deref(),
            Some("llegue justo entonces"),
            "and the work is there when the boundary looks again"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_empty_queue_no_longer_ends_the_session_by_itself() {
        // `try_take` is the half of `take_or_close` that survives: it reports
        // "nothing right now" WITHOUT deciding that the session is over. What
        // ends a session is now a stated reason, never an empty queue.
        let (queue, _log, dir) = queue_fixture("empty-is-not-the-end").await;
        assert_eq!(queue.try_take().await, None);
        assert!(
            queue.accepting().await,
            "an empty queue is a session between turns, not a session that ended"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn cancelling_wakes_a_parked_boundary_and_closes_it() {
        // A session parked at the boundary waits on this signal and nothing
        // else. If cancel did not reach it, `session/cancel` would stop meaning
        // anything for a session that is not running a turn — which, after this
        // change, is most of the time.
        let (queue, _log, dir) = queue_fixture("cancel-wakes").await;
        queue.mark_cancelled().await;
        tokio::time::timeout(std::time::Duration::from_millis(500), queue.wait_for_work())
            .await
            .expect("cancelling wakes the parked boundary");
        assert_eq!(queue.try_take().await, None, "and nothing is dispatched");
        assert!(!queue.accepting().await, "the queue closed on the way out");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_interrupt_that_lands_on_a_turn_already_ending_does_not_haunt_the_next() {
        // The signal can arrive microseconds after the turn ended on its own.
        // Nothing was interrupted, and the flag must not survive to make a
        // LATER turn look interrupted — the boundary consumes it either way.
        let (queue, log, dir) = queue_fixture("late").await;
        queue
            .interrupt_with("llego tarde".into(), &log)
            .await
            .expect("enqueued");

        assert!(
            queue.take_interrupted().await,
            "the boundary consumes it, whatever the turn's stop reason was"
        );
        assert!(
            !queue.take_interrupted().await,
            "and it is gone: a later turn is not an interrupted one"
        );
        // The instruction is not lost by any of that: it runs as the next turn,
        // which is what the user asked for either way.
        assert_eq!(
            queue.take_or_close().await.as_deref(),
            Some("llego tarde"),
            "a lost interruption degrades to a queue, never to a dropped instruction"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn two_interruptions_in_a_row_relay_once_and_queue_the_rest() {
        // Impatience is not a protocol error. Both instructions are kept, in
        // order, and the flag is one fact rather than a counter: there is one
        // turn to stop, however many times it was asked for.
        let (queue, log, dir) = queue_fixture("twice").await;
        assert_eq!(queue.interrupt_with("primero".into(), &log).await, Some(1));
        assert_eq!(queue.interrupt_with("segundo".into(), &log).await, Some(2));

        assert!(queue.take_interrupted().await);
        assert!(
            !queue.take_interrupted().await,
            "one interruption stops one turn, no matter how often it was asked"
        );
        assert_eq!(queue.take_or_close().await.as_deref(), Some("primero"));
        assert_eq!(
            queue.take_or_close().await.as_deref(),
            Some("segundo"),
            "the second is not discarded: it is simply next"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: Cancelar gana a interrumpir
    #[tokio::test]
    async fn cancelling_beats_a_concurrent_interrupt() {
        let (queue, log, dir) = queue_fixture("cancel-wins").await;
        queue
            .interrupt_with("relevo".into(), &log)
            .await
            .expect("enqueued");

        queue.mark_cancelled().await;

        assert!(
            !queue.take_interrupted().await,
            "a cancellation clears the interruption: it is the stronger gesture"
        );
        assert_eq!(
            queue.take_or_close().await,
            None,
            "and nothing queued is dispatched, relay included"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_interrupt_is_refused_once_the_session_stopped_accepting() {
        let (queue, log, dir) = queue_fixture("closed").await;
        queue.close().await;
        assert_eq!(
            queue.interrupt_with("tarde".into(), &log).await,
            None,
            "no relay, and therefore no interruption signalled"
        );
        assert!(
            !queue.take_interrupted().await,
            "an interruption without a relay would be a cancellation nobody asked for"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn cancel_all_notifies_every_session() {
        let registry = SessionRegistry::default();
        let c1 = registry
            .register("s1", vec!["a".into()], "/repo".into())
            .await
            .cancel;
        let c2 = registry
            .register("s2", vec!["b".into()], "/repo".into())
            .await
            .cancel;

        let w1 = tokio::spawn(async move { c1.notified().await });
        let w2 = tokio::spawn(async move { c2.notified().await });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        registry.cancel_all().await;
        w1.await.unwrap();
        w2.await.unwrap();
        assert_eq!(registry.len().await, 2, "cancel does not deregister");
    }
}
