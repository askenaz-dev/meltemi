// SPDX-License-Identifier: Apache-2.0

//! The pulse of the shell (senales-de-vida D5): a spinner in the header while
//! something works, the time of a working session in its detail, and the tick
//! that animates them — armed only while a session works, so a shell at rest
//! adds no redraw of its own.
//!
//! Everything here is a function of what it is handed. The renderer reads the
//! frame and the instant from [`Pulse`], which the event loop fixes before each
//! draw, so a test fixes the clock instead of waiting for it.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::shell::live::{LiveData, SessionRow};
use crate::shell::present::Presentation;

/// How often the pulse ticks while something works: about ten frames a
/// second, the starting point the design names. How it looks in conhost is
/// measured by hand, not here.
pub const TICK: Duration = Duration::from_millis(100);

/// The braille spinner.
pub const SPINNER_UNICODE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Its ASCII twin, with a cycle of its own: ten frames folded onto four would
/// jump backwards once per turn.
pub const SPINNER_ASCII: &[&str] = &["-", "\\", "|", "/"];

/// What the renderer reads of time: the spinner's frame and the instant, in
/// Unix seconds, the clocks count against. `now` stays `None` until the loop
/// fixes it, and a clock with no instant is not drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pulse {
    pub frame: u64,
    pub now: Option<i64>,
}

/// The spinner's frame for `frame`, in the presentation's own cycle.
#[must_use]
pub fn spinner(frame: u64, present: &Presentation) -> &'static str {
    let cycle = if present.unicode {
        SPINNER_UNICODE
    } else {
        SPINNER_ASCII
    };
    let len = u64::try_from(cycle.len()).unwrap_or(1);
    cycle[usize::try_from(frame % len).unwrap_or(0)]
}

/// How many sessions are running a turn right now.
#[must_use]
pub fn working(live: &LiveData) -> usize {
    live.sessions.iter().filter(|row| row.is_working()).count()
}

/// Whether the tick is armed: only while at least one session works.
#[must_use]
pub fn armed(live: &LiveData) -> bool {
    working(live) > 0
}

