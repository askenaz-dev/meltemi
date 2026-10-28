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
pub mod reveal;
pub mod uistate;

use serde_json::{Value, json};
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, oneshot};

use bridge::{BridgeCommand, BridgeError, BridgeEvent};

/// Handle to the bridge actor, managed as Tauri state.
struct BridgeHandle(mpsc::UnboundedSender<BridgeCommand>);

/// The endpoint the client is talking to right now.
///
/// Held here rather than asked of the bridge because it must be answerable while
/// the bridge is between connections, which is exactly when somebody is most
/// likely to open the list and look.
struct ActiveMachine(std::sync::Mutex<String>);

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

/// What the surface can say about the waiting notice: whether one is configured,
/// and the result of rehearsing it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NoticeState {
    /// Whether the user configured a command at all.
    configured: bool,
}

/// Whether a waiting notice is configured for this user.
#[tauri::command]
fn notice_state() -> NoticeState {
    NoticeState {
        configured: meltemi_client::notices::configured(&meltemi_client::paths::config_dir())
            .is_some(),
    }
}

/// Runs the configured command with the test class of wait.
///
/// Exactly the same path as a real notice, because a rehearsal that took a
/// shortcut would prove nothing about the thing it rehearses. Answers whether a
/// command was launched, which is the only moment a misconfiguration is visible
/// (aviso-de-espera-opt-in design D6).
#[tauri::command]
fn notice_test() -> bool {
    let mut emitter = meltemi_client::notices::Emitter::new(meltemi_client::paths::config_dir());
    emitter.test("test")
}

/// One machine, as the surface shows it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MachineRow {
    name: String,
    /// What the user wrote, which is what they recognise.
    configured: String,
    /// What is actually dialed, which is what `active` compares against.
    endpoint: String,
    /// `ssh my-pc`, when the way in is a command.
    via: Option<String>,
    /// Whether this is the machine the client is talking to right now.
    active: bool,
}

/// The machines of this user, with whatever could not be read.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MachineList {
    machines: Vec<MachineRow>,
    diagnostics: Vec<String>,
}

/// The machines this user configured, and which one is active.
///
/// Read on demand. There is no watcher on the configuration file and no polling
/// of anything: the list is what the file says when somebody asks.
#[tauri::command]
fn machines_list(state: tauri::State<'_, ActiveMachine>) -> MachineList {
    let read = meltemi_client::machines::read(&meltemi_client::paths::config_dir());
    let active = state.0.lock().map(|at| at.clone()).unwrap_or_default();
    MachineList {
        machines: read
            .machines
            .iter()
            .map(|machine| MachineRow {
                name: machine.name.clone(),
                configured: machine.configured.clone(),
                endpoint: machine.endpoint.clone(),
                via: machine.via(),
                active: machine.endpoint == active,
            })
            .collect(),
        diagnostics: read.diagnostics,
    }
}

/// What a connection test found.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeResult {
    answered: bool,
    version: Option<String>,
    millis: u64,
    detail: Option<String>,
}

/// Opens a machine, says hello, and closes.
///
/// Never starts anything and never changes the active machine: testing is a
/// question, not a move.
#[tauri::command]
async fn machine_probe(name: String) -> Result<ProbeResult, BridgeError> {
    let read = meltemi_client::machines::read(&meltemi_client::paths::config_dir());
    let machine = read
        .by_name(&name)
        .ok_or_else(|| BridgeError::unreachable(format!("no machine called `{name}`")))?;
    let probe = meltemi_client::machines::probe(&machine.endpoint).await;
    Ok(ProbeResult {
        answered: probe.answered(),
        version: probe.version,
        millis: probe.millis,
        detail: probe.detail,
    })
}

/// What the diagnosis section shows: the rows, each with its words in both
/// languages, and the report as English text, which is what "Copy" copies.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorView {
    report: meltemi_client::doctor::Report,
    text: String,
    failing: bool,
}

