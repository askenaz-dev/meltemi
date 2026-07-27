// SPDX-License-Identifier: Apache-2.0

//! The session-event hub (eventos-para-tardios).
//!
//! A session's updates used to be written straight against the `Peer` captured
//! when the session started, so only that one connection ever saw the
//! transcript. They are published here instead, sealed with the connection that
//! originated them, and every connection decides what to forward: its own
//! sessions always, other sessions only while it watches them. One delivery
//! path, so nothing arrives twice.
//!
//! The channel is bounded (D4): a connection that falls further behind than the
//! buffer misses events and is told so by the channel, and the honest remedy is
//! `session/log`, which is the complete, paginated source. An unbounded channel
//! would turn one slow client into daemon memory with no ceiling.

use std::sync::Arc;

use tokio::sync::broadcast;

use meltemi_proto::SessionEventParams;

/// How many events the hub holds for a lagging connection before it starts
/// dropping the oldest. Generous for a local socket, bounded on purpose.
const CAPACITY: usize = 1024;

/// One published event and where it came from.
#[derive(Debug, Clone)]
pub struct Envelope {
    /// The connection that started the session this event belongs to.
    pub origin: u64,
    /// The notification payload, shared rather than copied per subscriber.
    pub params: Arc<SessionEventParams>,
}

/// The daemon's shared session-event hub.
#[derive(Clone)]
pub struct EventHub {
    tx: broadcast::Sender<Envelope>,
}

impl Default for EventHub {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(CAPACITY);
        Self { tx }
    }
}

impl EventHub {
    /// Publishes one session event. With no connection listening this is a
    /// no-op: the log already holds the event, and the hub never queues for
    /// connections that are gone.
    pub fn publish(&self, origin: u64, params: SessionEventParams) {
        let _ = self.tx.send(Envelope {
            origin,
            params: Arc::new(params),
        });
    }

    /// Subscribes a connection to the stream. The subscriber decides what to
    /// forward; the hub carries everything.
    pub fn subscribe(&self) -> broadcast::Receiver<Envelope> {
        self.tx.subscribe()
    }
}

/// Whether a connection forwards an event: its own sessions unconditionally,
/// another connection's session only while it watches it.
#[must_use]
pub fn delivers(
    envelope: &Envelope,
    connection: u64,
    watched: &std::collections::HashSet<String>,
) -> bool {
    envelope.origin == connection || watched.contains(&envelope.params.session_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use meltemi_proto::{SESSION_EVENT_VERSION, SessionEvent, SessionEventKind};

    fn params(session_id: &str) -> SessionEventParams {
        SessionEventParams {
            session_id: session_id.into(),
            event: SessionEvent {
                v: SESSION_EVENT_VERSION,
                ts: "2026-07-26T12:00:00Z".into(),
                kind: SessionEventKind::AgentUpdate {
                    update: serde_json::Value::Null,
                },
            },
        }
    }

    fn envelope(origin: u64, session_id: &str) -> Envelope {
        Envelope {
            origin,
            params: Arc::new(params(session_id)),
        }
    }

    #[test]
    fn the_origin_receives_without_watching() {
        // Scenario: Cliente que llega a mitad de sesión (the initiator half).
        let watched = HashSet::new();
        assert!(delivers(&envelope(7, "sess-1"), 7, &watched));
    }

    #[test]
    fn a_third_connection_receives_only_while_watching() {
        // Scenario: Sin declarar interés no llega el stream.
        let mut watched = HashSet::new();
        assert!(
            !delivers(&envelope(7, "sess-1"), 9, &watched),
            "not watching: nothing arrives"
        );
        watched.insert("sess-1".to_string());
        assert!(delivers(&envelope(7, "sess-1"), 9, &watched));
        // Watching one session says nothing about another.
        assert!(!delivers(&envelope(7, "sess-2"), 9, &watched));
    }

    #[tokio::test]
    async fn published_events_reach_every_subscriber_once() {
        let hub = EventHub::default();
        let mut first = hub.subscribe();
        let mut second = hub.subscribe();

        hub.publish(7, params("sess-1"));

        let a = first.recv().await.expect("first subscriber");
        let b = second.recv().await.expect("second subscriber");
        assert_eq!(a.origin, 7);
        assert_eq!(a.params.session_id, "sess-1");
        assert_eq!(b.params.session_id, "sess-1");
        // Exactly one delivery each: the channel is now empty.
        assert!(first.try_recv().is_err());
        assert!(second.try_recv().is_err());
    }

    #[tokio::test]
    async fn publishing_with_no_subscriber_is_harmless() {
        let hub = EventHub::default();
        hub.publish(1, params("sess-1"));
    }
}
