// SPDX-License-Identifier: Apache-2.0

//! Line-delimited JSON-RPC 2.0 peer (design D2): the same model ACP uses.
//!
//! The connection is bidirectional: both sides can issue requests and
//! notifications. Malformed input is answered with the standard protocol
//! errors (-32700/-32600) without tearing the connection down.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, Notify, mpsc, oneshot};

use crate::transport::Stream;

/// A message queued for the writer task.
enum Outgoing {
    /// A JSON-RPC line to send.
    Line(String),
    /// Flush queued lines, then close the write half (orderly half-close).
    Close,
}

/// JSON-RPC error codes reserved by the spec.
pub mod codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// A JSON-RPC error, optionally carrying structured `data`
/// (`{kind, detail, remedy}` for application errors, per D11).
#[derive(Debug, Clone)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Application error with the structured data payload of the contract.
    pub fn application(
        code: i64,
        message: impl Into<String>,
        kind: impl Into<String>,
        detail: impl Into<String>,
        remedy: Option<String>,
    ) -> Self {
        let data = meltemi_proto::ErrorData {
            kind: kind.into(),
            detail: detail.into(),
            remedy,
            candidates: None,
        };
        Self {
            code,
            message: message.into(),
            data: Some(serde_json::to_value(data).expect("ErrorData serializes")),
        }
    }

    /// Application error whose payload also carries the fleet agents detected
    /// on this system, so a surface can offer a choice instead of transcribing
    /// the diagnosis (lanzador-conversacional D7). The candidates are always
    /// serialized, empty list included: an empty list says the fleet was
    /// consulted and nothing was found, which a surface must be able to tell
    /// apart from an error that has nothing to do with resolving an agent.
    pub fn application_with_candidates(
        code: i64,
        message: impl Into<String>,
        kind: impl Into<String>,
        detail: impl Into<String>,
        remedy: Option<String>,
        candidates: Vec<meltemi_proto::AgentCandidate>,
    ) -> Self {
        let data = meltemi_proto::ErrorData {
            kind: kind.into(),
            detail: detail.into(),
            remedy,
            candidates: Some(candidates),
        };
        Self {
            code,
            message: message.into(),
            data: Some(serde_json::to_value(data).expect("ErrorData serializes")),
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            codes::METHOD_NOT_FOUND,
            format!("method not found: {method}"),
        )
    }

    pub fn invalid_params(detail: impl std::fmt::Display) -> Self {
        Self::new(codes::INVALID_PARAMS, format!("invalid params: {detail}"))
    }

    /// Internal failure from an underlying I/O or system error.
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::new(codes::INTERNAL_ERROR, format!("internal error: {error}"))
    }

    fn to_json(&self) -> Value {
        let mut err = json!({ "code": self.code, "message": self.message });
        if let Some(data) = &self.data {
            err["data"] = data.clone();
        }
        err
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

/// An incoming message that requires handling by the host.
#[derive(Debug)]
pub enum Incoming {
    /// A request: the handler must eventually call [`Peer::respond`] with
    /// this `id`.
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    /// A notification (no response expected).
    Notification { method: String, params: Value },
}

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, RpcError>>>>>;

/// One side of a line-delimited JSON-RPC 2.0 connection.
#[derive(Clone)]
pub struct Peer {
    outgoing: mpsc::UnboundedSender<Outgoing>,
    pending: PendingMap,
    next_id: Arc<AtomicI64>,
    close: Arc<Notify>,
    /// Identifies this connection among the live ones, so a daemon-side
    /// fan-out can tell where an event came from (eventos-para-tardios D3).
    /// Process-local and monotonic: it names no user and survives nothing.
    connection: u64,
}

impl Peer {
    /// Starts the reader/writer tasks over the stream. Returns the peer and
    /// the channel of incoming requests/notifications; the channel closes
    /// when the connection does.
    pub fn start(stream: Stream) -> (Self, mpsc::UnboundedReceiver<Incoming>) {
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Outgoing>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<Incoming>();
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let close = Arc::new(Notify::new());
        static NEXT_CONNECTION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let peer = Self {
            outgoing: outgoing_tx,
            pending: Arc::clone(&pending),
            next_id: Arc::new(AtomicI64::new(1)),
            close: Arc::clone(&close),
            connection: NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed),
        };

        let (read_half, mut write_half) = tokio::io::split(stream);

        tokio::spawn(async move {
            while let Some(message) = outgoing_rx.recv().await {
                match message {
                    Outgoing::Line(line) => {
                        if write_half.write_all(line.as_bytes()).await.is_err()
                            || write_half.write_all(b"\n").await.is_err()
                            || write_half.flush().await.is_err()
                        {
                            break;
                        }
                    }
                    Outgoing::Close => {
                        let _ = write_half.flush().await;
                        // Drop the write half: the peer sees EOF on its read.
                        break;
                    }
                }
            }
        });

        let reader_peer = peer.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(read_half).lines();
            loop {
                tokio::select! {
                    _ = close.notified() => break,
                    next = lines.next_line() => match next {
                        Ok(Some(line)) => {
                            if line.trim().is_empty() {
                                continue;
                            }
                            if reader_peer.process_line(&line, &incoming_tx).await.is_err() {
                                // The incoming consumer is gone; stop reading.
                                break;
                            }
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
            }
            // Connection gone: fail every in-flight outgoing request.
            let mut pending = pending.lock().await;
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err(RpcError::new(
                    codes::INTERNAL_ERROR,
                    "connection closed",
                )));
            }
            // incoming_tx drops here, closing the channel.
        });

        (peer, incoming_rx)
    }

    /// This connection's process-local identifier. Distinguishes live
    /// connections from each other; it is not an identity of any kind.
    #[must_use]
    pub fn connection_id(&self) -> u64 {
        self.connection
    }

    /// Requests an orderly close: queued output is flushed, then the write
    /// half is dropped (the peer sees EOF), and the reader stops.
    pub fn close(&self) {
        let _ = self.outgoing.send(Outgoing::Close);
        self.close.notify_waiters();
    }

    /// Processes one inbound line. Returns `Err(())` only when an incoming
    /// request/notification could not be delivered because the consumer is
    /// gone, signaling the reader to stop.
    async fn process_line(
        &self,
        line: &str,
        incoming: &mpsc::UnboundedSender<Incoming>,
    ) -> Result<(), ()> {
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                self.send_error_response(
                    Value::Null,
                    &RpcError::new(codes::PARSE_ERROR, "Parse error"),
                );
                return Ok(());
            }
        };
        let Some(obj) = value.as_object() else {
            self.send_error_response(
                Value::Null,
                &RpcError::new(codes::INVALID_REQUEST, "Invalid Request"),
            );
            return Ok(());
        };

        let id = obj.get("id").cloned();
        let has_valid_version = obj.get("jsonrpc").and_then(Value::as_str) == Some("2.0");

        // Response to one of our outgoing requests.
        if obj.contains_key("result") || obj.contains_key("error") {
            if let Some(Value::Number(n)) = &id
                && let Some(id) = n.as_i64()
                && let Some(tx) = self.pending.lock().await.remove(&id)
            {
                let outcome = if let Some(err) = obj.get("error") {
                    Err(RpcError {
                        code: err.get("code").and_then(Value::as_i64).unwrap_or(0),
                        message: err
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        data: err.get("data").cloned(),
                    })
                } else {
                    Ok(obj.get("result").cloned().unwrap_or(Value::Null))
                };
                let _ = tx.send(outcome);
            }
            return Ok(());
        }

        let method = obj.get("method").and_then(Value::as_str);
        let (Some(method), true) = (method, has_valid_version) else {
            self.send_error_response(
                id.unwrap_or(Value::Null),
                &RpcError::new(codes::INVALID_REQUEST, "Invalid Request"),
            );
            return Ok(());
        };

        let params = obj.get("params").cloned().unwrap_or(Value::Null);
        let message = match id {
            Some(id) if !id.is_null() => Incoming::Request {
                id,
                method: method.to_string(),
                params,
            },
            _ => Incoming::Notification {
                method: method.to_string(),
                params,
            },
        };
        incoming.send(message).map_err(|_| ())
    }

    fn send_line(&self, value: &Value) {
        let _ = self.outgoing.send(Outgoing::Line(
            serde_json::to_string(value).expect("JSON value serializes"),
        ));
    }

    fn send_error_response(&self, id: Value, error: &RpcError) {
        self.send_line(&json!({ "jsonrpc": "2.0", "id": id, "error": error.to_json() }));
    }

    /// Responds to an incoming request.
    pub fn respond(&self, id: Value, result: Result<Value, RpcError>) {
        match result {
            Ok(result) => self.send_line(&json!({ "jsonrpc": "2.0", "id": id, "result": result })),
            Err(error) => self.send_error_response(id, &error),
        }
    }

    /// Sends a notification.
    pub fn notify<P: serde::Serialize>(&self, method: &str, params: &P) {
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": serde_json::to_value(params).expect("params serialize"),
        }));
    }

    /// Sends a request and awaits its response.
    pub async fn request<P: serde::Serialize>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<Value, RpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": serde_json::to_value(params).expect("params serialize"),
        }));
        match rx.await {
            Ok(outcome) => outcome,
            Err(_) => Err(RpcError::new(codes::INTERNAL_ERROR, "connection closed")),
        }
    }

    /// Cancels an in-flight outgoing request without waiting for the peer.
    pub async fn abandon_request(&self, id: i64) {
        self.pending.lock().await.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

    fn pair() -> (Peer, mpsc::UnboundedReceiver<Incoming>, DuplexStream) {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        let (peer, incoming) = Peer::start(Box::new(ours));
        (peer, incoming, theirs)
    }

    async fn read_json(reader: &mut BufReader<tokio::io::ReadHalf<DuplexStream>>) -> Value {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read line");
        serde_json::from_str(&line).expect("valid JSON out")
    }

    #[test]
    fn only_the_candidates_constructor_carries_candidates() {
        let plain = RpcError::application(2000, "no agent configured", "k", "d", None);
        let data = plain.data.expect("application errors carry data");
        assert!(
            data.get("candidates").is_none(),
            "an error unrelated to resolving an agent must not offer a choice"
        );

        let none_found = RpcError::application_with_candidates(
            2000,
            "no agent configured",
            "agent_command_not_configured",
            "no agent is configured and none was detected",
            Some("Install one of the supported CLIs.".into()),
            vec![],
        );
        let data = none_found.data.expect("application errors carry data");
        assert_eq!(
            data["candidates"],
            json!([]),
            "an empty fleet is an answer, and must not read as an absent field"
        );

        let found = RpcError::application_with_candidates(
            2001,
            "agent not detected",
            "agent_not_detected",
            "the named agent is not detected",
            None,
            vec![meltemi_proto::AgentCandidate {
                id: "mock-agent".into(),
                detected: true,
                install_state: meltemi_proto::FleetInstallState::Ready,
                remedy: None,
                remedy_command: None,
            }],
        );
        let data = found.data.expect("application errors carry data");
        assert_eq!(data["candidates"][0]["id"], "mock-agent");
        assert_eq!(data["candidates"][0]["installState"], "ready");
    }

    #[tokio::test]
    async fn malformed_line_gets_parse_error_and_connection_survives() {
        let (peer, mut incoming, theirs) = pair();
        let (read, mut write) = tokio::io::split(theirs);
        let mut reader = BufReader::new(read);

        write.write_all(b"this is not json\n").await.unwrap();
        let err = read_json(&mut reader).await;
        assert_eq!(err["error"]["code"], codes::PARSE_ERROR);
        assert_eq!(err["id"], Value::Null);

        // The connection is still alive: a valid request flows through.
        write
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"status\"}\n")
            .await
            .unwrap();
        match incoming.recv().await.expect("incoming open") {
            Incoming::Request { id, method, .. } => {
                assert_eq!(id, json!(7));
                assert_eq!(method, "status");
                peer.respond(id, Ok(json!({"ok": true})));
            }
            other => panic!("unexpected message: {other:?}"),
        }
        let resp = read_json(&mut reader).await;
        assert_eq!(resp["result"]["ok"], true);
    }

    #[tokio::test]
    async fn invalid_request_shapes_are_rejected() {
        let (_peer, _incoming, theirs) = pair();
        let (read, mut write) = tokio::io::split(theirs);
        let mut reader = BufReader::new(read);

        // An array (batch) is not supported.
        write.write_all(b"[1,2,3]\n").await.unwrap();
        let err = read_json(&mut reader).await;
        assert_eq!(err["error"]["code"], codes::INVALID_REQUEST);

        // Missing jsonrpc version, with id: error echoes the id.
        write
            .write_all(b"{\"id\":3,\"method\":\"status\"}\n")
            .await
            .unwrap();
        let err = read_json(&mut reader).await;
        assert_eq!(err["error"]["code"], codes::INVALID_REQUEST);
        assert_eq!(err["id"], json!(3));
    }

    #[tokio::test]
    async fn outgoing_requests_route_responses_back() {
        let (peer, _incoming, theirs) = pair();
        let (read, mut write) = tokio::io::split(theirs);
        let mut reader = BufReader::new(read);

        let request = tokio::spawn({
            let peer = peer.clone();
            async move { peer.request("permission/request", &json!({"x": 1})).await }
        });

        let sent = read_json(&mut reader).await;
        assert_eq!(sent["method"], "permission/request");
        let id = sent["id"].clone();
        let response = json!({"jsonrpc": "2.0", "id": id, "result": {"granted": true}});
        write
            .write_all(format!("{response}\n").as_bytes())
            .await
            .unwrap();

        let result = request.await.unwrap().expect("response ok");
        assert_eq!(result["granted"], true);
    }

    #[tokio::test]
    async fn notifications_do_not_expect_responses() {
        let (_peer, mut incoming, theirs) = pair();
        let (_read, mut write) = tokio::io::split(theirs);
        write
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"session/cancel\",\"params\":{\"sessionId\":\"s\"}}\n")
            .await
            .unwrap();
        match incoming.recv().await.expect("incoming open") {
            Incoming::Notification { method, params } => {
                assert_eq!(method, "session/cancel");
                assert_eq!(params["sessionId"], "s");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn connection_close_fails_inflight_requests() {
        let (peer, _incoming, theirs) = pair();
        let request = tokio::spawn({
            let peer = peer.clone();
            async move { peer.request("status", &json!({})).await }
        });
        // Give the request a chance to be written, then drop the other end.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        drop(theirs);
        let result = request.await.unwrap();
        assert!(result.is_err(), "in-flight request must fail on close");
    }
}