/// The diagnosis of this installation, for the machine the client talks to
/// (meltemi-doctor). The same composition as `meltemi doctor`: it asks the
/// daemon on a connection of its own that never starts one, and sends nothing
/// anywhere — the surface shows it, and copies it when the person asks.
#[tauri::command]
async fn doctor_report(active: tauri::State<'_, ActiveMachine>) -> Result<DoctorView, BridgeError> {
    use meltemi_client::doctor;
    let endpoint = active.0.lock().map(|at| at.clone()).unwrap_or_default();
    let endpoint = if endpoint.is_empty() {
        meltemi_client::paths::endpoint()
    } else {
        endpoint
    };
    let at = endpoint.clone();
    let local = tauri::async_runtime::spawn_blocking(move || doctor::Local::gather_for(&at))
        .await
        .map_err(|error| BridgeError::unreachable(error.to_string()))?;
    let daemon = doctor::ask_daemon(&endpoint).await;
    let report = doctor::compose(&local, &daemon);
    Ok(DoctorView {
        text: report.text(doctor::Lang::En, true),
        failing: report.has_failures(),
        report,
    })
}

/// Talks to another machine from now on.
///
/// The webview asks for this only after its own unsaved-work guard has run —
/// the same guard as switching project, not a second one — because the tabs of
/// the machine being left point at sessions of another daemon.
#[tauri::command]
fn machine_switch(
    bridge: tauri::State<'_, BridgeHandle>,
    active: tauri::State<'_, ActiveMachine>,
    name: String,
) -> Result<String, BridgeError> {
    let read = meltemi_client::machines::read(&meltemi_client::paths::config_dir());
    let machine = read.by_name(&name).ok_or_else(|| {
        BridgeError::unreachable(format!(
            "no machine called `{name}`; configured: {}",
            read.names().join(", ")
        ))
    })?;
    if let Ok(mut at) = active.0.lock() {
        machine.endpoint.clone_into(&mut at);
    }
    bridge
        .0
        .send(BridgeCommand::Switch {
            endpoint: machine.endpoint.clone(),
        })
        .map_err(|_| BridgeError::unreachable("bridge stopped"))?;
    Ok(machine.name.clone())
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

/// Shows the main window, once: the first of the interface's signal and the
/// cap does it, and every later call finds the gate already passed
/// (senales-de-vida D6).
fn reveal_window(app: &tauri::AppHandle) {
    let Some(reveal) = app.try_state::<reveal::Reveal>() else {
        return;
    };
    if !reveal.first() {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

/// The interface has mounted its first tree, with its styles and theme
/// applied: showing the window now shows the interface, not an empty frame.
#[tauri::command]
fn first_frame(app: tauri::AppHandle) {
    reveal_window(&app);
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
        // The OS notice that says WHAT happened, beside the attention request
        // that only claims (avisos-de-escritorio design D1). The webview reaches
        // it through the capability below, which is deny-by-default like the
        // rest of this surface.
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let endpoint = meltemi_client::paths::endpoint();
            let started_at = endpoint.clone();
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

            // The window is created hidden (`visible: false`), so the
            // restored geometry lands before anyone sees it; it is shown on
            // the interface's first frame, or at the cap if that never comes
            // (senales-de-vida D6).
            let stored = uistate::UiState::load();
            restore_window(app.handle(), &stored);
            // The interface's own background, before anything can show the
            // window: the cap can fire before the interface has painted, and
            // what a webview shows until then is this color — not a white the
            // theme never had.
            if let Some(window) = app.get_webview_window("main") {
                let system_dark = matches!(window.theme(), Ok(tauri::Theme::Dark));
                let (r, g, b) = reveal::background(stored.theme.as_deref(), system_dark);
                let _ = window.set_background_color(Some(tauri::window::Color(r, g, b, 255)));
            }
            app.manage(reveal::Reveal::default());
            let capped = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(reveal::REVEAL_CAP);
                reveal_window(&capped);
            });

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
            // Seeded with the endpoint the process resolved at startup, which is
            // the machine it is already talking to before anybody chooses one.
            app.manage(ActiveMachine(std::sync::Mutex::new(started_at)));
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
            first_frame,
            request_attention,
            notice_state,
            notice_test,
            machines_list,
            machine_probe,
            machine_switch,
            doctor_report,
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