/// `mm:ss`, and `h:mm:ss` past the hour.
#[must_use]
pub fn clock(secs: u64) -> String {
    let (hours, minutes, seconds) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// The time a session's detail shows: counting while it works, its final
/// duration once it ended, and nothing while it lives without working — a
/// clock the shell does not redraw is a clock that lies.
#[must_use]
pub fn session_time(row: &SessionRow, pulse: Pulse) -> Option<String> {
    let start = epoch_secs(&row.started_at)?;
    let end = match &row.ended_at {
        Some(ended) => epoch_secs(ended)?,
        None if row.is_working() => pulse.now?,
        None => return None,
    };
    // A clock that went backwards reads zero, never a negative time.
    Some(clock(u64::try_from(end - start).unwrap_or(0)))
}

/// Unix seconds now, which the loop fixes before each draw.
#[must_use]
pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

/// Unix seconds of an RFC 3339 instant in UTC, as the daemon writes them
/// (`2026-09-26T10:00:00Z`, fractional seconds tolerated). Anything else is
/// `None`: a time the shell cannot read is not shown rather than guessed.
#[must_use]
pub fn epoch_secs(ts: &str) -> Option<i64> {
    let bytes = ts.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let field = |range: std::ops::Range<usize>| -> Option<i64> {
        let text = ts.get(range)?;
        if !text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    };
    let (year, month, day) = (field(0..4)?, field(5..7)?, field(8..10)?);
    let (hour, minute, second) = (field(11..13)?, field(14..16)?, field(17..19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    // Fractional seconds, then UTC and nothing else.
    let mut rest = &ts[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &fraction[digits..];
    }
    if rest != "Z" && rest != "+00:00" {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Howard Hinnant's `days_from_civil`: a civil date to days since 1970-01-01
/// (the algorithm the daemon's clock already uses).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::live::{SessionRow, Update};
    use crate::shell::present::PresentationEnv;
    use meltemi_proto::SessionState;

    fn row(id: &str, state: SessionState) -> SessionRow {
        SessionRow {
            id: id.into(),
            agent: "mock".into(),
            state,
            project_root: "/repo".into(),
            resumable: false,
            agent_id: None,
            profile: None,
            title: None,
            mode: None,
            model: None,
            started_at: "2026-09-26T10:00:00Z".into(),
            ended_at: None,
        }
    }

    fn live_with(rows: Vec<SessionRow>) -> LiveData {
        let mut live = LiveData::new();
        live.apply(Update::Sessions(rows));
        live
    }

    fn ascii() -> Presentation {
        Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        })
    }

    fn unicode() -> Presentation {
        Presentation::resolve(&PresentationEnv {
            lang: Some("en_US.UTF-8".into()),
            ..Default::default()
        })
    }

    // Scenario: El spinner ASCII usa su propio ciclo
    #[test]
    fn the_ascii_spinner_turns_its_own_cycle() {
        let ascii = ascii();
        let frames: Vec<&str> = (0..8).map(|frame| spinner(frame, &ascii)).collect();
        assert_eq!(frames, ["-", "\\", "|", "/", "-", "\\", "|", "/"]);
        // The twin is ASCII through and through: no braille leaks into it.
        for frame in 0..40 {
            assert!(spinner(frame, &ascii).is_ascii(), "frame {frame} is ASCII");
        }
        assert!(SPINNER_ASCII.iter().all(|f| !f.is_empty() && f.is_ascii()));

        // The braille one turns its ten frames and comes back to the first.
        let unicode = unicode();
        let braille: Vec<&str> = (0..11).map(|frame| spinner(frame, &unicode)).collect();
        assert_eq!(braille[..10], *SPINNER_UNICODE);
        assert_eq!(braille[10], SPINNER_UNICODE[0]);
    }

    // Scenario: En reposo no hay tic
    #[test]
    fn at_rest_the_tick_is_not_armed() {
        // Nothing works: waiting on a decision, on an instruction, or over.
        let rest = live_with(vec![
            row("a", SessionState::WaitingPermission),
            row("b", SessionState::WaitingInstruction),
            row("c", SessionState::Ended),
            row("d", SessionState::Interrupted),
        ]);
        assert!(!armed(&rest), "a shell at rest arms no tick");
        assert!(!armed(&LiveData::new()), "nor does an empty one");

        // One running turn is enough to arm it, and starting counts.
        for state in [SessionState::Active, SessionState::Starting] {
            let busy = live_with(vec![row("a", SessionState::Ended), row("b", state)]);
            assert!(armed(&busy), "{state:?} arms the tick");
            assert_eq!(working(&busy), 1);
        }

        // And the loop asks exactly this before it lets the tick fire: the arm
        // is disabled, not merely ignored, while nothing works.
        let event_loop = include_str!("mod.rs");
        assert!(
            event_loop.contains("let armed = pulse::armed(&live);")
                && event_loop.contains("_ = tick.tick(), if armed =>"),
            "the loop's tick arm is guarded by `armed`"
        );
    }

    // Scenario: El detalle cuenta el tiempo de la sesión que trabaja
    #[test]
    fn the_detail_counts_only_a_time_it_can_keep() {
        let now = epoch_secs("2026-09-26T10:03:07Z");
        let pulse = Pulse { frame: 0, now };

        // Working: counted from its start against the instant of this draw.
        let working = row("a", SessionState::Active);
        assert_eq!(session_time(&working, pulse).as_deref(), Some("03:07"));
        // Without an instant fixed there is nothing to count against.
        assert_eq!(session_time(&working, Pulse::default()), None);

        // Ended: its final duration, whatever the clock says now.
        let ended = SessionRow {
            state: SessionState::Ended,
            ended_at: Some("2026-09-26T11:02:05Z".into()),
            ..row("b", SessionState::Ended)
        };
        assert_eq!(session_time(&ended, pulse).as_deref(), Some("1:02:05"));

        // Alive and not working: no tick keeps it current, so no time at all.
        for state in [
            SessionState::WaitingInstruction,
            SessionState::WaitingPermission,
        ] {
            assert_eq!(session_time(&row("c", state), pulse), None, "{state:?}");
        }
    }

    #[test]
    fn instants_are_read_as_the_daemon_writes_them_and_nothing_else() {
        assert_eq!(epoch_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch_secs("2001-09-09T01:46:40Z"), Some(1_000_000_000));
        assert_eq!(epoch_secs("2024-02-29T00:00:00Z"), Some(1_709_164_800));
        assert_eq!(epoch_secs("2026-07-11T12:34:56.789Z"), Some(1_783_773_296));
        assert_eq!(epoch_secs("2026-07-11T12:34:56+00:00"), Some(1_783_773_296));
        for unreadable in [
            "",
            "2026-07-11",
            "2026-07-11T12:34:56",
            "2026-07-11T12:34:56+02:00",
            "2026-13-11T12:34:56Z",
            "2026-07-11 12:34:56Z",
            "2026-07-11T12:34:56.Z",
        ] {
            assert_eq!(epoch_secs(unreadable), None, "{unreadable:?}");
        }
        assert_eq!(clock(0), "00:00");
        assert_eq!(clock(59), "00:59");
        assert_eq!(clock(3_599), "59:59");
        assert_eq!(clock(3_600), "1:00:00");
    }
}
