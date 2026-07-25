// SPDX-License-Identifier: Apache-2.0

//! The interactive TUI shell (`tui-shell`).
//!
//! Wave 4: an async event loop drives a persistent chrome over live daemon data.
//! A background [`conn::connection_actor`] keeps a reconnecting connection and
//! pushes [`live::Update`]s; a dedicated blocking thread reads terminal input
//! and forwards it, so the async loop never blocks on either source.

pub mod conn;
pub mod glyphs;
pub mod keymap;
pub mod live;
pub mod messages;
pub mod onboarding;
pub mod palette;
pub mod present;
pub mod render;
pub mod state;
pub mod terminal;

use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use conn::{Command, connection_actor};
use keymap::{Action, Direction, Key};
use live::LiveData;
use messages::Lang;
use present::Presentation;
use render::ShellCtx;
use state::{Effect, ShellState, View};
use terminal::TerminalGuard;

/// Runs the interactive shell against the daemon at `endpoint` until the user
/// quits. The [`TerminalGuard`] restores the terminal on every exit path,
/// including panic.
pub async fn run(endpoint: &str) -> io::Result<()> {
    let ctx = ShellCtx {
        present: Presentation::from_env(),
        lang: Lang::from_locale(std::env::var("LANG").ok().as_deref()),
        project: std::path::Path::new(".meltemi").is_dir(),
    };
    let mut state = ShellState::new();
    let mut live = LiveData::new();

    // First run teaches the navigation model; the marker is written now so it
    // shows once regardless of how it is dismissed.
    if onboarding::is_first_run() {
        onboarding::mark_seen();
        state.show_onboarding();
    }

    // The connection actor keeps a live, reconnecting connection.
    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel();
    tokio::spawn(connection_actor(endpoint.to_string(), cmd_rx, upd_tx));

    let mut guard = TerminalGuard::enter()?;

    // Read input on a dedicated blocking thread (raw mode is already on) and
    // forward it to the async loop; it exits when the receiver drops on quit.
    let (key_tx, mut key_rx) = unbounded_channel::<Event>();
    std::thread::spawn(move || {
        while let Ok(ev) = event::read() {
            if key_tx.send(ev).is_err() {
                break;
            }
        }
    });

    loop {
        guard
            .terminal()
            .draw(|frame| render::render(frame, &state, &live, &ctx))?;

        tokio::select! {
            input = key_rx.recv() => {
                let Some(event) = input else { break };
                if let Event::Key(key) = event
                    && key.kind == KeyEventKind::Press
                    && let Some(mapped) = map_key(key)
                {
                    let action = keymap::resolve(mapped, state.input_mode());
                    if handle_action(&mut state, &mut live, action, &cmd_tx) {
                        break;
                    }
                }
            }
            update = upd_rx.recv() => {
                if let Some(update) = update {
                    live.apply(update);
                }
            }
        }
    }
    Ok(())
}

