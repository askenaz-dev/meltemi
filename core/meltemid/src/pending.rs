// SPDX-License-Identifier: Apache-2.0

//! First-class pending-permission queue (proxy-permisos D2).
//!
//! When a permission request must be escalated to the human (no rule decides
//! it), it lives here — in the daemon, not in one connection — so the tray
//! survives reconnection and is shared across clients. Every resolution path
//! (the answer to the live `permission/request` push, a `permission/decide`
//! call from any client, a timeout, or a session cancel) funnels through
//! [`PendingQueue::resolve`], and the **first one wins**; later ones are told
//! the request was already resolved.
//!
//! A change to the queue ticks a broadcast so every connection re-sends a
//! fresh `permission/changed` snapshot. Expired entries stay visible (marked
//! `expired`) for a short grace so an immediate query still shows them, then
//! are pruned — never dropped silently.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, broadcast, oneshot};

use meltemi_proto::{
    PendingPermission, PermissionDecideStatus, PermissionDecidedBy, PermissionOption,
    PermissionOptionKind, PermissionOutcome, PermissionRule,
};

/// How long an expired entry stays visible before it is pruned.
const EXPIRED_GRACE: Duration = Duration::from_secs(10);

/// Approvals of the same request shape after which the daemon suggests a rule
/// (anti-fatigue). Initial constant; configurable later (design open question).
const FATIGUE_THRESHOLD: u32 = 3;

/// The resolution handed back to the ACP bridge for one request.
#[derive(Debug, Clone)]
pub struct Resolution {
    /// The ACP outcome to return to the agent.
    pub outcome: PermissionOutcome,
    /// Who/what decided it (for the audit trail).
    pub decided_by: PermissionDecidedBy,
    /// The rule that decided it, when `decided_by` is `rule`.
    pub rule: Option<PermissionRule>,
    /// Whether this counts as a denial (for the propose honesty count).
    pub denied: bool,
}

/// A stable, comparable shape of a request, for anti-fatigue counting: the
/// tool and the (whole) command, ignoring volatile ids.
type ShapeKey = (String, String);

struct Entry {
    request_id: String,
    session_id: String,
    tool: String,
    summary: String,
    options: Vec<PermissionOption>,
    /// The project root of the owning session, so a project-scoped rule
    /// created from the tray knows where to persist.
    project_root: Option<PathBuf>,
    created: Instant,
    deadline: Instant,
    shape: ShapeKey,
    suggested_rule: Option<PermissionRule>,
    /// `Some` while unresolved; taken by the first resolver (first-wins).
    resolver: Option<oneshot::Sender<Resolution>>,
    /// When the entry expired (timeout); kept visible until the grace passes.
    expired_at: Option<Instant>,
}

#[derive(Default)]
struct Inner {
    /// Live entries, oldest first (insertion order preserved).
    entries: Vec<Entry>,
    /// Anti-fatigue: how many times each shape has been approved by a human.
    approvals: HashMap<ShapeKey, u32>,
    /// Monotonic counter for request ids (avoids any time/random source).
    next_id: u64,
}

/// The daemon's shared pending-permission queue.
#[derive(Clone)]
pub struct PendingQueue {
    inner: Arc<Mutex<Inner>>,
    /// Ticks whenever the queue changes; connections re-broadcast a snapshot.
    changed: broadcast::Sender<()>,
}

impl Default for PendingQueue {
    fn default() -> Self {
        let (changed, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            changed,
        }
    }
}

/// What the ACP bridge needs to register one request.
pub struct NewRequest {
    pub session_id: String,
    pub tool: String,
    pub summary: String,
    pub command: Option<String>,
    pub options: Vec<PermissionOption>,
    pub project_root: Option<PathBuf>,
    pub deadline: Instant,
}

