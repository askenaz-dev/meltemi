// SPDX-License-Identifier: Apache-2.0

//! Live daemon-backed data and the update it receives from the connection actor
//! (design D1, D5). Kept separate from the pure navigation [`ShellState`] so the
//! reducer stays testable, and so rendering reads one coherent snapshot.

use meltemi_proto::{SessionState, SessionSummary};

use crate::shell::render::ConnState;

/// One session row materialized from `status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub agent: String,
    pub state: SessionState,
}

impl From<SessionSummary> for SessionRow {
    fn from(s: SessionSummary) -> Self {
        Self {
            id: s.session_id,
            agent: s.agent_command.join(" "),
            state: s.state,
        }
    }
}

/// An update pushed from the connection actor to the UI.
#[derive(Debug, Clone)]
pub enum Update {
    /// The connection status changed.
    Conn(ConnState),
    /// The full session list was refreshed.
    Sessions(Vec<SessionRow>),
    /// The count of pending permission requests changed.
    Pending(usize),
    /// A streamed transcript line for the observed session.
    TranscriptLine(String),
    /// A persistent, labeled notice (e.g. a permission expiry).
    Notice(String),
}

/// The live snapshot the renderer reads. Mutated only by [`LiveData::apply`] and
/// by local navigation (selection, scroll).
#[derive(Debug, Clone)]
pub struct LiveData {
    pub conn: ConnState,
    pub sessions: Vec<SessionRow>,
    pub selected: usize,
    pub pending_permissions: usize,
    pub transcript: Vec<String>,
    /// Whether the transcript auto-follows its tail.
    pub follow_tail: bool,
    /// Scroll offset from the top of the transcript when not following.
    pub scroll: usize,
    /// Horizontal scroll offset (columns) for wide tables.
    pub h_scroll: usize,
    pub notices: Vec<String>,
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
            selected: 0,
            pending_permissions: 0,
            transcript: Vec::new(),
            follow_tail: true,
            scroll: 0,
            h_scroll: 0,
            notices: Vec::new(),
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
            Update::Notice(text) => self.notices.push(text),
        }
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
    fn pending_and_notice_updates() {
        let mut live = LiveData::new();
        live.apply(Update::Pending(3));
        assert_eq!(live.pending_permissions, 3);
        live.apply(Update::Notice("permiso vencido".into()));
        assert_eq!(live.notices, vec!["permiso vencido".to_string()]);
    }

    #[test]
    fn daemon_loss_freezes_the_transcript_with_a_cut_marker() {
        // Scenario: Caída durante el streaming — marca de corte, sin auto-follow.
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
