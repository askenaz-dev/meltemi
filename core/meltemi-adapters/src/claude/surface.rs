// SPDX-License-Identifier: Apache-2.0

//! What the piloted CLI announces about itself, and the refusal when it is not
//! the surface this adapter is allowed to pilot (adaptadores-propios-acp tasks
//! 3.1 and 5.3, design D4).
//!
//! The pinned risk this module exists for: the provider's documentation
//! announces that the flag which **skips the sign-in and demands an API key**
//! will one day become the default of headless mode. If that flip arrives and
//! the adapter does not notice, the user's subscription session dies in silence
//! and their key starts paying instead — a degradation nobody asked for, which
//! is exactly what constitution §2 and the BYOK principle refuse.
//!
//! So the surface is read, never assumed, out of the initial event: the source
//! of the credentials in use. **When** it is read moved in task 5.3 and the
//! guard did not: the CLI says nothing until it has been given a turn, so the
//! event arrives on the first one — and the check runs there, before a single
//! event of that turn is mapped into the session. What the adapter does with
//! each answer:
//!
//! - **The signed-in session**: the turn runs.
//! - **Any other credential source**: the session is refused, with a diagnosis
//!   naming what was announced and a remedy. No credential is ever injected,
//!   and no alternative way in is tried — the refusal *is* the answer.
//! - **No announcement at all**: the turn runs and the silence is recorded.
//!   A field this adapter cannot see is not evidence of an API key, and
//!   refusing every session because a name moved would break precisely when the
//!   provider ships an improvement. What the guard prevents is a *silent*
//!   switch; a switch nobody can see is put on the record instead of guessed at
//!   in either direction.

use agent_client_protocol::schema::v1::Meta;
use serde_json::json;

use super::wire;
use crate::diagnostic::Refusal;

/// Where the piloted CLI says its credentials come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySource {
    /// The account the user already signed into: no key anywhere.
    SignedIn,
    /// A key-bearing source, named as the CLI named it.
    Key(String),
    /// The CLI announced nothing about it.
    Unannounced,
}

/// What the initial event turned out to announce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    /// Where the credentials come from.
    pub key_source: KeySource,
    /// The model the CLI said it would run, when it named one.
    pub model: Option<String>,
    /// The permission mode the CLI came up in.
    pub permission_mode: Option<String>,
}

impl Surface {
    /// Reads the surface out of the initial event.
    #[must_use]
    pub fn of(init: &wire::InitEvent) -> Self {
        let key_source = match init.api_key_source.as_deref() {
            None => KeySource::Unannounced,
            Some(source) if source == wire::SIGNED_IN_KEY_SOURCE => KeySource::SignedIn,
            Some(source) => KeySource::Key(source.to_string()),
        };
        Self {
            key_source,
            model: init.model.clone(),
            permission_mode: init.permission_mode.clone(),
        }
    }

    /// The refusal when this is not a surface the adapter may pilot.
    ///
    /// # Errors
    ///
    /// Refuses when the CLI announced a credential source that is not the
    /// signed-in session.
    pub fn check(&self, layer: &str) -> Result<(), Refusal> {
        match &self.key_source {
            KeySource::SignedIn | KeySource::Unannounced => Ok(()),
            KeySource::Key(source) => Err(refusal(layer, source)),
        }
    }

    /// What the session log should say about a surface that was not announced.
    ///
    /// `None` when there is nothing to note.
    #[must_use]
    pub fn note(&self) -> Option<String> {
        (self.key_source == KeySource::Unannounced).then(|| {
            "the CLI did not announce the source of its credentials; \
             the session was opened on the mode Meltemi asked for, \
             which is the signed-in one"
                .to_string()
        })
    }

    /// What the CLI announced, as ACP extension metadata.
    ///
    /// It rides ACP's own `_meta` (design D7) rather than any private channel,
    /// and it is a second update rather than part of the one the launch sends,
    /// because it is known at a different moment: which binary was launched is
    /// a fact at `session/new`, and which credential it came up on is a fact
    /// only once the CLI has spoken.
    #[must_use]
    pub fn meta(&self) -> Meta {
        let mut meta = Meta::new();
        meta.insert(
            "meltemi".into(),
            json!({
                "providerKeySource": self.key_source_label(),
                "providerModel": self.model,
                "providerPermissionMode": self.permission_mode,
            }),
        );
        meta
    }

