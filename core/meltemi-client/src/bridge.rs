// SPDX-License-Identifier: Apache-2.0

//! The daemon bridge (gui-tauri-paridad 1.4, design D1; shared since
//! companero-que-conecta D1): a background actor that owns the only
//! connection to `meltemid` — dial, mandatory `initialize`, reconnection with
//! backoff — and serves a surface through two channels: commands in (RPC
//! passthrough) and events out (connection state, daemon-initiated traffic).
//! The surface never touches a socket.
//!
//! One actor, two dialers. The desktop dials the local endpoint — or the
//! command the user named — through [`LocalDialer`]; the mobile companion
//! dials its own SSH channel. What differs between them is [`Dialer::dial`]
//! and nothing else: the loop, the backoff, the held permission pushes and
//! the contract towards the surface are this one piece of code.
//!
//! Shape mirrors the TUI connection actor (`tui/src/shell/conn.rs`) so every
//! surface gives the same guarantees: noisy disconnection, backoff, and
//! permission pushes held unanswered (the daemon's timeout denies; the tray
//! resolves via `permission/decide`).

use std::future::Future;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::bootstrap;
use crate::notices;
use crate::rpc::{Incoming, Peer, RpcError};
use crate::transport::Stream;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, StatusResult, methods};

const MIN_BACKOFF: Duration = Duration::from_millis(200);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
const REFRESH_EVERY: Duration = Duration::from_secs(2);

/// Connection state pushed to the webview as the `daemon:state` event.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ConnState {
    Connecting,
    #[serde(rename_all = "camelCase")]
    Connected {
        version: String,
        uptime_seconds: u64,
        sessions: usize,
        endpoint: String,
    },
    /// The daemon cannot be reached or started. Carries the endpoint so the
    /// surface can show the socket path with the diagnosis (gui-shell
    /// "Fallo de arranque diagnosticado").
    #[serde(rename_all = "camelCase")]
    Unreachable {
        detail: String,
        endpoint: String,
    },
    /// The far side answered and said no, and no amount of waiting changes
    /// that: the server rejected this client's key, or the host's key is not
    /// the one remembered. The actor stops on it rather than retrying — a
    /// screen that showed a changed host key as "retrying in 4 s" would offer
    /// exactly what the spec forbids (companero-que-conecta D3).
    #[serde(rename_all = "camelCase")]
    Refused {
        kind: Refused,
        detail: String,
        endpoint: String,
    },
}

/// Why a dial was refused for good, in the words the surface uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Refused {
    /// The host's key is not the one fixed on first use.
    HostKeyChanged,
    /// The server did not accept this client's key.
    Rejected,
}

/// How a dial failed.
#[derive(Debug)]
pub enum Dial {
    /// The far side could not be reached; the actor backs off and retries.
    Transient(String),
    /// The far side refused; the actor publishes it and stops.
    Terminal { kind: Refused, detail: String },
}

/// Whoever opens the stream the actor serves.
///
/// The desktop's is [`LocalDialer`]; the mobile companion's opens its SSH
/// channel. An implementation owns whatever keeps its stream alive — a session
/// handle, say — and is dropped with the actor.
pub trait Dialer: Send {
    /// Opens a fresh stream to the daemon.
    fn dial(&mut self) -> impl Future<Output = Result<Stream, Dial>> + Send;
    /// What the state shows as the endpoint: a socket path, or `user@host`.
    fn shown(&self) -> String;
    /// `BridgeCommand::Switch` lands here: talk to this target from now on.
    fn switch(&mut self, to: String);
}

/// The desktop's dialer: connect-or-start at a local endpoint, or the command
/// the user named (via-ssh-del-cliente). Never refuses for good — a local
/// daemon that is not there is one that may be there in a moment.
#[derive(Debug, Clone)]
pub struct LocalDialer {
    endpoint: String,
}

impl LocalDialer {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}

