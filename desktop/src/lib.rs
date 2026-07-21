// SPDX-License-Identifier: Apache-2.0

//! Meltemi desktop client (gui-tauri-paridad design D1).
//!
//! The Tauri process is the only owner of the daemon connection: it speaks
//! line-delimited JSON-RPC over the local socket via `meltemi-client`, and the
//! webview consumes Tauri IPC commands/events exclusively — it never opens
//! sockets nor fetches remote content (CSP, deny-by-default capabilities).

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the Meltemi desktop client");
}
