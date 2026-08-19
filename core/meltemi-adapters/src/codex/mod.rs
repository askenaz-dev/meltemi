// SPDX-License-Identifier: Apache-2.0

//! The JSON-RPC server dialect (adaptadores-propios-acp design D6).
//!
//! One provider ships a documented server mode — JSON-RPC 2.0 with newline
//! delimitation over stdio, the same interface its own editor extension uses —
//! and this module is the translation between that surface and ACP. The
//! official binary is launched as a subprocess and nothing else: the pattern
//! the archived and community Rust adapters take, embedding the provider's
//! runtime as a library, would put the network and the provider's auth store
//! inside this process, which constitution §2 forbids however permissive the
//! licence is.

pub mod mapping;
pub mod permission;
pub mod version;
pub mod wire;

use std::sync::Mutex as StdMutex;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    Meta, NewSessionRequest, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    SessionId, SessionInfoUpdate, SessionNotification, SessionUpdate, StopReason,
};
use agent_client_protocol::{Client, ConnectionTo};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::ChildStdin;
use tokio::sync::{Mutex, mpsc, watch};

use crate::adapter::{AdapterSpec, Dialect, Opened, ProviderDialect, ProviderSession};
use crate::diagnostic::Refusal;
use crate::jsonrpc::{Incoming, Peer};
use crate::ndjson::Frame;
use crate::supervisor::{
    self, ChildProcess, ProcessControl, ProviderCommand, ProviderProcess, ShutdownPolicy,
    resolve_program, spawn,
};

/// What this binary announces over ACP and which CLI it pilots.
pub const SPEC: AdapterSpec = AdapterSpec {
    name: "meltemi-codex-acp",
    provider_layer: "the official `codex` CLI",
    provider_bin: "codex",
    dialect: Dialect::JsonRpcServer,
};

/// Points the adapter at a specific CLI binary instead of the one on the PATH.
///
/// It is how the end-to-end tests put a scripted wire where the provider would
/// be, and an escape hatch for a CLI installed somewhere the PATH does not
/// reach. It changes *which binary* is launched, never *what* is spoken to it.
pub const PROVIDER_BIN_ENV: &str = "MELTEMI_CODEX_BIN";

/// How long the handshake waits for the server to answer before refusing.
///
/// Generous, because a cold CLI on Windows can take seconds to come up; finite,
/// because a session that never opens and never fails is the worst of both.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// The id the handshake request carries. The adapter's own request numbering
/// starts after it.
const HANDSHAKE_ID: i64 = 1;

/// How long the server gets to acknowledge an interruption before the provider
/// is ended outright. A turn that will not stop must not keep the session — or
/// the worktree it runs in — hostage.
const INTERRUPT_GRACE: Duration = Duration::from_secs(5);

/// A deadline far enough away to mean "not yet". The turn loop's grace sits
/// here until an interruption moves it.
const NEVER: Duration = Duration::from_secs(60 * 60 * 24 * 365);

/// JSON-RPC's own code for a method the receiver does not implement. The server
/// asking something this adapter has no answer for gets the protocol's answer,
/// not an invented one.
const METHOD_NOT_FOUND: i64 = -32601;

/// The JSON-RPC server dialect: launches the official CLI in its server mode
/// and pilots it.
pub struct CodexDialect {
    /// The binary to launch, after the environment override.
    program: String,
    /// How long a provider gets to exit on its own when a session closes.
    shutdown: ShutdownPolicy,
}

impl Default for CodexDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexDialect {
    /// The dialect as the binary runs it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: resolve_program(std::env::var(PROVIDER_BIN_ENV).ok(), SPEC.provider_bin),
            shutdown: ShutdownPolicy::default(),
        }
    }
}

/// What was actually launched and what it turned out to be: the two facts a
/// session must be able to prove afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The binary the adapter launched, as it was resolved.
    pub program: String,
    /// The version read out of the handshake.
    pub version: String,
    /// The user agent the server announced, verbatim.
    pub user_agent: String,
}

impl Provenance {
    /// The provenance as ACP extension metadata.
    ///
    /// It travels in the `_meta` of a session update, which is the protocol's
    /// own extensibility point — not a private channel between this adapter and
    /// meltemid (design D7). The daemon records agent updates verbatim, so this
    /// lands in the session log like any other, and any third-party agent can
    /// say the same thing the same way.
    #[must_use]
    pub fn meta(&self) -> Meta {
        let mut meta = Meta::new();
        meta.insert(
            "meltemi".into(),
            json!({
                "providerBin": self.program,
                "providerVersion": self.version,
                "providerUserAgent": self.user_agent,
            }),
        );
        meta
    }
}

/// A session over the official CLI's server mode.
pub struct CodexSession {
    /// The conversation with the provider.
    peer: Peer<ChildStdin>,
    /// Everything the provider says on its own, consumed one turn at a time.
    incoming: Mutex<mpsc::UnboundedReceiver<Incoming>>,
    /// The process handle, so the session can end what it started. `None` once
    /// it has been shut down.
    control: Mutex<Option<ChildProcess>>,
    /// The turn in flight, and whether it has been asked to stop.
    turn: TurnControl,
    /// Where this session's updates go.
    stream: AcpStream,
    /// How long the provider gets to exit on its own.
    shutdown: ShutdownPolicy,
}

impl ProviderSession for CodexSession {
    fn begin_turn(&self) {
        self.turn.begin();
    }

    async fn run_turn(&self, prompt: PromptRequest) -> Result<StopReason, Refusal> {
        // One turn at a time is the bridge's own invariant, so holding the
        // incoming half for the length of a turn takes nothing from anyone.
        let mut incoming = self.incoming.lock().await;

        let params = wire::TurnStartParams {
            thread_id: self.turn.thread_id.clone(),
            input: vec![wire::UserInput::Text {
                text: mapping::prompt_text(&prompt),
            }],
            // Per turn, because that is the only place this provider's schema
            // defines it.
            effort: crate::session_effort(),
        };
        match drive_turn(
            &self.peer,
            &mut incoming,
            &self.turn,
            &self.stream,
            &self.stream.session_id,
            &params,
        )
        .await?
        {
            TurnOutcome::Ended(stop) => Ok(stop),
            TurnOutcome::Abandoned => {
                // The server was asked to stop and said nothing. It does not get
                // to hold the session — or the worktree — open while it decides.
                self.stream.note(&format!(
                    "the server did not acknowledge the interruption within {:?}; \
                     ending the provider",
                    self.turn.grace
                ));
                self.shutdown(self.shutdown).await;
                Ok(StopReason::Cancelled)
            }
        }
    }

