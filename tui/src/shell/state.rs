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

/// An overlay stacked above the views. [`Overlay::Palette`] and
/// [`Overlay::Filter`] capture text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Palette {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// The full navigation state of the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    view: View,
    /// Whether the Sessions view is drilled into a Session detail (one level).
    drilled: bool,
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
            drilled: false,
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
        self.drilled
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
            Some(Overlay::Palette { .. } | Overlay::Filter { .. }) => InputMode::TextInput,
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
                    self.drilled = false;
                }
                None
            }
            Action::DrillIn => {
                match self.view {
                    View::Sessions if !self.drilled => self.drilled = true,
                    // In the tray, Enter approves the selected request.
                    View::Permissions => return Some(Effect::ApprovePermission),
                    // In the Project view, Enter regenerates the projection.
                    View::Project => return Some(Effect::ProjectContext),
                    _ => {}
                }
                None
            }
            Action::Back => {
                if self.drilled {
                    self.drilled = false;
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
            Action::Filter if self.view == View::Sessions && !self.drilled => {
                self.overlays.push(Overlay::Filter {
                    input: self.session_filter.clone(),
                });
                None
            }
            Action::AttendPermissions => {
                self.view = View::Permissions;
                self.drilled = false;
                None
            }
            Action::CancelSession => {
                // Only meaningful inside a live session; gated by confirmation.
                if self.view == View::Sessions && self.drilled {
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
                let command = match self.overlays.last() {
                    Some(Overlay::Palette { input }) => input.trim().to_ascii_lowercase(),
                    _ => String::new(),
                };
                self.overlays.pop();
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
                        self.drilled = false;
                        Some(Effect::RefreshProjects)
                    }
                    _ if command.starts_with("projects ") || command.starts_with("proyectos ") => {
                        let arg = command.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
                        self.project_scope = Some(arg.trim().to_string());
                        self.view = View::Sessions;
                        self.drilled = false;
                        Some(Effect::RefreshProjects)
                    }
                    "sessions" | "sesiones" => {
                        self.view = View::Sessions;
                        self.drilled = false;
                        Some(Effect::RefreshStatus)
                    }
                    "project" | "proyecto" => {
                        self.view = View::Project;
                        self.drilled = false;
                        Some(Effect::ProjectContext)
                    }
                    "fleet" | "flota" => {
                        // Core parity: the palette reaches the Fleet like the
                        // digit does, and re-queries the catalog on demand.
                        self.view = View::Fleet;
                        self.drilled = false;
                        Some(Effect::RefreshFleet)
                    }
                    _ => None,
                };
            }
            _ => {}
        }
        None
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
