// SPDX-License-Identifier: Apache-2.0

//! Where the approver's keys come from, and when one counts (design D7).
//!
//! The terminal is read in raw mode without the alternate screen, from a
//! blocking thread. A key answers only once the question could be read: what
//! arrives before is discarded, and keys that arrive together — a paste, keys
//! mashed — answer nothing. The source is a trait so the tests can script what
//! the human types, and count that raw mode was left as often as it was
//! entered.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal;

use super::Key;

/// The source can give no more keys: the terminal failed, or a script ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceGone;

/// A source of keys, read from a blocking thread.
pub trait KeySource: Send {
    /// Called once per question, before its first read.
    fn begin(&mut self) {}
    /// Enters the mode in which keys arrive one at a time; `false` when the
    /// terminal refuses it.
    fn enter(&mut self) -> bool;
    /// Leaves that mode, restoring the terminal.
    fn leave(&mut self);
    /// Takes every key already waiting, without blocking.
    fn drain(&mut self) -> Vec<Key>;
    /// Waits up to `wait` for the next key: `Ok(None)` when none came in time.
    fn poll(&mut self, wait: Duration) -> Result<Option<Key>, SourceGone>;
    /// A key arriving within `within` of the last one — the rest of a burst.
    fn follows(&mut self, within: Duration) -> Result<Option<Key>, SourceGone> {
        self.poll(within)
    }
}

/// What one read of the source yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// A key pressed after the question could be read, and alone.
    Key(Key),
    /// Keys arrived before the question could be read: none answers it.
    Early,
    /// Keys arrived together — a paste, or keys mashed — so none is an answer.
    Burst,
    /// The read was stopped from outside: a withdrawal, a redraw, the end.
    Stopped,
    /// The source could not be read: nothing more can be asked here.
    Failed,
}

/// How long a question is shown before a key counts, how close together keys
/// make a burst, and how often a blocked read looks at its stop flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arming {
    pub dwell: Duration,
    pub burst: Duration,
    pub slice: Duration,
}

impl Arming {
    /// A person's pace: a question cannot be read in less than this.
    pub const TERMINAL: Self = Self {
        dwell: Duration::from_millis(400),
        burst: Duration::from_millis(40),
        slice: Duration::from_millis(50),
    };
    /// A scripted source, whose keys carry their timing in the script.
    pub const SCRIPTED: Self = Self {
        dwell: Duration::ZERO,
        burst: Duration::ZERO,
        slice: Duration::from_millis(5),
    };
}

/// Reads one answer (design D7). The source is in raw mode only inside this
/// call, and always leaves it before returning.
pub fn read_answer(source: &mut dyn KeySource, stop: &AtomicBool, arming: &Arming) -> Read {
    if !source.enter() {
        return Read::Failed;
    }
    let read = arm_and_read(source, stop, arming);
    source.leave();
    read
}

fn arm_and_read(source: &mut dyn KeySource, stop: &AtomicBool, arming: &Arming) -> Read {
    // The question is on screen; nothing answers it until it could be read.
    let armed = Instant::now() + arming.dwell;
    while let Some(left) = armed.checked_duration_since(Instant::now()) {
        if left.is_zero() {
            break;
        }
        if stop.load(Ordering::SeqCst) {
            return Read::Stopped;
        }
        std::thread::sleep(left.min(arming.slice));
    }
    // What arrived before answers nothing — except Ctrl+C, which only stops.
    let early = source.drain();
    if early.contains(&Key::CtrlC) {
        return Read::Key(Key::CtrlC);
    }
    if !early.is_empty() {
        return Read::Early;
    }
    loop {
        if stop.load(Ordering::SeqCst) {
            return Read::Stopped;
        }
        let key = match source.poll(arming.slice) {
            Err(SourceGone) => return Read::Failed,
            Ok(None) => continue,
            Ok(Some(key)) => key,
        };
        if key == Key::CtrlC {
            return Read::Key(key);
        }
        // A key counts only alone: one followed at once by another is part
        // of a paste or a mash, and the whole of it is set aside.
        return match source.follows(arming.burst) {
            Err(SourceGone) => Read::Failed,
            Ok(None) => Read::Key(key),
            Ok(Some(Key::CtrlC)) => Read::Key(Key::CtrlC),
            Ok(Some(_)) => {
                if source.drain().contains(&Key::CtrlC) {
                    Read::Key(Key::CtrlC)
                } else {
                    Read::Burst
                }
            }
        };
    }
}

/// Turns terminal events into keys: presses only, and — where the terminal
/// reports releases (Windows) — only the first press of a key held down, since
/// a held key repeats as presses.
#[derive(Debug, Default)]
pub struct Presses {
    down: HashSet<KeyCode>,
}