    async fn interrupt(&self) {
        interrupt(&self.peer, &self.turn).await;
    }

    async fn shutdown(&self, policy: ShutdownPolicy) {
        self.peer.close_input().await;
        if let Some(mut control) = self.control.lock().await.take() {
            let _ = supervisor::end(&mut control, policy).await;
        }
    }
}

/// The turn in flight: its id once the server names it, and whether it has been
/// asked to stop.
///
/// A cancellation can arrive before the server has named the turn, which is
/// precisely when a naive adapter drops it on the floor: there is nothing to
/// name in the interruption yet. The intent is remembered instead, and sent the
/// moment the id is known.
struct TurnControl {
    thread_id: String,
    state: StdMutex<TurnState>,
    /// Watched by everything that must give way when an interruption is asked
    /// for: the turn loop, and any permission still waiting on a human. A
    /// `watch` and not a one-shot wake-up precisely because there is more than
    /// one of them, and each has to see it.
    interrupted: watch::Sender<bool>,
    /// How long the server gets to acknowledge an interruption. A field rather
    /// than a constant so a test can watch the whole "it will not stop" path
    /// without waiting out a human-scale timeout.
    grace: Duration,
}

#[derive(Debug, Default, Clone)]
struct TurnState {
    id: Option<String>,
    interrupt_requested: bool,
    interrupt_sent: bool,
}

impl TurnControl {
    fn new(thread_id: String, grace: Duration) -> Self {
        Self {
            thread_id,
            state: StdMutex::new(TurnState::default()),
            interrupted: watch::Sender::new(false),
            grace,
        }
    }

    fn with<T>(&self, change: impl FnOnce(&mut TurnState) -> T) -> T {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        change(&mut state)
    }

    /// A new turn starts with nothing remembered: a cancellation of the last
    /// one must not kill this one.
    ///
    /// Called from [`CodexSession::begin_turn`] and from nowhere else, and that
    /// is load-bearing rather than tidy. This reset wipes the recorded intent
    /// of [`interrupt`] as well as the flag, so running it from inside the turn
    /// — where it used to live — left a window in which a cancellation was
    /// recorded, erased, and never sent: the server ran the turn to the end,
    /// commands and file changes included, while the session answered
    /// `cancelled`. Run from the ACP dispatch loop in the same breath as the
    /// lifecycle's own turn guard, that window does not exist: `session/cancel`
    /// is delivered on that same loop, so it is either before this reset or
    /// after it.
    ///
    /// `send_replace` and not `send`: between turns nobody is subscribed, and
    /// `send` refuses to change a value nobody is listening to — which would
    /// leave a cancelled session's next turn born already cancelled, abandoned
    /// the moment its grace ran out.
    fn begin(&self) {
        self.with(|state| *state = TurnState::default());
        self.interrupted.send_replace(false);
    }

    /// Resolves once an interruption has been asked for — immediately if it
    /// already was.
    async fn wait_interrupted(&self) {
        let mut watching = self.interrupted.subscribe();
        while !*watching.borrow_and_update() {
            if watching.changed().await.is_err() {
                // The sender lives as long as this control does; if it is gone,
                // so is the turn.
                return;
            }
        }
    }
}

/// Asks the server to stop the turn in flight.
///
/// When the turn has no id yet the intent is recorded and the turn loop sends
/// it as soon as the server names one — a cancellation that arrived a
/// millisecond early must not be a cancellation that never happened.
async fn interrupt<W: AsyncWrite + Unpin + Send>(peer: &Peer<W>, turn: &TurnControl) {
    let id = turn.with(|state| {
        state.interrupt_requested = true;
        if state.interrupt_sent {
            return None;
        }
        state.id.clone().inspect(|_| state.interrupt_sent = true)
    });
    if let Some(turn_id) = id {
        send_interrupt(peer, turn, &turn_id).await;
    }
    // The loop arms the grace after which the provider is ended outright, and
    // any permission still waiting on a human gives way. `send_replace`,
    // because a cancellation that arrives while nothing is subscribed still has
    // to be remembered.
    turn.interrupted.send_replace(true);
}

/// Sends the interruption, and does not wait forever for an answer: the answer
/// that matters is the turn ending, and the loop is already watching for it.
async fn send_interrupt<W: AsyncWrite + Unpin + Send>(
    peer: &Peer<W>,
    turn: &TurnControl,
    turn_id: &str,
) {
    let params = wire::TurnInterruptParams {
        thread_id: turn.thread_id.clone(),
        turn_id: turn_id.to_string(),
    };
    let _ = tokio::time::timeout(turn.grace, peer.request(wire::TURN_INTERRUPT, &params)).await;
}

/// How a turn ended, before the session decides what to do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnOutcome {
    /// The turn is over.
    Ended(StopReason),
    /// The server was asked to stop and never acknowledged it.
    Abandoned,
}

/// Runs one turn: start it, stream what the server says, and end when the
/// server says the turn is over — or when it refuses to.
///
/// Written to be correct under either ordering of the `turn/start` answer and
/// the streamed notifications, because the dumped schema does not settle which
/// comes first and observing it takes a real model call. A turn ends on
/// whichever terminal signal arrives first.
async fn drive_turn<W, S>(
    peer: &Peer<W>,
    incoming: &mut mpsc::UnboundedReceiver<Incoming>,
    turn: &TurnControl,
    stream: &S,
    session_id: &SessionId,
    params: &wire::TurnStartParams,
) -> Result<TurnOutcome, Refusal>
where
    W: AsyncWrite + Unpin + Send,
    S: TurnStream,
{
    let mut mapper = mapping::Mapper::new();
    let started = peer.request(wire::TURN_START, params);
    let mut started = std::pin::pin!(started);
    let mut answered = false;

    // Parked in the far future until an interruption arms it; from then on it
    // is the deadline after which the provider is ended outright.
    let mut grace = std::pin::pin!(tokio::time::sleep(NEVER));
    let interrupted = turn.wait_interrupted();
    let mut interrupted = std::pin::pin!(interrupted);
    let mut armed = false;

    loop {
        tokio::select! {
            answer = &mut started, if !answered => {
                answered = true;
                let value = answer.map_err(|error| turn_failed(&error.to_string()))?;
                let Ok(result) = serde_json::from_value::<wire::TurnStartResult>(value) else {
                    return Err(turn_failed("the server started a turn this adapter cannot read"));
                };
                learned_turn_id(peer, turn, &result.turn.id).await;
                if let Some(outcome) = ended(result.turn.status)? {
                    // The answer says the turn is over, and everything the
                    // server already said belongs to it. Returning without
                    // showing what is still queued would silently swallow the
                    // whole turn whenever the answer happened to win the race.
                    drain(peer, incoming, turn, stream, session_id, &mut mapper).await?;
                    return Ok(outcome);
                }
            }
            () = &mut interrupted, if !armed => {
                armed = true;
                grace.as_mut().reset(tokio::time::Instant::now() + turn.grace);
            }
            () = &mut grace, if armed => return Ok(TurnOutcome::Abandoned),
            message = incoming.recv() => {
                let Some(message) = message else {
                    return Err(provider_gone());
                };
                if let Some(outcome) =
                    handle(peer, turn, stream, session_id, &mut mapper, message).await?
                {
                    return Ok(outcome);
                }
            }
        }
    }
}

