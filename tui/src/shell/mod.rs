// SPDX-License-Identifier: Apache-2.0

//! The interactive TUI shell (`tui-shell`).
//!
//! This is the shell foundation (waves 1–3 of the change): the persistent
//! chrome, the keyboard navigation contract, the empty states and the
//! accessibility baseline. Live daemon wiring (streaming sessions, the live
//! permission tray) and hardening are later waves; here the chrome reflects a
//! one-shot connection snapshot ([`probe`]).

pub mod glyphs;
pub mod keymap;
pub mod messages;
pub mod present;
pub mod render;
pub mod state;
pub mod terminal;

use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::run::connect_and_init;
use keymap::Key;
use messages::Lang;
use present::Presentation;
use render::{ConnState, ShellCtx};
use state::{Effect, ShellState};
use terminal::TerminalGuard;

use meltemi_proto::{StatusResult, methods};
use serde_json::json;

/// Takes a one-shot snapshot of the daemon connection for the chrome. A live,
/// reconnecting connection is a later wave; this is honest about the moment it
/// was taken.
pub async fn probe(endpoint: &str) -> ConnState {
    match connect_and_init(endpoint).await {
        Err(error) => ConnState::Unreachable {
            detail: error.message,
        },
        Ok((peer, background)) => {
            let response = peer.request(methods::STATUS, &json!({})).await;
            peer.close();
            background.abort();
            match response
                .ok()
                .and_then(|value| serde_json::from_value::<StatusResult>(value).ok())
            {
                Some(status) => ConnState::Connected {
                    version: status.daemon_version,
                    uptime_s: status.uptime_seconds,
                    sessions: status.sessions.len(),
                },
                None => ConnState::Unreachable {
                    detail: "status unavailable".into(),
                },
            }
        }
    }
}

/// Runs the interactive shell against a connection snapshot until the user
/// quits. The [`TerminalGuard`] restores the terminal on every exit path,
/// including panic.
pub fn run(conn: ConnState, project: bool, pending_permissions: usize) -> io::Result<()> {
    let lang = Lang::from_locale(std::env::var("LANG").ok().as_deref());
    let ctx = ShellCtx {
        present: Presentation::from_env(),
        lang,
        conn,
        pending_permissions,
        project,
    };
    let mut state = ShellState::new();
    let mut guard = TerminalGuard::enter()?;

    loop {
        guard
            .terminal()
            .draw(|frame| render::render(frame, &state, &ctx))?;

        let Some(key) = next_key()? else { continue };
        let action = keymap::resolve(key, state.input_mode());
        if let Some(effect) = state.reduce(action) {
            match effect {
                Effect::Quit => break,
                // Cancelling a session and shutting down the daemon are wired to
                // the daemon in a later wave; the confirmation flow is in place.
                Effect::CancelActiveSession | Effect::ShutdownDaemon => {}
            }
        }
    }
    Ok(())
}

/// Reads the next key press, mapping it to the robust key set and dropping keys
/// outside it (Alt/Meta, Ctrl, F1–F12) — design D2.
fn next_key() -> io::Result<Option<Key>> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(map_key(key)),
        _ => Ok(None),
    }
}

/// Maps a raw key event to a [`Key`], rejecting keys outside the robust set.
fn map_key(key: KeyEvent) -> Option<Key> {
    // Reject Alt/Meta and Ctrl (SSH and the TTY eat them); Shift is allowed
    // (it produces BackTab and uppercase characters).
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
        // Function keys and everything else are outside the robust set.
        _ => None,
    }
}
