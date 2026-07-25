// SPDX-License-Identifier: Apache-2.0

//! Keymap as data and the consistency contract (design D2, D8).
//!
//! [`resolve`] is a pure function of a [`Key`] and the current [`InputMode`]; it
//! is the single source of truth for "same key = same category of action". The
//! robust key set (no Alt/Meta, no F1–F12, no Ctrl of the TTY) is enforced when
//! raw terminal events are mapped to [`Key`] (see the event loop); this module
//! only sees the already-normalized keys.

/// A normalized key from the robust set the shell accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character (letters, digits, punctuation like `/ : ?`).
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
}

/// Whether the focused context captures text (compositor, palette, filter) or
/// is navigating. Derived from shell state, never from the key itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Navigation,
    TextInput,
}

/// A movement direction for list/selection navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The category of action a key maps to. The mapping is stable across every
/// view: this enum *is* the consistency contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Switch to a first-level view (digits 1–4, global).
    SwitchView(u8),
    /// Drill into the focused element (Enter).
    DrillIn,
    /// Back / close overlay (Esc, `q`, Backspace).
    Back,
    /// Cycle panel focus (Tab / Shift-Tab).
    CycleFocus { backward: bool },
    /// Move the selection within the focused list (arrows).
    Move(Direction),
    /// Open the filter of the focused list (`/`).
    Filter,
    /// Open the command palette (`:`).
    OpenPalette,
    /// Open help (`?`).
    OpenHelp,
    /// Jump to the permission tray (`a`, reserved global).
    AttendPermissions,
    /// Cancel the active session (`x`, reserved global).
    CancelSession,
    /// A letter that acts within the focused view (view-local).
    Local(char),
    /// Insert a character into a text-input context.
    InsertChar(char),
    /// Delete the previous character in a text-input context.
    DeleteBack,
    /// Submit the current text-input context (Enter).
    Submit,
    /// A key with no effect in the current context.
    Ignored,
}

/// The keys reserved globally with a defined meaning in this change: they map to
/// the same category in every view or stay inactive, never colliding (D8).
pub const RESERVED_GLOBAL: &[(char, &str)] =
    &[('a', "attend-permissions"), ('x', "cancel-session")];

/// Resolves a key to an action given the current input mode. Pure: no I/O.
#[must_use]
pub fn resolve(key: Key, mode: InputMode) -> Action {
    match mode {
        InputMode::TextInput => resolve_text_input(key),
        InputMode::Navigation => resolve_navigation(key),
    }
}

fn resolve_text_input(key: Key) -> Action {
    match key {
        // Digits and letters are captured as text, NOT global commands.
        Key::Char(c) => Action::InsertChar(c),
        Key::Backspace => Action::DeleteBack,
        Key::Enter => Action::Submit,
        Key::Esc => Action::Back,
        _ => Action::Ignored,
    }
}

fn resolve_navigation(key: Key) -> Action {
    match key {
        Key::Char(c @ '1'..='4') => Action::SwitchView(c as u8 - b'0'),
        Key::Char('/') => Action::Filter,
        Key::Char(':') => Action::OpenPalette,
        Key::Char('?') => Action::OpenHelp,
        Key::Char('a') => Action::AttendPermissions,
        Key::Char('x') => Action::CancelSession,
        Key::Char('q') => Action::Back,
        Key::Char(c) if c.is_ascii_alphabetic() => Action::Local(c),
        Key::Char(_) => Action::Ignored,
        Key::Enter => Action::DrillIn,
        Key::Esc | Key::Backspace => Action::Back,
        Key::Tab => Action::CycleFocus { backward: false },
        Key::BackTab => Action::CycleFocus { backward: true },
        Key::Up => Action::Move(Direction::Up),
        Key::Down => Action::Move(Direction::Down),
        Key::Left => Action::Move(Direction::Left),
        Key::Right => Action::Move(Direction::Right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_switch_view_globally_in_navigation() {
        // Scenario: Conmutación global por dígito.
        assert_eq!(
            resolve(Key::Char('1'), InputMode::Navigation),
            Action::SwitchView(1)
        );
        assert_eq!(
            resolve(Key::Char('4'), InputMode::Navigation),
            Action::SwitchView(4)
        );
    }

    #[test]
    fn letters_act_local_in_navigation() {
        // Scenario: Letra actúa local sin conmutar de vista.
        assert_eq!(
            resolve(Key::Char('j'), InputMode::Navigation),
            Action::Local('j')
        );
        // A local letter is never a view switch.
        assert!(!matches!(
            resolve(Key::Char('j'), InputMode::Navigation),
            Action::SwitchView(_)
        ));
    }

    #[test]
    fn reserved_keys_are_stable_categories() {
        // Scenario: reserva global respetada
        // `a`/`x` no son Local.
        assert_eq!(
            resolve(Key::Char('a'), InputMode::Navigation),
            Action::AttendPermissions
        );
        assert_eq!(
            resolve(Key::Char('x'), InputMode::Navigation),
            Action::CancelSession
        );
        for &(ch, _) in RESERVED_GLOBAL {
            assert!(
                !matches!(
                    resolve(Key::Char(ch), InputMode::Navigation),
                    Action::Local(_)
                ),
                "reserved key `{ch}` must never resolve to a local action"
            );
        }
    }

    #[test]
    fn text_input_captures_digits_as_text() {
        // Scenario: Dígitos no se filtran al redactar.
        assert_eq!(
            resolve(Key::Char('3'), InputMode::TextInput),
            Action::InsertChar('3')
        );
        assert_eq!(
            resolve(Key::Char('a'), InputMode::TextInput),
            Action::InsertChar('a')
        );
        // And a digit in text-input is never a view switch.
        assert!(!matches!(
            resolve(Key::Char('3'), InputMode::TextInput),
            Action::SwitchView(_)
        ));
    }

    #[test]
    fn back_has_three_equivalent_keys() {
        // Scenario: Alternativa siempre disponible (Esc / q / Backspace).
        assert_eq!(resolve(Key::Esc, InputMode::Navigation), Action::Back);
        assert_eq!(resolve(Key::Char('q'), InputMode::Navigation), Action::Back);
        assert_eq!(resolve(Key::Backspace, InputMode::Navigation), Action::Back);
    }

    #[test]
    fn enter_and_tab_categories() {
        assert_eq!(resolve(Key::Enter, InputMode::Navigation), Action::DrillIn);
        assert_eq!(
            resolve(Key::Tab, InputMode::Navigation),
            Action::CycleFocus { backward: false }
        );
        assert_eq!(
            resolve(Key::BackTab, InputMode::Navigation),
            Action::CycleFocus { backward: true }
        );
    }
}