impl PendingQueue {
    /// Subscribes to change ticks; a connection forwards a fresh snapshot on
    /// each tick as `permission/changed`.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    /// Registers a request and returns its id plus the receiver the bridge
    /// awaits. Attaches an anti-fatigue rule suggestion when the same shape
    /// has been approved past the threshold.
    pub async fn register(&self, request: NewRequest) -> (String, oneshot::Receiver<Resolution>) {
        let (tx, rx) = oneshot::channel();
        let mut inner = self.inner.lock().await;
        inner.next_id += 1;
        let request_id = format!("perm-{}", inner.next_id);
        let shape: ShapeKey = (
            request.tool.clone(),
            request.command.clone().unwrap_or_default(),
        );
        let suggested_rule = (inner.approvals.get(&shape).copied().unwrap_or(0)
            >= FATIGUE_THRESHOLD)
            .then(|| suggest_rule(&request));
        inner.entries.push(Entry {
            request_id: request_id.clone(),
            session_id: request.session_id,
            tool: request.tool,
            summary: request.summary,
            options: request.options,
            project_root: request.project_root,
            created: Instant::now(),
            deadline: request.deadline,
            shape,
            suggested_rule,
            resolver: Some(tx),
            expired_at: None,
        });
        drop(inner);
        let _ = self.changed.send(());
        (request_id, rx)
    }

    /// Resolves a request by id (first-wins). Feeds the awaiting bridge and
    /// removes the entry. Returns whether this call applied or lost the race.
    pub async fn resolve(
        &self,
        request_id: &str,
        resolution: Resolution,
    ) -> PermissionDecideStatus {
        let mut inner = self.inner.lock().await;
        let Some(pos) = inner
            .entries
            .iter()
            .position(|e| e.request_id == request_id)
        else {
            return PermissionDecideStatus::AlreadyResolved;
        };
        let Some(sender) = inner.entries[pos].resolver.take() else {
            return PermissionDecideStatus::AlreadyResolved;
        };
        // Human approvals feed the anti-fatigue counter.
        if matches!(resolution.decided_by, PermissionDecidedBy::Client) && !resolution.denied {
            let shape = inner.entries[pos].shape.clone();
            *inner.approvals.entry(shape).or_insert(0) += 1;
        }
        inner.entries.remove(pos);
        drop(inner);
        // The receiver may be gone if the bridge already stopped awaiting
        // (e.g. its own timeout fired first); that simply means we lost.
        if sender.send(resolution).is_err() {
            return PermissionDecideStatus::AlreadyResolved;
        }
        let _ = self.changed.send(());
        PermissionDecideStatus::Applied
    }

    /// Resolves a request from a client's chosen option id (the human path,
    /// via the live push answer or `permission/decide`). `None` cancels.
    /// Classifies the denial from the option kind so the propose honesty
    /// count and the audit stay accurate.
    pub async fn decide(
        &self,
        request_id: &str,
        option_id: Option<String>,
    ) -> PermissionDecideStatus {
        let denied = {
            let inner = self.inner.lock().await;
            match inner.entries.iter().find(|e| e.request_id == request_id) {
                Some(entry) => is_denial(&entry.options, option_id.as_deref()),
                // Unknown/already-resolved: resolve() reports it below.
                None => return PermissionDecideStatus::AlreadyResolved,
            }
        };
        let outcome = match option_id {
            Some(option_id) => PermissionOutcome::Selected { option_id },
            None => PermissionOutcome::Cancelled,
        };
        self.resolve(
            request_id,
            Resolution {
                outcome,
                decided_by: PermissionDecidedBy::Client,
                rule: None,
                denied,
            },
        )
        .await
    }

    /// The project root of the session that owns a pending request.
    pub async fn project_root_of(&self, request_id: &str) -> Option<PathBuf> {
        let inner = self.inner.lock().await;
        inner
            .entries
            .iter()
            .find(|e| e.request_id == request_id)
            .and_then(|e| e.project_root.clone())
    }

    /// Marks a request expired: its resolver is consumed (so a late decide is
    /// told "already resolved"), and it stays visible as expired for the grace
    /// window. Called by the bridge when its own timeout fires.
    pub async fn expire(&self, request_id: &str) {
        let mut inner = self.inner.lock().await;
        if let Some(entry) = inner
            .entries
            .iter_mut()
            .find(|e| e.request_id == request_id)
        {
            entry.resolver = None;
            entry.expired_at = Some(Instant::now());
        }
        drop(inner);
        let _ = self.changed.send(());
    }

    /// Drops a request outright (session cancelled); no grace.
    pub async fn drop_request(&self, request_id: &str) {
        let mut inner = self.inner.lock().await;
        inner.entries.retain(|e| e.request_id != request_id);
        drop(inner);
        let _ = self.changed.send(());
    }

