// SPDX-License-Identifier: Apache-2.0

//! The desktop's side of the daemon bridge.
//!
//! The actor itself lives in `meltemi_client::bridge` and is the same code the
//! mobile companion runs (companero-que-conecta D1). What this file holds is
//! what only the desktop decides: it dials the local endpoint — or the command
//! the user named — and it is the surface with a waiting notice to emit,
//! because it is the one that can run the user's command
//! (aviso-de-espera-opt-in design D2).

pub use meltemi_client::bridge::{BridgeCommand, BridgeError, BridgeEvent, ConnState};
use meltemi_client::bridge::{LocalDialer, bridge_actor};
use meltemi_client::notices;
use tokio::sync::mpsc;

/// Runs the bridge for this desktop until the host drops its command sender.
pub async fn run(
    endpoint: String,
    commands: mpsc::UnboundedReceiver<BridgeCommand>,
    events: mpsc::UnboundedSender<BridgeEvent>,
) {
    // Created once and handed to the actor, which owns it across
    // reconnections: the record of when the last notice went out must survive
    // a daemon that drops and comes back, or the floor would reset and let a
    // burst through (aviso-de-espera-opt-in design D5).
    let notices = notices::Emitter::new(meltemi_client::paths::config_dir());
    bridge_actor(
        LocalDialer::new(endpoint),
        commands,
        events,
        Some(notices),
        "meltemi-desktop",
    )
    .await;
}