impl Dialer for LocalDialer {
    async fn dial(&mut self) -> Result<Stream, Dial> {
        bootstrap::connect_or_start(&self.endpoint)
            .await
            .map_err(|error| Dial::Transient(error.to_string()))
    }

    fn shown(&self) -> String {
        self.endpoint.clone()
    }

    fn switch(&mut self, to: String) {
        self.endpoint = to;
    }
}

/// Emits the waiting notice when a notification says something started waiting
/// for a person.
///
/// Reads the daemon's own pushes and nothing else: the queue's snapshot says how
/// many permissions wait, and the live state says when a session parked. A wait
/// that has gone away emits nothing, which is what keeps a resolved queue from
/// spending the floor the next real wait needs.
fn notice_for(notices: &mut notices::Emitter, method: &str, params: &serde_json::Value) {
    use notices::Waiting;

    let (waiting, session, count) = match method {
        methods::PERMISSION_CHANGED => {
            let pending = params
                .get("pending")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            if pending == 0 {
                // The queue emptied: somebody decided. Nothing to say.
                notices.resolved();
                return;
            }
            let session = params["pending"][0]["sessionId"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            (
                Waiting::Permission,
                session,
                u32::try_from(pending).unwrap_or(u32::MAX),
            )
        }
        methods::SESSION_STATE => {
            let state = params.get("state").and_then(serde_json::Value::as_str);
            if state != Some("waiting_instruction") {
                return;
            }
            let session = params
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            (Waiting::Instruction, session, 1)
        }
        _ => return,
    };
    notices.waiting(waiting, &session, count);
}

/// An event pushed from the actor to the host (the Tauri layer forwards it to
/// the webview; tests consume it directly).
#[derive(Debug, Clone, Serialize)]
pub enum BridgeEvent {
    State(ConnState),
    /// A daemon-initiated notification or held permission push, forwarded
    /// verbatim: `{method, params}`.
    Incoming {
        method: String,
        params: Value,
    },
}

/// A structured error for the webview: the contract's `ErrorData` when the
/// daemon answered, or a local `unreachable` when it could not be reached.
/// Unlike the CLI (which flattens to exit codes), the surface gets the remedy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    /// JSON-RPC error code when the daemon answered; absent for local errors.
    pub code: Option<i64>,
    pub message: String,
    /// `unreachable` for local connection failures, the contract's
    /// `data.kind` when present, `contract` otherwise.
    pub kind: String,
    pub detail: Option<String>,
    pub remedy: Option<String>,
    /// The fleet candidates a resolution refusal carries (2000/2001). Passed
    /// through so the surface can offer a choice instead of transcribing the
    /// lament; ids, detection and remedies only — the daemon never puts
    /// anything shaped like a secret in here (constitution §2).
    ///
    /// Boxed on purpose: this payload exists for one class of refusal, and an
    /// inline vector puts every `Result<_, BridgeError>` in the crate over
    /// clippy's large-error threshold to carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Box<Vec<meltemi_proto::AgentCandidate>>>,
}

impl BridgeError {
    pub fn unreachable(detail: impl Into<String>) -> Self {
        Self {
            code: None,
            message: "daemon unreachable".into(),
            kind: "unreachable".into(),
            detail: Some(detail.into()),
            remedy: None,
            candidates: None,
        }
    }
}

