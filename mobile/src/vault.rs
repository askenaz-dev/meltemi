// SPDX-License-Identifier: Apache-2.0

//! The companion's vault (companero-que-conecta D5): the sealed private key on
//! disk, opened and sealed through the platform's keystore.
//!
//! The keystore is asked to open the key when the vault is opened, not when
//! somebody reads it: a keystore that refuses says so here, with its own
//! words, where it cannot be mistaken for "no key yet" — which is the one
//! mistake that would mint a second key and ask the user to authorise it
//! again.

use std::path::{Path, PathBuf};

use tauri_plugin_meltemi_vault::Sealer;

use crate::keys::Vault;

/// The file holding the sealed key.
pub const FILE: &str = "key.sealed";

/// The sealed key on disk, with the keystore to seal and open it.
pub struct Sealed<'a> {
    path: PathBuf,
    sealer: &'a dyn Sealer,
    plain: Option<Vec<u8>>,
}

impl<'a> Sealed<'a> {
    /// Opens the vault in `dir`, asking the keystore to open the key now if
    /// there is one.
    ///
    /// # Errors
    ///
    /// When the file is there and cannot be read, or the keystore refuses to
    /// open it. Neither reads as "no key": a refusal stops here.
    pub fn open(dir: &Path, sealer: &'a dyn Sealer) -> Result<Self, String> {
        let path = dir.join(FILE);
        let plain = match std::fs::read(&path) {
            Ok(sealed) => Some(sealer.open(&sealed)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("could not read {}: {error}", path.display())),
        };
        Ok(Self {
            path,
            sealer,
            plain,
        })
    }
}

impl Vault for Sealed<'_> {
    fn read(&self) -> Option<Vec<u8>> {
        self.plain.clone()
    }

    fn write(&mut self, key: &[u8]) -> Result<(), String> {
        let sealed = self.sealer.seal(key)?;
        let staged = self.path.with_extension("tmp");
        std::fs::write(&staged, &sealed)
            .map_err(|e| format!("could not write {}: {e}", staged.display()))?;
        std::fs::rename(&staged, &self.path)
            .map_err(|e| format!("could not write {}: {e}", self.path.display()))?;
        self.plain = Some(key.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A keystore for tests: it marks what it sealed so the file is visibly
    /// not the key, and it opens only what it sealed.
    struct Marking;

    impl Sealer for Marking {
        fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
            let mut sealed = b"sealed:".to_vec();
            sealed.extend(plain.iter().map(|b| b ^ 0x5a));
            Ok(sealed)
        }
        fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, String> {
            let body = sealed
                .strip_prefix(b"sealed:")
                .ok_or_else(|| "not something this keystore sealed".to_string())?;
            Ok(body.iter().map(|b| b ^ 0x5a).collect())
        }
    }

    /// A keystore that refuses everything.
    struct Refusing;

    impl Sealer for Refusing {
        fn seal(&self, _: &[u8]) -> Result<Vec<u8>, String> {
            Err("no keystore here".into())
        }
        fn open(&self, _: &[u8]) -> Result<Vec<u8>, String> {
            Err("no keystore here".into())
        }
    }

    struct Dir(PathBuf);

    impl Dir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "meltemi-companion-vault-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a test directory");
            Self(path)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Scenario: La clave es de la app y de nadie más
    #[test]
    fn the_key_on_disk_is_sealed_and_a_second_run_opens_the_same_one() {
        let dir = Dir::new("round-trip");
        let mut vault = Sealed::open(&dir.0, &Marking).expect("no key yet");
        assert_eq!(vault.read(), None);
        vault
            .write(b"-----BEGIN KEY-----")
            .expect("sealed and written");
        assert_eq!(vault.read().as_deref(), Some(&b"-----BEGIN KEY-----"[..]));

        let on_disk = std::fs::read(dir.0.join(FILE)).expect("the file is there");
        assert!(
            on_disk.starts_with(b"sealed:"),
            "what is on disk is the sealed form"
        );
        assert!(
            !on_disk.windows(5).any(|w| w == b"BEGIN"),
            "and never the key in the clear"
        );

        let again = Sealed::open(&dir.0, &Marking).expect("opened by the keystore");
        assert_eq!(again.read().as_deref(), Some(&b"-----BEGIN KEY-----"[..]));
    }

    #[test]
    fn a_keystore_that_refuses_to_seal_stops_the_flow_and_writes_nothing() {
        let dir = Dir::new("refuses");
        let mut vault = Sealed::open(&dir.0, &Refusing).expect("nothing to open yet");
        let error = vault
            .write(b"a key")
            .expect_err("the refusal is the caller's to stop on");
        assert!(error.contains("no keystore"), "{error}");
        assert!(
            !dir.0.join(FILE).exists(),
            "nothing was written in the clear"
        );
        assert_eq!(vault.read(), None);
    }

    #[test]
    fn a_sealed_key_the_keystore_cannot_open_is_a_refusal_and_not_a_fresh_start() {
        let dir = Dir::new("cannot-open");
        std::fs::write(dir.0.join(FILE), b"sealed by some other device").expect("written");
        let error = match Sealed::open(&dir.0, &Marking) {
            Ok(_) => panic!("an unopenable key must not read as 'no key yet'"),
            Err(error) => error,
        };
        assert!(
            error.contains("not something this keystore sealed"),
            "{error}"
        );
    }
}
