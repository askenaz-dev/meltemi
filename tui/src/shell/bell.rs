// SPDX-License-Identifier: Apache-2.0

//! The terminal bell, asked for and never imposed (avisos-de-escritorio design
//! D4). A bell that rings without being asked is the reason people turn bells
//! off, so this one is silent until `MELTEMI_BELL` says otherwise — the same
//! shape `MELTEMI_ASCII` already uses for presentation.
//!
//! Pure: `decide` takes what changed and answers whether to ring. Nothing here
//! writes to a terminal.

use meltemi_proto::SessionState;

use crate::shell::live::{GateRow, Update};

/// Whether the bell is on, given the environment's value.
///
/// Absent, empty or `0` means off. Anything else means on: the user who typed
/// `MELTEMI_BELL=1` asked for it, and guessing at spellings would only produce
/// a bell that ignores half the people who asked.
pub fn enabled(value: Option<&str>) -> bool {
    !matches!(value, None | Some("") | Some("0"))
}

/// Whether this update is one of the moments a person is needed.
///
/// The same three the desktop announces, so the two surfaces do not disagree
/// about what deserves attention. A gate that goes away (`None`) is not a
/// moment, and neither is a queue that empties.
pub fn rings(update: &Update, previous_gate: Option<&GateRow>) -> bool {
    match update {
        Update::Pending(n) => *n > 0,
        // Only the arrival, never the repaint: the same gate reported twice is
        // one moment, and a bell that repeats is a bell that gets silenced.
        Update::Gate(Some(gate)) => previous_gate != Some(gate),
        Update::Sessions(rows) => rows
            .iter()
            .any(|row| matches!(row.state, SessionState::Ended | SessionState::Interrupted)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: Sin activar, el terminal no suena
    #[test]
    fn the_bell_is_silent_until_it_is_asked_for() {
        assert!(!enabled(None), "absent means off");
        assert!(!enabled(Some("")), "empty means off");
        assert!(!enabled(Some("0")), "zero means off");
        assert!(enabled(Some("1")));
        assert!(enabled(Some("true")), "whoever typed a word asked for it");
    }

    // Scenario: Activada, el terminal suena en los mismos momentos
    #[test]
    fn the_bell_rings_for_the_moments_a_person_is_needed() {
        assert!(rings(&Update::Pending(1), None));
        assert!(
            !rings(&Update::Pending(0), None),
            "an empty queue is not a moment"
        );

        let gate = GateRow {
            change: "c".into(),
            artifact: "specs".into(),
        };
        assert!(
            rings(&Update::Gate(Some(gate.clone())), None),
            "a gate arriving rings"
        );
        assert!(
            !rings(&Update::Gate(Some(gate.clone())), Some(&gate)),
            "the same gate reported again does not"
        );
        assert!(
            !rings(&Update::Gate(None), Some(&gate)),
            "a gate going away is not a moment"
        );
    }
}
