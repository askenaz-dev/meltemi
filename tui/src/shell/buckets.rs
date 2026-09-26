// SPDX-License-Identifier: Apache-2.0

//! How a project's sessions are ordered by what they ask of you
//! (sesiones-en-la-barra design D1, D6).
//!
//! This is the terminal's own copy of the table the desktop keeps in
//! `desktop/ui/src/lib/tree.ts`. TypeScript and Rust share no function, so what
//! keeps the two from drifting is not a shared module but a pin: a test reads
//! BOTH files and fails if the order of the buckets or the states inside any of
//! them stop matching. The copy is deliberate; the divergence is not allowed.

use meltemi_proto::SessionState;

use crate::shell::glyphs::{self, Glyph};
use crate::shell::messages::{Lang, Msg, text};

/// One bucket: what the sessions in it are asking of you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Waiting on your decision.
    Decision,
    /// Ready for your instruction.
    Instruction,
    /// Working.
    Working,
    /// Stopped.
    Stopped,
}

/// Signal order: what needs a human decision first, what is over last.
pub const BUCKET_ORDER: &[Bucket] = &[
    Bucket::Decision,
    Bucket::Instruction,
    Bucket::Working,
    Bucket::Stopped,
];

/// Which bucket a contract state falls into.
///
/// An exhaustive `match` on purpose: a state added to the contract does not
/// compile until it has been given a bucket, which is the same guarantee the
/// desktop gets from its `Record` over the state union.
#[must_use]
pub fn bucket_of(state: SessionState) -> Bucket {
    match state {
        SessionState::WaitingPermission => Bucket::Decision,
        SessionState::WaitingInstruction => Bucket::Instruction,
        SessionState::Active | SessionState::Starting => Bucket::Working,
        SessionState::Ended | SessionState::Interrupted => Bucket::Stopped,
    }
}

impl Bucket {
    /// The glyph of the bucket, with its ASCII twin, and its word. Taken from
    /// the state that defines the bucket, so a header and the rows under it
    /// cannot disagree.
    #[must_use]
    pub fn glyph(self) -> Glyph {
        match self {
            Bucket::Decision => glyphs::WAITING,
            Bucket::Instruction => glyphs::IDLE,
            Bucket::Working => glyphs::ACTIVE,
            Bucket::Stopped => glyphs::ENDED,
        }
    }

    /// The localized word of the bucket.
    #[must_use]
    pub fn word(self, lang: Lang) -> &'static str {
        text(
            match self {
                Bucket::Decision => Msg::BucketDecision,
                Bucket::Instruction => Msg::BucketInstruction,
                Bucket::Working => Msg::BucketWorking,
                Bucket::Stopped => Msg::BucketStopped,
            },
            lang,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_state_has_a_bucket_and_the_two_pairs_share_one() {
        // Six states, four buckets: `starting` joins `active`, and `interrupted`
        // joins `ended`. A state that fell out of every bucket would vanish from
        // the listing, which is why the mapping is exhaustive rather than a
        // lookup with a default.
        assert_eq!(bucket_of(SessionState::WaitingPermission), Bucket::Decision);
        assert_eq!(
            bucket_of(SessionState::WaitingInstruction),
            Bucket::Instruction
        );
        assert_eq!(bucket_of(SessionState::Active), Bucket::Working);
        assert_eq!(bucket_of(SessionState::Starting), Bucket::Working);
        assert_eq!(bucket_of(SessionState::Ended), Bucket::Stopped);
        assert_eq!(bucket_of(SessionState::Interrupted), Bucket::Stopped);
    }

    #[test]
    fn each_bucket_has_a_glyph_with_an_ascii_twin_and_a_word_in_both_languages() {
        for &bucket in BUCKET_ORDER {
            let glyph = bucket.glyph();
            assert!(!glyph.unicode.is_empty() && !glyph.ascii.is_empty());
            assert!(!bucket.word(Lang::Es).is_empty());
            assert!(!bucket.word(Lang::En).is_empty());
        }
    }
}
