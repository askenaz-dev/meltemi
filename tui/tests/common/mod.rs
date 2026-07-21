// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for the CLI end-to-end mapping tests. Each test file is its
//! own crate, so unused helpers there are expected; silence the lint.
#![allow(dead_code)]

use std::path::PathBuf;

use tokio::sync::mpsc;

use meltemi_client::transport::Listener;
use meltemid::server::{DaemonState, serve_until_shutdown};

/// A fresh, process-unique endpoint for an ephemeral test daemon.
pub fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemi-cli-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemi-cli-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

/// Resolves the built `mock-agent` binary next to the test executable. A
/// workspace `cargo test` builds it.
pub fn mock_agent_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop(); // drop the test executable name -> .../deps
    if dir.ends_with("deps") {
        dir.pop(); // -> .../<profile>
    }
    let name = if cfg!(windows) {
        "mock-agent.exe"
    } else {
        "mock-agent"
    };
    dir.join(name)
}

/// Spins an in-process daemon at a fresh endpoint. Returns the endpoint and the
/// serve task handle (which resolves once the daemon shuts down).
pub async fn spawn_daemon(tag: &str) -> (String, tokio::task::JoinHandle<()>) {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind endpoint");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    (endpoint, handle)
}
