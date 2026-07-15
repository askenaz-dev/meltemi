// SPDX-License-Identifier: Apache-2.0

//! Rendering of the shell frame (design D1, D3, D4).
//!
//! The chrome (header + footer) frames the body; the connection status and the
//! permission counter are always drawn. Meaning is carried by glyph + word, and
//! color is applied only when the presentation policy allows it — so the frame
//! is legible under `NO_COLOR` and in ASCII.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use meltemi_proto::SessionState;

use crate::shell::glyphs::{self, Glyph};
use crate::shell::live::LiveData;
use crate::shell::messages::{Lang, Msg, text};
use crate::shell::present::Presentation;
use crate::shell::state::{ConfirmAction, Overlay, ShellState, View};

/// ASCII twin of the box-drawing border set, used when Unicode is off.
const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

/// The daemon connection status the chrome reflects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnState {
    Connecting,
    Connected {
        version: String,
        uptime_s: u64,
        sessions: usize,
    },
    Unreachable {
        detail: String,
    },
}

impl ConnState {
    /// Whether a view that needs the daemon can show its own content.
    fn is_up(&self) -> bool {
        !matches!(self, ConnState::Unreachable { .. })
    }
}

/// Static presentation context (the live snapshot is passed separately).
pub struct ShellCtx {
    pub present: Presentation,
    pub lang: Lang,
    pub project: bool,
}

impl ShellCtx {
    fn msg(&self, m: Msg) -> &'static str {
        text(m, self.lang)
    }

    /// Emphasis style: reverse video (works without color) plus bold when color
    /// is allowed. Never relies on color alone.
    fn emphasis(&self) -> Style {
        let mut s = Style::default().add_modifier(Modifier::REVERSED);
        if self.present.color {
            s = s.add_modifier(Modifier::BOLD);
        }
        s
    }

    fn border_set(&self) -> border::Set {
        if self.present.unicode {
            border::PLAIN
        } else {
            ASCII_BORDER
        }
    }
}

/// Minimum usable terminal size; below it the shell shows a floor state.
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;

/// Draws the whole shell frame.
pub fn render(frame: &mut Frame, state: &ShellState, live: &LiveData, ctx: &ShellCtx) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_size_floor(frame, area, live, ctx);
        return;
    }

    // An alert banner surfaces the top-priority signal from any view (design D3):
    // daemon-down over pending-permission over a recent notice.
    let banner = alert_banner(live, ctx);
    let areas = if banner.is_some() {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area)
    };
    let (header, body, footer) = if banner.is_some() {
        (areas[0], areas[2], areas[3])
    } else {
        (areas[0], areas[1], areas[2])
    };

    render_header(frame, header, live, ctx);
    if let Some(text) = banner {
        frame.render_widget(Paragraph::new(Line::styled(text, ctx.emphasis())), areas[1]);
    }
    render_body(frame, body, state, live, ctx);
    render_footer(frame, footer, state, ctx);

    if let Some(overlay) = state.top_overlay() {
        render_overlay(frame, area, overlay, ctx);
    }
}

/// The highest-priority signal to show in the alert banner, if any.
fn alert_banner(live: &LiveData, ctx: &ShellCtx) -> Option<String> {
    match &live.conn {
        ConnState::Unreachable { .. } => Some(format!(
            "{} {}",
            glyphs::ERROR.text(&ctx.present),
            ctx.msg(Msg::DisconnectBanner)
        )),
        _ if live.pending_permissions > 0 => Some(format!(
            "{} {} {} - a",
            glyphs::PERMISSION.text(&ctx.present),
            live.pending_permissions,
            if ctx.lang == Lang::Es {
                "permisos esperando"
            } else {
                "permissions waiting"
            }
        )),
        _ => live.notices.last().cloned(),
    }
}

/// The floor state when the terminal is below the minimum usable size: the
/// connection status and permission count are the last things sacrificed.
fn render_size_floor(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let conn_glyph = match &live.conn {
        ConnState::Connected { .. } => glyphs::OK,
        ConnState::Connecting => glyphs::PENDING,
        ConnState::Unreachable { .. } => glyphs::ERROR,
    };
    let critical = format!(
        "{} | {} {}",
        conn_glyph.text(&ctx.present),
        glyphs::PERMISSION.text(&ctx.present),
        live.pending_permissions
    );
    let body = if area.height >= 2 {
        format!(
            "{}\n{}x{}\n{critical}",
            ctx.msg(Msg::SizeFloor),
            area.width,
            area.height
        )
    } else {
        critical
    };
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: true }), area);
}

