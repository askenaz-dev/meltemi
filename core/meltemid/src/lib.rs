// SPDX-License-Identifier: Apache-2.0

//! Meltemi headless daemon library.
//!
//! `meltemid` exposes its internals as a library so that development tooling
//! (`meltemi-devclient`) and the e2e suite can reuse the local transport, the
//! protocol server and the client-side bootstrap helpers. The binary in
//! `main.rs` is a thin wrapper over [`run`].

pub mod acp;
pub mod analytics;
pub mod archive;
pub mod checkpoints;
pub mod clients;
pub mod clock;
pub mod commit;
pub mod config;
pub mod conformance;
pub mod context;
pub mod edits;
pub mod events;
pub mod fleet;
pub mod free_session;
pub mod git;
pub mod implement;
pub mod levels;
pub mod logging;
pub mod mcp;
pub mod navigate;
pub mod pending;
pub mod permissions;
pub mod projects;
pub mod propose;
pub mod repo_map;
pub mod review;
pub mod sdd;
pub mod sdd_flow;
pub mod server;
pub mod session;
pub mod session_finalize;
pub mod session_index;
pub mod session_log;
pub mod subscriptions;
pub mod verify;
pub mod worktrees;

// The client-side stack (transport, JSON-RPC peer, bootstrap, paths) lives in
// the shared `meltemi-client` crate since gui-tauri-paridad 1.2; re-exported
// here so daemon internals and existing consumers keep their paths.
pub use meltemi_client::{bootstrap, paths, rpc, transport};

use std::sync::Arc;

use tokio::sync::mpsc;

use server::{DaemonState, serve_until_shutdown};
use transport::Listener;

/// Runs the daemon: binds the endpoint (enforcing single instance), then
/// serves connections until `shutdown` is requested. Returns once the accept
/// loop stops, after which the process may exit.
pub async fn run() -> anyhow::Result<()> {
    let data_dir = paths::data_dir();
    let config_dir = paths::config_dir();
    let endpoint = paths::endpoint();

    let listener = match Listener::bind(&endpoint).await {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            // Another instance already owns the endpoint; nothing to do.
            tracing::info!("another meltemid instance is already running; exiting");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state: Arc<DaemonState> = DaemonState::new(data_dir, config_dir, shutdown_tx);

    tracing::info!(endpoint = %endpoint, version = state.version, "meltemid listening");
    serve_until_shutdown(listener, state, shutdown_rx).await;
    tracing::info!("meltemid stopped");
    Ok(())
}
