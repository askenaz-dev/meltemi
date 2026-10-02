// SPDX-License-Identifier: Apache-2.0

//! The app's own key pair (companero-movil design D3).
//!
//! **The app has a key; the user has their own and the app never touches it.**
//! This is §2 at the one place a phone makes it tempting to bend: the easy
//! version would be "paste your private key" or "read the one from that other
//! SSH app". Both are refused by construction — there is no code here that
//! imports a key, and none that reads one from anywhere but this app's own
//! storage.
//!
//! **Where the private half lives, said concretely.** Android's keystore does
//! not export private asymmetric keys, and an SSH library in user space needs
//! the key to sign. So the pair is generated here, and the private half is
//! stored encrypted at rest with a symmetric key held by the platform's
//! keystore — which is the real pattern, and not the comfortable sentence "the
//! key lives in the keystore". Signing through the keystore would need a JNI
//! bridge and a signing backend in the SSH library; that is not in this change,
//! and saying so is better than implying otherwise.
//!
//! This module owns the key's shape and lifecycle. Where the bytes are sealed
//! is the platform's business, behind [`Vault`], so the rule is testable on a
//! desktop and the Android implementation is the only thing that varies.

use russh::keys::{Algorithm, PrivateKey, PublicKeyBase64};

/// Where the sealed private key is kept between runs.
///
/// A trait, not a file path: on Android the bytes are sealed with keystore
/// material, and in a test they are a `Vec`. What must not vary is everything
/// above this line.
pub trait Vault: Send {
    /// The sealed key, if this app has made one.
    fn read(&self) -> Option<Vec<u8>>;
    /// Seals `key` for next time.
    ///
    /// # Errors
    ///
    /// When the platform refuses to seal it, which must stop the flow rather
    /// than fall back to writing it in the clear.
    fn write(&mut self, key: &[u8]) -> Result<(), String>;
}

/// A vault that keeps the bytes in memory. For tests, and never for a phone.
#[derive(Debug, Default)]
pub struct InMemory(Option<Vec<u8>>);

impl Vault for InMemory {
    fn read(&self) -> Option<Vec<u8>> {
        self.0.clone()
    }
    fn write(&mut self, key: &[u8]) -> Result<(), String> {
        self.0 = Some(key.to_vec());
        Ok(())
    }
}

/// The comment the public key carries into `authorized_keys`.
///
/// It names the app, so a year later the user reading that file knows what this
/// line is and can delete it on purpose. A key nobody can identify is a key
/// nobody dares remove.
pub const COMMENT: &str = "meltemi-mobile";

/// This app's key pair.
///
/// Deliberately not `Debug`: a key that can be printed is a key that ends up in
/// a log. The tests below match on the error rather than asking for this to be
/// printable, which is the cost of that and worth it.
pub struct AppKey {
    key: PrivateKey,
}

impl AppKey {
    /// Loads this app's key, making one the first time.
    ///
    /// ed25519 because it is small, fast on a phone, and every OpenSSH since
    /// 6.5 accepts it — and because the alternative, RSA, would mean a bigger
    /// key doing more work on the battery for no gain.
    ///
    /// # Errors
    ///
    /// When the key cannot be generated or sealed. A key that cannot be stored
    /// must stop here: generating a new one on every launch would ask the user
    /// to authorise a new line in `authorized_keys` every time.
    pub fn load_or_create(vault: &mut dyn Vault) -> Result<Self, String> {
        if let Some(sealed) = vault.read() {
            let text = String::from_utf8(sealed)
                .map_err(|_| "the stored key is not readable text".to_string())?;
            let key = PrivateKey::from_openssh(&text)
                .map_err(|e| format!("the stored key could not be read: {e}"))?;
            return Ok(Self { key });
        }
        // The system's randomness, reached through the crate that defines the
        // trait: `rand_core` 0.10 moved `OsRng` out, and borrowing a different
        // `rand_core` from elsewhere in the tree is how version skew starts.
        let mut rng =
            russh::keys::ssh_key::rand_core::UnwrapErr(russh::keys::ssh_key::getrandom::SysRng);
        let key = PrivateKey::random(&mut rng, Algorithm::Ed25519)
            .map_err(|e| format!("could not generate a key: {e}"))?;
        let sealed = key
            .to_openssh(russh::keys::ssh_key::LineEnding::LF)
            .map_err(|e| format!("could not serialise the key: {e}"))?;
        vault.write(sealed.as_bytes())?;
        Ok(Self { key })
    }

    /// The private key, for the SSH dialogue. Never leaves this process.
    #[must_use]
    pub fn private(&self) -> PrivateKey {
        self.key.clone()
    }

    /// The line to add to `authorized_keys`, which is the only half that leaves.
    #[must_use]
    pub fn authorized_line(&self) -> String {
        format!(
            "{} {} {COMMENT}",
            self.key.public_key().algorithm().as_str(),
            self.key.public_key().public_key_base64()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: La clave es de la app y de nadie más
    #[test]
    fn the_app_makes_its_own_key_once_and_keeps_it() {
        let mut vault = InMemory::default();
        let first = AppKey::load_or_create(&mut vault).expect("a key");
        let line = first.authorized_line();

        // Made once: a second load is the SAME key, or the user would have to
        // authorise a new line in `authorized_keys` on every launch.
        let again = AppKey::load_or_create(&mut vault).expect("the same key");
        assert_eq!(again.authorized_line(), line, "the key survives a restart");
    }

    #[test]
    fn what_leaves_is_the_public_half_and_it_names_this_app() {
        let mut vault = InMemory::default();
        let key = AppKey::load_or_create(&mut vault).expect("a key");
        let line = key.authorized_line();

        assert!(line.starts_with("ssh-ed25519 "), "{line}");
        assert!(
            line.ends_with(COMMENT),
            "the line names the app so it can be deleted on purpose: {line}"
        );
        // And nothing of the private half is in it. Asserted by absence, which
        // is the only way it stays true when somebody edits this tomorrow.
        for secret in ["PRIVATE", "BEGIN OPENSSH", "-----"] {
            assert!(!line.contains(secret), "`{secret}` in {line}");
        }
    }

    #[test]
    fn a_key_that_cannot_be_sealed_stops_rather_than_falling_back() {
        struct Refuses;
        impl Vault for Refuses {
            fn read(&self) -> Option<Vec<u8>> {
                None
            }
            fn write(&mut self, _: &[u8]) -> Result<(), String> {
                Err("the keystore said no".into())
            }
        }
        let said = match AppKey::load_or_create(&mut Refuses) {
            Err(said) => said,
            Ok(_) => panic!("a key that cannot be sealed must not be handed back"),
        };
        assert!(said.contains("keystore said no"), "{said}");
        // The failure that matters is the one that does NOT happen: no path
        // here writes the key in the clear when sealing fails.
    }

    #[test]
    fn a_stored_key_that_is_rubbish_is_refused_and_not_silently_replaced() {
        // Replacing it silently would change the app's identity without the
        // user knowing, and their `authorized_keys` line would stop working
        // with no explanation.
        struct Rubbish;
        impl Vault for Rubbish {
            fn read(&self) -> Option<Vec<u8>> {
                Some(b"not a key".to_vec())
            }
            fn write(&mut self, _: &[u8]) -> Result<(), String> {
                panic!("must not write over a key it could not read")
            }
        }
        let said = match AppKey::load_or_create(&mut Rubbish) {
            Err(said) => said,
            Ok(_) => panic!("unreadable stored key must not be replaced silently"),
        };
        assert!(said.contains("could not be read"), "{said}");
    }
}
