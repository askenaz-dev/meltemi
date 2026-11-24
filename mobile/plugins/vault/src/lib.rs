// SPDX-License-Identifier: Apache-2.0

//! The companion's vault: bytes sealed with a key that never leaves Android's
//! keystore (companero-que-conecta D5).
//!
//! Two operations and nothing else — seal and open — with an AES-256-GCM key
//! the keystore holds under a fixed alias. What that key protects is the
//! companion's own private key, which the app keeps sealed on disk; what this
//! plugin never does is hand the keystore's key to anyone, the app included.
//! On a platform without that keystore the vault refuses, so a key that
//! cannot be sealed stops the flow instead of being written in the clear —
//! which is the rule `keys.rs` already tests.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "dev.meltemi.vault";

/// Seals and opens with the platform's keystore.
pub trait Sealer: Send + Sync {
    /// Seals `plain` for this app on this device.
    ///
    /// # Errors
    ///
    /// When the keystore refuses, which must stop the caller.
    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, String>;
    /// Opens what [`Sealer::seal`] produced.
    ///
    /// # Errors
    ///
    /// When the keystore refuses or the bytes are not its own.
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, String>;
}

/// The plugin's handle, managed as Tauri state for the host to take.
pub struct Vault<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _runtime: std::marker::PhantomData<fn() -> R>,
}

#[cfg(not(target_os = "android"))]
const NO_KEYSTORE: &str =
    "this platform has no keystore to seal the key with; the companion's key stays unmade";

impl<R: Runtime> Sealer for Vault<R> {
    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
        #[cfg(target_os = "android")]
        {
            call(&self.handle, "seal", plain)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = plain;
            Err(NO_KEYSTORE.into())
        }
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, String> {
        #[cfg(target_os = "android")]
        {
            call(&self.handle, "open", sealed)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = sealed;
            Err(NO_KEYSTORE.into())
        }
    }
}

/// Registers the Kotlin side and manages the handle for the host.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("meltemi-vault")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let vault: Vault<R> = Vault {
                handle: api.register_android_plugin(PLUGIN_IDENTIFIER, "VaultPlugin")?,
            };
            #[cfg(not(target_os = "android"))]
            let vault: Vault<R> = {
                let _ = &api;
                Vault {
                    _runtime: std::marker::PhantomData,
                }
            };
            app.manage(vault);
            Ok(())
        })
        .build()
}

/// One round trip to the Kotlin side. Bytes travel as lowercase hex: no
/// encoding crate on either side, and nothing a log line could mistake for
/// text.
#[cfg(target_os = "android")]
fn call<R: Runtime>(
    handle: &tauri::plugin::PluginHandle<R>,
    command: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, String> {
    #[derive(serde::Serialize)]
    struct Request {
        data: String,
    }
    #[derive(serde::Deserialize)]
    struct Response {
        data: String,
    }
    let response: Response = handle
        .run_mobile_plugin(
            command,
            Request {
                data: hex_encode(bytes),
            },
        )
        .map_err(|e| format!("the keystore refused to {command}: {e}"))?;
    hex_decode(&response.data)
}

#[cfg(any(target_os = "android", test))]
fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(any(target_os = "android", test))]
fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("odd-length hex from the keystore".into());
    }
    (0..text.len())
        .step_by(2)
        .map(|at| {
            u8::from_str_radix(&text[at..at + 2], 16)
                .map_err(|e| format!("bad hex from the keystore: {e}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_survive_the_trip_to_kotlin_and_back() {
        let bytes: Vec<u8> = (0..=255).collect();
        let text = hex_encode(&bytes);
        assert_eq!(text.len(), 512);
        assert!(text.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(hex_decode(&text).expect("round trip"), bytes);
    }

    #[test]
    fn what_is_not_hex_is_refused_rather_than_guessed() {
        assert!(hex_decode("abc").is_err(), "odd length");
        assert!(hex_decode("zz").is_err(), "not hex");
        assert_eq!(hex_decode("").expect("empty is empty"), Vec::<u8>::new());
    }
}