/// Handles one thing the server said. `Some` means the turn is over.
///
/// # Errors
///
/// A turn the server reports as failed is an error, not a stop reason.
async fn handle<W, S>(
    peer: &Peer<W>,
    turn: &TurnControl,
    stream: &S,
    session_id: &SessionId,
    mapper: &mut mapping::Mapper,
    message: Incoming,
) -> Result<Option<TurnOutcome>, Refusal>
where
    W: AsyncWrite + Unpin + Send,
    S: TurnStream,
{
    match message {
        Incoming::Notification { method, params } => {
            let mapped = mapper.map(&method, &params);
            for update in mapped.updates {
                stream.emit(update);
            }
            if let Some(note) = mapped.noted {
                stream.note(&note);
            }
            match mapped.signal {
                Some(mapping::Signal::TurnStarted(id)) => {
                    learned_turn_id(peer, turn, &id).await;
                    Ok(None)
                }
                Some(mapping::Signal::TurnEnded(status)) => ended(status),
                Some(mapping::Signal::TurnFailed(message)) => Err(turn_failed(&message)),
                None => Ok(None),
            }
        }
        Incoming::Request { id, method, params } => {
            relay_approval(peer, turn, stream, session_id, &id, &method, &params).await;
            Ok(None)
        }
        Incoming::Noise(line) => {
            stream.note(&format!("provider said: {line}"));
            Ok(None)
        }
    }
}

/// Handles everything the server has already said and nobody has read yet,
/// without waiting for anything more.
///
/// # Errors
///
/// Propagates a failed turn found among what was queued.
async fn drain<W, S>(
    peer: &Peer<W>,
    incoming: &mut mpsc::UnboundedReceiver<Incoming>,
    turn: &TurnControl,
    stream: &S,
    session_id: &SessionId,
    mapper: &mut mapping::Mapper,
) -> Result<(), Refusal>
where
    W: AsyncWrite + Unpin + Send,
    S: TurnStream,
{
    while let Ok(message) = incoming.try_recv() {
        handle(peer, turn, stream, session_id, mapper, message).await?;
    }
    Ok(())
}

/// Puts one of the server's questions to the permission proxy, and answers the
/// server with what the proxy said.
///
/// While this waits, the turn loop is not reading — which costs nothing: the
/// peer's reader keeps draining the provider into its queue, and the server is
/// itself waiting for this very answer before going on. A cancellation during
/// the wait still lands, because ACP requires the client to answer every pending
/// permission request with `Cancelled` when it cancels, and because the
/// interruption is sent from the cancel handler's own task, not from here.
async fn relay_approval<W, S>(
    peer: &Peer<W>,
    turn: &TurnControl,
    stream: &S,
    session_id: &SessionId,
    id: &Value,
    method: &str,
    params: &Value,
) where
    W: AsyncWrite + Unpin + Send,
    S: TurnStream,
{
    if !permission::is_approval(method) {
        stream.note(&format!(
            "the server asked `{method}`, which this adapter has no answer for"
        ));
        let _ = peer
            .respond_with_error(
                id,
                METHOD_NOT_FOUND,
                "this adapter does not implement that method",
            )
            .await;
        return;
    }

    let Ok(approval) = serde_json::from_value::<wire::ApprovalParams>(params.clone()) else {
        // A question that cannot be read cannot be decided, and something that
        // cannot be decided is refused.
        stream.note(&format!("unreadable `{method}`, refused: {params}"));
        let _ = peer
            .respond(
                id,
                &wire::ApprovalResponse::new(wire::ApprovalDecision::Decline),
            )
            .await;
        return;
    };

    // The wait gives way to a cancellation. Nothing else would: the proxy is
    // free to take as long as a human takes, and a turn that was told to stop
    // while a question sat unanswered would otherwise wait for both forever.
    let decision = tokio::select! {
        decision = stream.ask(permission::request_for(session_id, method, &approval)) => decision,
        () = turn.wait_interrupted() => permission::Decision::Cancelled,
    };
    let answer = wire::ApprovalResponse::new(permission::decide(&decision));
    let _ = peer.respond(id, &answer).await;
}

/// Records the turn's id and sends the interruption that was waiting for it.
async fn learned_turn_id<W: AsyncWrite + Unpin + Send>(
    peer: &Peer<W>,
    turn: &TurnControl,
    id: &str,
) {
    let owed = turn.with(|state| {
        state.id = Some(id.to_string());
        if state.interrupt_requested && !state.interrupt_sent {
            state.interrupt_sent = true;
            return true;
        }
        false
    });
    if owed {
        send_interrupt(peer, turn, id).await;
    }
}

/// What a turn status means for the ACP turn: `None` while it is still running.
///
/// # Errors
///
/// A failed turn is an error, not a stop reason: ACP has no way to say "the
/// turn did not work", and answering `end_turn` would report a failure as a
/// finished piece of work.
fn ended(status: wire::TurnStatus) -> Result<Option<TurnOutcome>, Refusal> {
    Ok(match status {
        wire::TurnStatus::InProgress => None,
        wire::TurnStatus::Completed => Some(TurnOutcome::Ended(StopReason::EndTurn)),
        wire::TurnStatus::Interrupted => Some(TurnOutcome::Ended(StopReason::Cancelled)),
        wire::TurnStatus::Failed => {
            return Err(turn_failed("the server reported the turn failed"));
        }
    })
}

