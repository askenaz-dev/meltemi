// SPDX-License-Identifier: Apache-2.0

//! Live daemon-backed data and the update it receives from the connection actor
//! (design D1, D5). Kept separate from the pure navigation [`ShellState`] so the
//! reducer stays testable, and so rendering reads one coherent snapshot.

use meltemi_proto::{
    FleetAgent, FleetAgentSource, PendingPermission, PermissionOptionKind, PermissionRule,
    PermissionRuleEffect, PermissionRuleScope, SessionInfo, SessionState,
};

use crate::shell::render::ConnState;

/// One session row materialized from `session/list` (active or historical).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub agent: String,
    pub state: SessionState,
    /// The project root, needed to read the session's log by contract.
    pub project_root: String,
    /// Whether the session can be resumed (agent supports load).
    pub resumable: bool,
    /// The catalog id the agent resolved to, when the daemon recorded one
    /// (multiproyecto-suscripciones D5).
    pub agent_id: Option<String>,
    /// The launch profile — the subscription — by name only, never its env.
    pub profile: Option<String>,
}

impl SessionRow {
    /// The agent identity to show: the resolved catalog id when the daemon
    /// recorded one, else the leaf of the command — never a guess dressed up
    /// as a catalog id.
    #[must_use]
    pub fn agent_label(&self) -> &str {
        if let Some(id) = &self.agent_id {
            return id;
        }
        let program = self.agent.split_whitespace().next().unwrap_or(&self.agent);
        let leaf = program.rsplit(['/', '\\']).next().unwrap_or(program);
        leaf.strip_suffix(".exe")
            .or_else(|| leaf.strip_suffix(".cmd"))
            .or_else(|| leaf.strip_suffix(".bat"))
            .unwrap_or(leaf)
    }

    /// Whether the session is historical (finished or interrupted) rather than
    /// currently live.
    #[must_use]
    pub fn is_historical(&self) -> bool {
        matches!(self.state, SessionState::Ended | SessionState::Interrupted)
    }
}

impl From<SessionInfo> for SessionRow {
    fn from(s: SessionInfo) -> Self {
        Self {
            id: s.session_id,
            agent: s.agent_command.join(" "),
            state: s.state,
            project_root: s.project_root,
            resumable: s.resumable,
            agent_id: s.agent_id,
            profile: s.profile,
        }
    }
}

/// One known project from `project/list` (multiproyecto-suscripciones D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub root: String,
    /// False when the root no longer exists on disk; the entry stays listed.
    pub exists: bool,
    pub sessions_total: u32,
}

impl From<meltemi_proto::ProjectInfo> for ProjectRow {
    fn from(p: meltemi_proto::ProjectInfo) -> Self {
        Self {
            root: p.root,
            exists: p.exists,
            sessions_total: p.sessions_total,
        }
    }
}

/// One fleet catalog row materialized from `fleet/list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    /// The composed install state, when the daemon reports one
    /// (flota-deteccion-guia D8).
    pub install_state: Option<meltemi_proto::FleetInstallState>,
    /// The layers with their per-layer detection.
    pub layers: Vec<meltemi_proto::FleetLayer>,
    /// Which layer is missing and how to install it.
    pub remedy: Option<String>,
    /// The provider's legal note for this integration path.
    pub legal_note: Option<String>,
    pub id: String,
    pub name: String,
    pub level: u8,
    /// The level verified by conformance, when one is recorded.
    pub verified_level: Option<u8>,
    pub detected: bool,
    pub binary_path: Option<String>,
    pub configured: bool,
    pub custom: bool,
    /// For a launch profile, the underlying agent it runs (flota-multiproveedor
    /// D4 parity); `None` for registry/custom rows.
    pub underlying_agent: Option<String>,
}

impl From<FleetAgent> for FleetRow {
    fn from(a: FleetAgent) -> Self {
        Self {
            id: a.id,
            name: a.display_name,
            level: a.integration_level,
            verified_level: a.verified_level,
            detected: a.detected,
            binary_path: a.binary_path,
            configured: a.configured,
            custom: a.source == FleetAgentSource::Custom,
            underlying_agent: a.underlying_agent,
            install_state: a.install_state,
            layers: a.layers,
            remedy: a.remedy,
            legal_note: a.legal_note,
        }
    }
}

