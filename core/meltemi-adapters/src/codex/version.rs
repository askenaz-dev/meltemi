// SPDX-License-Identifier: Apache-2.0

//! The piloted CLI's version, and the refusal when it is not one this adapter
//! was verified against (adaptadores-propios-acp task 2.2, design D6;
//! re-anchored by codex-al-dia, design D1).
//!
//! The server's handshake answers no version field: it answers a user agent,
//! and the CLI's own version is its FIRST token, `<product>/<version>`. That is
//! the only version the handshake offers, so that is the one the skew check
//! reads.
//!
//! Which product that token names changed under this adapter's feet. Up to
//! 0.80 the CLI always wrote its own name, `codex_cli_rs`. Since 0.81 the
//! server writes the name the CLIENT introduced itself with in `initialize`
//! (`clientInfo.name`), so the handshake of 0.156.1 answers
//! `meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64) xterm-256color
//! (meltemi-codex-acp; 0.1.1)`. Both forms are read; nothing else is:
//!
//! - only the first token, because later ones may carry a version of their own
//!   (the terminal's, `vscode/1.95.3`) and a product name the client chose must
//!   never make one of those pass for the CLI's;
//! - never by introducing the adapter as somebody else. The client's name is
//!   the `originator` the CLI reports to its own backend, and presenting
//!   ourselves as a first-party client to get the old form back would be
//!   impersonating it (constitution §2).
//!
//! Reading it can fail, and when it does the adapter **refuses**. A version it
//! cannot read is not a version it may assume: the whole point of a per-version
//! schema dump is that compatibility is checked, never presumed.

use super::wire;
use crate::diagnostic::Refusal;

/// A `(major, minor, patch)` version.
pub type Version = (u64, u64, u64);

/// The CLI's own version, read out of the first token of the user agent it
/// announced, when that token names the CLI (`codex_cli_rs`) or the client the
/// adapter introduced itself as (`announced`).
///
/// Anything after the patch number — a pre-release or build suffix — is
/// ignored: it does not change which wire is being spoken.
#[must_use]
pub fn of(user_agent: &str, announced: &str) -> Option<Version> {
    let first = user_agent.split_whitespace().next()?;
    let (product, token) = first.split_once('/')?;
    if product != wire::USER_AGENT_PRODUCT && product != announced {
        return None;
    }
    parse(token)
}