fn render_header(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    let (glyph, word) = match &live.conn {
        ConnState::Connecting => (glyphs::PENDING, ctx.msg(Msg::Connecting).to_string()),
        ConnState::Connected {
            version,
            uptime_s,
            sessions,
        } => (
            glyphs::OK,
            format!("daemon {version} | {uptime_s}s | {sessions} ses."),
        ),
        ConnState::Unreachable { .. } => (glyphs::ERROR, ctx.msg(Msg::Unreachable).to_string()),
    };
    // Permission indicator: glyph + count + word, always present (never a bare dot).
    let perms = format!(
        "{} {} {}",
        glyphs::PERMISSION.text(&ctx.present),
        live.pending_permissions,
        if ctx.lang == Lang::Es {
            "esperando"
        } else {
            "waiting"
        }
    );
    let left = format!("{} {}", glyph.text(&ctx.present), word);
    let line = Line::from(vec![
        Span::raw(left),
        Span::raw("   "),
        Span::styled(perms, ctx.emphasis()),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &ShellState, ctx: &ShellCtx) {
    let hint = if state.input_mode() == crate::shell::keymap::InputMode::TextInput {
        ctx.msg(Msg::HintExitField)
    } else {
        ctx.msg(Msg::HintKeys)
    };
    frame.render_widget(Paragraph::new(Line::from(hint)), area);
}

fn render_body(frame: &mut Frame, area: Rect, state: &ShellState, live: &LiveData, ctx: &ShellCtx) {
    let title = view_title(state.view(), ctx.lang);
    let crumb = if state.is_drilled() {
        format!(
            "{title} > {}",
            if ctx.lang == Lang::Es {
                "sesión"
            } else {
                "session"
            }
        )
    } else {
        title.to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ctx.border_set())
        .title(Span::styled(format!(" {crumb} "), ctx.emphasis()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // A view that needs the daemon shows the unreachable card when it is down;
    // Project is local and stays usable.
    if !live.conn.is_up() && state.view() != View::Project {
        render_unreachable(frame, inner, live, ctx);
        return;
    }

    match state.view() {
        View::Sessions if state.is_drilled() => render_session_detail(frame, inner, live, ctx),
        View::Sessions => render_sessions(frame, inner, live, ctx),
        View::Project => render_project(frame, inner, ctx),
        View::Permissions => render_permissions(frame, inner, live, ctx),
        View::Fleet => render_lines(frame, inner, vec![Line::from(ctx.msg(Msg::NoAgents))]),
    }
}

fn render_unreachable(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    let detail = match &live.conn {
        ConnState::Unreachable { detail } => detail.as_str(),
        _ => "",
    };
    let body = format!(
        "{}\n\n{detail}\n\n{}",
        ctx.msg(Msg::Unreachable),
        if ctx.lang == Lang::Es {
            "reconectando... (el daemon nunca abre un puerto de red)"
        } else {
            "reconnecting... (the daemon never opens a network port)"
        }
    );
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: true }), area);
}

fn render_sessions(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    if live.sessions.is_empty() {
        // Launchpad, not a mute table.
        let text = format!(
            "{}\n\n{}",
            ctx.msg(Msg::NoSessions),
            if ctx.lang == Lang::Es {
                "4 Flota para elegir agente | : paleta"
            } else {
                "4 Fleet to pick an agent | : palette"
            }
        );
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), area);
        return;
    }
    // Reflow: drop the agent column on a narrow terminal.
    let wide = area.width >= 50;
    let lines: Vec<Line> = live
        .sessions
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let (glyph, word) = session_state_label(row.state, ctx.lang);
            let marker = if i == live.selected {
                glyphs::FOCUS.text(&ctx.present)
            } else {
                " "
            };
            let mut label = format!("{marker} {} {} {}", glyph.text(&ctx.present), word, row.id);
            if wide {
                label.push_str(&format!("  {}", row.agent));
            }
            if i == live.selected {
                Line::styled(label, ctx.emphasis())
            } else {
                Line::from(label)
            }
        })
        .collect();
    render_lines(frame, area, lines);
}