    /// How the announced credential source reads in the session log.
    fn key_source_label(&self) -> String {
        match &self.key_source {
            KeySource::SignedIn => wire::SIGNED_IN_KEY_SOURCE.to_string(),
            KeySource::Key(source) => source.clone(),
            KeySource::Unannounced => "unannounced".to_string(),
        }
    }
}

/// The refusal for a CLI that will run on a key instead of the signed-in
/// session.
///
/// It names what was announced, because a human whose subscription just stopped
/// being used deserves to be told which credential took over, and it says in as
/// many words that Meltemi will not supply one — the remedy is the user's
/// session, never a key this process went looking for.
#[must_use]
pub fn refusal(layer: &str, announced: &str) -> Refusal {
    Refusal::new(
        "provider_surface_not_signed_in",
        layer,
        format!(
            "the CLI announced that it would run on credentials from `{announced}` \
             instead of the session you signed into"
        ),
        "Sign in to the provider's CLI and run it once by hand to confirm the session works; \
         if an API key variable is set in the environment Meltemi was launched from, that is \
         what the CLI picked up. Meltemi will not supply a credential of its own."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init(source: Option<&str>) -> wire::InitEvent {
        wire::InitEvent {
            api_key_source: source.map(str::to_string),
            model: Some("mock-sonnet".into()),
            permission_mode: Some("default".into()),
        }
    }

    #[test]
    fn the_signed_in_session_is_the_one_surface_that_opens() {
        // Scenario: Rehúso ante modo que exige clave de API
        let signed_in = Surface::of(&init(Some(wire::SIGNED_IN_KEY_SOURCE)));
        assert_eq!(signed_in.key_source, KeySource::SignedIn);
        assert!(signed_in.check("the official CLI").is_ok());
        assert!(signed_in.note().is_none());
    }

    #[test]
    fn a_key_bearing_surface_is_refused_by_name_with_a_remedy() {
        // Scenario: Rehúso ante modo que exige clave de API
        //
        // The pinned risk, exercised: the CLI comes up on a key instead of the
        // session the user signed into. The adapter refuses and says which
        // credential took over — it never quietly runs the turn anyway, and it
        // has no credential of its own to offer.
        let surface = Surface::of(&init(Some("ANTHROPIC_API_KEY")));
        assert_eq!(
            surface.key_source,
            KeySource::Key("ANTHROPIC_API_KEY".into())
        );
        let refusal = surface
            .check("the official `claude` CLI")
            .expect_err("a key-bearing surface is not the one this adapter pilots");
        assert_eq!(refusal.kind, "provider_surface_not_signed_in");
        assert_eq!(refusal.layer, "the official `claude` CLI");
        assert!(
            refusal.detail.contains("ANTHROPIC_API_KEY"),
            "the refusal names what took over: {}",
            refusal.detail
        );
        assert!(
            refusal.remedy.contains("will not supply"),
            "and says Meltemi supplies nothing of its own: {}",
            refusal.remedy
        );
    }

    #[test]
    fn a_surface_that_announces_nothing_opens_and_is_put_on_the_record() {
        // A field this adapter cannot see is not evidence of a key. Refusing
        // every session because a name moved would break exactly when the
        // provider ships an improvement — so the silence is recorded, which is
        // the one thing that is neither a guess nor a lie.
        let quiet = Surface::of(&init(None));
        assert_eq!(quiet.key_source, KeySource::Unannounced);
        assert!(quiet.check("the official CLI").is_ok());
        assert!(
            quiet
                .note()
                .expect("the silence is noted")
                .contains("did not announce")
        );
        assert_eq!(
            quiet.meta()["meltemi"]["providerKeySource"],
            "unannounced",
            "and the log says so in as many words rather than leaving a gap"
        );
    }

    #[test]
    fn what_the_cli_announced_travels_as_acp_extension_metadata() {
        // Scenario: Versión efectiva registrada en el log
        let meta = Surface::of(&init(Some(wire::SIGNED_IN_KEY_SOURCE))).meta();
        assert_eq!(meta["meltemi"]["providerKeySource"], "none");
        assert_eq!(meta["meltemi"]["providerModel"], "mock-sonnet");
        assert_eq!(meta["meltemi"]["providerPermissionMode"], "default");
    }
}
