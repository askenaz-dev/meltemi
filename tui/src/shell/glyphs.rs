// SPDX-License-Identifier: Apache-2.0

//! The single glyph table with an ASCII twin for every symbol (design D4).
//!
//! The invariant "ningún glifo Unicode sin gemelo ASCII" is enforced by the
//! `every_glyph_has_an_ascii_twin` test: adding a symbol without a twin fails
//! the build.

use crate::shell::present::Presentation;

/// A symbol with a Unicode form and its ASCII twin. Meaning must never depend on
/// which form is shown, nor on color.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub unicode: &'static str,
    pub ascii: &'static str,
}

impl Glyph {
    /// The text to render given the presentation policy.
    #[must_use]
    pub fn text(&self, present: &Presentation) -> &'static str {
        if present.unicode {
            self.unicode
        } else {
            self.ascii
        }
    }
}

/// Focus marker, shown next to the focused element (never color alone).
pub const FOCUS: Glyph = Glyph {
    unicode: "▸",
    ascii: ">",
};
/// Selection gutter marker.
pub const SELECT: Glyph = Glyph {
    unicode: "›",
    ascii: ">",
};
/// Pending-permission badge, always paired with a count and a word.
pub const PERMISSION: Glyph = Glyph {
    unicode: "!",
    ascii: "!",
};
/// Daemon reachable / connected; also a detected fleet entry.
pub const OK: Glyph = Glyph {
    unicode: "●",
    ascii: "*",
};
/// A fleet entry not detected on this system (hollow twin of [`OK`]).
pub const ABSENT: Glyph = Glyph {
    unicode: "○",
    ascii: "o",
};
/// Daemon unreachable / error.
pub const ERROR: Glyph = Glyph {
    unicode: "✕",
    ascii: "x",
};
/// Transient / connecting.
pub const PENDING: Glyph = Glyph {
    unicode: "…",
    ascii: "...",
};
/// Session state: starting.
pub const STARTING: Glyph = Glyph {
    unicode: "◔",
    ascii: "~",
};
/// Session state: active.
pub const ACTIVE: Glyph = Glyph {
    unicode: "▶",
    ascii: ">",
};
/// Session state: waiting on a permission.
pub const WAITING: Glyph = Glyph {
    unicode: "‖",
    ascii: "!",
};
/// Session state: ended.
pub const ENDED: Glyph = Glyph {
    unicode: "✓",
    ascii: "x",
};

/// Every glyph in the table, for the twin-invariant test and audits.
pub const ALL: &[Glyph] = &[
    FOCUS, SELECT, PERMISSION, OK, ABSENT, ERROR, PENDING, STARTING, ACTIVE, WAITING, ENDED,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::present::{Presentation, PresentationEnv};

    #[test]
    fn every_glyph_has_an_ascii_twin() {
        // Invariante verificable: ningún glifo Unicode sin gemelo ASCII.
        for g in ALL {
            assert!(
                !g.ascii.is_empty(),
                "glyph {:?} has an empty ASCII twin",
                g.unicode
            );
            assert!(
                g.ascii.is_ascii(),
                "the ASCII twin of {:?} is not ASCII: {:?}",
                g.unicode,
                g.ascii
            );
        }
    }

    #[test]
    fn ascii_presentation_uses_the_twin() {
        let ascii = Presentation::resolve(&PresentationEnv {
            ascii_flag: true,
            ..Default::default()
        });
        assert_eq!(FOCUS.text(&ascii), ">");
        let unicode = Presentation::resolve(&PresentationEnv {
            lang: Some("en_US.UTF-8".into()),
            ..Default::default()
        });
        assert_eq!(FOCUS.text(&unicode), "▸");
    }
}
