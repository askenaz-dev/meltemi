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
    tokio::spawn(connection_actor(
        endpoint.to_string(),
        ctx.lang,
        cmd_rx,
        upd_tx,
    ));

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
                    if handle_action(&mut state, &mut live, action, ctx.lang, &cmd_tx) {
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
    lang: Lang,
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

    // The race board owns its own selection and panning, whatever view it was
    // opened from: j/k walk the lanes, the arrows pan the widest row
    // (tablero-de-carrera design D4).
    if state.top_overlay().is_none()
        && matches!(state.drill(), Some(crate::shell::state::Drill::Race { .. }))
    {
        match &action {
            Action::Move(Direction::Down) | Action::Local('j') => {
                live.move_race_lane(true);
                return false;
            }
            Action::Move(Direction::Up) | Action::Local('k') => {
                live.move_race_lane(false);
                return false;
            }
            Action::Move(Direction::Right) => {
                live.scroll_horizontal(true);
                return false;
            }
            Action::Move(Direction::Left) => {
                live.scroll_horizontal(false);
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
    let was_project = state.view() == View::Project;
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
        Some(Effect::RaceBoard { change, task }) => {
            let _ = commands.send(Command::RaceDiff { change, task });
        }
        // Which lane the turn runs on is a live fact, not a reducer one: the
        // board holds the selection, so the effect names the gesture and the
        // loop resolves the lane. An empty board has nothing to dispatch, and
        // says so rather than sending a request with no agent in it.
        Some(Effect::DispatchRaceLane) => match live.race.as_ref() {
            Some(board) if !board.lanes.is_empty() => {
                let lane = &board.lanes[board.selected.min(board.lanes.len() - 1)];
                let _ = commands.send(Command::RaceDispatch {
                    change: board.change.clone(),
                    task: board.task.clone(),
                    agent: lane.agent.clone(),
                });
            }
            _ => live
                .notices
                .push(messages::text(messages::Msg::RaceEmpty, lang).to_string()),
        },
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
        Some(Effect::DirectSession(instruction)) => match live.selected_session() {
            // Nothing to aim at: say so. Dropping the text would be the silent
            // no-op the tui-shell delta forbids.
            None => live
                .notices
                .push(messages::text(messages::Msg::DirectNoSession, lang).to_string()),
            // A session that ended and cannot be resumed has nowhere to put an
            // instruction. The field already said so; sending anyway would only
            // trade a stated fact for a round trip that fails.
            Some(row) if !row.accepts_instruction() => live
                .notices
                .push(messages::text(messages::Msg::DirectNotResumable, lang).to_string()),
            // The instruction belongs to the session on screen. Whether it
            // queues or resumes is the daemon's call — the shell states the
            // prospect beforehand and reports what actually happened after.
            Some(row) => {
                let _ = commands.send(Command::Direct {
                    session_id: row.id.clone(),
                    project_root: row.project_root.clone(),
                    instruction,
                });
            }
        },
        // The registry verbs take a path the user typed, and it goes to the
        // daemon exactly as written: the daemon validates and canonicalizes it,
        // the shell does not second-guess a path it did not author.
        Some(Effect::RegisterProject(root)) => {
            let _ = commands.send(Command::RegisterProject(root));
        }
        Some(Effect::ForgetProject(root)) => {
            let _ = commands.send(Command::ForgetProject(root));
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
    // Entering the Project view asks for the registry, for the same reason the
    // Fleet view asks for the catalog: a view that renders a list the daemon
    // owns must not show whatever was last seen and call it the truth.
    if !was_project && state.view() == View::Project {
        let _ = commands.send(Command::ProjectList);
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
            Some(row) => {
                // A live session: no log to fetch, but its transcript is the
                // only one this view accepts.
                live.observed_live = Some(row.id.clone());
                live.observe_session_log(None);
            }
            None => live.observe_session_log(None),
        }
    } else if was_drilled && !state.is_drilled() {
        // Left the detail view: forget the fetched historical log and stop
        // filtering the streamed transcript.
        live.observe_session_log(None);
        live.observed_live = None;
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
    use crate::shell::live::{SessionRow, Update};
    use meltemi_proto::SessionState;

    // Scenario: Instrucción dirigida desde el drill-in
    #[test]
    fn a_directed_instruction_reaches_the_session_on_screen() {
        let (tx, mut rx) = unbounded_channel::<Command>();
        let mut state = ShellState::new();
        let mut live = LiveData::new();
        live.apply(Update::Sessions(vec![SessionRow {
            id: "s-1".into(),
            agent: "mock".into(),
            state: SessionState::Active,
            project_root: "/repo".into(),
            resumable: false,
            agent_id: None,
            profile: None,
        }]));

        // Drill into it and direct it from there: the instruction carries the
        // open session's id and its project, not a scope the user last typed.
        handle_action(&mut state, &mut live, Action::DrillIn, Lang::Es, &tx);
        assert!(state.is_drilled());
        while rx.try_recv().is_ok() {}

        direct_from_palette(&mut state, &mut live, &tx, "Arregla el Build");

        match rx.try_recv() {
            Ok(Command::Direct {
                session_id,
                project_root,
                instruction,
            }) => {
                assert_eq!(session_id, "s-1");
                assert_eq!(project_root, "/repo");
                assert_eq!(instruction, "Arregla el Build");
            }
            other => panic!("the instruction must reach `session/direct`, got {other:?}"),
        }
    }

    #[test]
    fn a_spent_session_is_told_apart_instead_of_being_sent_to() {
        // Scenario: Instrucción dirigida desde el drill-in
        // Ended and not resumable: there is nowhere for the instruction to go,
        // and the shell says that rather than spending a round trip to be told.
        let (tx, mut rx) = unbounded_channel::<Command>();
        let mut state = ShellState::new();
        let mut live = LiveData::new();
        live.apply(Update::Sessions(vec![SessionRow {
            id: "s-old".into(),
            agent: "mock".into(),
            state: SessionState::Ended,
            project_root: "/repo".into(),
            resumable: false,
            agent_id: None,
            profile: None,
        }]));

        direct_from_palette(&mut state, &mut live, &tx, "algo");
        assert!(rx.try_recv().is_err(), "nothing goes out");
        assert!(
            live.notices
                .last()
                .is_some_and(|notice| notice.contains("no admite reanuda")),
            "the shell says why, and what to do instead: {:?}",
            live.notices
        );

        // The same session, resumable, does go out: the distinction is the
        // agent's ability to resume, not the fact that it ended.
        live.apply(Update::Sessions(vec![SessionRow {
            id: "s-old".into(),
            agent: "mock".into(),
            state: SessionState::Ended,
            project_root: "/repo".into(),
            resumable: true,
            agent_id: None,
            profile: None,
        }]));
        direct_from_palette(&mut state, &mut live, &tx, "algo");
        assert!(matches!(rx.try_recv(), Ok(Command::Direct { .. })));
    }

    /// Drives the palette exactly as a user would: open, type the verb, submit,
    /// type the instruction, submit.
    fn direct_from_palette(
        state: &mut ShellState,
        live: &mut LiveData,
        tx: &UnboundedSender<Command>,
        instruction: &str,
    ) {
        handle_action(state, live, Action::OpenPalette, Lang::Es, tx);
        for c in "direct".chars() {
            handle_action(state, live, Action::InsertChar(c), Lang::Es, tx);
        }
        handle_action(state, live, Action::Submit, Lang::Es, tx);
        for c in instruction.chars() {
            handle_action(state, live, Action::InsertChar(c), Lang::Es, tx);
        }
        handle_action(state, live, Action::Submit, Lang::Es, tx);
    }

    #[test]
    fn directing_with_nothing_selected_says_so_instead_of_dropping_the_text() {
        let (tx, mut rx) = unbounded_channel::<Command>();
        let mut state = ShellState::new();
        let mut live = LiveData::new(); // no sessions at all

        direct_from_palette(&mut state, &mut live, &tx, "algo");

        assert!(
            rx.try_recv().is_err(),
            "nothing is sent to a session that is not there"
        );
        assert!(
            live.notices
                .last()
                .is_some_and(|notice| notice.contains("sesión")),
            "the shell says why nothing happened: {:?}",
            live.notices
        );
    }

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
            Lang::Es,
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
            Lang::Es,
            &tx
        ));
        assert!(rx.try_recv().is_err());
        // Leaving and coming back re-queries (fresh detection per entry).
        handle_action(&mut state, &mut live, Action::SwitchView(1), Lang::Es, &tx);
        handle_action(&mut state, &mut live, Action::SwitchView(4), Lang::Es, &tx);
        assert!(matches!(rx.try_recv(), Ok(Command::FleetList)));
    }
}
