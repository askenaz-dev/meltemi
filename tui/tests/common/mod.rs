// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for the CLI end-to-end mapping tests. Each test file is its
//! own crate, so unused helpers there are expected; silence the lint.
#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use meltemi_client::transport::{Listener, Stream};
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

/// The test as the daemon (the bridge's pattern): the same listener the daemon
/// binds, answering from a script what a command asks — for the moments no
/// real daemon can be made to stage on cue.
pub struct FakeDaemon {
    lines: tokio::io::Lines<BufReader<tokio::io::ReadHalf<Stream>>>,
    write: tokio::io::WriteHalf<Stream>,
}

impl FakeDaemon {
    /// Accepts the command's connection.
    pub async fn accept(listener: &mut Listener) -> Self {
        let stream = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .expect("the command dials the endpoint")
            .expect("accept");
        let (read, write) = tokio::io::split(stream);
        Self {
            lines: BufReader::new(read).lines(),
            write,
        }
    }

    /// Answers the next request, which must be `method`, with `result`.
    pub async fn answer(&mut self, method: &str, result: Value) {
        let line = tokio::time::timeout(Duration::from_secs(10), self.lines.next_line())
            .await
            .unwrap_or_else(|_| panic!("the command asked for `{method}`"))
            .expect("read")
            .expect("a line");
        let request: Value = serde_json::from_str(&line).expect("a JSON-RPC line");
        assert_eq!(request["method"], method, "{request}");
        self.send(json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }))
            .await;
    }

    /// Answers `initialize` as a daemon of this protocol version.
    pub async fn initialize(&mut self) {
        self.answer(
            meltemi_proto::methods::INITIALIZE,
            json!({
                "protocolVersion": meltemi_proto::PROTOCOL_VERSION,
                "daemon": { "name": "fake", "version": "0" },
            }),
        )
        .await;
    }

    /// Sends one message the daemon starts.
    pub async fn send(&mut self, message: Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.write.write_all(line.as_bytes()).await.expect("write");
        self.write.flush().await.expect("flush");
    }
}
