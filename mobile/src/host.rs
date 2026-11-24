// SPDX-License-Identifier: Apache-2.0

//! The companion's host (companero-que-conecta D4): what the webview may
//! invoke, and the actor's life — which is the app's time in front.
//!
//! The webview's door to the daemon is the desktop's door, `daemon_request`
//! and `daemon_notify` with the same names and the same events back, so
//! `daemon.ts` and every store above it run here unchanged. What is the
//! companion's own is the pairing: the key to authorise, the destination to
//! dial, and connect/disconnect, which the surface ties to visibility. Nothing
//! here holds a socket while the app is in the background, because nothing
//! here holds an actor then.

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::{mpsc, oneshot};

use meltemi_client::bridge::{BridgeCommand, BridgeError, BridgeEvent, bridge_actor};
use tauri_plugin_meltemi_vault::Vault;

use crate::dial::SshDialer;
use crate::keys::AppKey;
use crate::ssh::Destination;
use crate::store::{KnownHosts, Remembered};
use crate::vault::Sealed;

/// The name this surface gives the daemon in `initialize`.
pub const CLIENT: &str = "meltemi-mobile";

/// What the surface is told when it asks the daemon something with no actor.
const NO_TUNNEL: &str = "no tunnel: the companion is not connected";

/// The running actor's command sender, while the app is in front.
#[derive(Default)]
pub struct Host {
    actor: Mutex<Option<mpsc::UnboundedSender<BridgeCommand>>>,
}

impl Host {
    fn sender(&self) -> Option<mpsc::UnboundedSender<BridgeCommand>> {
        self.actor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Installs `next` as the actor, which ends the previous one: dropping
    /// its sender is how the actor learns the host is gone, and its tunnel
    /// falls with it.
    fn replace(&self, next: Option<mpsc::UnboundedSender<BridgeCommand>>) {
        *self
            .actor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = next;
    }
}

/// Where the companion keeps its files, made on first use.
fn data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no data directory for the app: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    Ok(dir)
}

/// This app's key, through the keystore: made once, opened every time after.
fn app_key<R: Runtime>(app: &AppHandle<R>) -> Result<AppKey, String> {
    let dir = data_dir(app)?;
    let sealer = app.state::<Vault<R>>();
    let mut vault = Sealed::open(&dir, &*sealer)?;
    AppKey::load_or_create(&mut vault)
}

/// Generic RPC passthrough for the webview: the same door the desktop has.
#[tauri::command]
pub async fn daemon_request(
    host: State<'_, Host>,
    method: String,
    params: Option<Value>,
) -> Result<Value, BridgeError> {
    let sender = host
        .sender()
        .ok_or_else(|| BridgeError::unreachable(NO_TUNNEL))?;
    let (reply, response) = oneshot::channel();
    sender
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
pub fn daemon_notify(
    host: State<'_, Host>,
    method: String,
    params: Option<Value>,
) -> Result<(), BridgeError> {
    let sender = host
        .sender()
        .ok_or_else(|| BridgeError::unreachable(NO_TUNNEL))?;
    sender
        .send(BridgeCommand::Notify {
            method,
            params: params.unwrap_or_else(|| json!({})),
        })
        .map_err(|_| BridgeError::unreachable("bridge stopped"))
}

/// The line to add to `authorized_keys`: the only half of the key that leaves.
#[tauri::command]
pub fn companion_key<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    Ok(app_key(&app)?.authorized_line())
}

/// The destination remembered from the last connection, if any.
#[tauri::command]
pub fn companion_destination<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, String> {
    Ok(Remembered::in_dir(&data_dir(&app)?).read())
}

/// Dials `destination`, and keeps dialling it until told to stop — or refused.
///
/// Parsed before anything else: a destination that does not read as
/// `user@host[:port]` is refused here, with the shape it wanted, and is never
/// remembered. Connecting while connected replaces the actor.
#[tauri::command]
pub fn companion_connect<R: Runtime>(
    app: AppHandle<R>,
    host: State<'_, Host>,
    destination: String,
) -> Result<(), String> {
    let parsed = Destination::parse(&destination)?;
    let dir = data_dir(&app)?;
    let key = app_key(&app)?;
    let hosts = KnownHosts::open(&dir)?;
    Remembered::in_dir(&dir).write(&destination)?;

    let dialer = SshDialer::new(parsed, key.private(), Box::new(hosts));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, mut events) = mpsc::unbounded_channel();
    host.replace(Some(command_tx));
    // No notice emitter: a phone has no command of the user's to run, and the
    // notice that brings them back comes from their own infrastructure (D2).
    tauri::async_runtime::spawn(bridge_actor(dialer, command_rx, event_tx, None, CLIENT));
    let surface = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            forward(&surface, event);
        }
    });
    Ok(())
}

/// Ends the actor. The surface calls this when it leaves the front: suspended
/// is not connected, and nothing holds the socket in the background.
#[tauri::command]
pub fn companion_disconnect(host: State<'_, Host>) {
    host.replace(None);
}

/// Forgets the key of the host in `destination`, which is what a reinstalled
/// machine needs — and the only way past a changed host key, because the
/// actor never offers one. Takes the destination as typed so the host is
/// named by the same parse that dialled it, and not by a second one in the
/// surface.
#[tauri::command]
pub fn companion_forget_host<R: Runtime>(
    app: AppHandle<R>,
    destination: String,
) -> Result<bool, String> {
    let parsed = Destination::parse(&destination)?;
    KnownHosts::open(&data_dir(&app)?)?.forget(&parsed.host)
}

/// Forgets the destination and ends the actor.
#[tauri::command]
pub fn companion_forget_destination<R: Runtime>(
    app: AppHandle<R>,
    host: State<'_, Host>,
) -> Result<(), String> {
    host.replace(None);
    Remembered::in_dir(&data_dir(&app)?).clear()
}

/// The actor's events, as the webview expects them: the desktop's two names.
fn forward<R: Runtime>(app: &AppHandle<R>, event: BridgeEvent) {
    match event {
        BridgeEvent::State(state) => {
            let _ = app.emit("daemon:state", &state);
        }
        BridgeEvent::Incoming { method, params } => {
            let _ = app.emit(
                "daemon:incoming",
                &json!({ "method": method, "params": params }),
            );
        }
    }
}