    /// The current queue snapshot, pruning entries whose expiry grace has
    /// passed. Waiting and freshly-expired entries are included.
    pub async fn snapshot(&self) -> Vec<PendingPermission> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        inner.entries.retain(|e| {
            e.expired_at
                .is_none_or(|at| now.duration_since(at) < EXPIRED_GRACE)
        });
        inner
            .entries
            .iter()
            .map(|e| {
                let expired = e.expired_at.is_some() || now >= e.deadline;
                // Signed seconds to the deadline: negative once past it, and
                // clamped to at most -1 for any expired entry so it always
                // reads as overdue.
                let remaining = if now >= e.deadline {
                    -(now.duration_since(e.deadline).as_secs() as i64)
                } else {
                    e.deadline.duration_since(now).as_secs() as i64
                };
                let expires_in_seconds = if expired {
                    remaining.min(-1)
                } else {
                    remaining
                };
                PendingPermission {
                    request_id: e.request_id.clone(),
                    session_id: e.session_id.clone(),
                    tool: e.tool.clone(),
                    summary: e.summary.clone(),
                    options: e.options.clone(),
                    waiting_seconds: now.duration_since(e.created).as_secs(),
                    expires_in_seconds,
                    expired,
                    suggested_rule: e.suggested_rule.clone(),
                }
            })
            .collect()
    }

    /// The session a pending request belongs to, when it exists.
    pub async fn session_of(&self, request_id: &str) -> Option<String> {
        let inner = self.inner.lock().await;
        inner
            .entries
            .iter()
            .find(|e| e.request_id == request_id)
            .map(|e| e.session_id.clone())
    }
}

/// Whether choosing `option_id` (or cancelling, when `None`) denies the
/// request: a missing choice denies, and a chosen reject option denies.
pub fn is_denial(options: &[PermissionOption], option_id: Option<&str>) -> bool {
    match option_id {
        None => true,
        Some(id) => match options.iter().find(|o| o.option_id == id) {
            Some(option) => matches!(
                option.kind,
                PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
            ),
            // An unknown option id is treated as a denial (safe default).
            None => true,
        },
    }
}

