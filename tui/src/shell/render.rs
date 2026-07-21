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
        render_overlay(frame, area, overlay, live, ctx);
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
        View::Fleet => render_fleet(frame, inner, live, ctx),
    }
}

/// The Fleet view: the catalog with detection (glyph + word), the declared
/// level as a textual label, and the configured-agent marker. With zero
/// detected agents the registry is shown anyway — what could be orchestrated
/// — together with the BYO-agent hint: content, never a mute screen.
fn render_fleet(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    let Some(fleet) = &live.fleet else {
        let text = format!(
            "{} {}",
            glyphs::PENDING.text(&ctx.present),
            ctx.msg(Msg::FleetLoading)
        );
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), area);
        return;
    };

    let detected = fleet.rows.iter().filter(|r| r.detected).count();
    let header = if ctx.lang == Lang::Es {
        format!(
            "registro {} — {} agentes, {} detectados",
            fleet.registry_version,
            fleet.rows.len(),
            detected
        )
    } else {
        format!(
            "registry {} — {} agents, {} detected",
            fleet.registry_version,
            fleet.rows.len(),
            detected
        )
    };
    let mut lines = vec![Line::styled(header, ctx.emphasis()), Line::from("")];

    let id_width = fleet.rows.iter().map(|r| r.id.len()).max().unwrap_or(0);
    for row in &fleet.rows {
        let (glyph, word) = detection_label(row.detected, ctx.lang);
        let level_word = level_label(row.level, ctx.lang);
        // Declared vs verified level (design D4): declared is always shown;
        // verified is shown when a conformance run recorded it, else "sin
        // verificar" so the distinction is visible, not hidden.
        let verified = match row.verified_level {
            Some(v) if ctx.lang == Lang::Es => format!("verif L{v}"),
            Some(v) => format!("verif L{v}"),
            None if ctx.lang == Lang::Es => "sin verificar".to_string(),
            None => "unverified".to_string(),
        };
        let mut label = format!(
            "{} {word:<13} L{} {level_word:<10} {verified:<13} {:<id_width$}  {}",
            glyph.text(&ctx.present),
            row.level,
            row.id,
            row.name,
        );
        if row.custom {
            label.push_str(" [custom]");
        }
        if row.configured {
            label.push_str(&format!(
                " {} {}",
                glyphs::SELECT.text(&ctx.present),
                if ctx.lang == Lang::Es {
                    "configurado"
                } else {
                    "configured"
                }
            ));
            lines.push(Line::styled(label, ctx.emphasis()));
        } else {
            lines.push(Line::from(label));
        }
    }

    if detected == 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(ctx.msg(Msg::NoAgents)));
        lines.push(Line::from(ctx.msg(Msg::FleetByoHint)));
    }
    render_lines(frame, area, lines);
}

/// How one scenario differs between the living and the modified requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMark {
    /// Present in both (unchanged position by name).
    Kept,
    /// Only in the modified version (added by the delta).
    Added,
    /// Only in the living version (removed by the delta).
    Removed,
}

/// Aligns two scenario-name lists (before/after) by name, classifying each as
/// kept, added, or removed — the core of a MODIFIED delta's semantic diff
/// (revision-specs-ux). Deterministic: befores first (kept/removed in their
/// order), then adds in the after order.
#[must_use]
pub fn align_scenarios(before: &[String], after: &[String]) -> Vec<(DiffMark, String)> {
    let after_set: std::collections::HashSet<&str> = after.iter().map(String::as_str).collect();
    let before_set: std::collections::HashSet<&str> = before.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for name in before {
        let mark = if after_set.contains(name.as_str()) {
            DiffMark::Kept
        } else {
            DiffMark::Removed
        };
        out.push((mark, name.clone()));
    }
    for name in after {
        if !before_set.contains(name.as_str()) {
            out.push((DiffMark::Added, name.clone()));
        }
    }
    out
}