impl Presses {
    /// The key an event means, if it is a key the approver reads.
    pub fn take(&mut self, event: &Event) -> Option<Key> {
        match event {
            Event::Paste(_) => Some(Key::Paste),
            Event::Key(key) => match key.kind {
                KeyEventKind::Release => {
                    self.down.remove(&key.code);
                    None
                }
                KeyEventKind::Repeat => None,
                KeyEventKind::Press => {
                    if cfg!(windows) && !self.down.insert(key.code) {
                        return None;
                    }
                    Some(translate(key))
                }
            },
            _ => None,
        }
    }

    /// Forgets which keys were down: a new question starts clean.
    pub fn forget(&mut self) {
        self.down.clear();
    }
}

/// A pressed key as the approver reads it.
#[must_use]
pub fn translate(key: &KeyEvent) -> Key {
    let chorded = key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
    match key.code {
        KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => Key::CtrlC,
        KeyCode::Char(c) if !chorded => Key::Char(c),
        KeyCode::Esc => Key::Esc,
        KeyCode::Enter => Key::Enter,
        _ => Key::Other,
    }
}

/// The terminal itself, in raw mode without the alternate screen.
#[derive(Debug, Default)]
pub struct TerminalKeys {
    presses: Presses,
}

impl TerminalKeys {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this process can read keys from a terminal: stdin and stderr
    /// are terminals and raw mode can be entered. On Windows the keys come
    /// from the console, so stdin must be a real console (crossterm's check,
    /// which a pipe from a terminal emulator does not pass).
    #[must_use]
    pub fn available() -> bool {
        use ratatui::crossterm::tty::IsTty;
        use std::io::IsTerminal;
        if !std::io::stdin().is_tty() || !std::io::stderr().is_terminal() {
            return false;
        }
        if terminal::enable_raw_mode().is_err() {
            return false;
        }
        terminal::disable_raw_mode().is_ok()
    }

    /// Asks the terminal to mark pasted text (Unix): a pasted "1" then reads
    /// as a paste, not as a key. Best effort; written to the terminal device
    /// itself, never through the stdio handles `main` holds locked.
    fn bracket_pastes(on: bool) {
        #[cfg(unix)]
        {
            use std::io::Write;
            if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
                let sequence: &[u8] = if on { b"\x1b[?2004h" } else { b"\x1b[?2004l" };
                let _ = tty.write_all(sequence);
                let _ = tty.flush();
            }
        }
        #[cfg(not(unix))]
        let _ = on;
    }
}

impl KeySource for TerminalKeys {
    fn begin(&mut self) {
        self.presses.forget();
    }

    fn enter(&mut self) -> bool {
        if terminal::enable_raw_mode().is_err() {
            return false;
        }
        Self::bracket_pastes(true);
        true
    }

    fn leave(&mut self) {
        Self::bracket_pastes(false);
        let _ = terminal::disable_raw_mode();
    }

    fn drain(&mut self) -> Vec<Key> {
        let mut keys = Vec::new();
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read() {
                Ok(event) => keys.extend(self.presses.take(&event)),
                Err(_) => break,
            }
        }
        keys
    }

    fn poll(&mut self, wait: Duration) -> Result<Option<Key>, SourceGone> {
        let until = Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match event::poll(left) {
                Ok(false) => return Ok(None),
                Ok(true) => match event::read() {
                    // Releases, resizes and focus changes are not keys: keep
                    // waiting within the same budget.
                    Ok(event) => {
                        if let Some(key) = self.presses.take(&event) {
                            return Ok(Some(key));
                        }
                    }
                    Err(_) => return Err(SourceGone),
                },
                Err(_) => return Err(SourceGone),
            }
        }
    }
}

/// One step of what a test's human does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scripted {
    /// A key already typed when the question appears.
    Early(Key),
    /// A key pressed once the question can be read.
    Key(Key),
    /// A key that arrives together with the one before it.
    Then(Key),
    /// No key until the read is stopped from outside.
    Hold,
}

/// How often a source entered and left raw mode.
#[derive(Debug, Clone, Default)]
pub struct RawCounts {
    entered: Arc<AtomicUsize>,
    left: Arc<AtomicUsize>,
}

impl RawCounts {
    /// Times raw mode was entered.
    #[must_use]
    pub fn entered(&self) -> usize {
        self.entered.load(Ordering::SeqCst)
    }

    /// Times raw mode was left.
    #[must_use]
    pub fn left(&self) -> usize {
        self.left.load(Ordering::SeqCst)
    }
}

/// Keys scripted by a test.
#[derive(Debug)]
pub struct ScriptedKeys {
    script: VecDeque<Scripted>,
    counts: RawCounts,
}