/// The fleet catalog as last reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetSnapshot {
    pub registry_version: String,
    pub rows: Vec<FleetRow>,
}

/// An update pushed from the connection actor to the UI.
#[derive(Debug, Clone)]
pub enum Update {
    /// The connection status changed.
    Conn(ConnState),
    /// The full session list was refreshed.
    Sessions(Vec<SessionRow>),
    /// The fleet catalog was (re)queried.
    Fleet(FleetSnapshot),
    /// The known-project registry was (re)queried.
    Projects(Vec<ProjectRow>),
    /// The pending-permission queue snapshot (from `permission/pending` on
    /// connect, or a `permission/changed` broadcast). Drives the tray and the
    /// chrome counter, so it survives reconnection.
    PermissionQueue(Vec<PendingPermission>),
    /// The count of pending permission requests changed.
    Pending(usize),
    /// A streamed transcript line for the observed session.
    TranscriptLine(String),
    /// The fetched log of a historical session (summarized lines).
    SessionLog {
        session_id: String,
        lines: Vec<String>,
    },
    /// A persistent, labeled notice (e.g. a permission expiry).
    Notice(String),
}

/// The live snapshot the renderer reads. Mutated only by [`LiveData::apply`] and
/// by local navigation (selection, scroll).
#[derive(Debug, Clone)]
pub struct LiveData {
    pub conn: ConnState,
    pub sessions: Vec<SessionRow>,
    /// The known projects, most recently used first; the spine of the grouped
    /// Sessions view. Empty until the first `project/list` answer arrives.
    pub projects: Vec<ProjectRow>,
    /// The fleet catalog; `None` until the first `fleet/list` answer arrives.
    pub fleet: Option<FleetSnapshot>,
    pub selected: usize,
    /// The pending-permission queue (tray). The daemon owns it, so it survives
    /// reconnection.
    pub permission_queue: Vec<PendingPermission>,
    /// Selection index within the permission tray.
    pub permission_selected: usize,
    pub pending_permissions: usize,
    pub transcript: Vec<String>,
    /// Whether the transcript auto-follows its tail.
    pub follow_tail: bool,
    /// Scroll offset from the top of the transcript when not following.
    pub scroll: usize,
    /// Horizontal scroll offset (columns) for wide tables.
    pub h_scroll: usize,
    pub notices: Vec<String>,
    /// The fetched transcript of a historical session being inspected, and the
    /// session id it belongs to (so a stale fetch is ignored).
    pub session_log: Vec<String>,
    pub session_log_for: Option<String>,
}