/// Renders an aligned MODIFIED diff into lines: each scenario carries a glyph
/// and a word (never color alone), legible in ASCII / NO_COLOR. A review view
/// mounts these lines in the Project surface (revision-specs-ux).
pub fn render_diff_lines(diff: &[(DiffMark, String)], ctx: &ShellCtx) -> Vec<Line<'static>> {
    diff.iter()
        .map(|(mark, name)| {
            let (glyph, word) = match mark {
                DiffMark::Kept => (
                    glyphs::OK,
                    if ctx.lang == Lang::Es {
                        "igual"
                    } else {
                        "kept"
                    },
                ),
                DiffMark::Added => (
                    glyphs::SELECT,
                    if ctx.lang == Lang::Es {
                        "añadido"
                    } else {
                        "added"
                    },
                ),
                DiffMark::Removed => (
                    glyphs::ABSENT,
                    if ctx.lang == Lang::Es {
                        "retirado"
                    } else {
                        "removed"
                    },
                ),
            };
            Line::from(format!("{} {word:<9} {name}", glyph.text(&ctx.present)))
        })
        .collect()
}

/// (glyph, word) for a fleet detection state — never color alone.
fn detection_label(detected: bool, lang: Lang) -> (Glyph, &'static str) {
    if detected {
        (
            glyphs::OK,
            if lang == Lang::Es {
                "detectado"
            } else {
                "detected"
            },
        )
    } else {
        (
            glyphs::ABSENT,
            if lang == Lang::Es {
                "no-detectado"
            } else {
                "not-detected"
            },
        )
    }
}

/// The word of a declared integration level: the label is `L<n>` plus this.
fn level_label(level: u8, lang: Lang) -> &'static str {
    match (level, lang) {
        (1, Lang::Es) => "nativo",
        (1, Lang::En) => "native",
        (2, Lang::Es) => "adaptador",
        (2, Lang::En) => "adapter",
        (3, _) => "headless",
        (4, Lang::Es) => "artefactos",
        (4, Lang::En) => "artifacts",
        _ => "?",
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
    // Reflow: rows are not wrapped; content wider than the area is reached by
    // horizontal scroll (Left/Right) instead of being truncated-and-hidden.
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
            let label = format!(
                "{marker} {} {} {}  {}",
                glyph.text(&ctx.present),
                word,
                row.id,
                row.agent
            );
            let shown: String = label.chars().skip(live.h_scroll).collect();
            if i == live.selected {
                Line::styled(shown, ctx.emphasis())
            } else {
                Line::from(shown)
            }
        })
        .collect();
    // No wrap: long rows clip at the edge and are panned with h_scroll.
    frame.render_widget(Paragraph::new(lines), area);
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
    // A historical session shows its transcript read by `session/log`; a live
    // one shows the streamed transcript with its follow indicator.
    let historical = live
        .selected_session()
        .is_some_and(|row| row.is_historical());
    let subtitle = if historical {
        let resumable = live.selected_session().is_some_and(|r| r.resumable);
        if ctx.lang == Lang::Es {
            if resumable {
                "histórica — reanudable | Esc atrás".to_string()
            } else {
                "histórica — inspeccionable, no reanudable | Esc atrás".to_string()
            }
        } else if resumable {
            "historical — resumable | Esc back".to_string()
        } else {
            "historical — inspectable, not resumable | Esc back".to_string()
        }
    } else {
        format!("[{follow}] x cancela | Esc atrás")
    };

    let mut lines = vec![
        Line::styled(header, ctx.emphasis()),
        Line::from(subtitle),
        Line::from(""),
    ];

    let source = if historical {
        &live.session_log
    } else {
        &live.transcript
    };
    if historical && source.is_empty() {
        lines.push(Line::from(if ctx.lang == Lang::Es {
            "leyendo el registro..."
        } else {
            "reading the log..."
        }));
    }
    // Show the tail of the transcript that fits.
    let capacity = area.height.saturating_sub(3) as usize;
    let start = source.len().saturating_sub(capacity);
    for line in &source[start..] {
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
        "Enter regenera la proyección de contexto (AGENTS.md, ...)"
    } else {
        "Enter regenerates the projected context (AGENTS.md, ...)"
    }));
    lines.push(Line::from(if ctx.lang == Lang::Es {
        "verbos SDD reservados: explore review plan implement verify archive (próximamente)"
    } else {
        "reserved SDD verbs: explore review plan implement verify archive (coming soon)"
    }));
    render_lines(frame, area, lines);
}

