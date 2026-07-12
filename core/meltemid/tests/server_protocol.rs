// SPDX-License-Identifier: Apache-2.0

//! Protocol-level integration tests over the real local transport:
//! initialize gating, version negotiation and standard JSON-RPC errors.

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, Stream, connect};

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-proto-test-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-proto-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

async fn start_server(tag: &str) -> String {
    let endpoint = test_endpoint(tag);
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test(tag, shutdown_tx);
    tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));
    endpoint
}

struct TestClient {
    reader: BufReader<tokio::io::ReadHalf<Stream>>,
    writer: tokio::io::WriteHalf<Stream>,
}

impl TestClient {
    async fn connect(endpoint: &str) -> Self {
        let stream = connect(endpoint).await.expect("connect");
        let (read, writer) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(read),
            writer,
        }
    }

    async fn send(&mut self, value: Value) {
        self.writer
            .write_all(format!("{value}\n").as_bytes())
            .await
            .expect("write");
        self.writer.flush().await.expect("flush");
    }

    async fn recv(&mut self) -> Option<Value> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await.expect("read");
        if n == 0 {
            return None;
        }
        Some(serde_json::from_str(&line).expect("valid JSON"))
    }
}

fn initialize_request(id: i64, version: i64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": version,
            "client": {"name": "test-client", "version": "0.0.0"},
        },
    })
}

#[tokio::test]
async fn initialize_negotiates_supported_version() {
    let endpoint = start_server("ok").await;
    let mut client = TestClient::connect(&endpoint).await;

    client.send(initialize_request(1, 1)).await;
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], 1);
    assert_eq!(resp["result"]["daemon"]["name"], "meltemid");
}

#[tokio::test]
async fn unsupported_version_gets_both_versions_and_close() {
    let endpoint = start_server("badver").await;
    let mut client = TestClient::connect(&endpoint).await;

    client.send(initialize_request(1, 99)).await;
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["error"]["code"], 1000);
    let detail = resp["error"]["data"]["detail"].as_str().unwrap();
    assert!(
        detail.contains("99"),
        "detail must carry the declared version"
    );
    assert!(
        detail.contains('1'),
        "detail must carry the supported versions"
    );
    // The daemon closes the connection in an orderly fashion.
    assert_eq!(client.recv().await, None, "connection must be closed");
}

#[tokio::test]
async fn methods_before_initialize_are_rejected() {
    let endpoint = start_server("gate").await;
    let mut client = TestClient::connect(&endpoint).await;

    client
        .send(json!({"jsonrpc": "2.0", "id": 5, "method": "status"}))
        .await;
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["error"]["code"], 1001);
    assert_eq!(resp["error"]["data"]["kind"], "not_initialized");

    // Same connection can still initialize afterwards.
    client.send(initialize_request(6, 1)).await;
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["result"]["protocolVersion"], 1);
}

#[tokio::test]
async fn status_reports_version_uptime_and_no_sessions() {
    let endpoint = start_server("status").await;
    let mut client = TestClient::connect(&endpoint).await;
    client.send(initialize_request(1, 1)).await;
    client.recv().await.expect("init response");

    client
        .send(json!({"jsonrpc": "2.0", "id": 2, "method": "status"}))
        .await;
    let resp = client.recv().await.expect("status response");
    assert!(!resp["result"]["daemonVersion"].as_str().unwrap().is_empty());
    assert!(resp["result"]["uptimeSeconds"].as_u64().is_some());
    assert_eq!(resp["result"]["sessions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn shutdown_responds_then_stops_accepting() {
    let endpoint = start_server("shutdown").await;
    let mut client = TestClient::connect(&endpoint).await;
    client.send(initialize_request(1, 1)).await;
    client.recv().await.expect("init response");

    client
        .send(json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}))
        .await;
    let resp = client.recv().await.expect("shutdown response");
    assert!(resp["result"].is_object(), "shutdown returns a result");

    // The accept loop stops: new connections are no longer served. On both
    // platforms a fresh connect either fails, times out, or never gets a
    // response. The connect itself is bounded so this can never hang.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let connected =
        tokio::time::timeout(std::time::Duration::from_secs(3), connect(&endpoint)).await;
    if let Ok(Ok(stream)) = connected {
        let (read, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(read);
        let _ = writer
            .write_all(format!("{}\n", initialize_request(1, 1)).as_bytes())
            .await;
        let _ = writer.flush().await;
        let mut line = String::new();
        let got = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            reader.read_line(&mut line),
        )
        .await;
        assert!(
            matches!(got, Ok(Ok(0)) | Err(_)),
            "daemon must not serve after shutdown, got: {line:?}"
        );
    }
}

#[tokio::test]
async fn cancel_for_unknown_session_is_ignored() {
    let endpoint = start_server("cancel").await;
    let mut client = TestClient::connect(&endpoint).await;
    client.send(initialize_request(1, 1)).await;
    client.recv().await.expect("init response");

    // A cancel notification for a session that does not exist must not crash
    // the daemon; a subsequent request still works.
    client
        .send(json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": {"sessionId": "nope"},
        }))
        .await;
    client
        .send(json!({"jsonrpc": "2.0", "id": 3, "method": "status"}))
        .await;
    let resp = client.recv().await.expect("status response");
    assert_eq!(resp["id"], 3);
}

#[tokio::test]
async fn malformed_message_does_not_kill_the_daemon() {
    let endpoint = start_server("malformed").await;
    let mut client = TestClient::connect(&endpoint).await;

    client
        .writer
        .write_all(b"{{{ definitely not json\n")
        .await
        .expect("write");
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["error"]["code"], -32700);

    // Daemon and connection both survive.
    client.send(initialize_request(2, 1)).await;
    let resp = client.recv().await.expect("response");
    assert_eq!(resp["result"]["protocolVersion"], 1);

    // And new connections are still accepted.
    let mut second = TestClient::connect(&endpoint).await;
    second.send(initialize_request(1, 1)).await;
    let resp = second.recv().await.expect("response");
    assert_eq!(resp["result"]["protocolVersion"], 1);
}