impl ScriptedKeys {
    #[must_use]
    pub fn new(script: impl IntoIterator<Item = Scripted>) -> Self {
        Self {
            script: script.into_iter().collect(),
            counts: RawCounts::default(),
        }
    }

    /// The counts of raw-mode entries and exits, shared with the source.
    #[must_use]
    pub fn counts(&self) -> RawCounts {
        self.counts.clone()
    }
}

impl KeySource for ScriptedKeys {
    fn enter(&mut self) -> bool {
        self.counts.entered.fetch_add(1, Ordering::SeqCst);
        true
    }

    fn leave(&mut self) {
        // A hold lasts one read: the one it was stopped from.
        if self.script.front() == Some(&Scripted::Hold) {
            self.script.pop_front();
        }
        self.counts.left.fetch_add(1, Ordering::SeqCst);
    }

    fn drain(&mut self) -> Vec<Key> {
        let mut keys = Vec::new();
        while let Some(Scripted::Early(key) | Scripted::Then(key)) = self.script.front().copied() {
            keys.push(key);
            self.script.pop_front();
        }
        keys
    }

    fn poll(&mut self, wait: Duration) -> Result<Option<Key>, SourceGone> {
        match self.script.front().copied() {
            Some(Scripted::Key(key) | Scripted::Then(key)) => {
                self.script.pop_front();
                Ok(Some(key))
            }
            // An early key belongs to a question not yet shown, and a hold
            // waits to be stopped: nothing arrives now.
            Some(Scripted::Early(_) | Scripted::Hold) => {
                std::thread::sleep(wait);
                Ok(None)
            }
            None => Err(SourceGone),
        }
    }

    fn follows(&mut self, _within: Duration) -> Result<Option<Key>, SourceGone> {
        match self.script.front().copied() {
            Some(Scripted::Then(key)) => {
                self.script.pop_front();
                Ok(Some(key))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn read(script: &[Scripted]) -> (Read, ScriptedKeys) {
        let mut source = ScriptedKeys::new(script.iter().copied());
        let stop = AtomicBool::new(false);
        let read = read_answer(&mut source, &stop, &Arming::SCRIPTED);
        (read, source)
    }

    #[test]
    fn a_key_pressed_after_the_question_answers_it() {
        let (answer, source) = read(&[Scripted::Key(Key::Char('1'))]);
        assert_eq!(answer, Read::Key(Key::Char('1')));
        let counts = source.counts();
        assert_eq!((counts.entered(), counts.left()), (1, 1));
    }

    #[test]
    fn keys_typed_before_the_question_answer_nothing() {
        let (answer, source) = read(&[
            Scripted::Early(Key::Char('1')),
            Scripted::Key(Key::Char('2')),
        ]);
        assert_eq!(answer, Read::Early);
        // The key typed afterwards is still there for the next read.
        let mut source = source;
        let stop = AtomicBool::new(false);
        assert_eq!(
            read_answer(&mut source, &stop, &Arming::SCRIPTED),
            Read::Key(Key::Char('2'))
        );
        let counts = source.counts();
        assert_eq!((counts.entered(), counts.left()), (2, 2));
    }

    #[test]
    fn ctrl_c_counts_even_before_the_question_can_be_read() {
        let (answer, _) = read(&[Scripted::Early(Key::Char('x')), Scripted::Early(Key::CtrlC)]);
        assert_eq!(answer, Read::Key(Key::CtrlC));
    }

    #[test]
    fn keys_arriving_together_are_a_burst() {
        // A pasted "try": the `r` would remember and the `y` confirm.
        let (answer, source) = read(&[
            Scripted::Key(Key::Char('t')),
            Scripted::Then(Key::Char('r')),
            Scripted::Then(Key::Char('y')),
        ]);
        assert_eq!(answer, Read::Burst);
        assert!(source.script.is_empty(), "the whole burst is set aside");

        // A pasted "1" and its Enter.
        let (answer, _) = read(&[Scripted::Key(Key::Char('1')), Scripted::Then(Key::Enter)]);
        assert_eq!(answer, Read::Burst);

        // Ctrl+C inside a burst still stops.
        let (answer, _) = read(&[
            Scripted::Key(Key::Char('1')),
            Scripted::Then(Key::Char('2')),
            Scripted::Then(Key::CtrlC),
        ]);
        assert_eq!(answer, Read::Key(Key::CtrlC));
    }

    #[test]
    fn a_hold_waits_until_the_read_is_stopped_and_leaves_raw_mode() {
        let mut source = ScriptedKeys::new([Scripted::Hold, Scripted::Key(Key::Char('d'))]);
        let counts = source.counts();
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let read = read_answer(&mut source, &stop, &Arming::SCRIPTED);
                (read, source)
            })
        };
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(
            (counts.entered(), counts.left()),
            (1, 0),
            "held in raw mode"
        );
        stop.store(true, Ordering::SeqCst);
        let (read, mut source) = reader.join().expect("the reader ends");
        assert_eq!(read, Read::Stopped);
        assert_eq!((counts.entered(), counts.left()), (1, 1));
        // The hold served its read; the next one gets the next key.
        let stop = AtomicBool::new(false);
        assert_eq!(
            read_answer(&mut source, &stop, &Arming::SCRIPTED),
            Read::Key(Key::Char('d'))
        );
    }

