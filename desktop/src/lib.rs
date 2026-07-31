// SPDX-License-Identifier: Apache-2.0

//! Meltemi desktop client (gui-tauri-paridad design D1).
//!
//! The Tauri process is the only owner of the daemon connection: it speaks
//! line-delimited JSON-RPC over the local socket via `meltemi-client`, and the
//! webview consumes Tauri IPC commands/events exclusively — it never opens
//! sockets nor fetches remote content (CSP, deny-by-default capabilities).

pub mod bridge;
pub mod fsops;
pub mod lsp;
pub mod uistate;

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

/// Fire-and-forget notification passthrough (e.g. `session/cancel`).
#[tauri::command]
fn daemon_notify(
    state: tauri::State<'_, BridgeHandle>,
    method: String,
    params: Option<Value>,
) -> Result<(), BridgeError> {
    state
        .0
        .send(BridgeCommand::Notify {
            method,
            params: params.unwrap_or_else(|| json!({})),
        })
        .map_err(|_| BridgeError::unreachable("bridge stopped"))
}

/// The working directory this process was launched in: the initial project
/// scope. The active project itself is surface state (uistate), so the cwd is
/// a starting point, never a cage.
#[tauri::command]
fn project_root() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|root| root.display().to_string())
}

/// Opens the OS folder picker and answers with the chosen path, or `None` when
/// the user dismissed it (lanzador-conversacional design D8).
///
/// The dialog is chrome of the client and the boundary of constitution §3 runs
/// right here: the daemon opens no window and enumerates no filesystem — it
/// receives one path from `project/register` and validates it. The plugin is
/// initialized in the builder but deliberately NOT exposed to the webview: the
/// surface reaches it through this command alone, so `capabilities/default.json`
/// stays at `core:default` and the CSP is untouched.
#[tauri::command]
async fn pick_project_folder(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = oneshot::channel();
    app.dialog().file().pick_folder(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.await.ok().flatten()?;
    // A dismissed dialog and an unrepresentable path are both "nothing chosen":
    // the surface asks again, and nothing is sent to the daemon either way.
    picked
        .into_path()
        .ok()
        .map(|path| path.display().to_string())
}

/// First-use flag, persisted in the user's data directory (gui-shell
/// "Onboarding de primer uso"): no account, no network, no telemetry.
fn onboarding_flag() -> std::path::PathBuf {
    meltemi_client::paths::data_dir().join("desktop-onboarding-seen")
}

#[tauri::command]
fn onboarding_seen() -> bool {
    onboarding_flag().exists()
}

#[tauri::command]
fn onboarding_mark_seen() {
    let flag = onboarding_flag();
    if let Some(parent) = flag.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(flag, b"seen\n");
}

/// Asks the OS for attention when a permission arrives with the window
/// unfocused (design D4): taskbar flash on Windows, dock bounce on macOS,
/// urgency hint on Linux — the core window API, no plugin, no sound of our
/// own, nothing leaving the machine. The title carries the counter so the
/// reason is legible from the taskbar.
#[tauri::command]
fn request_attention(app: tauri::AppHandle, pending: u32, title: Option<String>) {
    use tauri::{UserAttentionType, Window};
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let focused = window.is_focused().unwrap_or(false);
    // The surface owns the message catalog (§11), so it passes the localized
    // title; the host only falls back to the product name, never to prose in a
    // language the user did not choose.
    let title = title.unwrap_or_else(|| "Meltemi".to_string());
    let _ = window.set_title(&title);

    let attention = if pending > 0 && !focused {
        Some(UserAttentionType::Informational)
    } else {
        None
    };
    // `None` clears a pending request on the platforms that keep one.
    let _: Result<(), _> = Window::request_user_attention(&window.as_ref().window(), attention);
}

/// Persists the window geometry, and restores it when the remembered position
/// still falls inside an attached monitor (multi-monitor rule, design D3).
fn restore_window(app: &tauri::AppHandle, state: &uistate::UiState) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(geometry) = state.window else {
        return;
    };
    let monitors: Vec<(i32, i32, u32, u32)> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            (position.x, position.y, size.width, size.height)
        })
        .collect();
    if !uistate::geometry_is_visible(&monitors, &geometry) {
        return; // Stranded geometry: fall back to the configured defaults.
    }
    let _ = window.set_size(tauri::PhysicalSize::new(geometry.width, geometry.height));
    let _ = window.set_position(tauri::PhysicalPosition::new(geometry.x, geometry.y));
    if geometry.maximized {
        let _ = window.maximize();
    }
}

/// Stores the current geometry into the persisted state.
#[tauri::command]
fn window_state_store(app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let mut state = uistate::UiState::load();
    state.window = Some(uistate::WindowState {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: window.is_maximized().unwrap_or(false),
    });
    state.save();
}

/// Closes the window once the surface has resolved its unsaved editors
/// (the dirty guard answers here after the user decides).
#[tauri::command]
fn close_confirmed(app: tauri::AppHandle) {
    window_state_store(app.clone());
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.destroy();
    }
}

pub fn run() {
    tauri::Builder::default()
        // Initialized for the host, never handed to the webview: the front has
        // no `dialog:` permission and cannot call the plugin directly.
        .plugin(tauri_plugin_dialog::init())
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

            let stored = uistate::UiState::load();
            restore_window(app.handle(), &stored);

            // Closing with unsaved editors asks the surface first: no human
            // work is ever discarded silently (spec "Ninguna pérdida
            // silenciosa de edición").
            if let Some(window) = app.get_webview_window("main") {
                let closer = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = closer.emit("app:close-requested", ());
                    }
                });
            }

            app.manage(BridgeHandle(command_tx));
            app.manage(lsp::LspHub::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_request,
            daemon_notify,
            project_root,
            pick_project_folder,
            onboarding_seen,
            onboarding_mark_seen,
            request_attention,
            window_state_store,
            close_confirmed,
            uistate::ui_state_load,
            uistate::ui_state_save,
            fsops::tree_read,
            fsops::tree_search,
            fsops::open_with,
            lsp::lsp_ensure,
            lsp::lsp_open,
            lsp::lsp_change,
            lsp::lsp_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Meltemi desktop client");
}