/// The most specific rule a repeated approval justifies: exact tool, and the
/// command as a prefix when present, at project scope.
fn suggest_rule(request: &NewRequest) -> PermissionRule {
    use meltemi_proto::{PermissionRuleEffect, PermissionRuleScope};
    PermissionRule {
        effect: PermissionRuleEffect::Allow,
        tool: Some(request.tool.clone()),
        command_prefix: request.command.clone(),
        path_prefix: None,
        scope: PermissionRuleScope::Project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::PermissionOptionKind;

    fn option(id: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption {
            option_id: id.into(),
            name: id.into(),
            kind,
        }
    }

    fn new_request(command: &str) -> NewRequest {
        NewRequest {
            session_id: "sess-1".into(),
            tool: "execute".into(),
            summary: "run a command".into(),
            command: Some(command.into()),
            options: vec![
                option("allow", PermissionOptionKind::AllowOnce),
                option("reject", PermissionOptionKind::RejectOnce),
            ],
            project_root: None,
            deadline: Instant::now() + Duration::from_secs(120),
        }
    }

    fn allow_resolution() -> Resolution {
        Resolution {
            outcome: PermissionOutcome::Selected {
                option_id: "allow".into(),
            },
            decided_by: PermissionDecidedBy::Client,
            rule: None,
            denied: false,
        }
    }

    #[tokio::test]
    async fn register_appears_in_snapshot_then_resolves_away() {
        let queue = PendingQueue::default();
        let (id, rx) = queue.register(new_request("cargo test")).await;

        let snap = queue.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].request_id, id);
        assert!(!snap[0].expired);
        assert_eq!(snap[0].options.len(), 2);

        let status = queue.resolve(&id, allow_resolution()).await;
        assert_eq!(status, PermissionDecideStatus::Applied);
        // The bridge receives the resolution.
        let resolution = rx.await.expect("resolver fired");
        assert!(matches!(
            resolution.outcome,
            PermissionOutcome::Selected { .. }
        ));
        assert!(queue.snapshot().await.is_empty(), "resolved entry is gone");
    }

    #[tokio::test]
    async fn first_resolution_wins_second_is_already_resolved() {
        // Scenario: Doble resolución reconciliada.
        let queue = PendingQueue::default();
        let (id, _rx) = queue.register(new_request("cargo test")).await;

        assert_eq!(
            queue.resolve(&id, allow_resolution()).await,
            PermissionDecideStatus::Applied
        );
        assert_eq!(
            queue.resolve(&id, allow_resolution()).await,
            PermissionDecideStatus::AlreadyResolved,
            "the second path is told it was already resolved"
        );
    }

    #[tokio::test]
    async fn deciding_an_unknown_request_is_already_resolved() {
        let queue = PendingQueue::default();
        assert_eq!(
            queue.resolve("perm-999", allow_resolution()).await,
            PermissionDecideStatus::AlreadyResolved
        );
    }

    #[tokio::test]
    async fn expired_entry_stays_visible_then_prunes() {
        let queue = PendingQueue::default();
        // A request already past its deadline.
        let mut request = new_request("cargo test");
        request.deadline = Instant::now();
        let (id, _rx) = queue.register(request).await;
        queue.expire(&id).await;

        let snap = queue.snapshot().await;
        assert_eq!(snap.len(), 1, "expired entry stays visible immediately");
        assert!(snap[0].expired);
        assert!(snap[0].expires_in_seconds < 0);
        // A late decide finds it already resolved (resolver consumed).
        assert_eq!(
            queue.resolve(&id, allow_resolution()).await,
            PermissionDecideStatus::AlreadyResolved
        );
    }

    #[tokio::test]
    async fn anti_fatigue_suggests_a_rule_after_repeated_approvals() {
        // Scenario: Sugerencia anti-fatiga.
        let queue = PendingQueue::default();
        // Approve the same shape FATIGUE_THRESHOLD times.
        for _ in 0..FATIGUE_THRESHOLD {
            let (id, _rx) = queue.register(new_request("cargo test")).await;
            queue.resolve(&id, allow_resolution()).await;
        }
        // The next identical request carries a suggested rule.
        let (_id, _rx) = queue.register(new_request("cargo test")).await;
        let snap = queue.snapshot().await;
        let entry = snap.last().unwrap();
        let rule = entry.suggested_rule.as_ref().expect("rule suggested");
        assert_eq!(rule.tool.as_deref(), Some("execute"));
        assert_eq!(rule.command_prefix.as_deref(), Some("cargo test"));
    }

    #[tokio::test]
    async fn decide_classifies_denial_from_the_chosen_option() {
        let queue = PendingQueue::default();

        // Choosing the reject option is a denial.
        let (deny_id, deny_rx) = queue.register(new_request("cargo test")).await;
        queue.decide(&deny_id, Some("reject".into())).await;
        assert!(deny_rx.await.unwrap().denied);

        // Choosing the allow option is not.
        let (allow_id, allow_rx) = queue.register(new_request("cargo test")).await;
        queue.decide(&allow_id, Some("allow".into())).await;
        assert!(!allow_rx.await.unwrap().denied);

        // Cancelling (no option) is a denial.
        let (cancel_id, cancel_rx) = queue.register(new_request("cargo test")).await;
        queue.decide(&cancel_id, None).await;
        assert!(cancel_rx.await.unwrap().denied);
    }

    #[tokio::test]
    async fn is_denial_reads_the_option_kind() {
        let options = [
            option("ok", PermissionOptionKind::AllowOnce),
            option("no", PermissionOptionKind::RejectOnce),
        ];
        assert!(!is_denial(&options, Some("ok")));
        assert!(is_denial(&options, Some("no")));
        assert!(is_denial(&options, None), "cancel denies");
        assert!(is_denial(&options, Some("ghost")), "unknown id denies");
    }

    #[tokio::test]
    async fn subscribers_are_ticked_on_change() {
        let queue = PendingQueue::default();
        let mut rx = queue.subscribe();
        let (id, _keep) = queue.register(new_request("cargo test")).await;
        assert!(rx.try_recv().is_ok(), "register ticks subscribers");
        queue.resolve(&id, allow_resolution()).await;
        assert!(rx.try_recv().is_ok(), "resolve ticks subscribers");
    }
}
