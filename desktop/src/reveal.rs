// SPDX-License-Identifier: Apache-2.0

//! The main window is created hidden and shown once the interface exists
//! (senales-de-vida design D6): on the interface's signal, sent right after it
//! mounts its first tree, or at a cap if the signal never arrives — whichever
//! comes first, and only once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The longest the window stays hidden when the interface never signals: an
/// interface that fails to start must not leave an invisible window behind.
pub const REVEAL_CAP: Duration = Duration::from_millis(500);

/// The background the interface paints in each theme: the `--bg` token of
/// `desktop/ui/src/app.css`, which a test pins to these values.
pub const BACKGROUND_LIGHT: (u8, u8, u8) = (0xf8, 0xfa, 0xfc);
pub const BACKGROUND_DARK: (u8, u8, u8) = (0x0b, 0x12, 0x20);

/// The background the window wears before the interface has painted: the
/// theme the user chose, or the system's when they chose none. Whenever the
/// window is shown — on the signal or at the cap — a webview that has not
/// painted yet shows this and not a white the theme never had.
pub fn background(theme: Option<&str>, system_dark: bool) -> (u8, u8, u8) {
    match theme {
        Some("dark") => BACKGROUND_DARK,
        Some("light") => BACKGROUND_LIGHT,
        _ if system_dark => BACKGROUND_DARK,
        _ => BACKGROUND_LIGHT,
    }
}

/// The gate that lets exactly one caller show the window.
#[derive(Debug, Default)]
pub struct Reveal(AtomicBool);

impl Reveal {
    /// Whether this call is the first, which is the one that shows the window.
    /// Every later call — the cap after the signal, or a late signal after
    /// the cap — finds the gate already passed.
    pub fn first(&self) -> bool {
        !self.0.swap(true, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: Una interfaz muda no deja la ventana invisible
    #[test]
    fn the_window_is_shown_once_whoever_arrives_first() {
        // The cap arrives first: it shows the window, and the late signal does
        // not show it again.
        let reveal = Reveal::default();
        assert!(reveal.first(), "the first caller shows the window");
        assert!(!reveal.first(), "a late signal finds it already shown");
        assert!(!reveal.first(), "and so does any other");

        // Concurrent callers — the signal and the cap racing — still yield one.
        let reveal = std::sync::Arc::new(Reveal::default());
        let winners: usize = (0..8)
            .map(|_| {
                let reveal = reveal.clone();
                std::thread::spawn(move || usize::from(reveal.first()))
            })
            .map(|handle| handle.join().expect("a caller finishes"))
            .sum();
        assert_eq!(winners, 1, "exactly one caller shows the window");

        assert_eq!(REVEAL_CAP, Duration::from_millis(500));
    }

    // Scenario: La ventana nunca enseña un fondo ajeno al tema
    #[test]
    fn the_window_wears_the_theme_before_the_interface_paints() {
        // The theme the user chose wins; with none, the system's decides.
        assert_eq!(background(Some("dark"), false), BACKGROUND_DARK);
        assert_eq!(background(Some("light"), true), BACKGROUND_LIGHT);
        assert_eq!(background(Some("system"), true), BACKGROUND_DARK);
        assert_eq!(background(None, false), BACKGROUND_LIGHT);

        // And the two colors are the interface's own `--bg`, read from the
        // stylesheet rather than trusted: a token that moves without them
        // would bring the flash back.
        let css = include_str!("../ui/src/app.css");
        let hex = |(r, g, b): (u8, u8, u8)| format!("--bg: #{r:02x}{g:02x}{b:02x};");
        let light = hex(BACKGROUND_LIGHT);
        let dark = hex(BACKGROUND_DARK);
        let root = css.split(":root {").nth(1).expect("the light tokens");
        assert!(
            root.split('}')
                .next()
                .is_some_and(|block| block.contains(&light)),
            "the light background is {light}"
        );
        let forced = css
            .split(":root[data-theme=\"dark\"] {")
            .nth(1)
            .expect("the dark tokens");
        assert!(
            forced
                .split('}')
                .next()
                .is_some_and(|block| block.contains(&dark)),
            "the dark background is {dark}"
        );
    }
}
