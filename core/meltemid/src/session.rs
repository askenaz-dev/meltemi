// SPDX-License-Identifier: Apache-2.0

//! Session registry: the daemon's view of active agent sessions.
//!
//! The ACP layer registers a session when it spawns an agent and deregisters
//! it when the turn ends. `status` reads this registry; `shutdown` and
//! `session/cancel` use the per-session cancel signal to terminate agents
//! without leaving orphan processes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use meltemi_proto::{SessionState, SessionSummary};

/// Handle the registry keeps for one live session.
struct Entry {
    agent_command: Vec<String>,
    state: SessionState,
    /// Fired to ask the owning ACP task to cancel and terminate the agent.
    cancel: Arc<Notify>,
}

/// Thread-safe registry of active sessions.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

impl SessionRegistry {
    /// Registers a new session in the `Starting` state, returning the cancel
    /// signal the ACP task should await.
    pub async fn register(&self, session_id: &str, agent_command: Vec<String>) -> Arc<Notify> {
        let cancel = Arc::new(Notify::new());
        self.inner.lock().await.insert(
            session_id.to_string(),
            Entry {
                agent_command,
                state: SessionState::Starting,
                cancel: Arc::clone(&cancel),
            },
        );
        cancel
    }

    /// Updates the lifecycle state of a session, if still present.
    pub async fn set_state(&self, session_id: &str, state: SessionState) {
        if let Some(entry) = self.inner.lock().await.get_mut(session_id) {
            entry.state = state;
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

    /// Signals cancellation for one session. Returns whether it existed.
    pub async fn cancel(&self, session_id: &str) -> bool {
        match self.inner.lock().await.get(session_id) {
            Some(entry) => {
                entry.cancel.notify_waiters();
                true
            }
            None => false,
        }
    }

    /// Signals cancellation for every session (used by shutdown).
    pub async fn cancel_all(&self) {
        for entry in self.inner.lock().await.values() {
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

        let _cancel = registry.register("s1", vec!["mock-agent".into()]).await;
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
    async fn cancel_signals_the_waiter() {
        let registry = SessionRegistry::default();
        let cancel = registry.register("s1", vec!["a".into()]).await;

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
    async fn cancel_all_notifies_every_session() {
        let registry = SessionRegistry::default();
        let c1 = registry.register("s1", vec!["a".into()]).await;
        let c2 = registry.register("s2", vec!["b".into()]).await;

        let w1 = tokio::spawn(async move { c1.notified().await });
        let w2 = tokio::spawn(async move { c2.notified().await });
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        registry.cancel_all().await;
        w1.await.unwrap();
        w2.await.unwrap();
        assert_eq!(registry.len().await, 2, "cancel does not deregister");
    }
}