fn render_session_detail(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    let header = match live.selected_session() {
        Some(row) => {
            let (glyph, word) = session_state_label(row.state, ctx.lang);
            format!("{} {} {}", glyph.text(&ctx.present), word, row.id)
        }
        None => "-".to_string(),
    };
    let follow = if live.follow_tail {
        if ctx.lang == Lang::Es {
            "siguiendo"
        } else {
            "following"
        }
    } else if ctx.lang == Lang::Es {
        "desplazado"
    } else {
        "scrolled"
    };
    let mut lines = vec![
        Line::styled(header, ctx.emphasis()),
        Line::from(format!("[{follow}] x cancela | Esc atrás")),
        Line::from(""),
    ];
    // Show the tail of the transcript that fits.
    let capacity = area.height.saturating_sub(3) as usize;
    let start = live.transcript.len().saturating_sub(capacity);
    for line in &live.transcript[start..] {
        lines.push(Line::from(line.clone()));
    }
    render_lines(frame, area, lines);
}

fn render_project(frame: &mut Frame, area: Rect, ctx: &ShellCtx) {
    if !ctx.project {
        let text = format!(
            "{}\n\n{}",
            ctx.msg(Msg::NoProject),
            if ctx.lang == Lang::Es {
                "c iniciar constitución (próximamente)"
            } else {
                "c start constitution (coming soon)"
            }
        );
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), area);
        return;
    }
    let entries = read_meltemi_entries();
    let mut lines: Vec<Line> = entries.into_iter().map(Line::from).collect();
    lines.push(Line::from(""));
    lines.push(Line::from(if ctx.lang == Lang::Es {
        "verbos SDD reservados: explore review plan implement verify archive (próximamente)"
    } else {
        "reserved SDD verbs: explore review plan implement verify archive (coming soon)"
    }));
    render_lines(frame, area, lines);
}

fn render_permissions(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    if live.pending_permissions == 0 && live.notices.is_empty() {
        frame.render_widget(Paragraph::new(ctx.msg(Msg::NoPermissions)), area);
        return;
    }
    let mut lines = vec![Line::styled(
        format!(
            "{} {}",
            live.pending_permissions,
            if ctx.lang == Lang::Es {
                "pendientes (bandeja: #9)"
            } else {
                "pending (tray: #9)"
            }
        ),
        ctx.emphasis(),
    )];
    for notice in live
        .notices
        .iter()
        .rev()
        .take(area.height.saturating_sub(1) as usize)
    {
        lines.push(Line::from(notice.clone()));
    }
    render_lines(frame, area, lines);
}

fn render_lines(frame: &mut Frame, area: Rect, lines: Vec<Line<'static>>) {
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

/// Reads the top-level entries of `.meltemi/` for the Project view.
fn read_meltemi_entries() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(".meltemi")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// (glyph, word) for a session state — meaning never depends on color.
fn session_state_label(state: SessionState, lang: Lang) -> (Glyph, &'static str) {
    match state {
        SessionState::Starting => (
            glyphs::STARTING,
            if lang == Lang::Es {
                "iniciando"
            } else {
                "starting"
            },
        ),
        SessionState::Active => (
            glyphs::ACTIVE,
            if lang == Lang::Es { "activa" } else { "active" },
        ),
        SessionState::WaitingPermission => (
            glyphs::WAITING,
            if lang == Lang::Es {
                "esperando-permiso"
            } else {
                "waiting-permission"
            },
        ),
        SessionState::Ended => (
            glyphs::ENDED,
            if lang == Lang::Es {
                "finalizada"
            } else {
                "ended"
            },
        ),
    }
}

fn view_title(view: View, lang: Lang) -> &'static str {
    match (view, lang) {
        (View::Sessions, Lang::Es) => "1 Sesiones",
        (View::Sessions, Lang::En) => "1 Sessions",
        (View::Project, Lang::Es) => "2 Proyecto",
        (View::Project, Lang::En) => "2 Project",
        (View::Permissions, Lang::Es) => "3 Permisos",
        (View::Permissions, Lang::En) => "3 Permissions",
        (View::Fleet, Lang::Es) => "4 Flota",
        (View::Fleet, Lang::En) => "4 Fleet",
    }
}