/// Applies an action across the navigation state and the live data, issuing
/// commands to the connection actor. Returns `true` if the shell should quit.
fn handle_action(
    state: &mut ShellState,
    live: &mut LiveData,
    action: Action,
    commands: &UnboundedSender<Command>,
) -> bool {
    // Tray selection is a live concern, handled in the Permissions view with
    // no overlay open (Up/Down or j/k move the highlighted request).
    if state.top_overlay().is_none() && state.view() == View::Permissions {
        match &action {
            Action::Move(Direction::Down) | Action::Local('j') => {
                live.move_permission_selection(true);
                return false;
            }
            Action::Move(Direction::Up) | Action::Local('k') => {
                live.move_permission_selection(false);
                return false;
            }
            _ => {}
        }
    }

    // Selection and transcript scroll are live concerns, handled only in the
    // Sessions view with no overlay open.
    if state.top_overlay().is_none() && state.view() == View::Sessions {
        match (&action, state.is_drilled()) {
            (Action::Move(Direction::Down) | Action::Local('j'), false) => {
                live.move_selection(true);
                return false;
            }
            (Action::Move(Direction::Up) | Action::Local('k'), false) => {
                live.move_selection(false);
                return false;
            }
            (Action::Move(Direction::Down) | Action::Local('j'), true) => {
                live.scroll_down();
                return false;
            }
            (Action::Move(Direction::Up) | Action::Local('k'), true) => {
                live.scroll_up();
                return false;
            }
            (Action::Move(Direction::Right), false) => {
                live.scroll_horizontal(true);
                return false;
            }
            (Action::Move(Direction::Left), false) => {
                live.scroll_horizontal(false);
                return false;
            }
            _ => {}
        }
    }

    let was_fleet = state.view() == View::Fleet;
    let was_drilled = state.is_drilled();
    let mut refresh_fleet = false;
    match state.reduce(action) {
        Some(Effect::Quit) => return true,
        Some(Effect::CancelActiveSession) => {
            if let Some(row) = live.selected_session() {
                let _ = commands.send(Command::CancelSession(row.id.clone()));
            }
        }
        Some(Effect::ShutdownDaemon) => {
            let _ = commands.send(Command::Shutdown);
        }
        Some(Effect::RefreshStatus) => {
            let _ = commands.send(Command::Refresh);
        }
        Some(Effect::RefreshFleet) => refresh_fleet = true,
        Some(Effect::RefreshProjects) => {
            let _ = commands.send(Command::ProjectList);
            // The typed text becomes a REAL scope: resolved against the projects
            // already known, it is the root every scoped call then uses. An
            // unresolvable text scopes nothing rather than scoping to a guess.
            let resolved = state.project_scope().and_then(|typed| {
                let needle = typed.to_lowercase();
                live.projects
                    .iter()
                    .find(|project| project.root.to_lowercase().contains(&needle))
                    .map(|project| project.root.clone())
            });
            let _ = commands.send(Command::SetScope(resolved));
            let _ = commands.send(Command::Refresh);
        }
        Some(Effect::ProjectContext) => {
            let _ = commands.send(Command::ProjectContext);
        }
        Some(Effect::ApprovePermission) => {
            if let Some((request_id, option_id)) = live.selected_allow_option() {
                let _ = commands.send(Command::DecidePermission {
                    request_id,
                    option_id: Some(option_id),
                    persist_rule: None,
                });
            }
        }
        Some(Effect::DenyPermission) => {
            if let Some((request_id, option_id)) = live.selected_deny() {
                let _ = commands.send(Command::DecidePermission {
                    request_id,
                    option_id,
                    persist_rule: None,
                });
            }
        }
        Some(Effect::CreateRuleForPermission) => {
            // Approve the request and persist the proposed rule in one gesture.
            if let (Some((request_id, option_id)), Some(rule)) =
                (live.selected_allow_option(), live.selected_rule_proposal())
            {
                let _ = commands.send(Command::DecidePermission {
                    request_id,
                    option_id: Some(option_id),
                    persist_rule: Some(rule),
                });
            }
        }
        None => {}
    }
    // Entering the Fleet view requests the catalog; the palette's `fleet`
    // additionally re-queries it on demand (design D5).
    if refresh_fleet || (!was_fleet && state.view() == View::Fleet) {
        let _ = commands.send(Command::FleetList);
    }

    // Drilling into a session: a historical one shows its transcript read by
    // `session/log`; a live one shows the streamed transcript.
    let drilled_now = !was_drilled && state.is_drilled() && state.view() == View::Sessions;
    if drilled_now {
        match live.selected_session() {
            Some(row) if row.is_historical() => {
                let (id, root) = (row.id.clone(), row.project_root.clone());
                live.observe_session_log(Some(id.clone()));
                let _ = commands.send(Command::FetchSessionLog {
                    session_id: id,
                    project_root: root,
                });
            }
            _ => live.observe_session_log(None),
        }
    } else if was_drilled && !state.is_drilled() {
        // Left the detail view: forget the fetched historical log.
        live.observe_session_log(None);
    }
    false
}

/// Maps a raw key event to a [`Key`], rejecting keys outside the robust set
/// (Alt/Meta, Ctrl, F1–F12) — design D2.
fn map_key(key: KeyEvent) -> Option<Key> {
    if key
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SUPER)
    {
        return None;
    }
    match key.code {
        KeyCode::Char(c) => Some(Key::Char(c)),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Esc),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::BackTab => Some(Key::BackTab),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_the_fleet_view_requests_the_catalog_once() {
        // Scenario: solicitud al entrar a la vista 4 (design D5).
        let (tx, mut rx) = unbounded_channel::<Command>();
        let mut state = ShellState::new();
        let mut live = LiveData::new();

        assert!(!handle_action(
            &mut state,
            &mut live,
            Action::SwitchView(4),
            &tx
        ));
        assert!(
            matches!(rx.try_recv(), Ok(Command::FleetList)),
            "entering the Fleet view must query the catalog"
        );
        // Re-selecting the already-open view does not spam the daemon.
        assert!(!handle_action(
            &mut state,
            &mut live,
            Action::SwitchView(4),
            &tx
        ));
        assert!(rx.try_recv().is_err());
        // Leaving and coming back re-queries (fresh detection per entry).
        handle_action(&mut state, &mut live, Action::SwitchView(1), &tx);
        handle_action(&mut state, &mut live, Action::SwitchView(4), &tx);
        assert!(matches!(rx.try_recv(), Ok(Command::FleetList)));
    }
}
