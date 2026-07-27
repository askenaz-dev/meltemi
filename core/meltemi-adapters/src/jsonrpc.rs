// SPDX-License-Identifier: Apache-2.0

//! A JSON-RPC 2.0 peer over a provider's newline-delimited stdio
//! (adaptadores-propios-acp task 2.3).
//!
//! The server dialect is bidirectional: the adapter asks (start a turn,
//! interrupt one), the server asks back (may I run this, may I write that), and
//! the server streams notifications the whole time. That needs exactly three
//! things, and this module is them: correlate answers with the requests that
//! earned them, hand everything else to whoever is listening, and never make
//! the caller of one of those wait on the other.
//!
//! Two deliberate readings of the protocol:
//!
//! - **A message is classified by shape, not by its `jsonrpc` member.** The
//!   official CLI omits that member from its responses (verified against
//!   0.77.0). Demanding it would mean refusing every real session while every
//!   fixture stayed green — the worst possible place to be strict. What the
//!   adapter *sends* carries it, because the specification says so.
//! - **A line that is not a JSON-RPC message is surfaced, not swallowed.**
//!   Provider CLIs print banners and warnings; erasing them would destroy
//!   evidence exactly when a session is going wrong.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicI64, Ordering};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::ndjson::{Frame, FrameReader, FrameWriter};

/// Something the provider said that was not an answer to a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    /// A notification: nothing to answer.
    Notification {
        /// The method.
        method: String,
        /// Its parameters, or `null` when it carried none.
        params: Value,
    },
    /// A request: it **must** be answered, or the provider waits forever.
    Request {
        /// The id to answer with, echoed verbatim — JSON-RPC allows strings.
        id: Value,
        /// The method.
        method: String,
        /// Its parameters, or `null` when it carried none.
        params: Value,
    },
    /// A line that is not a JSON-RPC message at all.
    Noise(String),
}

/// Why a call on the peer did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerError {
    /// The provider answered with a JSON-RPC error.
    Rpc {
        /// The code it chose.
        code: i64,
        /// What it said.
        message: String,
    },
    /// The conversation with the provider ended or failed.
    Transport(String),
}

impl std::fmt::Display for PeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerError::Rpc { code, message } => {
                write!(f, "the provider refused it ({code}): {message}")
            }
            PeerError::Transport(detail) => f.write_str(detail),
        }
    }
}

/// The requests still waiting for an answer.
///
/// Shared with the reader task, and free of the writer's type so the reader
/// never has to know what it is writing to.
#[derive(Default)]
struct Pending {
    waiters: StdMutex<HashMap<i64, oneshot::Sender<Result<Value, PeerError>>>>,
}

impl Pending {
    fn insert(&self, id: i64, waiter: oneshot::Sender<Result<Value, PeerError>>) {
        self.lock().insert(id, waiter);
    }

    fn take(&self, id: i64) -> Option<oneshot::Sender<Result<Value, PeerError>>> {
        self.lock().remove(&id)
    }

    /// Fails everything still waiting. Called once the provider's output ends:
    /// a caller blocked on an answer that can never arrive is a hung session.
    fn fail_all(&self, reason: &str) {
        for (_, waiter) in self.lock().drain() {
            let _ = waiter.send(Err(PeerError::Transport(reason.to_string())));
        }
    }

    fn lock(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<i64, oneshot::Sender<Result<Value, PeerError>>>> {
        self.waiters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// One end of a JSON-RPC conversation with a provider process.
pub struct Peer<W> {
    writer: Mutex<FrameWriter<W>>,
    pending: Arc<Pending>,
    next_id: AtomicI64,
}

impl<W: AsyncWrite + Unpin + Send> Peer<W> {
    /// Starts the conversation: takes over both halves of the provider's stdio
    /// and returns the peer plus everything the provider says on its own.
    ///
    /// `first_id` is where this peer's request numbering starts, so it can
    /// continue after a handshake that was done by hand.
    pub fn start<R: AsyncRead + Unpin + Send + 'static>(
        writer: FrameWriter<W>,
        mut reader: FrameReader<R>,
        first_id: i64,
    ) -> (Self, mpsc::UnboundedReceiver<Incoming>) {
        let pending = Arc::new(Pending::default());
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn({
            let pending = pending.clone();
            async move {
                loop {
                    match reader.next_frame().await {
                        Ok(Some(Frame::Json(message))) => {
                            if let Some(incoming) = route(&pending, message)
                                && tx.send(incoming).is_err()
                            {
                                // Nobody is listening any more: the session is
                                // gone, and so is the reason to keep reading.
                                break;
                            }
                        }
                        Ok(Some(Frame::Unparsed(line))) => {
                            if tx.send(Incoming::Noise(line)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            pending.fail_all("the provider closed its output");
                            break;
                        }
                        Err(error) => {
                            pending.fail_all(&format!(
                                "the provider's output could not be read ({error})"
                            ));
                            break;
                        }
                    }
                }
            }
        });

        (
            Self {
                writer: Mutex::new(writer),
                pending,
                next_id: AtomicI64::new(first_id),
            },
            rx,
        )
    }

    /// Asks the provider and waits for its answer.
    ///
    /// # Errors
    ///
    /// Returns the provider's own error, or the transport failure that means
    /// there will never be an answer.
    pub async fn request<P: Serialize>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<Value, PeerError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        // Registered before it is sent: an answer that arrived while we were
        // still writing would otherwise find nobody waiting for it.
        self.pending.insert(id, tx);

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": encode(params)?,
        });
        if let Err(error) = self.send(&message).await {
            self.pending.take(id);
            return Err(error);
        }

        rx.await.unwrap_or_else(|_| {
            Err(PeerError::Transport(
                "the conversation with the provider ended before it answered".into(),
            ))
        })
    }