fn render_overlay(frame: &mut Frame, area: Rect, overlay: &Overlay, ctx: &ShellCtx) {
    let (title, content) = match overlay {
        Overlay::Help => (
            ctx.msg(Msg::HelpTitle).to_string(),
            ctx.msg(Msg::HintKeys).to_string(),
        ),
        Overlay::Palette { input } => (
            ctx.msg(Msg::PaletteTitle).to_string(),
            format!(":{input}\n\n{}", ctx.msg(Msg::HintExitField)),
        ),
        Overlay::Confirm { action } => {
            let what = match action {
                ConfirmAction::Quit => ctx.msg(Msg::QuitConfirm).to_string(),
                ConfirmAction::CancelSession => {
                    if ctx.lang == Lang::Es {
                        "¿Cancelar la sesión? Termina el agente. Enter confirma - Esc cancela"
                            .into()
                    } else {
                        "Cancel the session? It ends the agent. Enter confirms - Esc cancels".into()
                    }
                }
                ConfirmAction::Shutdown => {
                    if ctx.lang == Lang::Es {
                        "¿Apagar el daemon? Afecta a todas las sesiones. Enter confirma - Esc cancela".into()
                    } else {
                        "Shut down the daemon? Affects all sessions. Enter confirms - Esc cancels"
                            .into()
                    }
                }
            };
            (
                if ctx.lang == Lang::Es {
                    "Confirmar".to_string()
                } else {
                    "Confirm".to_string()
                },
                what,
            )
        }
        Overlay::Onboarding => (
            ctx.msg(Msg::OnboardingTitle).to_string(),
            ctx.msg(Msg::OnboardingBody).to_string(),
        ),
    };

    let (pct_w, pct_h) = match overlay {
        Overlay::Onboarding => (72, 72),
        Overlay::Help => (60, 50),
        _ => (60, 30),
    };
    let popup = centered(area, pct_w, pct_h);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ctx.border_set())
        .title(Span::styled(format!(" {title} "), ctx.emphasis()));
    frame.render_widget(
        Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        popup,
    );
}

