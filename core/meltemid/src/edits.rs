// SPDX-License-Identifier: Apache-2.0

//! Human-edit coordination (gui-tauri-paridad design D4/D5): the observable
//! per-tree activity behind the soft-lock policy — free / session active /
//! turn in flight — plus the pending-notes buffer that `worktree/apply-edit`
//! fills and the agent's next turn drains, and the project-scoped fallback
//! log for edits applied while no session runs on the tree.
//!
//! Everything here is synchronous (std mutexes, no await points) so the RAII
//! guards stay panic-safe and droppable from any context.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use meltemi_proto::{SESSION_EVENT_VERSION, SessionEvent, SessionEventKind, TreeEditState};
use tokio::sync::Mutex as AsyncMutex;

use crate::session_log::SessionLog;

/// One session's registration over a tree (project root or managed worktree).
struct TreeActivity {
    session_id: String,
    /// Nested turn depth; > 0 means a prompt is in flight.
    in_flight: u32,
    /// The session's log, so an edit during the session lands in its JSONL.
    log: Arc<AsyncMutex<SessionLog>>,
}

/// The daemon-wide hub: activity and pending notes, keyed by canonical tree.
/// A tree may carry several concurrent sessions (e.g. two proposals over the
/// same project root), so activity is a list and each scope removes only its
/// own registration.
#[derive(Default)]
pub struct EditHub {
    trees: Mutex<HashMap<String, Vec<TreeActivity>>>,
    notes: Mutex<HashMap<String, Vec<String>>>,
}

/// Canonical map key for a tree path: canonicalized when possible, lowercased
/// on Windows (paths there compare case-insensitively).
pub fn tree_key(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let key = canonical.to_string_lossy().into_owned();
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

impl EditHub {
    /// The activity observed on a tree right now: any in-flight turn wins,
    /// then any registered session, else free.
    pub fn state_of(&self, key: &str) -> TreeEditState {
        let trees = self.trees.lock().expect("edit hub lock");
        match trees.get(key) {
            None => TreeEditState::Free,
            Some(activities) if activities.is_empty() => TreeEditState::Free,
            Some(activities) if activities.iter().any(|a| a.in_flight > 0) => {
                TreeEditState::TurnInFlight
            }
            Some(_) => TreeEditState::SessionActive,
        }
    }

    /// A session registered on the tree, with its log handle — the one inside
    /// a turn when any, else the most recent registration.
    pub fn active_session(&self, key: &str) -> Option<(String, Arc<AsyncMutex<SessionLog>>)> {
        let trees = self.trees.lock().expect("edit hub lock");
        let activities = trees.get(key)?;
        let activity = activities
            .iter()
            .find(|a| a.in_flight > 0)
            .or_else(|| activities.last())?;
        Some((activity.session_id.clone(), Arc::clone(&activity.log)))
    }

    /// Records a human-edited file so the agent's next turn is told about it.
    pub fn note_edit(&self, key: &str, file: &str) {
        let mut notes = self.notes.lock().expect("edit hub lock");
        let entry = notes.entry(key.to_string()).or_default();
        if !entry.iter().any(|existing| existing == file) {
            entry.push(file.to_string());
        }
    }

    fn drain_notes(&self, key: &str) -> Vec<String> {
        let mut notes = self.notes.lock().expect("edit hub lock");
        notes.remove(key).unwrap_or_default()
    }

    /// Registers a session over a tree for the duration of the returned scope.
    pub fn enter(
        self: &Arc<Self>,
        tree: &Path,
        session_id: &str,
        log: Arc<AsyncMutex<SessionLog>>,
    ) -> EditScope {
        let key = tree_key(tree);
        self.trees
            .lock()
            .expect("edit hub lock")
            .entry(key.clone())
            .or_default()
            .push(TreeActivity {
                session_id: session_id.to_string(),
                in_flight: 0,
                log,
            });
        EditScope {
            hub: Arc::clone(self),
            key,
            session_id: session_id.to_string(),
        }
    }
}

/// RAII registration of a session over a tree; unregisters only its own
/// session on drop, so a panicked or errored session never leaves a stuck
/// activity flag and concurrent sessions never erase each other.
pub struct EditScope {
    hub: Arc<EditHub>,
    key: String,
    session_id: String,
}

impl EditScope {
    /// A cloneable handle for the turn loop (note drain + in-flight marks).
    #[must_use]
    pub fn handle(&self) -> EditScopeHandle {
        EditScopeHandle {
            hub: Arc::clone(&self.hub),
            key: self.key.clone(),
            session_id: self.session_id.clone(),
        }
    }
}

impl Drop for EditScope {
    fn drop(&mut self) {
        let mut trees = self.hub.trees.lock().expect("edit hub lock");
        if let Some(activities) = trees.get_mut(&self.key) {
            activities.retain(|a| a.session_id != self.session_id);
            if activities.is_empty() {
                trees.remove(&self.key);
            }
        }
    }
}

/// Handle used inside the multi-turn loop.
#[derive(Clone)]
pub struct EditScopeHandle {
    hub: Arc<EditHub>,
    key: String,
    session_id: String,
}

impl EditScopeHandle {
    fn with_own_activity(&self, apply: impl FnOnce(&mut TreeActivity)) {
        if let Some(activity) = self
            .hub
            .trees
            .lock()
            .expect("edit hub lock")
            .get_mut(&self.key)
            .and_then(|activities| {
                activities
                    .iter_mut()
                    .find(|a| a.session_id == self.session_id)
            })
        {
            apply(activity);
        }
    }

    pub fn begin_turn(&self) {
        self.with_own_activity(|activity| activity.in_flight += 1);
    }

    pub fn end_turn(&self) {
        self.with_own_activity(|activity| {
            activity.in_flight = activity.in_flight.saturating_sub(1);
        });
    }

    /// The note prepended to the agent's next turn when the human edited
    /// files since its last one; drains the buffer (an edit is announced
    /// exactly once).
    pub fn note_prefix(&self) -> Option<String> {
        let files = self.hub.drain_notes(&self.key);
        if files.is_empty() {
            return None;
        }
        Some(format!(
            "Note: since your last turn, the human edited the following files \
             in this worktree: {}. Review them before continuing.",
            files.join(", ")
        ))
    }
}

/// The acknowledgement `worktree/apply-edit` demands for a tree state
/// (design D4): none when free, a simple one with a session active, a
/// reinforced one with a turn in flight. Soft lock — never a hard lock.
#[must_use]
pub fn required_ack(state: TreeEditState) -> Option<&'static str> {
    match state {
        TreeEditState::Free => None,
        TreeEditState::SessionActive => Some("session_active"),
        TreeEditState::TurnInFlight => Some("turn_in_flight"),
    }
}