/// Where a turn's updates go.
///
/// The session sends them over ACP; a test collects them. The turn loop is the
/// piece with the ordering subtleties — an interruption arriving before the
/// server has named the turn, an answer arriving before or after the stream —
/// and this is what lets all of it be exercised without a process in sight.
trait TurnStream: Send + Sync {
    /// Shows something in the session.
    fn emit(&self, update: SessionUpdate);
    /// Keeps something on the record without showing it as the agent's words.
    fn note(&self, line: &str);
    /// Puts a permission request to the proxy and waits for what it decides.
    ///
    /// The adapter never decides; it only asks and relays (design D5).
    fn ask(
        &self,
        request: RequestPermissionRequest,
    ) -> impl Future<Output = permission::Decision> + Send;
}

/// The real destination: the ACP connection this session was opened on.
struct AcpStream {
    session_id: SessionId,
    cx: ConnectionTo<Client>,
}

impl TurnStream for AcpStream {
    fn emit(&self, update: SessionUpdate) {
        let _ = self
            .cx
            .send_notification(SessionNotification::new(self.session_id.clone(), update));
    }

    fn note(&self, line: &str) {
        // The adapter's own stderr, which the daemon already collects: evidence
        // kept without dressing it up as something the agent said.
        eprintln!("{}: {line}", SPEC.name);
    }

    async fn ask(&self, request: RequestPermissionRequest) -> permission::Decision {
        // The daemon's proxy answers this: rules first, the human's tray when no
        // rule decides, and the constitutional denial when no client is
        // connected at all. None of that logic is here, and none of it should
        // be.
        match self.cx.send_request(request).block_task().await {
            Ok(response) => match response.outcome {
                RequestPermissionOutcome::Selected(selected) => {
                    permission::Decision::Selected(selected.option_id.0.as_ref().to_string())
                }
                RequestPermissionOutcome::Cancelled => permission::Decision::Cancelled,
                // An outcome this adapter does not recognise decides nothing,
                // and deciding nothing is a refusal.
                _ => permission::Decision::Unavailable,
            },
            Err(error) => {
                self.note(&format!("the permission proxy could not decide: {error}"));
                permission::Decision::Unavailable
            }
        }
    }
}

impl ProviderDialect for CodexDialect {
    type Session = CodexSession;

    fn spec(&self) -> AdapterSpec {
        SPEC
    }

    async fn open(
        &self,
        session_id: SessionId,
        request: NewSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> Result<Opened<Self::Session>, Refusal> {
        let command = ProviderCommand {
            program: self.program.clone(),
            args: vec![wire::SERVER_MODE_ARG.to_string()],
            cwd: request.cwd.clone(),
        };
        let mut provider = spawn(&command, SPEC.provider_layer)?;

        // From here on the process exists, so every refusal has to end it. A
        // session that failed to open and left a CLI running would leak one
        // process per attempt, holding the worktree it was launched in.
        let provenance = match handshake(&mut provider, &self.program).await {
            Ok(provenance) => provenance,
            Err(refusal) => {
                let _ = provider.shutdown(self.shutdown).await;
                return Err(refusal);
            }
        };

        // The handshake was a strict question and answer, so it was done by
        // hand. From here the conversation goes both ways at once, and reading
        // becomes a task of its own.
        let (mut control, writer, reader) = provider.into_parts();
        let (peer, incoming) = Peer::start(writer, reader, HANDSHAKE_ID + 1);

        let thread_id = match open_thread(&peer, &request).await {
            Ok(thread_id) => thread_id,
            Err(refusal) => {
                peer.close_input().await;
                let _ = supervisor::end(&mut control, self.shutdown).await;
                return Err(refusal);
            }
        };

        // The effective binary and the version it turned out to be, into the
        // session log, before anything else this session does.
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta())),
        ));

        // The id it was handed: this dialect keeps no sessions of its own to
        // resume, so it has no better name to answer to.
        Ok(Opened {
            id: session_id.clone(),
            session: CodexSession {
                peer,
                incoming: Mutex::new(incoming),
                control: Mutex::new(Some(control)),
                turn: TurnControl::new(thread_id, INTERRUPT_GRACE),
                stream: AcpStream { session_id, cx },
                shutdown: self.shutdown,
            },
        })
    }
}

/// Opens the thread the ACP session maps onto, in the directory the session
/// works in.
///
/// # Errors
///
/// Refuses when the server will not open one: a session with no thread would
/// accept prompts it could never send anywhere.
async fn open_thread<W: AsyncWrite + Unpin + Send>(
    peer: &Peer<W>,
    request: &NewSessionRequest,
) -> Result<String, Refusal> {
    let params = wire::ThreadStartParams {
        cwd: request.cwd.display().to_string(),
        // The user's choice, carried verbatim to where this provider's schema
        // accepts it: the thread.
        model: crate::session_model(),
    };
    let answer = peer
        .request(wire::THREAD_START, &params)
        .await
        .map_err(|error| thread_refused(&error.to_string()))?;
    serde_json::from_value::<wire::ThreadStartResult>(answer)
        .map(|result| result.thread.id)
        .map_err(|error| {
            thread_refused(&format!(
                "the server opened a thread this adapter cannot read ({error})"
            ))
        })
}