    /// Tells the provider something. Nothing answers a notification.
    ///
    /// # Errors
    ///
    /// Returns the transport failure when it cannot be written.
    pub async fn notify<P: Serialize>(&self, method: &str, params: &P) -> Result<(), PeerError> {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": encode(params)?,
        }))
        .await
    }

    /// Answers a request the provider made.
    ///
    /// # Errors
    ///
    /// Returns the transport failure when it cannot be written.
    pub async fn respond<P: Serialize>(&self, id: &Value, result: &P) -> Result<(), PeerError> {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": encode(result)?,
        }))
        .await
    }

    /// Answers a request the provider made with an error.
    ///
    /// # Errors
    ///
    /// Returns the transport failure when it cannot be written.
    pub async fn respond_with_error(
        &self,
        id: &Value,
        code: i64,
        message: &str,
    ) -> Result<(), PeerError> {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message},
        }))
        .await
    }

    /// Closes the provider's input: for this wire, end of input is the polite
    /// way to say the conversation is over.
    pub async fn close_input(&self) {
        let _ = self.writer.lock().await.close().await;
    }

    async fn send(&self, message: &Value) -> Result<(), PeerError> {
        self.writer
            .lock()
            .await
            .write_frame(message)
            .await
            .map_err(|error| {
                PeerError::Transport(format!(
                    "the provider's input could not be written ({error})"
                ))
            })
    }
}

fn encode<P: Serialize>(params: &P) -> Result<Value, PeerError> {
    serde_json::to_value(params).map_err(|error| {
        PeerError::Transport(format!("the message could not be encoded ({error})"))
    })
}