    #[test]
    fn a_source_that_ends_or_refuses_fails_the_read() {
        let (answer, source) = read(&[]);
        assert_eq!(answer, Read::Failed);
        let counts = source.counts();
        assert_eq!(
            (counts.entered(), counts.left()),
            (1, 1),
            "left even on failure"
        );

        struct Refusing;
        impl KeySource for Refusing {
            fn enter(&mut self) -> bool {
                false
            }
            fn leave(&mut self) {
                panic!("never entered, never left");
            }
            fn drain(&mut self) -> Vec<Key> {
                Vec::new()
            }
            fn poll(&mut self, _: Duration) -> Result<Option<Key>, SourceGone> {
                Ok(None)
            }
        }
        let stop = AtomicBool::new(false);
        assert_eq!(
            read_answer(&mut Refusing, &stop, &Arming::SCRIPTED),
            Read::Failed
        );
    }

    #[test]
    fn the_dwell_holds_keys_back_until_the_question_can_be_read() {
        // With a real dwell, the read cannot return before it has passed.
        let mut source = ScriptedKeys::new([Scripted::Key(Key::Char('1'))]);
        let stop = AtomicBool::new(false);
        let arming = Arming {
            dwell: Duration::from_millis(60),
            ..Arming::SCRIPTED
        };
        let started = Instant::now();
        assert_eq!(
            read_answer(&mut source, &stop, &arming),
            Read::Key(Key::Char('1'))
        );
        assert!(started.elapsed() >= Duration::from_millis(60));
    }

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn release(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn terminal_events_translate_to_keys() {
        let mut presses = Presses::default();
        let none = KeyModifiers::NONE;
        let cases = [
            (press(KeyCode::Char('1'), none), Some(Key::Char('1'))),
            (
                press(KeyCode::Char('D'), KeyModifiers::SHIFT),
                Some(Key::Char('D')),
            ),
            (
                press(KeyCode::Char('c'), KeyModifiers::CONTROL),
                Some(Key::CtrlC),
            ),
            (
                press(KeyCode::Char('x'), KeyModifiers::CONTROL),
                Some(Key::Other),
            ),
            (
                press(KeyCode::Char('y'), KeyModifiers::ALT),
                Some(Key::Other),
            ),
            (press(KeyCode::Esc, none), Some(Key::Esc)),
            (press(KeyCode::Enter, none), Some(Key::Enter)),
            (press(KeyCode::Tab, none), Some(Key::Other)),
            (Event::Paste("1".into()), Some(Key::Paste)),
            (Event::Resize(80, 24), None),
            (Event::FocusGained, None),
        ];
        for (event, expected) in cases {
            assert_eq!(presses.take(&event), expected, "{event:?}");
            // Each key goes back up, so the next case starts clean.
            if let Event::Key(key) = event {
                presses.take(&release(key.code));
            }
        }
        let repeat = Event::Key(KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: none,
            kind: KeyEventKind::Repeat,
            state: KeyEventState::NONE,
        });
        assert_eq!(presses.take(&repeat), None, "a repeat is not a press");
        assert_eq!(presses.take(&release(KeyCode::Char('1'))), None);
    }

    #[test]
    fn a_held_key_counts_once_where_releases_are_reported() {
        let mut presses = Presses::default();
        let one = press(KeyCode::Char('1'), KeyModifiers::NONE);
        assert_eq!(presses.take(&one), Some(Key::Char('1')));
        // Windows reports a held key's repeats as presses with no release in
        // between; elsewhere the terminal reports no releases at all, so every
        // press is a press.
        let again = presses.take(&one);
        if cfg!(windows) {
            assert_eq!(again, None);
            presses.take(&release(KeyCode::Char('1')));
            assert_eq!(presses.take(&one), Some(Key::Char('1')));
            // A new question starts clean.
            presses.forget();
            assert_eq!(presses.take(&one), Some(Key::Char('1')));
        } else {
            assert_eq!(again, Some(Key::Char('1')));
        }
    }
}