fn render_permissions(frame: &mut Frame, area: Rect, live: &LiveData, ctx: &ShellCtx) {
    if live.permission_queue.is_empty() {
        let mut lines = vec![Line::from(ctx.msg(Msg::NoPermissions))];
        for notice in live.notices.iter().rev().take(3) {
            lines.push(Line::from(notice.clone()));
        }
        render_lines(frame, area, lines);
        return;
    }

    let waiting = live.permission_queue.iter().filter(|p| !p.expired).count();
    let header = if ctx.lang == Lang::Es {
        format!("{waiting} esperando decisión")
    } else {
        format!("{waiting} awaiting a decision")
    };
    let mut lines = vec![
        Line::styled(header, ctx.emphasis()),
        Line::from(ctx.msg(Msg::TrayHint)),
        Line::from(""),
    ];

    for (i, row) in live.permission_queue.iter().enumerate() {
        let (glyph, word) = if row.expired {
            (
                glyphs::ERROR,
                if ctx.lang == Lang::Es {
                    "vencido"
                } else {
                    "expired"
                },
            )
        } else {
            (
                glyphs::WAITING,
                if ctx.lang == Lang::Es {
                    "esperando"
                } else {
                    "waiting"
                },
            )
        };
        let marker = if i == live.permission_selected {
            glyphs::FOCUS.text(&ctx.present)
        } else {
            " "
        };
        let timing = tray_timing(
            row.waiting_seconds,
            row.expires_in_seconds,
            row.expired,
            ctx.lang,
        );
        let label = format!(
            "{marker} {} {word}  {}  {}  [{timing}]",
            glyph.text(&ctx.present),
            row.tool,
            row.summary
        );
        if i == live.permission_selected {
            lines.push(Line::styled(label, ctx.emphasis()));
        } else {
            lines.push(Line::from(label));
        }
        if row.suggested_rule.is_some() {
            lines.push(Line::from(format!("    {}", ctx.msg(Msg::TrayFatigueHint))));
        }
    }

    for notice in live.notices.iter().rev().take(2) {
        lines.push(Line::from(notice.clone()));
    }
    render_lines(frame, area, lines);
}

