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

use meltemi_proto::{SessionEventKind, SessionState, SessionSummary};

use crate::session_log::SessionLog;

/// A per-session FIFO of directed instructions (control-remoto-asistido).
///
/// While the session is `accepting`, `session/direct` enqueues an instruction
/// and the multi-turn loop dispatches it as the next turn. Once the loop finds
/// the queue empty at a turn boundary — or the session is cancelled — it stops
/// accepting, so a later instruction takes the resume path instead of queueing
/// into a session that will never drain it.
#[derive(Clone)]
pub struct InstructionQueue {
    inner: Arc<Mutex<QueueInner>>,
}

struct QueueInner {
    items: VecDeque<String>,
    accepting: bool,
    /// Set when the session is cancelled. Consulted by `take_or_close` under the
    /// same lock, so a cancel and a dequeue can never both win: once cancelled,
    /// no queued instruction is dispatched, however the turn's stop reason came
    /// back.
    cancelled: bool,
}

impl InstructionQueue {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueInner {
                items: VecDeque::new(),
                accepting: true,
                cancelled: false,
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
        Some(inner.items.len())
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

    /// Mark the session cancelled: no queued instruction is dispatched afterward,
    /// even one already sitting in the queue. Set under the queue lock so it is
    /// atomic with `take_or_close`'s decision to pop.
    pub async fn mark_cancelled(&self) {
        let mut inner = self.inner.lock().await;
        inner.cancelled = true;
        inner.accepting = false;
    }

    /// Stop accepting further turns without draining (used when the loop ends a
    /// session): a late instruction then takes the resume path rather than
    /// queueing into a turn that will never run.
    pub async fn close(&self) {
        self.inner.lock().await.accepting = false;
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
    state: SessionState,
    /// Fired to ask the owning ACP task to cancel and terminate the agent.
    cancel: Arc<Notify>,
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
}

/// What [`SessionRegistry::register`] hands back to the ACP task that runs the
/// session: the cancel signal it awaits and the cancellation flag it checks at
/// each turn boundary.
pub struct Registration {
    /// Await this to cancel and terminate the agent mid-turn.
    pub cancel: Arc<Notify>,
    /// Observed at turn boundaries by the multi-turn loop.
    pub cancelled: Arc<AtomicBool>,
}

/// Thread-safe registry of active sessions.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

impl SessionRegistry {
    /// Registers a new session in the `Starting` state, returning the cancel
    /// signal the ACP task should await and the cancellation flag it checks at
    /// each turn boundary.
    pub async fn register(&self, session_id: &str, agent_command: Vec<String>) -> Registration {
        let cancel = Arc::new(Notify::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        self.inner.lock().await.insert(
            session_id.to_string(),
            Entry {
                agent_command,
                state: SessionState::Starting,
                cancel: Arc::clone(&cancel),
                cancelled: Arc::clone(&cancelled),
                direction: None,
                waiting: 0,
            },
        );
        Registration { cancel, cancelled }
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

    /// Updates the lifecycle state of a session, if still present.
    pub async fn set_state(&self, session_id: &str, state: SessionState) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.state = state;
        }
    }

    /// A permission request of this session began waiting for a human: the
    /// first one blocks the session (`waiting_permission`); further concurrent
    /// ones only deepen the count (sesion-esperando D1).
    pub async fn begin_waiting(&self, session_id: &str) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.waiting += 1;
            if entry.waiting == 1 {
                entry.state = SessionState::WaitingPermission;
            }
        }
    }

    /// A waiting request resolved (any way). The session goes back to `active`
    /// only when none is left waiting; a session already deregistered is left
    /// alone, so a turn that ended mid-wait never resurfaces as active (D2).
    pub async fn end_waiting(&self, session_id: &str) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.waiting = entry.waiting.saturating_sub(1);
            if entry.waiting == 0 {
                entry.state = SessionState::Active;
            }
        }
    }

    /// Removes a session from the registry (turn ended).
    pub async fn deregister(&self, session_id: &str) {
        self.inner.lock().await.remove(session_id);
    }

    /// Whether a session id is currently registered.
    pub async fn contains(&self, session_id: &str) -> bool {
        self.inner.lock().await.contains_key(session_id)
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

    #[tokio::test]
    async fn register_report_and_deregister() {
        let registry = SessionRegistry::default();
        assert!(registry.is_empty().await);

        let _reg = registry.register("s1", vec!["mock-agent".into()]).await;
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
    async fn waiting_is_counted_not_toggled() {
        // Scenario: Sesión esperando se declara esperando
        // Scenario: Resuelta la petición, la sesión vuelve a activa
        // Scenario: Varias esperas simultáneas
        let registry = SessionRegistry::default();
        let _reg = registry.register("s1", vec!["mock-agent".into()]).await;
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
        let cancel = registry.register("s1", vec!["a".into()]).await.cancel;

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

    #[tokio::test]
    async fn cancel_all_notifies_every_session() {
        let registry = SessionRegistry::default();
        let c1 = registry.register("s1", vec!["a".into()]).await.cancel;
        let c2 = registry.register("s2", vec!["b".into()]).await.cancel;

        let w1 = tokio::spawn(async move { c1.notified().await });
        let w2 = tokio::spawn(async move { c2.notified().await });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        registry.cancel_all().await;
        w1.await.unwrap();
        w2.await.unwrap();
        assert_eq!(registry.len().await, 2, "cancel does not deregister");
    }
}
