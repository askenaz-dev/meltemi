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

use crate::shell::glyphs;
use crate::shell::messages::{Lang, Msg, text};
use crate::shell::present::Presentation;
use crate::shell::state::{ConfirmAction, Overlay, ShellState, View};

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

/// Everything the renderer needs beyond the navigation state.
pub struct ShellCtx {
    pub present: Presentation,
    pub lang: Lang,
    pub conn: ConnState,
    pub pending_permissions: usize,
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

    /// The border set honoring the Unicode capability.
    fn border_set(&self) -> border::Set {
        if self.present.unicode {
            border::PLAIN
        } else {
            ASCII_BORDER
        }
    }
}

/// Draws the whole shell frame.
pub fn render(frame: &mut Frame, state: &ShellState, ctx: &ShellCtx) {
    let areas = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());
    let (header, body, footer) = (areas[0], areas[1], areas[2]);

    render_header(frame, header, ctx);
    render_body(frame, body, state, ctx);
    render_footer(frame, footer, state, ctx);

    if let Some(overlay) = state.top_overlay() {
        render_overlay(frame, frame.area(), overlay, ctx);
    }
}

fn render_header(frame: &mut Frame, area: Rect, ctx: &ShellCtx) {
    let (glyph, word) = match &ctx.conn {
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
        ctx.pending_permissions,
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

fn render_body(frame: &mut Frame, area: Rect, state: &ShellState, ctx: &ShellCtx) {
    // Breadcrumb reflects drill-in depth.
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

    let empty = empty_state(state.view(), ctx);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ctx.border_set())
        .title(Span::styled(format!(" {crumb} "), ctx.emphasis()));
    let paragraph = Paragraph::new(empty).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

/// The empty-state / placeholder text for a view. In this foundation every view
/// shows its labeled empty state; live data wires in later waves.
fn empty_state(view: View, ctx: &ShellCtx) -> String {
    if let ConnState::Unreachable { detail } = &ctx.conn {
        return format!("{}\n\n{detail}", ctx.msg(Msg::Unreachable));
    }
    match view {
        View::Sessions => ctx.msg(Msg::NoSessions).to_string(),
        View::Project => {
            if ctx.project {
                String::new()
            } else {
                ctx.msg(Msg::NoProject).to_string()
            }
        }
        View::Permissions => ctx.msg(Msg::NoPermissions).to_string(),
        View::Fleet => ctx.msg(Msg::NoAgents).to_string(),
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
    };

    let popup = centered(area, 60, 30);
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
    use crate::shell::present::{Presentation, PresentationEnv};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn ctx(present: Presentation, conn: ConnState) -> ShellCtx {
        ShellCtx {
            present,
            lang: Lang::Es,
            conn,
            pending_permissions: 3,
            project: false,
        }
    }

    fn render_to_string(state: &ShellState, ctx: &ShellCtx, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| render(f, state, ctx)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    }

    #[test]
    fn permission_indicator_is_always_drawn_with_count_and_word() {
        // Scenario: Indicador legible sin color — símbolo + contador + palabra.
        let present = Presentation::resolve(&PresentationEnv::default());
        let state = ShellState::new();
        let out = render_to_string(&state, &ctx(present, ConnState::Connecting), 80, 24);
        assert!(out.contains('3'), "permission count must be visible");
        assert!(out.contains("esperando"), "permission word must be visible");
    }

    #[test]
    fn ascii_presentation_avoids_unicode_glyphs_and_box_drawing() {
        // Scenario: Conmutación a ASCII — cuadros por + - |, sin glifos Unicode.
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        let state = ShellState::new();
        let out = render_to_string(&state, &ctx(ascii, ConnState::Connecting), 80, 24);
        for forbidden in ['…', '▸', '·', '›', '┌', '┐', '└', '┘', '─', '│', '●']
        {
            assert!(
                !out.contains(forbidden),
                "ASCII render must not contain the Unicode glyph {forbidden:?}"
            );
        }
        // The ASCII border twins are present instead.
        assert!(
            out.contains('+') && out.contains('|'),
            "ASCII borders use + and |"
        );
    }

    #[test]
    fn no_color_render_emits_no_colored_cells() {
        // Scenario: Render monocromo — ninguna celda lleva color de primer/fondo.
        let mono = Presentation::resolve(&PresentationEnv {
            no_color: Some("1".into()),
            lang: Some("en_US.UTF-8".into()),
            ..Default::default()
        });
        assert!(!mono.color);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        // Open a confirmation to exercise emphasized (reversed) cells too.
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::Back); // pushes a quit confirmation
        terminal
            .draw(|f| render(f, &s, &ctx(mono, ConnState::Connecting)))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        use ratatui::style::Color;
        for cell in buffer.content() {
            assert_eq!(cell.fg, Color::Reset, "no cell may set a foreground color");
            assert_eq!(cell.bg, Color::Reset, "no cell may set a background color");
        }
    }

    #[test]
    fn unreachable_shows_the_detail_in_the_body() {
        let present = Presentation::resolve(&PresentationEnv::default());
        let state = ShellState::new();
        let conn = ConnState::Unreachable {
            detail: "socket ausente".into(),
        };
        let out = render_to_string(&state, &ctx(present, conn), 80, 24);
        assert!(out.contains("inalcanzable"));
    }
}