impl From<RpcError> for BridgeError {
    fn from(error: RpcError) -> Self {
        let get = |key: &str| {
            error
                .data
                .as_ref()
                .and_then(|d| d.get(key))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        // A payload that does not parse as candidates is simply absent: the
        // surface then shows the prose, which is what it did before.
        let candidates = error
            .data
            .as_ref()
            .and_then(|d| d.get("candidates"))
            .and_then(|value| serde_json::from_value(value.clone()).ok());
        Self {
            code: Some(error.code),
            kind: get("kind").unwrap_or_else(|| "contract".into()),
            detail: get("detail"),
            remedy: get("remedy"),
            candidates,
            message: error.message,
        }
    }
}

/// A command from the host into the actor.
#[derive(Debug)]
pub enum BridgeCommand {
    /// Generic RPC passthrough: send `method`/`params` to the daemon and
    /// reply with its result. Requests run concurrently; a slow turn never
    /// blocks the actor.
    Request {
        method: String,
        params: Value,
        reply: oneshot::Sender<Result<Value, BridgeError>>,
    },
    /// Fire-and-forget notification (e.g. `session/cancel`). Lost while
    /// disconnected — there is nothing meaningful to notify then.
    Notify { method: String, params: Value },
    /// Talk to another machine from now on (maquinas-remotas design D2).
    ///
    /// Handled wherever the actor can be: in a live connection it drops it so
    /// the loop dials the new endpoint, and during a backoff it cuts the wait
    /// short — a user who just chose a machine should not sit through the
    /// remains of the previous one's retry.
    Switch { endpoint: String },
}

/// Runs the bridge until the host drops its command sender, or until a dial
/// is refused for good.
///
/// `notices` is the waiting-notice emitter when this host has one: the desktop
/// runs the user's own command; the phone has nothing to run, and the notice
/// that brings it back comes from the user's infrastructure. The emitter is
/// owned here, across reconnections, on purpose: the record of when the last
/// notice went out has to survive a daemon that drops and comes back, or the
/// floor would reset and let a burst through (aviso-de-espera-opt-in D5).
/// `client` is the name this surface gives the daemon in `initialize`.
pub async fn bridge_actor<D: Dialer>(
    mut dialer: D,
    mut commands: mpsc::UnboundedReceiver<BridgeCommand>,
    events: mpsc::UnboundedSender<BridgeEvent>,
    mut notices: Option<notices::Emitter>,
    client: &'static str,
) {
    let mut backoff = MIN_BACKOFF;
    loop {
        let _ = events.send(BridgeEvent::State(ConnState::Connecting));
        let stream = match dialer.dial().await {
            Ok(stream) => stream,
            Err(Dial::Terminal { kind, detail }) => {
                // Published once, and then nothing: what a person does about it
                // is the surface's to offer, and this loop does not dial past it.
                let _ = events.send(BridgeEvent::State(ConnState::Refused {
                    kind,
                    detail,
                    endpoint: dialer.shown(),
                }));
                return;
            }
            Err(Dial::Transient(detail)) => {
                let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
                    detail: detail.clone(),
                    endpoint: dialer.shown(),
                }));
                match drain_or_stop(&mut commands, backoff, &detail).await {
                    Waited::HostGone => return,
                    // A machine chosen during the backoff starts its own attempt
                    // immediately and from the first delay: the new machine
                    // inherits nothing from the old one's bad luck.
                    Waited::Switch(next) => {
                        dialer.switch(next);
                        backoff = MIN_BACKOFF;
                        continue;
                    }
                    Waited::Elapsed => {}
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        backoff = MIN_BACKOFF;

        let shown = dialer.shown();
        match serve_connection(stream, &mut commands, &events, &shown, &mut notices, client).await {
            ConnExit::HostGone => return,
            ConnExit::Disconnected => continue,
            ConnExit::Switch(next) => {
                dialer.switch(next);
                continue;
            }
        }
    }
}

enum ConnExit {
    HostGone,
    Disconnected,
    /// The user chose another machine; dial this one instead.
    Switch(String),
}

/// How a backoff ended.
enum Waited {
    /// The wait ran out; try the same endpoint again.
    Elapsed,
    /// The host dropped its sender.
    HostGone,
    /// The user chose another machine while we were waiting.
    Switch(String),
}

async fn serve_connection(
    stream: Stream,
    commands: &mut mpsc::UnboundedReceiver<BridgeCommand>,
    events: &mpsc::UnboundedSender<BridgeEvent>,
    endpoint: &str,
    notices: &mut Option<notices::Emitter>,
    client: &str,
) -> ConnExit {
    let (peer, mut incoming) = Peer::start(stream);

    if let Err(error) = peer
        .request(methods::INITIALIZE, &init_params(client))
        .await
    {
        let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
            detail: format!("initialize failed: {error}"),
            endpoint: endpoint.to_string(),
        }));
        peer.close();
        return ConnExit::Disconnected;
    }
    push_status(&peer, events, endpoint).await;

    let mut ticker = tokio::time::interval(REFRESH_EVERY);
    ticker.tick().await; // consume the immediate first tick

    loop {
        tokio::select! {
            message = incoming.recv() => match message {
                None => {
                    peer.close();
                    return ConnExit::Disconnected;
                }
                Some(Incoming::Request { id, method, params }) if method == methods::PERMISSION_REQUEST => {
                    // Held unanswered on purpose: the daemon's queue drives the
                    // tray (permission/changed) and its timeout denies; the tray
                    // resolves via permission/decide. Forward for visibility.
                    let _ = events.send(BridgeEvent::Incoming { method, params });
                    let _ = id;
                }
                Some(Incoming::Request { id, method, .. }) => {
                    peer.respond(id, Err(RpcError::method_not_found(&method)));
                }
                Some(Incoming::Notification { method, params }) => {
                    // The waiting notice, decided here and not in the webview: a
                    // view that could run commands would be a surface nobody
                    // needs to open, and this process already talks to the
                    // operating system (aviso-de-espera-opt-in design D2). A
                    // host without an emitter — the phone — has nothing to run.
                    if let Some(notices) = notices.as_mut() {
                        notice_for(notices, &method, &params);
                    }
                    let _ = events.send(BridgeEvent::Incoming { method, params });
                }
            },
            command = commands.recv() => match command {
                None => {
                    peer.close();
                    return ConnExit::HostGone;
                }
                Some(BridgeCommand::Request { method, params, reply }) => {
                    // Concurrent by design: a long-running turn must not block
                    // the ticker or other requests.
                    let peer = peer.clone();
                    tokio::spawn(async move {
                        let result = peer
                            .request(&method, &params)
                            .await
                            .map_err(BridgeError::from);
                        let _ = reply.send(result);
                    });
                }
                Some(BridgeCommand::Notify { method, params }) => {
                    peer.notify(&method, &params);
                }
                Some(BridgeCommand::Switch { endpoint: next }) => {
                    // Dropped, not drained: what is in flight belongs to the
                    // machine being left, and an answer from it arriving after
                    // the switch would be an answer about the wrong computer.
                    peer.close();
                    return ConnExit::Switch(next);
                }
            },
            _ = ticker.tick() => push_status(&peer, events, endpoint).await,
        }
    }
}

