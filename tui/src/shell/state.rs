// SPDX-License-Identifier: Apache-2.0

//! Shell navigation state and its pure reducer (design D1).
//!
//! [`ShellState::reduce`] maps an [`Action`] to a state transition and an
//! optional [`Effect`] for the event loop to perform. It is pure and fully
//! unit-testable without a terminal or a daemon.

use crate::shell::keymap::{Action, InputMode};

/// A first-level view. The digit order is the navigation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Sessions,
    Project,
    Permissions,
    Fleet,
}

impl View {
    /// The view a digit 1–4 selects, if any.
    #[must_use]
    pub fn from_digit(d: u8) -> Option<View> {
        match d {
            1 => Some(View::Sessions),
            2 => Some(View::Project),
            3 => Some(View::Permissions),
            4 => Some(View::Fleet),
            _ => None,
        }
    }

    /// The digit that selects this view.
    #[must_use]
    pub fn digit(self) -> u8 {
        match self {
            View::Sessions => 1,
            View::Project => 2,
            View::Permissions => 3,
            View::Fleet => 4,
        }
    }
}

/// What a confirmation dialog will do if confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Quit,
    CancelSession,
    Shutdown,
    /// Approve the selected permission and persist the proposed rule.
    CreateRule,
}

/// What a free-text input overlay does with the text on Enter. The palette
/// line cannot carry these values itself: it is lowercased before it is
/// matched, and an instruction without its capitals is a different instruction
/// (design D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPurpose {
    /// An instruction aimed at the selected session (`session/direct`).
    DirectInstruction,
    /// A project root to add to the registry (`project/register`).
    RegisterProject,
    /// A project root to drop from the listing (`project/forget`).
    ForgetProject,
}

/// An overlay stacked above the views. [`Overlay::Palette`],
/// [`Overlay::Filter`] and [`Overlay::Input`] capture text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Palette {
        input: String,
    },
    /// A free-text field for a verb that takes an argument. Unlike the palette
    /// line, what is typed here reaches the daemon **verbatim**.
    Input {
        purpose: InputPurpose,
        input: String,
    },
    /// The list filter of the focused view (`/`): narrows rows while it is
    /// open, commits on Enter, reverts on Esc.
    Filter {
        input: String,
    },
    Confirm {
        action: ConfirmAction,
    },
    /// First-run welcome, teaching the navigation model.
    Onboarding,
}

/// A side effect the event loop must perform after a transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Quit,
    CancelActiveSession,
    ShutdownDaemon,
    RefreshStatus,
    RefreshFleet,
    /// Regenerate the projected context (`context/project`).
    ProjectContext,
    /// Approve the selected pending permission (grant option).
    ApprovePermission,
    /// Deny the selected pending permission (reject option / cancel).
    DenyPermission,
    /// Approve the selected permission and persist the proposed rule.
    CreateRuleForPermission,
    /// Re-query the known-project registry (`project/list`).
    RefreshProjects,
    /// Steer the selected session with this instruction (`session/direct`),
    /// carried exactly as it was typed.
    DirectSession(String),
    /// Add this root to the project registry (`project/register`), as typed.
    RegisterProject(String),
    /// Drop this root from the registry's listing (`project/forget`), as typed.
    ForgetProject(String),
    /// Read the lanes of a task for the race board (`worktree/diff`).
    RaceBoard {
        change: String,
        task: String,
    },
}

/// What the shell is drilled into, one level below a first-level view. The
/// shell had exactly one drill — the session detail — and named it with a
/// boolean; a second one (the race board) would have collided with the
/// transcript's own scroll routing, so the flag became the thing it always
/// meant: which surface is open underneath (tablero-de-carrera design D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drill {
    /// The Sessions view, drilled into the selected session's detail.
    Session,
    /// The race board of one task: its competitors side by side.
    Race { change: String, task: String },
}