/// A version as the CLI writes it, `<major>.<minor>.<patch>`, with any
/// pre-release or build suffix ignored.
#[must_use]
pub fn parse(text: &str) -> Option<Version> {
    let core = text
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

/// Whether a version inside the range is newer than the one this adapter was
/// verified against — the floor, which is the version of the vendored dump.
/// Such a version opens, declaring the distance, and is never presented as
/// verified (codex-al-dia design D2).
#[must_use]
pub fn beyond_verified(version: Version) -> bool {
    version > wire::SUPPORTED_FLOOR
}

/// Checks the version the thread says created it against the one the
/// handshake read (codex-al-dia design D1).
///
/// A thread that does not say is not contradicting anything. One that says
/// something else — or something that cannot be read as a version — was not
/// made by the CLI whose version was checked, and the session is refused.
///
/// # Errors
///
/// Refuses, naming both versions, when they differ.
pub fn contrast(layer: &str, read: &str, thread_cli: Option<&str>) -> Result<(), Refusal> {
    let Some(declared) = thread_cli else {
        return Ok(());
    };
    if parse(declared).map(render).as_deref() == Some(read) {
        return Ok(());
    }
    Err(Refusal::new(
        "provider_version_contradicted",
        layer,
        format!(
            "the handshake read version {read} from the user agent, and the thread it opened \
             says it was created by version `{declared}`; the CLI answering is not the one whose \
             version was checked"
        ),
        "Check that one Codex CLI answers where Meltemi launches it (`meltemi fleet` shows the \
         effective binary), and open the session again.",
    ))
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
///
/// It says what was seen and the two forms that would have been read, and it
/// does not blame the binary: the likeliest reasons are a CLI whose user agent
/// changed shape again, or an environment that overrides the name the CLI
/// announces — not an unofficial CLI.
#[must_use]
pub fn unreadable(layer: &str, user_agent: &str, announced: &str) -> Refusal {
    Refusal::new(
        "provider_version_unreadable",
        layer,
        format!(
            "the handshake announced `{user_agent}`; this adapter reads the CLI's version from the \
             first token, `{}/<version>` or `{announced}/<version>`, and found neither",
            wire::USER_AGENT_PRODUCT
        ),
        format!(
            "If `{}` is set in the environment Meltemi was started from, the CLI announces that \
             name instead of `{announced}`: start Meltemi without it. Otherwise this CLI speaks a \
             user agent this adapter was not verified against: update Meltemi, or tell the \
             project which CLI version answered this.",
            wire::ORIGINATOR_OVERRIDE_ENV
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

    /// The name the adapter introduces itself with, as the handshake sends it.
    const ANNOUNCED: &str = "meltemi-codex-acp";

    #[test]
    fn the_cli_writes_the_clients_name_in_its_user_agent_and_the_version_is_read() {
        // Scenario: El CLI escribe el nombre del cliente en su user agent
        //
        // Measured against the real CLI (0.156.1) on 2026-09-24: since 0.81 the
        // first token names the client, and the version beside it is the CLI's.
        assert_eq!(
            of(
                "meltemi-codex-acp/0.156.1 (Windows 10.0.26200; x86_64) xterm-256color (meltemi-codex-acp; 0.1.1)",
                ANNOUNCED
            ),
            Some((0, 156, 1))
        );
        // And the CLI's own name, which is what a CLI older than 0.81 still
        // writes (measured against 0.77.0), so that an old CLI is told to update
        // instead of being told its user agent is unreadable.
        assert_eq!(
            of(
                "codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) xterm-256color (meltemi-codex-acp; 0.1.0)",
                ANNOUNCED
            ),
            Some((0, 77, 0))
        );
        assert_eq!(
            of("codex_cli_rs/1.2.3-beta.1 (linux)", ANNOUNCED),
            Some((1, 2, 3))
        );
    }

    #[test]
    fn a_later_token_with_a_version_is_never_taken_for_the_clis() {
        // Scenario: Un token posterior con versión no se confunde con el CLI
        //
        // The terminal token carries a version of its own. Only the first token
        // is the CLI's; a later one is never read, whatever it says.
        assert_eq!(
            of(
                "meltemi-codex-acp/0.156.1 (Linux 6.8; x86_64) vscode/1.95.3 (meltemi-codex-acp; 0.1.1)",
                ANNOUNCED
            ),
            Some((0, 156, 1))
        );
        for announced in [
            "unknown/1.0.0 codex_cli_rs/0.156.1",
            "vscode/1.95.3 meltemi-codex-acp/0.156.1",
            "(meltemi-codex-acp; 0.1.1) meltemi-codex-acp/0.156.1",
        ] {
            assert_eq!(
                of(announced, ANNOUNCED),
                None,
                "`{announced}`: a product that is not accepted in first place is refused"
            );
        }
    }

    #[test]
    fn the_verified_version_is_the_floor_and_the_one_the_dump_came_from() {
        // Scenario: Versión posterior a la verificada declarada, no rehusada
        assert_eq!(
            parse(wire::VENDORED_SCHEMA_VERSION),
            Some(wire::SUPPORTED_FLOOR)
        );
        assert!(!beyond_verified(wire::SUPPORTED_FLOOR));
        assert!(beyond_verified((0, 156, 2)) && beyond_verified((0, 157, 0)));
        assert!(
            supported((0, 157, 0)),
            "newer and inside the range: it opens, declared, not refused"
        );
        assert_eq!(parse("0.157.0-alpha.2"), Some((0, 157, 0)));
        assert_eq!(parse("0.157"), None);
        assert_eq!(parse("0.157.0.1"), None);
    }

    #[test]
    fn a_thread_that_contradicts_the_version_read_is_refused_naming_both() {
        // Scenario: El hilo contradice la versión leída
        let layer = "the official `codex` CLI";
        assert!(contrast(layer, "0.156.1", Some("0.156.1")).is_ok());
        assert!(
            contrast(layer, "0.157.0", Some("0.157.0-alpha.2")).is_ok(),
            "the same suffix-free version the user agent is read with"
        );
        assert!(
            contrast(layer, "0.156.1", None).is_ok(),
            "a thread that does not say contradicts nothing"
        );
        for declared in ["0.157.0", "0.77.0", "not-a-version"] {
            let refusal = contrast(layer, "0.156.1", Some(declared))
                .expect_err("another CLI answered than the one checked");
            assert_eq!(refusal.kind, "provider_version_contradicted");
            assert!(
                refusal.detail.contains("0.156.1") && refusal.detail.contains(declared),
                "the diagnosis names both: {}",
                refusal.detail
            );
            assert!(!refusal.detail.contains("  ") && !refusal.remedy.contains("  "));
        }
    }

    #[test]
    fn a_user_agent_no_version_can_be_read_from_is_refused_not_guessed() {
        // Scenario: Una forma desconocida se rehúsa sin culpar al binario
        for announced in [
            "codex_cli_rs",
            "codex_cli_rs/",
            "codex_cli_rs/0.77",
            "codex_cli_rs/0.77.0.1",
            "codex_cli_rs/nightly",
            // The CLI degrades its user agent to the bare name when it cannot
            // build a valid header; with no version there is nothing to read.
            "meltemi-codex-acp",
            "meltemi-codex-acp/",
            "some-other-cli/9.9.9",
            // What an overridden originator looks like: a first-party name the
            // adapter never sent, and so never reads.
            "codex_vscode/0.156.1 (Windows 10.0.26200; x86_64)",
            "",
        ] {
            assert_eq!(
                of(announced, ANNOUNCED),
                None,
                "`{announced}` carries no version"
            );
        }

        let refusal = unreadable(
            "the official `codex` CLI",
            "codex_vscode/0.156.1 (Windows 10.0.26200; x86_64)",
            ANNOUNCED,
        );
        assert_eq!(refusal.kind, "provider_version_unreadable");
        assert!(refusal.detail.contains("codex_vscode/0.156.1"));
        assert!(
            refusal.detail.contains("codex_cli_rs/<version>")
                && refusal.detail.contains("meltemi-codex-acp/<version>"),
            "the refusal says which two forms would have been read: {}",
            refusal.detail
        );
        assert!(
            refusal.remedy.contains(wire::ORIGINATOR_OVERRIDE_ENV),
            "and names the one thing in the environment that changes the form: {}",
            refusal.remedy
        );
        assert!(
            !refusal.remedy.contains("official"),
            "it does not blame the binary for a user agent it could not read: {}",
            refusal.remedy
        );
        for text in [&refusal.detail, &refusal.remedy] {
            assert!(
                !text.contains("  "),
                "one space between words, as a human reads it: {text}"
            );
        }
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