/// Shakes hands with the server and checks that it is a version this adapter
/// was verified against.
///
/// # Errors
///
/// Refuses — and never assumes compatibility — when the server does not answer,
/// answers something unreadable, or reports a version outside the declared
/// range.
async fn handshake<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
    program: &str,
) -> Result<Provenance, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    let params = wire::InitializeParams {
        client_info: wire::ClientInfo {
            name: SPEC.name.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    let request = json!({
        "jsonrpc": "2.0",
        "id": HANDSHAKE_ID,
        "method": wire::INITIALIZE,
        "params": serde_json::to_value(&params).map_err(|error| handshake_failed(
            &format!("the handshake could not be encoded ({error})")
        ))?,
    });
    provider
        .send(&request)
        .await
        .map_err(|error| handshake_failed(&format!("the handshake could not be sent ({error})")))?;

    let answer = tokio::time::timeout(HANDSHAKE_TIMEOUT, await_answer(provider, HANDSHAKE_ID))
        .await
        .map_err(|_| {
            handshake_failed(&format!(
                "the server did not answer the handshake within {} seconds",
                HANDSHAKE_TIMEOUT.as_secs()
            ))
        })??;

    let result: wire::InitializeResult = serde_json::from_value(answer).map_err(|error| {
        handshake_failed(&format!(
            "the server answered the handshake with something this adapter cannot read ({error})"
        ))
    })?;

    let Some(version) = version::of(&result.user_agent) else {
        return Err(version::unreadable(SPEC.provider_layer, &result.user_agent));
    };
    if !version::supported(version) {
        return Err(version::skew(SPEC.provider_layer, version));
    }

    // The wire expects this once the handshake is done. It is a notification:
    // nothing answers it, and nothing should wait for anything to.
    provider
        .send(&json!({"jsonrpc": "2.0", "method": wire::INITIALIZED}))
        .await
        .map_err(|error| {
            handshake_failed(&format!("the server closed during the handshake ({error})"))
        })?;

    Ok(Provenance {
        program: program.to_string(),
        version: version::render(version),
        user_agent: result.user_agent,
    })
}

/// Reads until the answer to `id` arrives.
///
/// Anything else the server says meanwhile is skipped rather than treated as
/// the answer: the wire is free to start talking before it finishes replying,
/// and a handshake that mistook a notification for its own answer would be
/// wrong in the one place that decides whether the session may exist.
async fn await_answer<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
    id: i64,
) -> Result<Value, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    loop {
        let frame = provider.receive().await.map_err(|error| {
            handshake_failed(&format!("the server's output could not be read ({error})"))
        })?;
        let Some(frame) = frame else {
            return Err(handshake_failed(
                "the server closed its output without answering the handshake",
            ));
        };
        let message = match frame {
            Frame::Json(value) => value,
            // Not JSON at all: a banner or a warning. Kept on the adapter's own
            // stderr, where the daemon already collects the provider's noise,
            // and never mistaken for protocol.
            Frame::Unparsed(line) => {
                eprintln!("{}: provider said: {line}", SPEC.name);
                continue;
            }
        };
        if message.get("id").and_then(Value::as_i64) != Some(id) {
            continue;
        }
        if let Some(error) = message.get("error") {
            return Err(handshake_failed(&format!(
                "the server refused the handshake ({error})"
            )));
        }
        return message.get("result").cloned().ok_or_else(|| {
            handshake_failed("the server answered the handshake with neither a result nor an error")
        });
    }
}

/// The refusal for a thread the server would not open.
fn thread_refused(detail: &str) -> Refusal {
    Refusal::new(
        "provider_thread_refused",
        SPEC.provider_layer,
        detail.to_string(),
        "Check that the session's directory exists and that the CLI can work in it.".to_string(),
    )
}

/// The refusal for a turn that did not run.
fn turn_failed(detail: &str) -> Refusal {
    Refusal::new(
        "provider_turn_failed",
        SPEC.provider_layer,
        detail.to_string(),
        "The turn did not complete. The session is still open: try again, or read the \
         provider's own diagnostics in the session log."
            .to_string(),
    )
}

/// The refusal for a provider that went away mid-turn.
fn provider_gone() -> Refusal {
    Refusal::new(
        "provider_gone",
        SPEC.provider_layer,
        "the CLI ended while the turn was running".to_string(),
        "Open a new session; the provider's own diagnostics travel to the session log.".to_string(),
    )
}

