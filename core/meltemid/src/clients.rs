// SPDX-License-Identifier: Apache-2.0

//! Count of initialized client connections, observable via a watch channel
//! (espera-humana D3).
//!
//! The permission escalation waits on this count to apply the
//! wait-while-connected policy and to fire the constitutional no-client deny
//! (§3) behind its reconnect grace. The daemon always knew how many clients
//! it had; this module makes the fact consultable.

use std::sync::Arc;

use tokio::sync::watch;

/// Shared registry of initialized client connections.
#[derive(Clone)]
pub struct ClientRegistry {
    count: Arc<watch::Sender<usize>>,
}

impl Default for ClientRegistry {
    fn default() -> Self {
        let (count, _) = watch::channel(0);
        Self {
            count: Arc::new(count),
        }
    }
}

/// Registration of one initialized client; deregisters on drop, so a
/// connection that ends any way — clean close, error, task abort — always
/// leaves the count honest.
pub struct ClientGuard {
    count: Arc<watch::Sender<usize>>,
}

impl ClientRegistry {
    /// Counts one initialized client until the guard drops.
    pub fn register(&self) -> ClientGuard {
        self.count.send_modify(|n| *n += 1);
        ClientGuard {
            count: self.count.clone(),
        }
    }

    /// A receiver observing the live count.
    pub fn watch(&self) -> watch::Receiver<usize> {
        self.count.subscribe()
    }

    /// The current number of initialized clients.
    pub fn connected(&self) -> usize {
        *self.count.borrow()
    }
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.count.send_modify(|n| *n = n.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guards_count_up_and_down() {
        let registry = ClientRegistry::default();
        assert_eq!(registry.connected(), 0);
        let first = registry.register();
        let second = registry.register();
        assert_eq!(registry.connected(), 2);
        drop(first);
        assert_eq!(registry.connected(), 1);
        drop(second);
        assert_eq!(registry.connected(), 0);
    }

    #[tokio::test]
    async fn the_watch_observes_transitions() {
        let registry = ClientRegistry::default();
        let mut rx = registry.watch();
        let guard = registry.register();
        rx.changed().await.expect("count changed");
        assert_eq!(*rx.borrow_and_update(), 1);
        drop(guard);
        rx.changed().await.expect("count changed");
        assert_eq!(*rx.borrow_and_update(), 0);
    }
}