/// The full navigation state of the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    view: View,
    /// The surface open one level below the current view, when any.
    drill: Option<Drill>,
    overlays: Vec<Overlay>,
    /// The committed list filter of the Sessions view (`/`).
    session_filter: String,
    /// The project the shell is scoped to, as typed in the palette. `None`
    /// means every known project (multiproyecto-suscripciones D6).
    project_scope: Option<String>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            view: View::Sessions,
            drill: None,
            overlays: Vec::new(),
            session_filter: String::new(),
            project_scope: None,
        }
    }

    #[must_use]
    pub fn view(&self) -> View {
        self.view
    }

    #[must_use]
    pub fn is_drilled(&self) -> bool {
        self.drill.is_some()
    }

    /// The surface open below the current view, when any.
    #[must_use]
    pub fn drill(&self) -> Option<&Drill> {
        self.drill.as_ref()
    }

    #[must_use]
    pub fn top_overlay(&self) -> Option<&Overlay> {
        self.overlays.last()
    }

    /// Text is captured while a palette or filter overlay is on top;
    /// confirmations and help navigate modally, not as text.
    #[must_use]
    pub fn input_mode(&self) -> InputMode {
        match self.overlays.last() {
            Some(Overlay::Palette { .. } | Overlay::Filter { .. } | Overlay::Input { .. }) => {
                InputMode::TextInput
            }
            _ => InputMode::Navigation,
        }
    }

    /// The filter the Sessions view must apply right now: the live text while
    /// the filter is being typed, else the committed one.
    #[must_use]
    pub fn effective_filter(&self) -> &str {
        match self.overlays.last() {
            Some(Overlay::Filter { input }) => input,
            _ => &self.session_filter,
        }
    }

    /// The project scope, as typed in the palette (a root substring).
    #[must_use]
    pub fn project_scope(&self) -> Option<&str> {
        self.project_scope.as_deref()
    }

    /// Pushes the first-run onboarding overlay.
    pub fn show_onboarding(&mut self) {
        self.overlays.push(Overlay::Onboarding);
    }

    /// Applies an action, mutating state and returning an effect to perform.
    pub fn reduce(&mut self, action: Action) -> Option<Effect> {
        match self.overlays.last() {
            Some(Overlay::Confirm { .. }) => self.reduce_confirm(action),
            Some(Overlay::Palette { .. }) => self.reduce_palette(action),
            Some(Overlay::Filter { .. }) => self.reduce_filter(action),
            Some(Overlay::Input { .. }) => self.reduce_input(action),
            // Help and onboarding are passive overlays: Back closes them.
            Some(Overlay::Help | Overlay::Onboarding) => self.reduce_passive(action),
            None => self.reduce_navigation(action),
        }
    }

    fn reduce_navigation(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::SwitchView(d) => {
                if let Some(view) = View::from_digit(d) {
                    self.view = view;
                    self.drill = None;
                }
                None
            }
            Action::DrillIn => {
                match self.view {
                    View::Sessions if self.drill.is_none() => self.drill = Some(Drill::Session),
                    // In the tray, Enter approves the selected request.
                    View::Permissions => return Some(Effect::ApprovePermission),
                    // In the Project view, Enter regenerates the projection.
                    View::Project => return Some(Effect::ProjectContext),
                    _ => {}
                }
                None
            }
            Action::Back => {
                if self.drill.is_some() {
                    self.drill = None;
                    None
                } else {
                    // Top-level Back requests quit, gated by a confirmation.
                    self.overlays.push(Overlay::Confirm {
                        action: ConfirmAction::Quit,
                    });
                    None
                }
            }
            Action::OpenHelp => {
                self.overlays.push(Overlay::Help);
                None
            }
            Action::OpenPalette => {
                self.overlays.push(Overlay::Palette {
                    input: String::new(),
                });
                None
            }
            // `/` opens the filter of the focused list. Only the Sessions view
            // has one today; elsewhere the key stays inert rather than lying.
            Action::Filter if self.view == View::Sessions && self.drill.is_none() => {
                self.overlays.push(Overlay::Filter {
                    input: self.session_filter.clone(),
                });
                None
            }
            Action::AttendPermissions => {
                self.view = View::Permissions;
                self.drill = None;
                None
            }
            Action::CancelSession => {
                // Only meaningful inside a live session; gated by confirmation.
                if self.view == View::Sessions && matches!(self.drill, Some(Drill::Session)) {
                    self.overlays.push(Overlay::Confirm {
                        action: ConfirmAction::CancelSession,
                    });
                }
                None
            }
            // Tray-local letters: deny (`d`) and create-rule (`r`), only in the
            // Permissions view. Rule creation is gated by a confirmation that
            // shows the proposed rule (the most specific one).
            Action::Local('d') if self.view == View::Permissions => Some(Effect::DenyPermission),
            Action::Local('r') if self.view == View::Permissions => {
                self.overlays.push(Overlay::Confirm {
                    action: ConfirmAction::CreateRule,
                });
                None
            }
            // Filter, focus cycling, movement and other local letters are
            // view-local; the shell skeleton has no effect for them yet.
            _ => None,
        }
    }

    fn reduce_confirm(&mut self, action: Action) -> Option<Effect> {
        let Some(Overlay::Confirm { action: confirm }) = self.overlays.last().cloned() else {
            return None;
        };
        match action {
            // Enter or `y` confirms; the non-destructive default is cancel.
            Action::DrillIn | Action::Local('y') => {
                self.overlays.pop();
                Some(match confirm {
                    ConfirmAction::Quit => Effect::Quit,
                    ConfirmAction::CancelSession => Effect::CancelActiveSession,
                    ConfirmAction::Shutdown => Effect::ShutdownDaemon,
                    ConfirmAction::CreateRule => Effect::CreateRuleForPermission,
                })
            }
            // Esc / q / Backspace / `n` cancel safely without acting.
            Action::Back | Action::Local('n') => {
                self.overlays.pop();
                None
            }
            _ => None,
        }
    }

    fn reduce_palette(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::InsertChar(c) => {
                if let Some(Overlay::Palette { input }) = self.overlays.last_mut() {
                    input.push(c);
                }
            }
            Action::DeleteBack => {
                if let Some(Overlay::Palette { input }) = self.overlays.last_mut() {
                    input.pop();
                }
            }
            Action::Back => {
                self.overlays.pop();
            }
            Action::Submit => {
                // Execute a known command; unknown input just closes the palette.
                // The palette is the core-parity catch-all (design D7).
                let raw = match self.overlays.last() {
                    Some(Overlay::Palette { input }) => input.trim().to_string(),
                    _ => String::new(),
                };
                let command = raw.to_ascii_lowercase();
                self.overlays.pop();

                // Verbs that take free text are resolved FIRST, and they open
                // their own field. Two reasons, both of them traps: the line
                // above is lowercased, so an instruction typed here would lose
                // its capitals and a path would name a different file on the two
                // platforms that care; and `projects <text>` already means
                // "narrow the scope to these projects", so a verb sharing that
                // prefix would be read as a filter and the order decides which
                // wins (design D10).
                for (verbs, purpose) in [
                    (&["direct", "dirigir"][..], InputPurpose::DirectInstruction),
                    (
                        &["projects register", "proyectos alta"][..],
                        InputPurpose::RegisterProject,
                    ),
                    (
                        &["projects forget", "proyectos baja"][..],
                        InputPurpose::ForgetProject,
                    ),
                ] {
                    if let Some(argument) = argument_of(&command, &raw, verbs) {
                        self.overlays.push(Overlay::Input {
                            purpose,
                            input: argument,
                        });
                        return None;
                    }
                }

                return match command.as_str() {
                    "shutdown" | "apagar" => {
                        self.overlays.push(Overlay::Confirm {
                            action: ConfirmAction::Shutdown,
                        });
                        None
                    }
                    "quit" | "salir" => {
                        self.overlays.push(Overlay::Confirm {
                            action: ConfirmAction::Quit,
                        });
                        None
                    }
                    "status" | "refresh" => Some(Effect::RefreshStatus),
                    // `projects` alone widens to every known project; with an
                    // argument it scopes to the projects matching that text.
                    _ if command == "projects" || command == "proyectos" => {
                        self.project_scope = None;
                        self.view = View::Sessions;
                        self.drill = None;
                        Some(Effect::RefreshProjects)
                    }
                    _ if command.starts_with("projects ") || command.starts_with("proyectos ") => {
                        let arg = command.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
                        self.project_scope = Some(arg.trim().to_string());
                        self.view = View::Sessions;
                        self.drill = None;
                        Some(Effect::RefreshProjects)
                    }
                    "sessions" | "sesiones" => {
                        self.view = View::Sessions;
                        self.drill = None;
                        Some(Effect::RefreshStatus)
                    }
                    "project" | "proyecto" => {
                        self.view = View::Project;
                        self.drill = None;
                        Some(Effect::ProjectContext)
                    }
                    "fleet" | "flota" => {
                        // Core parity: the palette reaches the Fleet like the
                        // digit does, and re-queries the catalog on demand.
                        self.view = View::Fleet;
                        self.drill = None;
                        Some(Effect::RefreshFleet)
                    }
                    // `race <change> <task>` opens the board of that task's
                    // competitors. Both arguments survive the palette's
                    // lowercasing: change names are kebab-case and task labels
                    // are digits and dots (design D4/D10). Named without them,
                    // the verb has no task to open and closes without acting.
                    _ if command.starts_with("race ") || command.starts_with("carrera ") => {
                        let rest = command.split_once(' ').map(|(_, r)| r).unwrap_or("");
                        let mut parts = rest.split_whitespace();
                        match (parts.next(), parts.next()) {
                            (Some(change), Some(task)) => {
                                self.drill = Some(Drill::Race {
                                    change: change.to_string(),
                                    task: task.to_string(),
                                });
                                Some(Effect::RaceBoard {
                                    change: change.to_string(),
                                    task: task.to_string(),
                                })
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
            }
            _ => {}
        }
        None
    }

    /// A free-text field: the text is captured as typed, submitted on Enter and
    /// abandoned on Esc. Nothing here normalizes case or separators — that is
    /// the entire reason this overlay exists rather than the palette line.
    fn reduce_input(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::InsertChar(c) => {
                if let Some(Overlay::Input { input, .. }) = self.overlays.last_mut() {
                    input.push(c);
                }
                None
            }
            Action::DeleteBack => {
                if let Some(Overlay::Input { input, .. }) = self.overlays.last_mut() {
                    input.pop();
                }
                None
            }
            Action::Back => {
                self.overlays.pop();
                None
            }
            Action::Submit => {
                let Some(Overlay::Input { purpose, input }) = self.overlays.last().cloned() else {
                    return None;
                };
                // An empty field submits nothing and stays open: there is no
                // instruction to send, and closing on Enter would look exactly
                // like having sent one.
                if input.trim().is_empty() {
                    return None;
                }
                self.overlays.pop();
                let value = input.trim().to_string();
                match purpose {
                    InputPurpose::DirectInstruction => Some(Effect::DirectSession(input)),
                    // A path is trimmed of the whitespace around it and of
                    // nothing else: its case and its separators are what the
                    // filesystem was told to look for.
                    InputPurpose::RegisterProject => Some(Effect::RegisterProject(value)),
                    InputPurpose::ForgetProject => Some(Effect::ForgetProject(value)),
                }
            }
            _ => None,
        }
    }

    /// The list filter: live while open, committed on Enter, reverted on Esc.
    fn reduce_filter(&mut self, action: Action) -> Option<Effect> {
        match action {
            Action::InsertChar(c) => {
                if let Some(Overlay::Filter { input }) = self.overlays.last_mut() {
                    input.push(c);
                }
            }
            Action::DeleteBack => {
                if let Some(Overlay::Filter { input }) = self.overlays.last_mut() {
                    input.pop();
                }
            }
            Action::Submit => {
                if let Some(Overlay::Filter { input }) = self.overlays.last() {
                    self.session_filter = input.trim().to_string();
                }
                self.overlays.pop();
            }
            // Esc abandons the edit: the committed filter is untouched.
            Action::Back => {
                self.overlays.pop();
            }
            _ => {}
        }
        None
    }

    /// Passive overlays (help, onboarding) close on Back and ignore the rest.
    fn reduce_passive(&mut self, action: Action) -> Option<Effect> {
        if matches!(action, Action::Back) {
            self.overlays.pop();
        }
        None
    }
}

/// The argument written after one of `verbs` on the palette line, **as typed**.
///
/// The line is lowercased to match a verb, which is right for a verb and wrong
/// for everything after it, so the argument is sliced out of the raw line at the
/// same offset — the verbs are ASCII, so the two strings agree byte for byte up
/// to that point. Returns `None` when the line is not one of these verbs, and
/// `Some("")` when the verb was typed with nothing after it.
fn argument_of(command: &str, raw: &str, verbs: &[&str]) -> Option<String> {
    for verb in verbs {
        if command == *verb {
            return Some(String::new());
        }
        if let Some(rest) = command.strip_prefix(verb)
            && rest.starts_with(' ')
        {
            return Some(raw[verb.len()..].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::keymap::{Key, resolve};

    /// Drives the state the way the event loop does: resolve the key in the
    /// current input mode, then reduce.
    fn press(state: &mut ShellState, key: Key) -> Option<Effect> {
        let action = resolve(key, state.input_mode());
        state.reduce(action)
    }

    /// Types a palette line and submits it.
    fn palette(state: &mut ShellState, line: &str) -> Option<Effect> {
        press(state, Key::Char(':'));
        for c in line.chars() {
            press(state, Key::Char(c));
        }
        press(state, Key::Enter)
    }

    #[test]
    fn the_race_verb_opens_the_board_of_a_task() {
        let mut s = ShellState::new();
        let effect = palette(&mut s, "race dark-mode 1.1");
        assert_eq!(
            effect,
            Some(Effect::RaceBoard {
                change: "dark-mode".into(),
                task: "1.1".into()
            }),
            "the verb asks the daemon for that task's lanes"
        );
        assert_eq!(
            s.drill(),
            Some(&Drill::Race {
                change: "dark-mode".into(),
                task: "1.1".into()
            }),
            "and the shell is drilled into the board, not into a session"
        );
        // The Spanish spelling opens the same board.
        let mut es = ShellState::new();
        assert!(palette(&mut es, "carrera dark-mode 1.1").is_some());
    }

    #[test]
    fn the_race_verb_without_a_task_opens_nothing() {
        // Half an invocation has no board to open. Closing the palette without
        // acting is the honest answer; guessing a task would not be.
        let mut s = ShellState::new();
        assert_eq!(palette(&mut s, "race dark-mode"), None);
        assert!(!s.is_drilled());
        assert_eq!(palette(&mut ShellState::new(), "race"), None);
    }

    #[test]
    fn the_board_drill_does_not_disturb_the_session_drill() {
        // The two drills are one field now, so the reducer must still tell them
        // apart: cancelling a session is a session-drill gesture, and it must
        // not fire while the board is what is open.
        let mut s = ShellState::new();
        palette(&mut s, "race dark-mode 1.1");
        press(&mut s, Key::Char('x'));
        assert!(
            s.top_overlay().is_none(),
            "cancel-session does not trigger from the race board"
        );
        // Back leaves the board and returns to the list level, not to a quit.
        press(&mut s, Key::Esc);
        assert!(!s.is_drilled());
        assert!(
            s.top_overlay().is_none(),
            "leaving the board is not quitting"
        );
    }

    #[test]
    fn digit_switches_first_level_view_and_clears_drill() {
        let mut s = ShellState::new();
        press(&mut s, Key::Enter); // drill into Session
        assert!(s.is_drilled());
        press(&mut s, Key::Char('2')); // digit switches view globally
        assert_eq!(s.view(), View::Project);
        assert!(!s.is_drilled());
    }

    #[test]
    fn drill_in_and_back_pop_one_level() {
        let mut s = ShellState::new();
        press(&mut s, Key::Enter);
        assert!(s.is_drilled());
        press(&mut s, Key::Esc); // back closes the drill
        assert!(!s.is_drilled());
    }

    #[test]
    fn top_level_back_asks_to_quit_then_confirm_quits() {
        let mut s = ShellState::new();
        assert!(press(&mut s, Key::Char('q')).is_none());
        assert!(matches!(
            s.top_overlay(),
            Some(Overlay::Confirm {
                action: ConfirmAction::Quit
            })
        ));
        // Esc cancels the confirmation safely (no quit).
        assert!(press(&mut s, Key::Esc).is_none());
        assert!(s.top_overlay().is_none());
        // Ask again and confirm with Enter.
        press(&mut s, Key::Char('q'));
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::Quit));
    }

    #[test]
    fn attend_permissions_jumps_to_tray_from_any_view() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char('2'));
        assert_eq!(s.view(), View::Project);
        press(&mut s, Key::Char('a'));
        assert_eq!(s.view(), View::Permissions);
    }

    #[test]
    fn palette_captures_text_and_digits_do_not_switch_view() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        assert_eq!(s.input_mode(), InputMode::TextInput);
        press(&mut s, Key::Char('3')); // digit is text, not a view switch
        assert_eq!(s.view(), View::Sessions);
        assert!(matches!(s.top_overlay(), Some(Overlay::Palette { input }) if input == "3"));
        press(&mut s, Key::Esc); // Esc leaves the field
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn cancel_session_requires_confirmation_and_only_in_a_session() {
        let mut s = ShellState::new();
        // Not drilled: no confirmation appears.
        press(&mut s, Key::Char('x'));
        assert!(s.top_overlay().is_none());
        // Drill into a session, then `x` asks to confirm; confirm cancels.
        press(&mut s, Key::Enter);
        press(&mut s, Key::Char('x'));
        assert!(matches!(
            s.top_overlay(),
            Some(Overlay::Confirm {
                action: ConfirmAction::CancelSession
            })
        ));
        assert_eq!(
            press(&mut s, Key::Char('y')),
            Some(Effect::CancelActiveSession)
        );
    }

    #[test]
    fn help_opens_and_closes() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char('?'));
        assert!(matches!(s.top_overlay(), Some(Overlay::Help)));
        press(&mut s, Key::Esc);
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn palette_shutdown_command_asks_to_confirm_then_effects_shutdown() {
        // Scenario: Apagado del daemon superficiado
        // desde la paleta, con confirmación.
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "shutdown".chars() {
            press(&mut s, Key::Char(c));
        }
        press(&mut s, Key::Enter); // submit
        assert!(matches!(
            s.top_overlay(),
            Some(Overlay::Confirm {
                action: ConfirmAction::Shutdown
            })
        ));
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::ShutdownDaemon));
    }

    #[test]
    fn palette_unknown_command_just_closes() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "frobnicate".chars() {
            press(&mut s, Key::Char(c));
        }
        press(&mut s, Key::Enter);
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn palette_status_command_refreshes() {
        // Scenario: Capacidad alcanzable por la paleta
        // status refresca.
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "status".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::RefreshStatus));
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn tray_enter_approves_and_d_denies() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char('3')); // Permissions view
        assert_eq!(s.view(), View::Permissions);
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::ApprovePermission));
        assert_eq!(press(&mut s, Key::Char('d')), Some(Effect::DenyPermission));
    }

    #[test]
    fn tray_create_rule_is_gated_by_confirmation() {
        // Scenario: Aprobar y crear regla
        // confirmar la regla antes de persistir.
        let mut s = ShellState::new();
        press(&mut s, Key::Char('3'));
        assert!(press(&mut s, Key::Char('r')).is_none());
        assert!(matches!(
            s.top_overlay(),
            Some(Overlay::Confirm {
                action: ConfirmAction::CreateRule
            })
        ));
        // Enter confirms → the effect approves and persists the rule.
        assert_eq!(
            press(&mut s, Key::Enter),
            Some(Effect::CreateRuleForPermission)
        );
        // Esc would have cancelled without acting.
        press(&mut s, Key::Char('3'));
        press(&mut s, Key::Char('r'));
        assert!(press(&mut s, Key::Esc).is_none());
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn tray_decide_keys_do_nothing_outside_the_permissions_view() {
        let mut s = ShellState::new(); // Sessions view
        assert!(press(&mut s, Key::Char('d')).is_none());
        assert!(press(&mut s, Key::Char('r')).is_none());
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn palette_fleet_command_switches_view_and_refreshes() {
        // Scenario: registro de `fleet` en la paleta (obligación de tui-shell).
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "fleet".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::RefreshFleet));
        assert_eq!(s.view(), View::Fleet);
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn onboarding_shows_and_dismisses() {
        // Scenario: Saltable
        // Esc/`q` descarta el onboarding.
        let mut s = ShellState::new();
        s.show_onboarding();
        assert!(matches!(s.top_overlay(), Some(Overlay::Onboarding)));
        press(&mut s, Key::Char('q'));
        assert!(s.top_overlay().is_none());
    }
    #[test]
    fn the_filter_captures_text_commits_on_enter_and_reverts_on_esc() {
        // Scenario: Filtro por proyecto reduce a un grupo
        let mut s = ShellState::new();
        press(&mut s, Key::Char('/'));
        assert!(matches!(s.top_overlay(), Some(Overlay::Filter { .. })));
        assert_eq!(
            s.input_mode(),
            InputMode::TextInput,
            "the filter takes text"
        );
        // A digit typed into the filter is text, not navigation.
        for c in "beta2".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(s.view(), View::Sessions, "typing 2 did not switch views");
        assert_eq!(s.effective_filter(), "beta2", "filtering is live");
        press(&mut s, Key::Backspace);
        press(&mut s, Key::Enter);
        assert!(s.top_overlay().is_none());
        assert_eq!(s.effective_filter(), "beta", "Enter commits the filter");

        // Esc abandons the edit and keeps the committed filter.
        press(&mut s, Key::Char('/'));
        press(&mut s, Key::Char('x'));
        press(&mut s, Key::Esc);
        assert_eq!(s.effective_filter(), "beta");
    }

    #[test]
    fn the_filter_is_inert_where_no_list_has_one() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char('2')); // Project view
        press(&mut s, Key::Char('/'));
        assert!(s.top_overlay().is_none(), "no filter is announced falsely");
    }

    #[test]
    fn the_direction_verb_opens_its_field_and_sends_the_text_untouched() {
        // Scenario: El verbo de dirección deja de estar reservado y queda cableado
        // Scenario: El texto libre llega intacto al daemon
        // Scenario: El texto de la instrucción no se altera
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "direct".chars() {
            press(&mut s, Key::Char(c));
        }
        // Activating it produces its field. It used to fall through to `_ =>
        // None`, which closed the overlay and did nothing at all.
        assert!(press(&mut s, Key::Enter).is_none());
        assert!(
            matches!(
                s.top_overlay(),
                Some(Overlay::Input {
                    purpose: InputPurpose::DirectInstruction,
                    ..
                })
            ),
            "the verb opens its instruction field, never a silent close: {:?}",
            s.top_overlay()
        );
        assert_eq!(s.input_mode(), InputMode::TextInput);

        // Capitals, digits and punctuation are text here — the palette line
        // would have lowercased every one of them.
        for c in "Arregla el Build 2 ahora".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(
            s.view(),
            View::Sessions,
            "typing 2 did not switch views: this field takes text"
        );
        assert_eq!(
            press(&mut s, Key::Enter),
            Some(Effect::DirectSession("Arregla el Build 2 ahora".into())),
            "the instruction leaves exactly as it was typed"
        );
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn the_instruction_field_holds_on_empty_and_abandons_on_esc() {
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "dirigir".chars() {
            press(&mut s, Key::Char(c));
        }
        press(&mut s, Key::Enter);
        // Enter on an empty field sends nothing and keeps the field: closing it
        // would look exactly like having sent something.
        assert!(press(&mut s, Key::Enter).is_none());
        assert!(matches!(s.top_overlay(), Some(Overlay::Input { .. })));
        // Esc abandons it.
        press(&mut s, Key::Esc);
        assert!(s.top_overlay().is_none());
    }

    #[test]
    fn an_instruction_typed_on_the_palette_line_seeds_the_field_with_its_case() {
        // The line is lowercased before it is matched, so anything written
        // after the verb is taken from the raw line instead — otherwise typing
        // the instruction in one go would silently change it.
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "direct Arregla el Build".chars() {
            press(&mut s, Key::Char(c));
        }
        press(&mut s, Key::Enter);
        assert!(
            matches!(s.top_overlay(), Some(Overlay::Input { input, .. }) if input == "Arregla el Build"),
            "the argument keeps its capitals: {:?}",
            s.top_overlay()
        );
    }

    #[test]
    fn the_registry_verbs_are_read_before_the_scope_filter_that_shares_their_prefix() {
        // Scenario: El discriminador de un verbo no se confunde con un filtro
        // `projects <text>` already means "narrow the scope to these projects",
        // so `projects register /Repos/Beta` would have been read as a filter
        // called `register /repos/beta` — an order turned into a filter, in
        // silence. The verb is resolved first, and its path keeps its case.
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "projects register /Repos/Beta".chars() {
            press(&mut s, Key::Char(c));
        }
        assert!(
            press(&mut s, Key::Enter).is_none(),
            "no effect yet: a field"
        );
        assert!(
            matches!(
                s.top_overlay(),
                Some(Overlay::Input { purpose: InputPurpose::RegisterProject, input }) if input == "/Repos/Beta"
            ),
            "the path reaches the field with its capitals: {:?}",
            s.top_overlay()
        );
        assert_eq!(
            s.project_scope(),
            None,
            "and the scope filter never saw the line"
        );
        assert_eq!(
            press(&mut s, Key::Enter),
            Some(Effect::RegisterProject("/Repos/Beta".into()))
        );

        // The filter arm keeps working for everything that is not a verb.
        let mut filter = ShellState::new();
        press(&mut filter, Key::Char(':'));
        for c in "projects beta".chars() {
            press(&mut filter, Key::Char(c));
        }
        assert_eq!(
            press(&mut filter, Key::Enter),
            Some(Effect::RefreshProjects)
        );
        assert_eq!(filter.project_scope(), Some("beta"));
    }

    #[test]
    fn forgetting_a_project_takes_the_path_as_typed() {
        // Scenario: Alta y baja de proyecto tecleando la ruta
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "projects forget".chars() {
            press(&mut s, Key::Char(c));
        }
        press(&mut s, Key::Enter);
        for c in "/Repos/Beta".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(
            press(&mut s, Key::Enter),
            Some(Effect::ForgetProject("/Repos/Beta".into())),
            "a lowercased path names a different directory where case matters"
        );
    }

    #[test]
    fn the_palette_switches_and_clears_the_project_scope() {
        // Scenario: Ámbito de proyecto conmutado desde la paleta
        let mut s = ShellState::new();
        press(&mut s, Key::Char(':'));
        for c in "projects beta".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::RefreshProjects));
        assert_eq!(s.project_scope(), Some("beta"));
        assert_eq!(s.view(), View::Sessions);

        // The bare verb widens back to every known project.
        press(&mut s, Key::Char(':'));
        for c in "projects".chars() {
            press(&mut s, Key::Char(c));
        }
        assert_eq!(press(&mut s, Key::Enter), Some(Effect::RefreshProjects));
        assert_eq!(s.project_scope(), None);
    }
}