/// Sorts one message: an answer goes to whoever is waiting for it, anything
/// else goes out to the session.
fn route(pending: &Pending, message: Value) -> Option<Incoming> {
    let id = message.get("id").cloned();
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_string);
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    match (id, method) {
        (Some(id), Some(method)) => Some(Incoming::Request { id, method, params }),
        (None, Some(method)) => Some(Incoming::Notification { method, params }),
        (Some(id), None) => {
            let Some(waiter) = id.as_i64().and_then(|id| pending.take(id)) else {
                // An answer to a request this peer never made — or made and gave
                // up on. It is not the session's business, and inventing a
                // meaning for it would be worse than saying so.
                return Some(Incoming::Noise(message.to_string()));
            };
            let answer = match (message.get("result"), message.get("error")) {
                (Some(result), _) => Ok(result.clone()),
                (None, Some(error)) => Err(PeerError::Rpc {
                    code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                    message: error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("no message")
                        .to_string(),
                }),
                (None, None) => Err(PeerError::Transport(
                    "the provider answered with neither a result nor an error".into(),
                )),
            };
            let _ = waiter.send(answer);
            None
        }
        (None, None) => Some(Incoming::Noise(message.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

    /// The provider's end of a duplex pipe: what it hears, and what it says —
    /// written by hand so a test can also say something that is not protocol.
    struct Provider {
        heard: tokio::io::Lines<BufReader<DuplexStream>>,
        says: DuplexStream,
    }

    /// A peer and the provider it is talking to, kept apart so a test can hold
    /// one while it drives the other.
    fn wire() -> (
        Peer<DuplexStream>,
        mpsc::UnboundedReceiver<Incoming>,
        Provider,
    ) {
        let (adapter_out, provider_in) = tokio::io::duplex(4096);
        let (provider_out, adapter_in) = tokio::io::duplex(4096);
        let (peer, incoming) = Peer::start(
            FrameWriter::new(adapter_out),
            FrameReader::new(adapter_in),
            1,
        );
        (
            peer,
            incoming,
            Provider {
                heard: BufReader::new(provider_in).lines(),
                says: provider_out,
            },
        )
    }

    impl Provider {
        async fn next_heard(&mut self) -> Value {
            let line = self
                .heard
                .next_line()
                .await
                .unwrap()
                .expect("the adapter said something");
            serde_json::from_str(&line).expect("and it was JSON")
        }

        async fn say(&mut self, message: &Value) {
            self.say_raw(&message.to_string()).await;
        }

        async fn say_raw(&mut self, line: &str) {
            self.says.write_all(line.as_bytes()).await.unwrap();
            self.says
                .write_all(
                    b"
",
                )
                .await
                .unwrap();
            self.says.flush().await.unwrap();
        }
    }

    #[tokio::test]
    async fn an_answer_reaches_the_request_that_earned_it_and_nothing_else() {
        // Two requests in flight at once, answered out of order: if answers were
        // matched by arrival rather than by id, a turn would read somebody
        // else's result and never know.
        let (peer, _incoming, mut provider) = wire();
        let one = json!({"n": 1});
        let two = json!({"n": 2});
        let first = peer.request("turn/start", &one);
        let second = peer.request("thread/start", &two);

        let answers = async {
            let a = provider.next_heard().await;
            let b = provider.next_heard().await;
            assert_eq!(a["jsonrpc"], "2.0", "what we send carries the member");
            assert_ne!(a["id"], b["id"], "each request gets its own id");
            // Answered in reverse, and — as the real CLI does — with no
            // `jsonrpc` member at all.
            provider
                .say(&json!({"id": b["id"], "result": {"who": "second"}}))
                .await;
            provider
                .say(&json!({"id": a["id"], "result": {"who": "first"}}))
                .await;
        };

        let (first, second, ()) = tokio::join!(first, second, answers);
        assert_eq!(first.unwrap()["who"], "first");
        assert_eq!(second.unwrap()["who"], "second");
    }

    #[tokio::test]
    async fn what_the_provider_says_on_its_own_reaches_the_session() {
        let (_peer, mut incoming, mut provider) = wire();
        provider
            .say(&json!({"method": "turn/started", "params": {"turnId": "t1"}}))
            .await;
        provider
            .say(&json!({"id": "ask-1", "method": "approve", "params": {}}))
            .await;
        provider.say_raw("a banner").await;

        assert_eq!(
            incoming.recv().await.unwrap(),
            Incoming::Notification {
                method: "turn/started".into(),
                params: json!({"turnId": "t1"}),
            }
        );
        assert_eq!(
            incoming.recv().await.unwrap(),
            Incoming::Request {
                // A string id, echoed verbatim: JSON-RPC allows it and an
                // adapter that answered with a number would answer nobody.
                id: json!("ask-1"),
                method: "approve".into(),
                params: json!({}),
            }
        );
        assert_eq!(
            incoming.recv().await.unwrap(),
            Incoming::Noise("a banner".into()),
            "a line that is not protocol is surfaced, not swallowed"
        );
    }

    #[tokio::test]
    async fn a_provider_that_goes_away_fails_the_waiting_instead_of_hanging_them() {
        // The one failure a bridge must never have: a turn waiting forever on a
        // process that is not there any more.
        let (peer, _incoming, mut provider) = wire();
        let nothing = json!({});
        let asking = peer.request("turn/start", &nothing);
        let closing = async {
            let _ = provider.next_heard().await;
            drop(provider);
        };
        let (answer, ()) = tokio::join!(asking, closing);
        assert!(
            matches!(answer, Err(PeerError::Transport(_))),
            "{answer:?} should be a transport failure"
        );
    }

    #[tokio::test]
    async fn a_provider_error_arrives_as_the_providers_error_not_as_silence() {
        let (peer, _incoming, mut provider) = wire();
        let nothing = json!({});
        let asking = peer.request("turn/start", &nothing);
        let refusing = async {
            let sent = provider.next_heard().await;
            provider
                .say(&json!({"id": sent["id"], "error": {"code": -32602, "message": "no such thread"}}))
                .await;
        };
        let (answer, ()) = tokio::join!(asking, refusing);
        assert_eq!(
            answer,
            Err(PeerError::Rpc {
                code: -32602,
                message: "no such thread".into(),
            })
        );
    }
}