/// The refusal for a handshake that did not happen.
fn handshake_failed(detail: &str) -> Refusal {
    Refusal::new(
        "provider_handshake_failed",
        SPEC.provider_layer,
        detail.to_string(),
        format!(
            "Check that the official CLI is installed and signed in, and that `{} {}` starts its \
             documented server mode (`meltemi fleet` shows the entry and its remedy).",
            SPEC.provider_bin,
            wire::SERVER_MODE_ARG
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: El adaptador manda la palanca donde su proveedor la acepta
    #[test]
    fn each_lever_rides_where_this_providers_schema_puts_it() {
        // Verified against the vendored schema rather than remembered: `model`
        // is a thread field and `effort` is a turn field, and the adapter keeps
        // that split instead of flattening the two into one place.
        let started = serde_json::to_value(wire::ThreadStartParams {
            cwd: "/repo".into(),
            model: Some("some-provider-model".into()),
        })
        .expect("thread start serializes");
        assert_eq!(started["model"], "some-provider-model");
        assert!(
            started.get("effort").is_none(),
            "effort is not a thread field in this provider's schema: {started}"
        );

        let turn = serde_json::to_value(wire::TurnStartParams {
            thread_id: "t-1".into(),
            input: vec![wire::UserInput::Text { text: "go".into() }],
            effort: Some("high".into()),
        })
        .expect("turn start serializes");
        assert_eq!(turn["effort"], "high");
        assert!(
            turn.get("model").is_none(),
            "and model is not a turn field: {turn}"
        );

        // Nothing chosen means nothing sent: the CLI's own configuration
        // decides, exactly as before these fields existed.
        let bare = serde_json::to_value(wire::ThreadStartParams {
            cwd: "/repo".into(),
            model: None,
        })
        .expect("serializes");
        assert!(bare.get("model").is_none(), "{bare}");
    }

    /// A provider process that answers from a script, so a whole handshake runs
    /// in memory: no binary, no pipes, same behaviour on the three platforms.
    struct FakeProcess;

    impl ProcessControl for FakeProcess {
        async fn wait_within(&mut self, _grace: Duration) -> std::io::Result<bool> {
            Ok(true)
        }

        async fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Runs a handshake against a server that says exactly `lines`, and hands
    /// back the outcome together with everything the adapter sent.
    async fn handshake_against(lines: &str) -> (Result<Provenance, Refusal>, Vec<Value>) {
        let (adapter_out, mut server_in) = tokio::io::duplex(4096);
        let (mut server_out, adapter_in) = tokio::io::duplex(4096);
        tokio::io::AsyncWriteExt::write_all(&mut server_out, lines.as_bytes())
            .await
            .unwrap();
        drop(server_out);

        let mut provider = ProviderProcess::new(FakeProcess, adapter_out, adapter_in);
        let outcome = handshake(&mut provider, "codex").await;
        drop(provider);

        let mut sent = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut server_in, &mut sent)
            .await
            .unwrap();
        let sent = sent
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("the adapter speaks JSON"))
            .collect();
        (outcome, sent)
    }

    fn announcing(user_agent: &str) -> String {
        format!("{{\"id\":1,\"result\":{{\"userAgent\":\"{user_agent}\"}}}}\n")
    }

    /// A turn's updates, collected instead of sent, and a permission proxy that
    /// answers whatever the test arranged.
    #[derive(Default)]
    struct Collected {
        updates: StdMutex<Vec<SessionUpdate>>,
        notes: StdMutex<Vec<String>>,
        asked: StdMutex<Vec<RequestPermissionRequest>>,
        answers: StdMutex<Vec<permission::Decision>>,
    }

    impl TurnStream for Collected {
        fn emit(&self, update: SessionUpdate) {
            self.updates.lock().unwrap().push(update);
        }

        fn note(&self, line: &str) {
            self.notes.lock().unwrap().push(line.to_string());
        }

        async fn ask(&self, request: RequestPermissionRequest) -> permission::Decision {
            self.asked.lock().unwrap().push(request);
            self.answers
                .lock()
                .unwrap()
                .pop()
                // Nothing was arranged: the proxy could not be reached, which is
                // exactly the case that must end in a refusal.
                .unwrap_or(permission::Decision::Unavailable)
        }
    }

    impl Collected {
        fn said(&self) -> Vec<String> {
            self.updates
                .lock()
                .unwrap()
                .iter()
                .filter_map(|update| match update {
                    SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                        agent_client_protocol::schema::v1::ContentBlock::Text(text) => {
                            Some(text.text.clone())
                        }
                        _ => None,
                    },
                    _ => None,
                })
                .collect()
        }
    }

    /// The provider's side of a turn: one scripted server on a duplex pipe.
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    struct Server {
        heard: tokio::io::Lines<tokio::io::BufReader<tokio::io::DuplexStream>>,
        says: tokio::io::DuplexStream,
    }

    impl Server {
        async fn next_call(&mut self) -> Value {
            let line = self
                .heard
                .next_line()
                .await
                .unwrap()
                .expect("the adapter said something");
            serde_json::from_str(&line).expect("and it was JSON")
        }

        async fn say(&mut self, message: &Value) {
            self.says
                .write_all(format!("{message}\n").as_bytes())
                .await
                .unwrap();
            self.says.flush().await.unwrap();
        }

        async fn notify(&mut self, method: &str, params: Value) {
            self.say(&json!({"method": method, "params": params})).await;
        }
    }

    /// A peer and a turn control wired to a scripted server.
    fn wired(
        grace: Duration,
    ) -> (
        Peer<tokio::io::DuplexStream>,
        mpsc::UnboundedReceiver<Incoming>,
        TurnControl,
        Server,
    ) {
        use crate::ndjson::{FrameReader, FrameWriter};
        let (adapter_out, provider_in) = tokio::io::duplex(8192);
        let (provider_out, adapter_in) = tokio::io::duplex(8192);
        let (peer, incoming) = Peer::start(
            FrameWriter::new(adapter_out),
            FrameReader::new(adapter_in),
            2,
        );
        (
            peer,
            incoming,
            TurnControl::new("thread-1".into(), grace),
            Server {
                heard: tokio::io::BufReader::new(provider_in).lines(),
                says: provider_out,
            },
        )
    }

    fn turn_params() -> wire::TurnStartParams {
        wire::TurnStartParams {
            effort: None,
            thread_id: "thread-1".into(),
            input: vec![wire::UserInput::Text {
                text: "do it".into(),
            }],
        }
    }

    #[tokio::test]
    async fn a_turn_streams_what_the_server_says_and_ends_when_it_says_the_turn_is_over() {
        // Scenario: Conversación del servidor mapeada en streaming
        //
        // The answer to `turn/start` arrives last here, which is the ordering
        // the dumped schema does not settle: the turn must end on the terminal
        // signal, not on the answer it may never get first.
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_millis(50));
        let stream = Collected::default();

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            let call = server.next_call().await;
            assert_eq!(call["method"], wire::TURN_START);
            assert_eq!(call["params"]["input"][0]["text"], "do it");
            server
                .notify(
                    wire::TURN_STARTED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "inProgress", "items": []}}),
                )
                .await;
            server
                .notify(
                    wire::AGENT_MESSAGE_DELTA,
                    json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": "i1", "delta": "hola"}),
                )
                .await;
            server
                .notify(
                    wire::TURN_COMPLETED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "completed", "items": []}}),
                )
                .await;
            server
                .say(&json!({"id": call["id"], "result": {"turn": {"id": "turn-1", "status": "completed", "items": []}}}))
                .await;
        };

        let (outcome, ()) = tokio::join!(driving, serving);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::EndTurn));
        assert_eq!(stream.said(), vec!["hola".to_string()]);
    }

    #[tokio::test]
    async fn a_cancellation_that_arrives_before_the_turn_has_a_name_is_still_delivered() {
        // Scenario: Cancelación propagada al servidor
        //
        // The window every naive adapter drops the cancellation in: the human
        // hit stop before the server had named the turn, so there was nothing
        // to name in the interruption. The intent is remembered and sent the
        // moment the name arrives.
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_secs(30));
        let stream = Collected::default();

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            let start = server.next_call().await;
            interrupt(&peer, &turn).await;
            server
                .notify(
                    wire::TURN_STARTED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "inProgress", "items": []}}),
                )
                .await;

            let stop = server.next_call().await;
            assert_eq!(stop["method"], wire::TURN_INTERRUPT);
            assert_eq!(stop["params"]["turnId"], "turn-1");
            server.say(&json!({"id": stop["id"], "result": {}})).await;
            server
                .notify(
                    wire::TURN_COMPLETED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "interrupted", "items": []}}),
                )
                .await;
            server
                .say(&json!({"id": start["id"], "result": {"turn": {"id": "turn-1", "status": "interrupted", "items": []}}}))
                .await;
        };

        let (outcome, ()) = tokio::join!(driving, serving);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::Cancelled));
    }

    #[tokio::test]
    async fn a_cancellation_recorded_before_the_turn_starts_still_reaches_the_server() {
        // Scenario: Cancelación temprana no perdida entre la aceptación y el turno
        //
        // The other half of the bridge's ordering rule, from this dialect's
        // side. The reset happens where the prompt is accepted; the
        // cancellation lands after it and before the turn runs, which is the
        // sequence a serial dispatch loop produces. The intent survives the
        // whole of `turn/start` with nothing to name, and goes out the instant
        // the server names the turn.
        //
        // While the reset lived inside the turn it wiped exactly this, and the
        // server never heard a word about it.
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_secs(30));
        let stream = Collected::default();

        turn.begin();
        interrupt(&peer, &turn).await;
        assert!(
            turn.with(|state| state.interrupt_requested),
            "the intent is recorded even with no turn to name yet"
        );

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            let start = server.next_call().await;
            assert_eq!(start["method"], wire::TURN_START);
            server
                .notify(
                    wire::TURN_STARTED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "inProgress", "items": []}}),
                )
                .await;

            let stop = server.next_call().await;
            assert_eq!(
                stop["method"],
                wire::TURN_INTERRUPT,
                "the cancellation that arrived first is the one that arrives at all"
            );
            assert_eq!(stop["params"]["turnId"], "turn-1");
            server.say(&json!({"id": stop["id"], "result": {}})).await;
            server
                .notify(
                    wire::TURN_COMPLETED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "interrupted", "items": []}}),
                )
                .await;
            server
                .say(&json!({"id": start["id"], "result": {"turn": {"id": "turn-1", "status": "interrupted", "items": []}}}))
                .await;
        };

        let (outcome, ()) = tokio::join!(driving, serving);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::Cancelled));
    }

    #[test]
    fn a_new_turn_is_not_tainted_by_the_cancellation_of_the_last_one() {
        // Nobody is subscribed between turns, which is exactly when a `watch`
        // refuses to change: a session whose first turn was cancelled would
        // have started its second one already cancelled, and abandoned it the
        // moment the grace ran out.
        let turn = TurnControl::new("thread-1".into(), Duration::from_millis(10));
        turn.interrupted.send_replace(true);
        turn.begin();
        assert!(
            !*turn.interrupted.borrow(),
            "a stale cancellation must not kill the next honest turn"
        );
        assert!(
            !turn.with(|state| state.interrupt_requested),
            "and neither must the intent it left behind"
        );
    }

    #[tokio::test]
    async fn a_server_that_will_not_stop_is_abandoned_so_the_session_can_end_it() {
        // Scenario: Cancelación propagada al servidor
        //
        // Asked to stop, the server says nothing at all. It does not get to hold
        // the session — or the worktree it runs in — open while it thinks about
        // it: the turn is abandoned and the caller ends the process.
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_millis(20));
        let stream = Collected::default();

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            server.next_call().await;
            server
                .notify(
                    wire::TURN_STARTED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "inProgress", "items": []}}),
                )
                .await;
            // Give the loop the turn id before asking it to stop, then go quiet.
            tokio::time::sleep(Duration::from_millis(20)).await;
            interrupt(&peer, &turn).await;
            std::future::pending::<()>().await;
        };

        let outcome = tokio::select! {
            outcome = driving => outcome,
            () = serving => unreachable!("the server never finishes"),
        };
        assert_eq!(outcome.unwrap(), TurnOutcome::Abandoned);
    }

    #[tokio::test]
    async fn an_approval_goes_to_the_proxy_and_the_proxys_answer_is_what_the_server_gets() {
        // Scenario: Aprobación del servidor decidida por el proxy
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_millis(50));
        let stream = Collected::default();
        stream
            .answers
            .lock()
            .unwrap()
            .push(permission::Decision::Selected(
                permission::ALLOW_ONCE.into(),
            ));

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            let start = server.next_call().await;
            server
                .say(&json!({"id": 7, "method": wire::FILE_CHANGE_APPROVAL,
                             "params": {"threadId": "thread-1", "turnId": "turn-1", "itemId": "i2",
                                        "reason": "it writes one file"}}))
                .await;
            let answer = server.next_call().await;
            assert_eq!(answer["id"], 7);
            assert_eq!(
                answer["result"]["decision"], "accept",
                "what the server gets is what the proxy decided: {answer}"
            );
            server
                .notify(
                    wire::TURN_COMPLETED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "completed", "items": []}}),
                )
                .await;
            server
                .say(&json!({"id": start["id"], "result": {"turn": {"id": "turn-1", "status": "completed", "items": []}}}))
                .await;
        };

        let (outcome, ()) = tokio::join!(driving, serving);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::EndTurn));

        let asked = stream.asked.lock().unwrap();
        assert_eq!(asked.len(), 1, "the question was put once");
        assert_eq!(asked[0].session_id.0.as_ref(), "s-1");
        assert_eq!(
            asked[0].tool_call.tool_call_id.0.as_ref(),
            "i2",
            "against the very item the server asked about"
        );
    }

    #[tokio::test]
    async fn a_proxy_that_cannot_decide_is_a_refusal_and_the_server_hears_it() {
        // Scenario: Aprobación del servidor decidida por el proxy
        //
        // No decision — no client, an error, an outcome nobody recognises — is a
        // refusal. An adapter that let a gap in the chain read as consent would
        // be the one thing this architecture exists to prevent.
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_millis(50));
        let stream = Collected::default();

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            let start = server.next_call().await;
            server
                .say(&json!({"id": 7, "method": wire::COMMAND_EXECUTION_APPROVAL,
                             "params": {"threadId": "thread-1", "turnId": "turn-1", "itemId": "i2"}}))
                .await;
            assert_eq!(server.next_call().await["result"]["decision"], "decline");

            // And something this adapter has no answer for at all gets the
            // protocol's own answer, not an invented one — and certainly not a
            // yes.
            server
                .say(&json!({"id": 8, "method": "thread/somethingElse", "params": {}}))
                .await;
            assert_eq!(server.next_call().await["error"]["code"], METHOD_NOT_FOUND);

            server
                .notify(
                    wire::TURN_COMPLETED,
                    json!({"threadId": "thread-1", "turn": {"id": "turn-1", "status": "completed", "items": []}}),
                )
                .await;
            server
                .say(&json!({"id": start["id"], "result": {"turn": {"id": "turn-1", "status": "completed", "items": []}}}))
                .await;
        };

        let (outcome, ()) = tokio::join!(driving, serving);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::EndTurn));
    }

    #[tokio::test]
    async fn a_provider_that_disappears_mid_turn_refuses_instead_of_hanging() {
        let (peer, mut incoming, turn, mut server) = wired(Duration::from_millis(50));
        let stream = Collected::default();

        let params = turn_params();
        let session = SessionId::new("s-1");
        let driving = drive_turn(&peer, &mut incoming, &turn, &stream, &session, &params);
        let serving = async {
            server.next_call().await;
            drop(server);
        };
        let (outcome, ()) = tokio::join!(driving, serving);
        let refusal = outcome.expect_err("a turn cannot end well on a provider that is gone");
        assert!(
            refusal.kind == "provider_gone" || refusal.kind == "provider_turn_failed",
            "{refusal:?}"
        );
    }

    #[tokio::test]
    async fn the_handshake_reads_the_version_and_answers_with_what_was_launched() {
        // Scenario: Versión efectiva registrada en el log
        //
        // Note the server's answer carries no `jsonrpc` member: that is what the
        // real CLI sends (verified against 0.77.0), and an adapter that demanded
        // it would refuse every real session while every fixture stayed green.
        let (outcome, sent) = handshake_against(&announcing(
            "codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) (meltemi-codex-acp; 0.1.0)",
        ))
        .await;

        let provenance = outcome.expect("a supported version opens the session");
        assert_eq!(provenance.program, "codex");
        assert_eq!(provenance.version, "0.77.0");
        assert!(provenance.user_agent.starts_with("codex_cli_rs/0.77.0"));

        assert_eq!(sent[0]["method"], wire::INITIALIZE);
        assert_eq!(sent[0]["id"], HANDSHAKE_ID);
        assert_eq!(sent[0]["params"]["clientInfo"]["name"], SPEC.name);
        assert_eq!(
            sent[1]["method"],
            wire::INITIALIZED,
            "the wire expects this once the handshake is done"
        );
        assert!(
            sent[1].get("id").is_none(),
            "and it is a notification: nothing answers it"
        );
    }

    #[tokio::test]
    async fn what_the_server_says_before_answering_is_not_mistaken_for_the_answer() {
        // The wire may start talking before it finishes replying. A handshake
        // that took the first line it saw would read a version out of a
        // notification — and be wrong in the one place that decides whether the
        // session may exist at all.
        let noise = concat!(
            "starting up\n",
            "{\"method\":\"deprecationNotice\",\"params\":{\"summary\":\"soon\"}}\n",
            "{\"id\":99,\"result\":{\"userAgent\":\"codex_cli_rs/0.1.0\"}}\n",
        );
        let (outcome, _) = handshake_against(&format!(
            "{noise}{}",
            announcing("codex_cli_rs/0.77.0 (linux)")
        ))
        .await;
        assert_eq!(
            outcome.expect("the answer was the one with our id").version,
            "0.77.0"
        );
    }

    #[tokio::test]
    async fn a_version_outside_the_range_refuses_instead_of_assuming_compatibility() {
        // Scenario: Desfase de versión rehusado con remedio
        let (outcome, _) = handshake_against(&announcing("codex_cli_rs/0.1.0 (linux)")).await;
        let refusal = outcome.expect_err("an unverified version is not assumed compatible");
        assert_eq!(refusal.kind, "provider_version_skew");
        assert!(refusal.detail.contains("0.1.0"), "{}", refusal.detail);
        assert!(
            refusal.detail.contains(wire::VENDORED_SCHEMA_VERSION),
            "the refusal names both versions: {}",
            refusal.detail
        );
        assert!(refusal.remedy.contains("Update"), "{}", refusal.remedy);
    }

    #[tokio::test]
    async fn a_handshake_the_server_never_answers_refuses_rather_than_opening_a_session() {
        // Scenario: Desfase de versión rehusado con remedio
        for said in [
            "",
            "not json at all\n",
            "{\"id\":1,\"error\":{\"code\":-1}}\n",
        ] {
            let (outcome, _) = handshake_against(said).await;
            let refusal = outcome.expect_err("a session cannot open on a handshake that failed");
            assert_eq!(refusal.kind, "provider_handshake_failed", "for `{said}`");
            assert!(!refusal.remedy.is_empty());
        }

        // An answer whose user agent carries no version is the same story: read
        // it or refuse it, never guess it.
        let (outcome, _) = handshake_against(&announcing("codex_cli_rs/nightly")).await;
        assert_eq!(
            outcome.expect_err("no version, no session").kind,
            "provider_version_unreadable"
        );
    }

    #[test]
    fn the_provenance_travels_as_acp_extension_metadata() {
        // Scenario: Versión efectiva registrada en el log
        //
        // The daemon records an agent's session updates verbatim, so what the
        // log will hold is exactly this: which binary was launched, and which
        // version it turned out to be. Not a private channel — ACP's own `_meta`
        // (design D7), which any agent may fill the same way.
        let provenance = Provenance {
            program: "/opt/homebrew/bin/codex".into(),
            version: "0.77.0".into(),
            user_agent: "codex_cli_rs/0.77.0 (macOS)".into(),
        };
        let update =
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta()));
        let logged = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerBin"],
            "/opt/homebrew/bin/codex"
        );
        assert_eq!(logged["_meta"]["meltemi"]["providerVersion"], "0.77.0");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerUserAgent"],
            "codex_cli_rs/0.77.0 (macOS)"
        );
    }

    #[test]
    fn the_environment_override_decides_which_binary_is_launched() {
        // The knob the end-to-end test turns to put a scripted wire where the
        // provider would be. Without it, the registry's binary name stands —
        // and an empty variable is not an instruction to launch nothing. What
        // the name resolves to is the file the platform will really run, which
        // on Windows is the `.cmd` an `npm i -g` install drops, so the
        // assertion is about which binary was named.
        let named = |value: Option<&str>| {
            let resolved = resolve_program(value.map(str::to_string), SPEC.provider_bin);
            std::path::Path::new(&resolved)
                .file_stem()
                .is_some_and(|stem| stem.eq_ignore_ascii_case(SPEC.provider_bin))
        };
        assert!(named(None), "the registry's name stands");
        assert!(named(Some("  ")), "and an empty variable changes nothing");
        assert_eq!(
            resolve_program(Some("/tmp/mock-codex-wire".into()), SPEC.provider_bin),
            "/tmp/mock-codex-wire"
        );
    }

    #[test]
    fn a_handshake_that_did_not_happen_refuses_with_a_remedy() {
        let refusal = handshake_failed("the server closed its output");
        assert_eq!(refusal.kind, "provider_handshake_failed");
        assert_eq!(refusal.layer, SPEC.provider_layer);
        assert!(refusal.remedy.contains(wire::SERVER_MODE_ARG));
    }
}