/// Sleeps for `backoff`; requests arriving meanwhile fail fast as
/// unreachable. Returns `true` when the host dropped its sender.
async fn drain_or_stop(
    commands: &mut mpsc::UnboundedReceiver<BridgeCommand>,
    backoff: Duration,
    detail: &str,
) -> Waited {
    let sleep = tokio::time::sleep(backoff);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => return Waited::Elapsed,
            command = commands.recv() => match command {
                None => return Waited::HostGone,
                Some(BridgeCommand::Request { reply, .. }) => {
                    let _ = reply.send(Err(BridgeError::unreachable(detail)));
                }
                Some(BridgeCommand::Notify { .. }) => {}
                Some(BridgeCommand::Switch { endpoint }) => return Waited::Switch(endpoint),
            },
        }
    }
}

async fn push_status(peer: &Peer, events: &mpsc::UnboundedSender<BridgeEvent>, endpoint: &str) {
    match peer.request(methods::STATUS, &json!({})).await {
        Ok(value) => {
            if let Ok(status) = serde_json::from_value::<StatusResult>(value) {
                let _ = events.send(BridgeEvent::State(ConnState::Connected {
                    version: status.daemon_version,
                    uptime_seconds: status.uptime_seconds,
                    sessions: status.sessions.len(),
                    endpoint: endpoint.to_string(),
                }));
            }
        }
        Err(error) => {
            let _ = events.send(BridgeEvent::State(ConnState::Unreachable {
                detail: error.to_string(),
                endpoint: endpoint.to_string(),
            }));
        }
    }
}