impl Default for LiveData {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            conn: ConnState::Connecting,
            sessions: Vec::new(),
            projects: Vec::new(),
            fleet: None,
            selected: 0,
            permission_queue: Vec::new(),
            permission_selected: 0,
            pending_permissions: 0,
            transcript: Vec::new(),
            follow_tail: true,
            scroll: 0,
            h_scroll: 0,
            notices: Vec::new(),
            session_log: Vec::new(),
            session_log_for: None,
        }
    }

    /// Scrolls the table horizontally (columns), offered instead of truncating.
    pub fn scroll_horizontal(&mut self, right: bool) {
        const STEP: usize = 4;
        const MAX: usize = 200;
        self.h_scroll = if right {
            (self.h_scroll + STEP).min(MAX)
        } else {
            self.h_scroll.saturating_sub(STEP)
        };
    }

    /// Applies an update from the connection actor.
    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Conn(conn) => {
                // Daemon lost while we had a live transcript: freeze it honestly
                // with a cut marker instead of pretending the turn will resume.
                let was_up = matches!(self.conn, ConnState::Connected { .. });
                let now_down = matches!(conn, ConnState::Unreachable { .. });
                if was_up && now_down && !self.transcript.is_empty() {
                    self.transcript
                        .push("--- sesión cortada: daemon inalcanzable ---".into());
                    self.follow_tail = false;
                }
                self.conn = conn;
            }
            Update::Sessions(rows) => {
                self.sessions = rows;
                if self.selected >= self.sessions.len() {
                    self.selected = self.sessions.len().saturating_sub(1);
                }
            }
            Update::Fleet(snapshot) => self.fleet = Some(snapshot),
            Update::Projects(rows) => self.projects = rows,
            Update::PermissionQueue(queue) => {
                self.permission_queue = queue;
                if self.permission_selected >= self.permission_queue.len() {
                    self.permission_selected = self.permission_queue.len().saturating_sub(1);
                }
                // The chrome counter reflects the live queue (unexpired).
                self.pending_permissions =
                    self.permission_queue.iter().filter(|p| !p.expired).count();
            }
            Update::Pending(n) => self.pending_permissions = n,
            Update::TranscriptLine(line) => {
                self.transcript.push(line);
                if self.transcript.len() > TRANSCRIPT_CAP {
                    self.transcript.remove(0);
                }
                if self.follow_tail {
                    self.scroll = self.transcript.len();
                }
            }
            Update::SessionLog { session_id, lines } => {
                // Ignore a fetch for a session we are no longer inspecting.
                if self.session_log_for.as_deref() == Some(session_id.as_str()) {
                    self.session_log = lines;
                }
            }
            Update::Notice(text) => self.notices.push(text),
        }
    }

    /// Marks which historical session's log the detail view is showing, so the
    /// event loop can fetch it and stale answers are dropped.
    pub fn observe_session_log(&mut self, session_id: Option<String>) {
        if self.session_log_for != session_id {
            self.session_log.clear();
        }
        self.session_log_for = session_id;
    }

    /// Moves the session selection by one row (clamped).
    pub fn move_selection(&mut self, down: bool) {
        if self.sessions.is_empty() {
            return;
        }
        if down {
            self.selected = (self.selected + 1).min(self.sessions.len() - 1);
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// The currently selected session row, if any.
    #[must_use]
    pub fn selected_session(&self) -> Option<&SessionRow> {
        self.sessions.get(self.selected)
    }

    /// Moves the tray selection by one row (clamped).
    pub fn move_permission_selection(&mut self, down: bool) {
        if self.permission_queue.is_empty() {
            return;
        }
        if down {
            self.permission_selected =
                (self.permission_selected + 1).min(self.permission_queue.len() - 1);
        } else {
            self.permission_selected = self.permission_selected.saturating_sub(1);
        }
    }

    /// The currently selected pending permission, if any.
    #[must_use]
    pub fn selected_permission(&self) -> Option<&PendingPermission> {
        self.permission_queue.get(self.permission_selected)
    }

    /// The option id of the selected request that grants it, if the agent
    /// offered one (a tray "approve" never invents an option).
    #[must_use]
    pub fn selected_allow_option(&self) -> Option<(String, String)> {
        let row = self.selected_permission()?;
        let option = row.options.iter().find(|o| {
            matches!(
                o.kind,
                PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
            )
        })?;
        Some((row.request_id.clone(), option.option_id.clone()))
    }

    /// The option id of the selected request that refuses it (or `None` to
    /// cancel when the agent offered no reject option).
    #[must_use]
    pub fn selected_deny(&self) -> Option<(String, Option<String>)> {
        let row = self.selected_permission()?;
        let option = row
            .options
            .iter()
            .find(|o| {
                matches!(
                    o.kind,
                    PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
                )
            })
            .map(|o| o.option_id.clone());
        Some((row.request_id.clone(), option))
    }

    /// The most specific rule to propose for the selected request: the
    /// daemon's anti-fatigue suggestion when present, else tool + command
    /// prefix at project scope (never broader than what is known).
    #[must_use]
    pub fn selected_rule_proposal(&self) -> Option<PermissionRule> {
        let row = self.selected_permission()?;
        if let Some(rule) = &row.suggested_rule {
            return Some(rule.clone());
        }
        Some(PermissionRule {
            effect: PermissionRuleEffect::Allow,
            tool: (!row.tool.is_empty()).then(|| row.tool.clone()),
            command_prefix: None,
            path_prefix: None,
            scope: PermissionRuleScope::Project,
        })
    }

    /// Scrolls the transcript up, suspending auto-follow.
    pub fn scroll_up(&mut self) {
        self.follow_tail = false;
        self.scroll = self.scroll.saturating_sub(1);
    }

    /// Scrolls the transcript down; reaching the tail re-enables auto-follow.
    pub fn scroll_down(&mut self) {
        self.scroll = (self.scroll + 1).min(self.transcript.len());
        if self.scroll >= self.transcript.len() {
            self.follow_tail = true;
        }
    }
}

/// Maximum retained transcript lines (append-only, bounded).
const TRANSCRIPT_CAP: usize = 5000;

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, state: SessionState) -> SessionRow {
        SessionRow {
            id: id.into(),
            agent: "mock".into(),
            state,
            project_root: "/repo".into(),
            resumable: false,
            agent_id: None,
            profile: None,
        }
    }

    #[test]
    fn sessions_update_clamps_selection() {
        let mut live = LiveData::new();
        live.apply(Update::Sessions(vec![
            row("a", SessionState::Active),
            row("b", SessionState::Active),
        ]));
        live.selected = 1;
        live.apply(Update::Sessions(vec![row("a", SessionState::Active)]));
        assert_eq!(live.selected, 0, "selection clamps when the list shrinks");
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut live = LiveData::new();
        live.apply(Update::Sessions(vec![
            row("a", SessionState::Active),
            row("b", SessionState::Active),
        ]));
        live.move_selection(false); // already at top, stays
        assert_eq!(live.selected, 0);
        live.move_selection(true);
        assert_eq!(live.selected, 1);
        live.move_selection(true); // clamps at the bottom
        assert_eq!(live.selected, 1);
        assert_eq!(live.selected_session().unwrap().id, "b");
    }

    #[test]
    fn horizontal_scroll_steps_and_clamps_at_zero() {
        let mut live = LiveData::new();
        live.scroll_horizontal(false);
        assert_eq!(live.h_scroll, 0, "cannot scroll left of the origin");
        live.scroll_horizontal(true);
        let after = live.h_scroll;
        assert!(after > 0, "scrolling right advances");
        live.scroll_horizontal(false);
        assert!(live.h_scroll < after, "scrolling left retreats");
    }

    #[test]
    fn transcript_follows_tail_until_scrolled() {
        // Scenario: Desplazarse suspende el seguimiento; volver al final lo reanuda.
        let mut live = LiveData::new();
        live.apply(Update::TranscriptLine("l1".into()));
        live.apply(Update::TranscriptLine("l2".into()));
        assert!(live.follow_tail);
        assert_eq!(live.scroll, 2);
        live.scroll_up();
        assert!(!live.follow_tail, "scrolling up suspends auto-follow");
        // A new line arrives but does not yank the view because we are scrolled.
        live.apply(Update::TranscriptLine("l3".into()));
        assert!(!live.follow_tail);
        // Scrolling back to the tail re-enables following.
        live.scroll_down();
        live.scroll_down();
        assert!(live.follow_tail, "returning to the tail resumes following");
    }

    #[test]
    fn fleet_update_replaces_the_snapshot() {
        let mut live = LiveData::new();
        assert!(live.fleet.is_none(), "no snapshot before the first answer");
        live.apply(Update::Fleet(FleetSnapshot {
            registry_version: "fixture-1".into(),
            rows: vec![FleetRow {
                id: "one".into(),
                name: "One".into(),
                level: 1,
                verified_level: None,
                detected: true,
                binary_path: Some("/bin/one".into()),
                configured: false,
                custom: false,
                underlying_agent: None,
                install_state: None,
                layers: Vec::new(),
                remedy: None,
                legal_note: None,
            }],
        }));
        let snapshot = live.fleet.as_ref().expect("snapshot stored");
        assert_eq!(snapshot.registry_version, "fixture-1");
        assert_eq!(snapshot.rows.len(), 1);
    }

    #[test]
    fn pending_and_notice_updates() {
        let mut live = LiveData::new();
        live.apply(Update::Pending(3));
        assert_eq!(live.pending_permissions, 3);
        live.apply(Update::Notice("permiso vencido".into()));
        assert_eq!(live.notices, vec!["permiso vencido".to_string()]);
    }

    #[test]
    fn daemon_loss_freezes_the_transcript_with_a_cut_marker() {
        // Scenario: Caída durante el streaming
        // marca de corte, sin auto-follow.
        let mut live = LiveData::new();
        live.apply(Update::Conn(ConnState::Connected {
            version: "0.1.0".into(),
            uptime_s: 1,
            sessions: 1,
        }));
        live.apply(Update::TranscriptLine("[s1] agent_message".into()));
        live.apply(Update::Conn(ConnState::Unreachable {
            detail: "closed".into(),
        }));
        assert!(!live.follow_tail, "auto-follow stops on daemon loss");
        assert!(
            live.transcript.last().unwrap().contains("cortada"),
            "a cut marker is appended: {:?}",
            live.transcript
        );
    }
}
