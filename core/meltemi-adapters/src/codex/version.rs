// SPDX-License-Identifier: Apache-2.0

//! The piloted CLI's version, and the refusal when it is not one this adapter
//! was verified against (adaptadores-propios-acp task 2.2, design D6).
//!
//! The server's handshake answers no version field: it answers a user agent —
//! `codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) … (client; version)` —
//! and the CLI's own version is the token inside it. That is the only version
//! the handshake offers, so that is the one the skew check reads.
//!
//! Reading it can fail, and when it does the adapter **refuses**. A version it
//! cannot read is not a version it may assume: the whole point of a per-version
//! schema dump is that compatibility is checked, never presumed.

use super::wire;
use crate::diagnostic::Refusal;

/// A `(major, minor, patch)` version.
pub type Version = (u64, u64, u64);

/// The CLI's own version, read out of the user agent it announced.
///
/// Anything after the patch number — a pre-release or build suffix — is
/// ignored: it does not change which wire is being spoken.
#[must_use]
pub fn of(user_agent: &str) -> Option<Version> {
    let token = user_agent
        .split_whitespace()
        .find_map(|token| token.strip_prefix(&format!("{}/", wire::USER_AGENT_PRODUCT)))?;
    let core = token
        .split(['-', '+'])
        .next()
        .filter(|core| !core.is_empty())?;
    let mut parts = core.split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let version = (next()?, next()?, next()?);
    // A fourth component would mean this is not the versioning this adapter
    // was verified against, and guessing at it is exactly what the refusal is
    // for.
    parts.next().is_none().then_some(version)
}

/// Whether the version falls inside the range this adapter declares.
#[must_use]
pub fn supported(version: Version) -> bool {
    version >= wire::SUPPORTED_FLOOR && version < wire::SUPPORTED_CEILING
}

/// Renders a version the way a human reads one.
#[must_use]
pub fn render(version: Version) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

/// The refusal when the handshake announced something no version can be read
/// from.
#[must_use]
pub fn unreadable(layer: &str, user_agent: &str) -> Refusal {
    Refusal::new(
        "provider_version_unreadable",
        layer,
        format!(
            "the handshake announced `{user_agent}`, which carries no version this adapter can read"
        ),
        format!(
            "Check that the CLI is the official one and that it was started in its documented \
             `{}` mode; this adapter reads the version from a `{}/<version>` token.",
            wire::SERVER_MODE_ARG,
            wire::USER_AGENT_PRODUCT
        ),
    )
}

/// The refusal for a CLI outside the supported range, naming both versions and
/// what to update.
///
/// Which side is behind decides the remedy: an old CLI is updated, a CLI newer
/// than anything this adapter was verified against means Meltemi is the one
/// that has to catch up. Saying "versions differ" and leaving the human to
/// guess which is which would be no diagnosis at all.
#[must_use]
pub fn skew(layer: &str, found: Version) -> Refusal {
    let floor = render(wire::SUPPORTED_FLOOR);
    let ceiling = render(wire::SUPPORTED_CEILING);
    let remedy = if found < wire::SUPPORTED_FLOOR {
        format!("Update the provider CLI to {floor} or newer.")
    } else {
        format!(
            "Update Meltemi: this adapter has only been verified against the wire up to (not including) {ceiling}."
        )
    };
    Refusal::new(
        "provider_version_skew",
        layer,
        format!(
            "the CLI reports {}, and this adapter supports {floor} up to (not including) {ceiling}",
            render(found)
        ),
        remedy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_read_out_of_the_user_agent_the_cli_announces() {
        // Verified against the real CLI (0.77.0): the handshake answers exactly
        // this shape and nothing more structured.
        assert_eq!(
            of(
                "codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) xterm-256color (meltemi-codex-acp; 0.1.0)"
            ),
            Some((0, 77, 0))
        );
        assert_eq!(of("codex_cli_rs/1.2.3-beta.1 (linux)"), Some((1, 2, 3)));
    }

    #[test]
    fn a_user_agent_no_version_can_be_read_from_is_refused_not_guessed() {
        // Scenario: Desfase de versión rehusado con remedio
        for announced in [
            "codex_cli_rs",
            "codex_cli_rs/",
            "codex_cli_rs/0.77",
            "codex_cli_rs/0.77.0.1",
            "codex_cli_rs/nightly",
            "some-other-cli/9.9.9",
            "",
        ] {
            assert_eq!(of(announced), None, "`{announced}` carries no version");
        }

        let refusal = unreadable("the official `codex` CLI", "some-other-cli/9.9.9");
        assert_eq!(refusal.kind, "provider_version_unreadable");
        assert!(refusal.detail.contains("some-other-cli/9.9.9"));
        assert_eq!(
            refusal.remedy.matches(wire::SERVER_MODE_ARG).count(),
            1,
            "the remedy names the server mode once, not twice: {}",
            refusal.remedy
        );
    }

    #[test]
    fn a_version_outside_the_declared_range_is_refused_naming_both_and_what_to_update() {
        // Scenario: Desfase de versión rehusado con remedio
        assert!(supported(wire::SUPPORTED_FLOOR));
        assert!(!supported(wire::SUPPORTED_CEILING));

        let old = (0, 1, 0);
        assert!(!supported(old));
        let refusal = skew("the official `codex` CLI", old);
        assert_eq!(refusal.kind, "provider_version_skew");
        assert!(refusal.detail.contains(&render(old)), "{}", refusal.detail);
        assert!(
            refusal.detail.contains(&render(wire::SUPPORTED_FLOOR)),
            "the refusal names both versions: {}",
            refusal.detail
        );
        assert!(
            refusal.remedy.contains("provider CLI"),
            "an old CLI is what gets updated: {}",
            refusal.remedy
        );

        let ahead = (wire::SUPPORTED_CEILING.0 + 1, 0, 0);
        assert!(!supported(ahead));
        assert!(
            skew("the official `codex` CLI", ahead)
                .remedy
                .contains("Update Meltemi"),
            "a CLI ahead of the adapter is Meltemi's turn to catch up"
        );
    }
}