fn init_params(client: &str) -> InitializeParams {
    InitializeParams {
        protocol_version: PROTOCOL_VERSION,
        client: PeerInfo {
            name: client.into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    /// A daemon that answers `initialize` and `status`, and nothing else.
    fn fake_daemon() -> Stream {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(theirs);
            let mut lines = BufReader::new(read).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(request) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let result = match request["method"].as_str() {
                    Some(method) if method == methods::INITIALIZE => json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "daemon": { "name": "fake", "version": "0" },
                    }),
                    Some(method) if method == methods::STATUS => json!({
                        "daemonVersion": "0.0.0-fake",
                        "uptimeSeconds": 1,
                        "sessions": [],
                    }),
                    _ => Value::Null,
                };
                let reply = json!({ "jsonrpc": "2.0", "id": request["id"], "result": result });
                if write
                    .write_all(format!("{reply}\n").as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        Box::new(ours)
    }

    /// A dialer that follows a script: each entry is one dial's outcome, and
    /// past the end of the script every dial hands over a fake daemon.
    struct Scripted {
        outcomes: VecDeque<Result<(), Dial>>,
        shown: String,
        dials: Arc<AtomicUsize>,
    }

    impl Dialer for Scripted {
        async fn dial(&mut self) -> Result<Stream, Dial> {
            self.dials.fetch_add(1, Ordering::SeqCst);
            match self.outcomes.pop_front() {
                Some(Ok(())) | None => Ok(fake_daemon()),
                Some(Err(dial)) => Err(dial),
            }
        }

        fn shown(&self) -> String {
            self.shown.clone()
        }

        fn switch(&mut self, to: String) {
            self.shown = to;
        }
    }

    type Actor = (
        mpsc::UnboundedSender<BridgeCommand>,
        mpsc::UnboundedReceiver<BridgeEvent>,
        tokio::task::JoinHandle<()>,
        Arc<AtomicUsize>,
    );

    fn start(outcomes: Vec<Result<(), Dial>>, shown: &str) -> Actor {
        let dials = Arc::new(AtomicUsize::new(0));
        let dialer = Scripted {
            outcomes: outcomes.into(),
            shown: shown.into(),
            dials: Arc::clone(&dials),
        };
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, events) = mpsc::unbounded_channel();
        let actor = tokio::spawn(bridge_actor(dialer, command_rx, event_tx, None, "test"));
        (command_tx, events, actor, dials)
    }

    async fn next_state(events: &mut mpsc::UnboundedReceiver<BridgeEvent>) -> ConnState {
        loop {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .expect("an event in time")
                .expect("the actor is alive");
            if let BridgeEvent::State(state) = event {
                return state;
            }
        }
    }

    // Scenario: Un fallo de transporte reintenta con backoff
    #[tokio::test]
    async fn a_transient_failure_backs_off_and_fails_waiting_requests_fast() {
        let (commands, mut events, actor, dials) =
            start(vec![Err(Dial::Transient("nobody home".into()))], "local");

        assert!(matches!(
            next_state(&mut events).await,
            ConnState::Connecting
        ));
        match next_state(&mut events).await {
            ConnState::Unreachable { detail, endpoint } => {
                assert_eq!(detail, "nobody home");
                assert_eq!(endpoint, "local");
            }
            other => panic!("expected unreachable, got {other:?}"),
        }

        // A request during the backoff fails fast, as unreachable — nobody
        // waits out somebody else's retry.
        let (reply, response) = oneshot::channel();
        commands
            .send(BridgeCommand::Request {
                method: "status".into(),
                params: json!({}),
                reply,
            })
            .expect("the actor is listening");
        let answer = tokio::time::timeout(Duration::from_millis(100), response)
            .await
            .expect("fails fast")
            .expect("replied");
        assert_eq!(answer.expect_err("unreachable").kind, "unreachable");

        // Then the retry succeeds and the daemon's own status comes through.
        assert!(matches!(
            next_state(&mut events).await,
            ConnState::Connecting
        ));
        match next_state(&mut events).await {
            ConnState::Connected {
                version, endpoint, ..
            } => {
                assert_eq!(version, "0.0.0-fake");
                assert_eq!(endpoint, "local");
            }
            other => panic!("expected connected, got {other:?}"),
        }
        assert_eq!(dials.load(Ordering::SeqCst), 2);

        drop(commands);
        tokio::time::timeout(Duration::from_secs(5), actor)
            .await
            .expect("the actor stops when the host is gone")
            .expect("cleanly");
    }

    // Scenario: Un rechazo detiene el actor y lo publica
    #[tokio::test]
    async fn a_refusal_is_published_once_and_the_actor_stops_dialing() {
        let (commands, mut events, actor, dials) = start(
            vec![Err(Dial::Terminal {
                kind: Refused::HostKeyChanged,
                detail: "not that machine".into(),
            })],
            "ana@mac-studio",
        );

        assert!(matches!(
            next_state(&mut events).await,
            ConnState::Connecting
        ));
        match next_state(&mut events).await {
            ConnState::Refused {
                kind,
                detail,
                endpoint,
            } => {
                assert_eq!(kind, Refused::HostKeyChanged);
                assert_eq!(detail, "not that machine");
                assert_eq!(endpoint, "ana@mac-studio");
            }
            other => panic!("expected refused, got {other:?}"),
        }

        // The actor is gone on its own — with the host still holding its
        // sender — and it never dialled past the refusal.
        tokio::time::timeout(Duration::from_secs(5), actor)
            .await
            .expect("stops on its own")
            .expect("cleanly");
        assert_eq!(
            dials.load(Ordering::SeqCst),
            1,
            "a refusal is never dialled past"
        );
        assert!(
            events.recv().await.is_none(),
            "and nothing more is published"
        );
        drop(commands);
    }

    // Scenario: Un fallo de transporte reintenta con backoff
    #[tokio::test(start_paused = true)]
    async fn a_switch_during_the_backoff_cuts_the_wait_and_dials_the_new_target() {
        let (commands, mut events, actor, dials) = start(
            vec![
                Err(Dial::Transient("first is down".into())),
                Err(Dial::Transient("second is down".into())),
            ],
            "first",
        );
        let began = tokio::time::Instant::now();

        assert!(matches!(
            next_state(&mut events).await,
            ConnState::Connecting
        ));
        match next_state(&mut events).await {
            ConnState::Unreachable { endpoint, .. } => assert_eq!(endpoint, "first"),
            other => panic!("expected unreachable, got {other:?}"),
        }

        commands
            .send(BridgeCommand::Switch {
                endpoint: "second".into(),
            })
            .expect("the actor is listening");

        assert!(matches!(
            next_state(&mut events).await,
            ConnState::Connecting
        ));
        match next_state(&mut events).await {
            ConnState::Unreachable { detail, endpoint } => {
                assert_eq!(endpoint, "second");
                assert_eq!(detail, "second is down");
            }
            other => panic!("expected unreachable, got {other:?}"),
        }
        // The clock is paused: it only moves when every task is idle. Reaching
        // the second dial without it moving at all is the proof that the switch
        // cut the wait, rather than the backoff running out.
        assert_eq!(dials.load(Ordering::SeqCst), 2);
        assert!(
            began.elapsed() < MIN_BACKOFF,
            "the switch did not wait out the backoff"
        );

        drop(commands);
        tokio::time::timeout(Duration::from_secs(60), actor)
            .await
            .expect("the actor stops when the host is gone")
            .expect("cleanly");
    }
}
