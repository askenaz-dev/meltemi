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
}