/// The age/deadline label of a tray row, with textual escalation as expiry
/// nears — never color alone.
fn tray_timing(waiting: u64, expires_in: i64, expired: bool, lang: Lang) -> String {
    if expired {
        return if lang == Lang::Es {
            "vencido".into()
        } else {
            "expired".into()
        };
    }
    let soon = (0..15).contains(&expires_in);
    match (lang, soon) {
        (Lang::Es, true) => format!("{waiting}s — ¡vence en {expires_in}s!"),
        (Lang::Es, false) => format!("{waiting}s esperando — vence en {expires_in}s"),
        (Lang::En, true) => format!("{waiting}s — expires in {expires_in}s!"),
        (Lang::En, false) => format!("{waiting}s waiting — expires in {expires_in}s"),
    }
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
        SessionState::Interrupted => (
            glyphs::ERROR,
            if lang == Lang::Es {
                "interrumpida"
            } else {
                "interrupted"
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

fn render_overlay(
    frame: &mut Frame,
    area: Rect,
    overlay: &Overlay,
    live: &LiveData,
    ctx: &ShellCtx,
) {
    let (title, content) = match overlay {
        Overlay::Help => (
            ctx.msg(Msg::HelpTitle).to_string(),
            ctx.msg(Msg::HintKeys).to_string(),
        ),
        Overlay::Palette { input } => {
            // Show the filtered capability registry (core parity, design D7).
            let mut body = format!(":{input}\n\n");
            for entry in crate::shell::palette::matches(input) {
                let mark = if entry.reserved { " (reservado)" } else { "" };
                body.push_str(&format!(
                    "  {}{mark} — {}\n",
                    entry.name,
                    entry.desc(ctx.lang)
                ));
            }
            body.push_str(&format!("\n{}", ctx.msg(Msg::HintExitField)));
            (ctx.msg(Msg::PaletteTitle).to_string(), body)
        }
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
                ConfirmAction::CreateRule => create_rule_prompt(live, ctx),
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
        Overlay::Palette { .. } => (72, 90),
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

/// The confirmation text for creating a rule from the selected request: names
/// the proposed (most specific) rule so the human confirms exactly what will
/// persist.
fn create_rule_prompt(live: &LiveData, ctx: &ShellCtx) -> String {
    use meltemi_proto::{PermissionRuleEffect, PermissionRuleScope};
    let Some(rule) = live.selected_rule_proposal() else {
        return if ctx.lang == Lang::Es {
            "No hay petición seleccionada.".into()
        } else {
            "No request selected.".into()
        };
    };
    let effect = match rule.effect {
        PermissionRuleEffect::Allow => {
            if ctx.lang == Lang::Es {
                "permitir"
            } else {
                "allow"
            }
        }
        PermissionRuleEffect::Deny => {
            if ctx.lang == Lang::Es {
                "denegar"
            } else {
                "deny"
            }
        }
    };
    let scope = match rule.scope {
        PermissionRuleScope::Project => {
            if ctx.lang == Lang::Es {
                "este proyecto"
            } else {
                "this project"
            }
        }
        // Same word in both languages.
        PermissionRuleScope::Global => "global",
    };
    let mut matchers = Vec::new();
    if let Some(tool) = &rule.tool {
        matchers.push(format!("tool={tool}"));
    }
    if let Some(cmd) = &rule.command_prefix {
        matchers.push(format!("cmd~={cmd}"));
    }
    if let Some(path) = &rule.path_prefix {
        matchers.push(format!("path~={path}"));
    }
    let matcher_text = if matchers.is_empty() {
        if ctx.lang == Lang::Es {
            "(todo)".to_string()
        } else {
            "(anything)".to_string()
        }
    } else {
        matchers.join(" ")
    };
    if ctx.lang == Lang::Es {
        format!(
            "Crear regla: {effect} {matcher_text} en {scope}.\n\nEnter confirma y aprueba - Esc cancela"
        )
    } else {
        format!(
            "Create rule: {effect} {matcher_text} in {scope}.\n\nEnter confirms and approves - Esc cancels"
        )
    }
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
    use crate::shell::live::{FleetRow, FleetSnapshot, SessionRow, Update};
    use crate::shell::present::{Presentation, PresentationEnv};
    use meltemi_proto::{
        PendingPermission, PermissionOption, PermissionOptionKind, PermissionRule,
        PermissionRuleEffect, PermissionRuleScope,
    };
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
            project_root: "/repo".into(),
            resumable: false,
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

    fn fleet_row(id: &str, level: u8, detected: bool, configured: bool) -> FleetRow {
        FleetRow {
            id: id.into(),
            name: format!("Agent {id}"),
            level,
            verified_level: None,
            detected,
            binary_path: detected.then(|| format!("/bin/{id}")),
            configured,
            custom: false,
            underlying_agent: None,
        }
    }

    fn connected() -> LiveData {
        let mut live = LiveData::new();
        live.apply(Update::Conn(ConnState::Connected {
            version: "0.1.0".into(),
            uptime_s: 1,
            sessions: 0,
        }));
        live
    }

    fn fleet_view() -> ShellState {
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::SwitchView(4));
        s
    }

    #[test]
    fn fleet_without_data_yet_shows_the_loading_line() {
        let out = draw(&fleet_view(), &connected(), &ctx(default_present()), 80, 24);
        assert!(out.contains("consultando la flota"));
    }

    #[test]
    fn fleet_table_shows_detection_level_and_configured_marker() {
        // Scenario: Tabla con detectados y no detectados.
        let mut live = connected();
        live.apply(Update::Fleet(FleetSnapshot {
            registry_version: "2026-07-09".into(),
            rows: vec![
                fleet_row("uno", 1, true, true),
                fleet_row("dos", 4, false, false),
            ],
        }));
        let out = draw(&fleet_view(), &live, &ctx(default_present()), 80, 24);
        assert!(out.contains("registro 2026-07-09"), "version is visible");
        assert!(out.contains("detectado"), "detection carries a word");
        assert!(out.contains("no-detectado"), "absence carries a word too");
        assert!(out.contains("L1") && out.contains("nativo"), "level label");
        assert!(out.contains("L4") && out.contains("artefactos"));
        assert!(out.contains("configurado"), "configured agent is marked");
    }

    #[test]
    fn fleet_with_zero_detected_keeps_the_catalog_and_the_byo_hint() {
        // Scenario: Cero detectados sigue enseñando el camino.
        let mut live = connected();
        live.apply(Update::Fleet(FleetSnapshot {
            registry_version: "2026-07-09".into(),
            rows: vec![
                fleet_row("uno", 1, false, false),
                fleet_row("dos", 2, false, false),
            ],
        }));
        let out = draw(&fleet_view(), &live, &ctx(default_present()), 80, 24);
        assert!(out.contains("uno"), "the registry is still listed");
        assert!(out.contains("no-detectado"));
        assert!(out.contains("sin agentes detectados"));
        assert!(
            out.contains("fleet.custom"),
            "the BYO-agent remediation hint survives"
        );
    }

    #[test]
    fn fleet_ascii_render_uses_the_glyph_twins() {
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        let mut live = connected();
        live.apply(Update::Fleet(FleetSnapshot {
            registry_version: "v".into(),
            rows: vec![
                fleet_row("uno", 1, true, true),
                fleet_row("dos", 2, false, false),
            ],
        }));
        let out = draw(&fleet_view(), &live, &ctx(ascii), 80, 24);
        for forbidden in ['●', '○', '›', '…', '┌', '│'] {
            assert!(
                !out.contains(forbidden),
                "ASCII fleet render must not contain {forbidden:?}"
            );
        }
        assert!(out.contains("detectado"), "meaning survives in words");
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

    fn pending(request_id: &str, tool: &str, expired: bool, suggest: bool) -> PendingPermission {
        PendingPermission {
            request_id: request_id.into(),
            session_id: "sess-1".into(),
            tool: tool.into(),
            summary: format!("do {tool}"),
            options: vec![
                PermissionOption {
                    option_id: "allow".into(),
                    name: "Allow".into(),
                    kind: PermissionOptionKind::AllowOnce,
                },
                PermissionOption {
                    option_id: "reject".into(),
                    name: "Reject".into(),
                    kind: PermissionOptionKind::RejectOnce,
                },
            ],
            waiting_seconds: 5,
            expires_in_seconds: if expired { -1 } else { 100 },
            expired,
            suggested_rule: suggest.then(|| PermissionRule {
                effect: PermissionRuleEffect::Allow,
                tool: Some(tool.into()),
                command_prefix: None,
                path_prefix: None,
                scope: PermissionRuleScope::Project,
            }),
        }
    }

    fn permissions_view() -> ShellState {
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::SwitchView(3));
        s
    }

    #[test]
    fn tray_lists_pending_with_state_word_timing_and_decide_hint() {
        // Scenario: Bandeja interactiva — lista con edad/plazo y decisión.
        let mut live = connected();
        live.apply(Update::PermissionQueue(vec![
            pending("perm-1", "execute", false, false),
            pending("perm-2", "edit", true, false),
        ]));
        let out = draw(&permissions_view(), &live, &ctx(default_present()), 80, 24);
        assert!(
            out.contains("esperando"),
            "a waiting request carries a word"
        );
        assert!(out.contains("vencido"), "an expired request stays visible");
        assert!(
            out.contains("execute") && out.contains("edit"),
            "tools listed"
        );
        assert!(out.contains("Enter aprueba"), "the decide hint is shown");
    }

    #[test]
    fn tray_shows_the_anti_fatigue_suggestion() {
        // Scenario: Sugerencia anti-fatiga surfaced in the tray.
        let mut live = connected();
        live.apply(Update::PermissionQueue(vec![pending(
            "perm-1", "execute", false, true,
        )]));
        let out = draw(&permissions_view(), &live, &ctx(default_present()), 80, 24);
        assert!(
            out.contains("crea una regla"),
            "the fatigue suggestion is surfaced"
        );
    }

    #[test]
    fn tray_create_rule_confirm_names_the_proposed_rule() {
        // Scenario: Aprobar y crear regla — confirmar la regla propuesta.
        let mut live = connected();
        live.apply(Update::PermissionQueue(vec![pending(
            "perm-1", "execute", false, false,
        )]));
        let mut s = permissions_view();
        s.reduce(crate::shell::keymap::Action::Local('r')); // open the confirm
        let out = draw(&s, &live, &ctx(default_present()), 80, 24);
        assert!(out.contains("Crear regla"), "the confirm names the action");
        assert!(
            out.contains("tool=execute"),
            "the most specific rule is shown for confirmation"
        );
    }

    #[test]
    fn tray_ascii_render_keeps_meaning_in_words() {
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        let mut live = connected();
        live.apply(Update::PermissionQueue(vec![pending(
            "perm-1", "execute", false, false,
        )]));
        let out = draw(&permissions_view(), &live, &ctx(ascii), 80, 24);
        for forbidden in ['‖', '✕', '▸', '…', '│'] {
            assert!(
                !out.contains(forbidden),
                "ASCII tray must not contain {forbidden:?}"
            );
        }
        assert!(out.contains("esperando"), "meaning survives in words");
    }

    #[test]
    fn modified_diff_aligns_scenarios_by_name() {
        // Scenario: MODIFIED alineado por escenario.
        let before = vec!["opens".to_string(), "closes".to_string()];
        let after = vec!["opens".to_string(), "cancels".to_string()];
        let diff = align_scenarios(&before, &after);
        assert!(diff.contains(&(DiffMark::Kept, "opens".into())));
        assert!(diff.contains(&(DiffMark::Removed, "closes".into())));
        assert!(diff.contains(&(DiffMark::Added, "cancels".into())));
    }

    #[test]
    fn diff_render_is_legible_in_ascii_with_words_and_glyph_twins() {
        // Scenario: Legible sin color (ASCII twins + textual labels).
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        let diff = align_scenarios(
            &["opens".into(), "closes".into()],
            &["opens".into(), "cancels".into()],
        );
        let lines = render_diff_lines(&diff, &ctx(ascii));
        let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
        terminal
            .draw(|f| f.render_widget(Paragraph::new(lines), f.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("añadido") && text.contains("retirado") && text.contains("igual"));
        for forbidden in ['›', '○', '●'] {
            assert!(
                !text.contains(forbidden),
                "ASCII diff must avoid {forbidden:?}"
            );
        }
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

    #[test]
    fn palette_lists_the_capability_registry() {
        // Scenario: Capacidad sin vista dedicada alcanzable por la paleta.
        let mut s = ShellState::new();
        s.reduce(crate::shell::keymap::Action::OpenPalette);
        // The registry now spans every contract method (paridad D3): render
        // tall enough that the full list fits.
        let out = draw(&s, &LiveData::new(), &ctx(default_present()), 80, 48);
        assert!(out.contains("status"), "operational commands are listed");
        assert!(out.contains("archive"), "reserved SDD verbs are listed");
        assert!(
            out.contains("reservado"),
            "reserved verbs are announced, not errors"
        );
    }
}
