// SPDX-License-Identifier: Apache-2.0

//! The Meltemi mobile companion: the Agent Boss's remote post.
//!
//! Four verbs and no more — monitor, approve, review, direct — over the user's
//! own SSH, into the same local socket every other client reaches. The spec
//! `mobile-companion` governs the boundary and this crate does not widen it:
//! there is no authoring of code or specs here, and no path to any of it.
//!
//! **What is different from the desktop, and it is one thing.** A phone has no
//! system `ssh` to launch, so the `exec:` endpoint — which is the whole of
//! remote access everywhere else — has nothing to run. This crate speaks SSH
//! itself ([`ssh`]) and hands the shared bridge actor a stream that, on the
//! far side, is `meltemi bridge` ([`dial`]). Everything above that stream is
//! the code the desktop already runs: the actor is `meltemi_client::bridge`
//! with a different dialer, and the webview's door to it ([`host`]) has the
//! desktop's names.
//!
//! The SSH client is confined here. On the desktop the `ssh` is the user's own,
//! and that does not change. The app's key is its own ([`keys`]), sealed by the
//! platform's keystore ([`vault`]); what it remembers that is not secret lives
//! in plain files ([`store`]).

pub mod dial;
pub mod host;
pub mod keys;
pub mod ssh;
pub mod store;
pub mod vault;

/// The Android entry point.
///
/// Tauri calls this from the Java side; everything above it is the same Svelte
/// bundle the desktop builds, at its own entry.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // The keystore, for the host and never for the webview: there is no
        // capability for it because there is no command the surface could
        // call. The Rust side seals and opens; the surface sees a key line.
        .plugin(tauri_plugin_meltemi_vault::init())
        .manage(host::Host::default())
        .invoke_handler(tauri::generate_handler![
            host::daemon_request,
            host::daemon_notify,
            host::companion_key,
            host::companion_destination,
            host::companion_connect,
            host::companion_disconnect,
            host::companion_forget_host,
            host::companion_forget_destination,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Meltemi companion");
}