/// A rectangle centered in `area`, sized to the given width/height percentages.
fn centered(area: Rect, pct_w: u16, pct_h: u16) -> Rect {
    let [_, mid_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Percentage(pct_h),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .areas(area);
    let [_, mid, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Percentage(pct_w),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .areas(mid_v);
    mid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::live::{SessionRow, Update};
    use crate::shell::present::{Presentation, PresentationEnv};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn ctx(present: Presentation) -> ShellCtx {
        ShellCtx {
            present,
            lang: Lang::Es,
            project: false,
        }
    }

    fn draw(state: &ShellState, live: &LiveData, ctx: &ShellCtx, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| render(f, state, live, ctx)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    fn default_present() -> Presentation {
        Presentation::resolve(&PresentationEnv {
            lang: Some("es.UTF-8".into()),
            ..Default::default()
        })
    }

    #[test]
    fn permission_indicator_always_shows_count_and_word() {
        let mut live = LiveData::new();
        live.apply(Update::Pending(3));
        let out = draw(&ShellState::new(), &live, &ctx(default_present()), 80, 24);
        assert!(out.contains('3') && out.contains("esperando"));
    }

    #[test]
    fn ascii_render_has_no_unicode_glyphs_or_box_drawing() {
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        let mut live = LiveData::new();
        live.apply(Update::Sessions(vec![SessionRow {
            id: "s1".into(),
            agent: "mock".into(),
            state: SessionState::Active,
        }]));
        let out = draw(&ShellState::new(), &live, &ctx(ascii), 80, 24);
        for forbidden in ['…', '▸', '·', '›', '┌', '│', '●', '▶'] {
            assert!(
                !out.contains(forbidden),
                "ASCII render must not contain {forbidden:?}"
            );
        }
        assert!(out.contains('+') && out.contains('|'));
    }

    #[test]
    fn no_color_render_emits_no_colored_cells() {
        let mono = Presentation::resolve(&PresentationEnv {
            no_color: Some("1".into()),
            lang: Some("en_US.UTF-8".into()),
            ..Default::default()
        });
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::Back); // quit confirmation
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let live = LiveData::new();
        terminal.draw(|f| render(f, &s, &live, &ctx(mono))).unwrap();
        use ratatui::style::Color;
        for cell in terminal.backend().buffer().content() {
            assert_eq!(cell.fg, Color::Reset);
            assert_eq!(cell.bg, Color::Reset);
        }
    }

    #[test]
    fn empty_sessions_show_a_launchpad_not_a_mute_table() {
        // Scenario: Launchpad en lugar de tabla vacía.
        let live = LiveData::new(); // connected-less but not unreachable
        let mut connected = live;
        connected.apply(Update::Conn(ConnState::Connected {
            version: "0.1.0".into(),
            uptime_s: 1,
            sessions: 0,
        }));
        let out = draw(
            &ShellState::new(),
            &connected,
            &ctx(default_present()),
            80,
            24,
        );
        assert!(
            out.contains("sin sesiones"),
            "empty Sessions must teach the next step"
        );
        assert!(out.contains("Flota"));
    }

    #[test]
    fn fleet_shows_its_empty_state() {
        let mut live = LiveData::new();
        live.apply(Update::Conn(ConnState::Connected {
            version: "0.1.0".into(),
            uptime_s: 1,
            sessions: 0,
        }));
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::SwitchView(4));
        let out = draw(&s, &live, &ctx(default_present()), 80, 24);
        assert!(out.contains("sin agentes"));
    }

    #[test]
    fn unreachable_daemon_shows_the_card_on_sessions() {
        let mut live = LiveData::new();
        live.apply(Update::Conn(ConnState::Unreachable {
            detail: "socket ausente".into(),
        }));
        let out = draw(&ShellState::new(), &live, &ctx(default_present()), 80, 24);
        assert!(out.contains("inalcanzable"));
        assert!(out.contains("reconectando"));
    }

    #[test]
    fn below_minimum_size_shows_the_floor_with_critical_indicators() {
        // Scenario: Aviso de tamaño insuficiente; el contador crítico sobrevive.
        let mut live = LiveData::new();
        live.apply(Update::Pending(2));
        let out = draw(&ShellState::new(), &live, &ctx(default_present()), 40, 10);
        assert!(out.contains("80x24"), "the floor states the required size");
        assert!(out.contains('2'), "the pending count survives at the floor");
    }

    #[test]
    fn disconnect_banner_outranks_pending_in_the_alert() {
        // Scenario: Orden de prioridad — daemon caído por encima del permiso.
        let mut live = LiveData::new();
        live.apply(Update::Pending(5));
        live.apply(Update::Conn(ConnState::Unreachable {
            detail: "closed".into(),
        }));
        let out = draw(&ShellState::new(), &live, &ctx(default_present()), 80, 24);
        assert!(
            out.contains("reconectando"),
            "the disconnect banner outranks pending permissions"
        );
    }

    #[test]
    fn permission_banner_surfaces_from_any_view() {
        let mut live = LiveData::new();
        live.apply(Update::Conn(ConnState::Connected {
            version: "0.1.0".into(),
            uptime_s: 1,
            sessions: 0,
        }));
        live.apply(Update::Pending(4));
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::SwitchView(2)); // Project view
        let out = draw(&s, &live, &ctx(default_present()), 80, 24);
        assert!(
            out.contains("permisos esperando"),
            "pending permissions surface from any view"
        );
    }

    #[test]
    fn onboarding_overlay_teaches_navigation_and_exit() {
        // Scenario: Primer uso enseña navegación y salida.
        let mut s = ShellState::new();
        s.show_onboarding();
        let out = draw(&s, &LiveData::new(), &ctx(default_present()), 80, 24);
        assert!(out.contains("Bienvenido"));
        assert!(
            out.contains("Esc"),
            "it teaches how to escape a capture context"
        );
        assert!(out.contains('q'), "it teaches how to quit");
    }
}
