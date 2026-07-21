// SPDX-License-Identifier: Apache-2.0

//! Meltemi desktop client (gui-tauri-paridad design D1).
//!
//! The Tauri process is the only owner of the daemon connection: it speaks
//! line-delimited JSON-RPC over the local socket via `meltemi-client`, and the
//! webview consumes Tauri IPC commands/events exclusively — it never opens
//! sockets nor fetches remote content (CSP, deny-by-default capabilities).

pub mod bridge;

use serde_json::{Value, json};
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

use bridge::{BridgeCommand, BridgeError, BridgeEvent};

/// Handle to the bridge actor, managed as Tauri state.
struct BridgeHandle(mpsc::UnboundedSender<BridgeCommand>);

/// Generic RPC passthrough for the webview: every daemon capability is
/// reachable through this single command (core parity travels over one pipe).
#[tauri::command]
async fn daemon_request(
    state: tauri::State<'_, BridgeHandle>,
    method: String,
    params: Option<Value>,
) -> Result<Value, BridgeError> {
    let (reply, response) = oneshot::channel();
    state
        .0
        .send(BridgeCommand::Request {
            method,
            params: params.unwrap_or_else(|| json!({})),
            reply,
        })
        .map_err(|_| BridgeError::unreachable("bridge stopped"))?;
    response
        .await
        .map_err(|_| BridgeError::unreachable("bridge stopped"))?
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let endpoint = meltemi_client::paths::endpoint();
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();

            tauri::async_runtime::spawn(bridge::bridge_actor(endpoint, command_rx, event_tx));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        BridgeEvent::State(state) => {
                            let _ = handle.emit("daemon:state", &state);
                        }
                        BridgeEvent::Incoming { method, params } => {
                            let _ = handle.emit(
                                "daemon:incoming",
                                &json!({ "method": method, "params": params }),
                            );
                        }
                    }
                }
            });

            app.manage(BridgeHandle(command_tx));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![daemon_request])
        .run(tauri::generate_context!())
        .expect("error while running the Meltemi desktop client");
}