/// The project-scoped edits JSONL: human edits applied while no session runs
/// on the tree land here (same append-only pattern as the checkpoints log).
pub fn events_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".meltemi")
        .join("edits")
        .join("events.jsonl")
}

/// Appends a `human_edit` event to the project-scoped edits JSONL.
/// Best-effort: a logging failure never fails the edit it records — but the
/// caller reports the destination honestly.
pub fn log_project_event(repo_root: &Path, kind: SessionEventKind) {
    let event = SessionEvent {
        v: SESSION_EVENT_VERSION,
        ts: crate::clock::now_rfc3339(),
        kind,
    };
    let Ok(line) = serde_json::to_string(&event) else {
        return;
    };
    let path = events_path(repo_root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub_with_scope(dir: &Path) -> (Arc<EditHub>, EditScope) {
        let hub = Arc::new(EditHub::default());
        let log = SessionLog::create(dir, "k", "sess-test").expect("create log");
        let scope = hub.enter(dir, "sess-test", Arc::new(AsyncMutex::new(log)));
        (hub, scope)
    }

    #[test]
    fn a_free_tree_needs_no_acknowledgement() {
        // Scenario: Worktree libre sin fricción
        let hub = Arc::new(EditHub::default());
        let key = tree_key(Path::new("C:/nonexistent/tree"));
        assert_eq!(hub.state_of(&key), TreeEditState::Free);
        assert_eq!(required_ack(TreeEditState::Free), None);
    }

    #[test]
    fn an_active_session_without_a_turn_needs_a_simple_acknowledgement() {
        // Scenario: Edición entre turnos
        let dir = std::env::temp_dir().join(format!("mel-edits-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let (hub, scope) = hub_with_scope(&dir);
        let key = tree_key(&dir);
        assert_eq!(hub.state_of(&key), TreeEditState::SessionActive);
        assert_eq!(
            required_ack(TreeEditState::SessionActive),
            Some("session_active")
        );
        drop(scope);
        assert_eq!(hub.state_of(&key), TreeEditState::Free);
    }

    #[test]
    fn an_in_flight_turn_needs_the_reinforced_acknowledgement() {
        let dir = std::env::temp_dir().join(format!("mel-edits-turn-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let (hub, scope) = hub_with_scope(&dir);
        let key = tree_key(&dir);
        let handle = scope.handle();
        handle.begin_turn();
        assert_eq!(hub.state_of(&key), TreeEditState::TurnInFlight);
        assert_eq!(
            required_ack(TreeEditState::TurnInFlight),
            Some("turn_in_flight")
        );
        handle.end_turn();
        assert_eq!(hub.state_of(&key), TreeEditState::SessionActive);
    }

    #[test]
    fn concurrent_sessions_on_one_tree_do_not_erase_each_other() {
        let dir = std::env::temp_dir().join(format!("mel-edits-conc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let hub = Arc::new(EditHub::default());
        let key = tree_key(&dir);
        let log_a = SessionLog::create(&dir, "k", "sess-a").expect("log a");
        let log_b = SessionLog::create(&dir, "k", "sess-b").expect("log b");
        let scope_a = hub.enter(&dir, "sess-a", Arc::new(AsyncMutex::new(log_a)));
        let scope_b = hub.enter(&dir, "sess-b", Arc::new(AsyncMutex::new(log_b)));

        scope_b.handle().begin_turn();
        assert_eq!(hub.state_of(&key), TreeEditState::TurnInFlight);
        // The in-turn session is the one an edit gets attributed to.
        assert_eq!(hub.active_session(&key).unwrap().0, "sess-b");

        // Dropping the idle session must not erase the in-turn one.
        drop(scope_a);
        assert_eq!(hub.state_of(&key), TreeEditState::TurnInFlight);
        drop(scope_b);
        assert_eq!(hub.state_of(&key), TreeEditState::Free);
    }

    #[test]
    fn notes_are_deduplicated_and_drained_exactly_once() {
        let hub = Arc::new(EditHub::default());
        let key = "k".to_string();
        hub.note_edit(&key, "a.rs");
        hub.note_edit(&key, "a.rs");
        hub.note_edit(&key, "b.rs");
        let handle = EditScopeHandle {
            hub: Arc::clone(&hub),
            key: key.clone(),
            session_id: "sess-test".into(),
        };
        let prefix = handle.note_prefix().expect("note built");
        assert!(prefix.contains("a.rs, b.rs"));
        assert!(handle.note_prefix().is_none(), "drained exactly once");
    }
}
