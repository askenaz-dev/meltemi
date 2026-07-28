// SPDX-License-Identifier: Apache-2.0

//! The headless session dialect (adaptadores-propios-acp design D4).
//!
//! One provider ships no server mode at all: what it documents is a
//! **non-interactive session that speaks newline-delimited JSON events** over
//! stdio, running under the account the user already signed into. This module
//! is the translation between that surface and ACP.
//!
//! What is deliberately not here, and never will be:
//!
//! - **The provider's agent SDK.** The provider's own terms name it as not
//!   authorized for subscription sign-in, and the canonical third-party adapter
//!   wraps exactly that library. This adapter launches the official binary
//!   instead — the safe path the research named and nobody had taken.
//! - **Any mode that demands an API key.** The flag that skips the sign-in is
//!   announced to become the default of headless mode one day; when that
//!   arrives, this adapter refuses with a diagnosis rather than let a key
//!   quietly start paying for what a subscription was paying for (see
//!   [`surface`]).
//! - **Any undocumented channel.** The same wire carries the SDK's own control
//!   protocol; it is not documented, so nothing here hangs off it (design D7).

pub mod gate;
pub mod mapping;
pub mod permission;
pub mod rendezvous;
pub mod shim;
pub mod surface;
pub mod wire;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    AgentCapabilities, LoadSessionRequest, McpCapabilities, McpServer, Meta, NewSessionRequest,
    PromptRequest, RequestPermissionOutcome, RequestPermissionRequest, SessionId,
    SessionInfoUpdate, SessionNotification, SessionUpdate, StopReason,
};
use agent_client_protocol::{Client, ConnectionTo};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::{Mutex, watch};

use crate::adapter::{AdapterSpec, Dialect, Opened, ProviderDialect, ProviderSession};
use crate::diagnostic::Refusal;
use crate::ndjson::{Frame, FrameReader, FrameWriter};
use crate::supervisor::{
    self, ChildProcess, ProviderCommand, ShutdownPolicy, launch_refusal, resolve_program, spawn,
};

use rendezvous::Rendezvous;
use surface::Surface;

/// What this binary announces over ACP and which CLI it pilots.
pub const SPEC: AdapterSpec = AdapterSpec {
    name: "meltemi-claude-acp",
    provider_layer: "the official `claude` CLI",
    provider_bin: "claude",
    dialect: Dialect::HeadlessSession,
};

/// Points the adapter at a specific CLI binary instead of the one on the PATH.
///
/// It is how the end-to-end tests put a scripted wire where the provider would
/// be, and an escape hatch for a CLI installed somewhere the PATH does not
/// reach. It changes *which binary* is launched, never *what* is spoken to it.
pub const PROVIDER_BIN_ENV: &str = "MELTEMI_CLAUDE_BIN";

/// How long the initial event may take to arrive **after the first turn has
/// been sent** before the session is refused.
///
/// Generous, because this CLI runs on a language runtime that can take seconds
/// to come up cold on Windows and boots the session's MCP servers before it
/// speaks; finite, because a session that never opens and never fails is the
/// worst of both.
///
/// Counting from the turn and not from the launch is the whole of task 5.3: the
/// CLI emits nothing at all until it has been given something to do, so a
/// deadline that started at launch was a deadline nothing could ever meet.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the CLI gets to answer what version it is.
const VERSION_TIMEOUT: Duration = Duration::from_secs(30);

/// How long a cancelled turn gets to end on its own before the provider is
/// ended outright.
///
/// This surface documents no interruption at all — see [`ClaudeSession::interrupt`]
/// — so this grace is not politeness about an unanswered request: it is the
/// whole of the difference between asking and ending.
const INTERRUPT_GRACE: Duration = Duration::from_secs(5);

/// A deadline far enough away to mean "not yet". The turn loop's grace sits
/// here until a cancellation moves it.
const NEVER: Duration = Duration::from_secs(60 * 60 * 24 * 365);

/// What the session log says when the CLI would not say what version it is.
const UNKNOWN_VERSION: &str = "unknown";

/// The headless session dialect: launches the official CLI in its documented
/// non-interactive mode and pilots it.
pub struct ClaudeDialect {
    /// The binary to launch, after the environment override.
    program: String,
    /// How long a provider gets to exit on its own when a session closes.
    shutdown: ShutdownPolicy,
}

impl Default for ClaudeDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeDialect {
    /// The dialect as the binary runs it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: resolve_program(std::env::var(PROVIDER_BIN_ENV).ok(), SPEC.provider_bin),
            shutdown: ShutdownPolicy::default(),
        }
    }
}

/// What was actually launched: the facts a session must be able to prove
/// afterwards, all of them known before the CLI has said a word.
///
/// What the CLI then announces about *itself* is [`Surface`], and it lands in
/// the session as its own update — later, because it is known later. Splitting
/// them is not tidiness: pretending the credential source was known at launch
/// is what task 5.3 had to undo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The binary the adapter launched, as it was resolved.
    pub program: String,
    /// The version the CLI answered with, or [`UNKNOWN_VERSION`].
    pub version: String,
    /// The id this session answers to on both sides: dictated to the CLI at
    /// launch, and the name a later resume gives it.
    pub session: String,
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
                "providerSession": self.session,
            }),
        );
        meta
    }
}

/// A session over the official CLI's headless mode.
pub struct ClaudeSession {
    /// The CLI's input: one user turn per line, and the end of it is what ends
    /// the session.
    writer: Mutex<FrameWriter<ChildStdin>>,
    /// The CLI's output: everything the session is made of.
    reader: Mutex<FrameReader<ChildStdout>>,
    /// The process handle, so the session can end what it started. `None` once
    /// it has been shut down.
    control: Mutex<Option<ChildProcess>>,
    /// The turn in flight, and whether it has been asked to stop.
    turn: TurnControl,
    /// The private channel this session's permission shim asks over.
    rendezvous: Rendezvous,
    /// Where this session's updates go.
    stream: AcpStream,
    /// How long the provider gets to exit on its own.
    shutdown: ShutdownPolicy,
    /// Whether the CLI's initial event has already been read. It arrives on the
    /// first turn — see [`ClaudeSession::announcement`].
    announced: AtomicBool,
}

impl ProviderSession for ClaudeSession {
    fn begin_turn(&self) {
        self.turn.begin();
    }

    async fn run_turn(&self, prompt: PromptRequest) -> Result<StopReason, Refusal> {
        // One turn at a time is the bridge's own invariant, so holding the
        // output for the length of a turn takes nothing from anyone — and this
        // wire has no way to tell two turns apart, so it must not be lent out.
        let mut reader = self.reader.lock().await;

        let message = wire::user_message(&mapping::prompt_text(&prompt));
        send_turn(&mut *self.writer.lock().await, &message).await?;

        // Only now can the CLI be expected to say anything, so only now is its
        // initial event read and its surface checked — before a single event of
        // this turn is mapped into the session.
        if let Err(refusal) = self.announcement(&mut reader).await {
            self.shutdown(self.shutdown).await;
            return Err(refusal);
        }

        match drive_turn(
            &mut reader,
            &self.turn,
            &self.rendezvous,
            &self.stream,
            &self.stream.session_id,
        )
        .await?
        {
            TurnOutcome::Ended(stop) => Ok(stop),
            TurnOutcome::Abandoned => {
                // The CLI was told the conversation is over and kept working.
                // A human who pressed stop does not get to be made to wait out
                // the turn they cancelled, and the worktree does not get to be
                // held open by it.
                self.stream.note(&format!(
                    "the CLI did not end the cancelled turn within {:?}; ending the provider",
                    self.turn.grace
                ));
                self.shutdown(self.shutdown).await;
                Ok(StopReason::Cancelled)
            }
        }
    }

    /// Cancels by ending the session's input — which **ends the session**, not
    /// just the turn.
    ///
    /// What this surface documents is the end of its input. The same wire does
    /// carry the provider SDK's own control protocol, which would interrupt a
    /// turn properly and leave the session alive; it is not documented, and
    /// design D7 forbids hanging a capability off a channel nobody published.
    /// So the end of input is the only stop there is here, and taking it costs
    /// the session: a CLI that has been told the conversation is over cannot be
    /// given another turn, and this adapter will not pretend otherwise
    /// ([`session_finished`]).
    ///
    /// That is a decision and not an accident, and it was made twice. The close
    /// used to be `AsyncWrite::shutdown`, which against a child process does
    /// nothing at all — so a cancellation sent the CLI no signal whatsoever,
    /// the turn ran on until the grace expired, and the session then blamed the
    /// CLI for ignoring something it was never told. Between a cancellation
    /// that ends the session and one that does not happen, this dialect takes
    /// the first: the human pressed stop.
    async fn interrupt(&self) {
        self.writer.lock().await.close();
        self.turn.ask_to_stop();
    }

    async fn shutdown(&self, policy: ShutdownPolicy) {
        // Closing the input is what ends a headless session; the grace and the
        // kill are for a provider that ignores it.
        //
        // Worth knowing where this does *not* run: the daemon usually ends an
        // adapter by killing it, and a killed process runs no cleanup at all.
        // What saves this dialect there is the operating system — the CLI's
        // stdin is a pipe whose only writer is this process, so the kill closes
        // it and the CLI sees end of input on its own. The residual risk is a
        // provider that ignores EOF, and ending *that* with certainty takes a
        // job object on Windows and a process group elsewhere; that stronger
        // form belongs with `sandbox-propio`.
        self.writer.lock().await.close();
        if let Some(mut control) = self.control.lock().await.take() {
            let _ = supervisor::end(&mut control, policy).await;
        }
        // The channel goes with the session. A shim still waiting on an answer
        // reads its absence as "nobody can decide this" and denies, which is the
        // only thing it may do.
        self.rendezvous.close();
    }
}

impl ClaudeSession {
    /// Reads the CLI's initial event and checks the surface it announces —
    /// once, on the first turn.
    ///
    /// This is the handshake of this dialect, and it happens here rather than
    /// at `session/new` because the CLI **emits nothing until it has been given
    /// something to do** (task 5.3, verified against the real binary: launched
    /// with its input held open and nothing written, it says not one byte). An
    /// adapter that waited for the announcement before sending anything waited
    /// for something that was waiting for it, and every session died of the
    /// timeout.
    ///
    /// # Errors
    ///
    /// Refuses when the CLI never announces the session, ends before it does,
    /// or announces a surface this adapter may not pilot. The caller ends the
    /// provider on any of them: a turn is already in flight, and a CLI that may
    /// not be piloted must not be left running one.
    async fn announcement(&self, reader: &mut FrameReader<ChildStdout>) -> Result<(), Refusal> {
        if self.announced.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        let announced = Surface::of(&handshake(reader).await?);
        // Before anything of this turn is mapped: the guard exists to stop a
        // session, not to annotate one that already ran.
        announced.check(SPEC.provider_layer)?;
        if let Some(note) = announced.note() {
            self.stream.note(&note);
        }
        self.stream.emit(SessionUpdate::SessionInfoUpdate(
            SessionInfoUpdate::new().meta(announced.meta()),
        ));
        Ok(())
    }
}

/// The turn in flight, and whether it has been asked to stop.
struct TurnControl {
    /// Watched by the turn loop, which arms the grace when it changes. A
    /// `watch` rather than a one-shot wake-up so that a second waiter — the
    /// permission relay of task 3.3 — sees the same fact.
    interrupted: watch::Sender<bool>,
    /// How long a cancelled turn gets to end on its own. A field rather than a
    /// constant so a test can watch the whole "it will not stop" path without
    /// waiting out a human-scale timeout.
    grace: Duration,
}

impl TurnControl {
    fn new(grace: Duration) -> Self {
        Self {
            interrupted: watch::Sender::new(false),
            grace,
        }
    }

    /// A new turn starts with nothing remembered: a cancellation of the last one
    /// must not kill this one.
    ///
    /// Called from [`ClaudeSession::begin_turn`] and from nowhere else, which
    /// is the whole of what keeps it safe: the bridge runs it on the ACP
    /// dispatch loop, in the same breath as the lifecycle's own turn guard, so
    /// a cancellation delivered on that loop is either before this reset or
    /// after it — never erased by it.
    ///
    /// `send_replace` and not `send`: between turns nobody is subscribed, and
    /// `send` refuses to change a value nobody is listening to — which would
    /// leave a cancelled session's next turn born already cancelled.
    fn begin(&self) {
        self.interrupted.send_replace(false);
    }

    /// The turn has been asked to stop.
    fn ask_to_stop(&self) {
        self.interrupted.send_replace(true);
    }

    /// Whether the turn has been asked to stop.
    fn asked_to_stop(&self) -> bool {
        *self.interrupted.borrow()
    }

    /// Resolves once the turn has been asked to stop — immediately if it
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

/// How a turn ended, before the session decides what to do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnOutcome {
    /// The turn is over.
    Ended(StopReason),
    /// The turn was cancelled and the CLI kept going.
    Abandoned,
}

/// Everything a session needs in place before the CLI is launched: the private
/// channel, the configuration that tells the CLI where to ask and which tools
/// it has, and the settings that gate every call it makes.
struct Governed {
    /// The channel the shim and the gate ask over.
    rendezvous: Rendezvous,
    /// The MCP configuration the CLI is handed: where to ask, and the session's
    /// own projected servers.
    config: PathBuf,
    /// The settings the CLI is handed: the hook that runs before every tool
    /// call, which no mode of the CLI can skip.
    settings: PathBuf,
    /// Projected servers this adapter could not describe to the CLI, by name.
    /// Nothing is dropped quietly: the session says so.
    undelivered: Vec<String>,
}

impl Governed {
    /// Opens the channel for one session and writes the CLI's configuration
    /// into it.
    ///
    /// # Errors
    ///
    /// Refuses when the channel cannot be created or the configuration cannot
    /// be written. There is no session without it: a CLI launched with nowhere
    /// to ask would run tools nobody governs, which is the one outcome this
    /// whole architecture exists to prevent.
    fn open(session_id: &SessionId, projected: &[McpServer]) -> Result<Self, Refusal> {
        let dir = std::env::temp_dir().join(format!(
            "{}-{}-{}",
            SPEC.name,
            std::process::id(),
            sanitized(session_id.0.as_ref())
        ));
        let rendezvous = Rendezvous::open(dir).map_err(|error| ungoverned(&error.to_string()))?;

        let exe = std::env::current_exe()
            .map_err(|error| ungoverned(&format!("this adapter cannot find itself ({error})")))?;

        // The session's own MCP servers first, then the one that governs it:
        // insertion order decides a name collision, and the permission server
        // is not a thing a projected server may displace.
        let mut servers = serde_json::Map::new();
        let mut undelivered = Vec::new();
        for server in projected {
            match wire::server_entry(server) {
                Some((name, entry)) => {
                    servers.insert(name, entry);
                }
                None => undelivered.push(server_name(server)),
            }
        }
        let (name, server) = permission::shim_server(&exe, rendezvous.address());
        servers.insert(name, server);

        // Written inside the session's own private directory, which is where
        // resolved environment values may live and nowhere else: the daemon
        // already resolved them, this file is the only copy, and it goes when
        // the session does. Nothing of it reaches a log.
        let config = rendezvous.address().join("mcp.json");
        let settings = rendezvous.address().join("settings.json");
        let written =
            std::fs::write(&config, wire::mcp_config(servers).to_string()).and_then(|()| {
                std::fs::write(
                    &settings,
                    gate::settings(&exe, rendezvous.address()).to_string(),
                )
            });
        if let Err(error) = written {
            rendezvous.close();
            return Err(ungoverned(&format!(
                "the CLI's configuration could not be written ({error})"
            )));
        }
        Ok(Self {
            rendezvous,
            config,
            settings,
            undelivered,
        })
    }
}

/// The name of a projected server, whatever its transport.
fn server_name(server: &McpServer) -> String {
    match server {
        McpServer::Stdio(stdio) => stdio.name.clone(),
        McpServer::Http(http) => http.name.clone(),
        McpServer::Sse(sse) => sse.name.clone(),
        _ => "unnamed".to_string(),
    }
}

/// A session id as a directory name: ids are minted by this adapter and are
/// already tame, and this keeps them that way whatever a future id looks like.
fn sanitized(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The refusal for a session that could not be governed.
fn ungoverned(detail: &str) -> Refusal {
    Refusal::new(
        "permission_channel_unavailable",
        SPEC.provider_layer,
        format!("the session's permission channel could not be opened: {detail}"),
        "Meltemi opens a private channel per session inside your temporary directory. \
         Check that it exists and is writable, then open the session again."
            .to_string(),
    )
}

/// Where a turn's updates go.
///
/// The session sends them over ACP; a test collects them. The turn loop is the
/// piece with the ordering subtleties — a cancellation with no interruption to
/// send, an output that ends mid-turn — and this is what lets all of it be
/// exercised without a process in sight.
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
                // and deciding nothing is a denial.
                _ => permission::Decision::Unavailable,
            },
            Err(error) => {
                self.note(&format!("the permission proxy could not decide: {error}"));
                permission::Decision::Unavailable
            }
        }
    }
}

/// Puts a turn on the CLI's input, or says why there is no input left to put it
/// on.
///
/// # Errors
///
/// Refuses with [`session_finished`] when a cancellation already ended this
/// session's input — which on this surface is the only stop there is, so there
/// is nothing left to send a turn on and nothing a retry could fix — and with
/// [`turn_failed`] when the write itself fails, which is a different story and
/// gets a different remedy.
async fn send_turn<W: AsyncWrite + Unpin>(
    writer: &mut FrameWriter<W>,
    message: &Value,
) -> Result<(), Refusal> {
    if writer.is_closed() {
        return Err(session_finished());
    }
    writer
        .write_frame(message)
        .await
        .map_err(|error| turn_failed(&format!("the turn could not be sent ({error})")))
}

/// Runs one turn: read what the CLI says until it says the turn is over — or
/// until a cancellation runs out of patience with it.
///
/// # Errors
///
/// A turn the CLI reports as failed is an error, not a stop reason: ACP has no
/// way to say "the turn did not work", and answering `end_turn` would report a
/// failure as a finished piece of work.
async fn drive_turn<R, S>(
    reader: &mut FrameReader<R>,
    turn: &TurnControl,
    rendezvous: &Rendezvous,
    stream: &S,
    session_id: &SessionId,
) -> Result<TurnOutcome, Refusal>
where
    R: AsyncRead + Unpin + Send,
    S: TurnStream,
{
    let mut mapper = mapping::Mapper::new();

    // Parked in the far future until a cancellation arms it; from then on it is
    // the deadline after which the provider is ended outright.
    let mut grace = std::pin::pin!(tokio::time::sleep(NEVER));
    let interrupted = turn.wait_interrupted();
    let mut interrupted = std::pin::pin!(interrupted);
    let mut armed = false;

    loop {
        tokio::select! {
            () = &mut interrupted, if !armed => {
                armed = true;
                grace.as_mut().reset(tokio::time::Instant::now() + turn.grace);
            }
            () = &mut grace, if armed => return Ok(TurnOutcome::Abandoned),
            // The CLI wants to run something and is blocked until it hears
            // back. While this waits the loop is not reading — which costs
            // nothing, because the CLI is itself waiting for this very answer.
            ask = rendezvous.next_ask() => {
                relay(rendezvous, turn, stream, session_id, &ask).await;
            }
            // Reading a line is cancellation safe, so losing this branch to
            // another costs nothing: no byte of the session is dropped.
            frame = reader.next_frame() => {
                let frame = frame.map_err(|error| {
                    turn_failed(&format!("the CLI's output could not be read ({error})"))
                })?;
                let Some(frame) = frame else {
                    // The CLI closed its output. After a cancellation that is
                    // the cancellation landing; before one it is a provider
                    // that went away mid-turn, which is a different story and
                    // gets a different answer.
                    //
                    // The question is put to the shared fact and not to `armed`
                    // on purpose: `select!` picks at random among ready
                    // branches, so a CLI that closed promptly after being
                    // cancelled could be read here before this loop had even
                    // noticed the cancellation — and a cancellation would land
                    // as an error, at random.
                    return if turn.asked_to_stop() {
                        Ok(TurnOutcome::Ended(StopReason::Cancelled))
                    } else {
                        Err(provider_gone())
                    };
                };
                let event = match frame {
                    Frame::Json(event) => event,
                    Frame::Unparsed(line) => {
                        stream.note(&format!("provider said: {line}"));
                        continue;
                    }
                };
                let mapped = mapper.map(&event);
                for update in mapped.updates {
                    stream.emit(update);
                }
                if let Some(note) = mapped.noted {
                    stream.note(&note);
                }
                match mapped.signal {
                    Some(mapping::Signal::TurnEnded(stop)) => {
                        return Ok(TurnOutcome::Ended(stop));
                    }
                    Some(mapping::Signal::TurnFailed(said)) => return Err(turn_failed(&said)),
                    None => {}
                }
            }
        }
    }
}

impl ProviderDialect for ClaudeDialect {
    type Session = ClaudeSession;

    fn spec(&self) -> AdapterSpec {
        SPEC
    }

    /// What this dialect can honestly do, now that it can do it.
    ///
    /// MCP over both remote transports, because the CLI's own configuration
    /// takes them and every byte of that traffic leaves from the CLI, never
    /// from here. And session load, because the CLI keeps its sessions and this
    /// dialect names its ACP sessions after them.
    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities::new()
            .load_session(true)
            .mcp_capabilities(McpCapabilities::new().http(true).sse(true))
    }

    /// Opens a session under an id of this dialect's own minting.
    ///
    /// The id the bridge handed in is not used: the CLI takes a UUID for its
    /// session flag, and dictating one is what lets a session be named — and
    /// answered for — before the CLI has said anything (task 5.3). The name is
    /// the same on both sides from the first instant, which is what a later
    /// resume needs and what the initial event used to be read for.
    async fn open(
        &self,
        _minted_by_the_bridge: SessionId,
        request: NewSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> Result<Opened<Self::Session>, Refusal> {
        self.start(
            Start {
                session_id: SessionId::new(uuid::Uuid::new_v4().to_string()),
                cwd: request.cwd,
                mcp_servers: request.mcp_servers,
                resume: None,
            },
            cx,
        )
        .await
    }

    async fn load(
        &self,
        request: LoadSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> Result<Self::Session, Refusal> {
        // The id the daemon remembers is the CLI's own, because that is what
        // `open` dictated to it — so resuming needs nothing remembered on
        // either side.
        //
        // What may be resumed is scoped by the CLI's own rule plus this
        // launch's working directory: it looks for the session among those of
        // the directory it runs in, which is the project root or one of its
        // worktrees, exactly the directory the daemon asked for. A session
        // belonging somewhere else is not found, and that refusal is the CLI's
        // to give rather than this adapter's to guess at.
        let previous = request.session_id.0.as_ref().to_string();
        let opened = self
            .start(
                Start {
                    session_id: request.session_id,
                    cwd: request.cwd,
                    mcp_servers: request.mcp_servers,
                    resume: Some(previous),
                },
                cx,
            )
            .await?;
        Ok(opened.session)
    }
}

/// What a session needs to be launched, whether it is new or resumed.
struct Start {
    /// The id this session is addressed by, on both sides: dictated to the CLI
    /// when it is new, and the CLI's own when it is resumed.
    session_id: SessionId,
    /// Where the CLI runs: the project root or one of its worktrees.
    cwd: PathBuf,
    /// The MCP servers the daemon projected onto this session.
    mcp_servers: Vec<McpServer>,
    /// The CLI session to continue, when this is a resume.
    resume: Option<String>,
}

impl ClaudeDialect {
    /// Launches the CLI for one session.
    ///
    /// One path for a new session and a resumed one, because the difference
    /// between them is two arguments: everything that governs a session — the
    /// channel, the gate, the provenance — is the same either way, and a second
    /// path would be a second place for one of them to go missing.
    ///
    /// No handshake happens here, and that is the point of task 5.3: this CLI
    /// announces itself only once it has a turn to run, so the announcement and
    /// the surface check it feeds belong to the first turn
    /// ([`ClaudeSession::announcement`]). What is knowable at launch — which
    /// binary, which version, which id — is recorded here, where it is true.
    ///
    /// # Errors
    ///
    /// Refuses when the CLI cannot be launched or cannot be governed.
    async fn start(
        &self,
        start: Start,
        cx: ConnectionTo<Client>,
    ) -> Result<Opened<ClaudeSession>, Refusal> {
        // Asked before anything is piloted: a binary that cannot answer what it
        // is cannot be piloted either, and the answer is what the session log
        // will hold.
        let version = announced_version(&self.program, &start.cwd).await?;

        // Before the CLI exists, because the CLI is told where to ask as it is
        // launched. A session that could not open this channel is a session
        // whose tool calls nobody would govern, so it does not open at all.
        let governed = Governed::open(&start.session_id, &start.mcp_servers)?;

        let mut args = wire::session_args();
        args.extend(wire::permission_args(
            &governed.config,
            &permission::prompt_tool_reference(),
            &governed.settings,
        ));
        // A resume already names the session it continues; a new one is named
        // here. Never both: two identities in one launch is a question the CLI
        // should not have to answer.
        match &start.resume {
            Some(previous) => args.extend(wire::resume_args(previous)),
            None => args.extend(wire::session_id_args(start.session_id.0.as_ref())),
        }
        let command = ProviderCommand {
            program: self.program.clone(),
            args,
            cwd: start.cwd.clone(),
        };
        let provider = match spawn(&command, SPEC.provider_layer) {
            Ok(provider) => provider,
            Err(refusal) => {
                governed.rendezvous.close();
                return Err(refusal);
            }
        };

        for undelivered in &governed.undelivered {
            // A tool the user declared and never got is exactly the kind of
            // absence that reads as a bug in the agent. It is said out loud.
            eprintln!(
                "{}: the MCP server `{undelivered}` was not handed to the CLI: \
                 this adapter cannot describe its transport in the CLI's configuration",
                SPEC.name
            );
        }

        // The effective binary, the version it turned out to be and the id both
        // sides call this session, into the session log before anything else
        // this session does. What the CLI announces about itself joins them on
        // the first turn, which is the first moment it says anything.
        let session_id = start.session_id;
        let provenance = Provenance {
            program: self.program.clone(),
            version,
            session: session_id.0.as_ref().to_string(),
        };
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta())),
        ));

        // The session is a stream that must be readable while a cancellation
        // writes, so the halves go their separate ways.
        let (control, writer, reader) = provider.into_parts();
        Ok(Opened {
            id: session_id.clone(),
            session: ClaudeSession {
                writer: Mutex::new(writer),
                reader: Mutex::new(reader),
                control: Mutex::new(Some(control)),
                turn: TurnControl::new(INTERRUPT_GRACE),
                rendezvous: governed.rendezvous,
                stream: AcpStream { session_id, cx },
                shutdown: self.shutdown,
                announced: AtomicBool::new(false),
            },
        })
    }
}

/// Asks the CLI what version it is.
///
/// This wire announces no version of its own — the initial event carries a
/// feature array, not a number — so the version the session log records is the
/// one the binary answers to its documented version flag. Both of its output
/// streams are read, because which one a CLI prints its version on is a detail
/// no adapter should be brittle about.
///
/// # Errors
///
/// Refuses when the binary cannot be launched at all: that is the missing layer
/// the fleet catalog names, and no session can follow it.
async fn announced_version(program: &str, cwd: &Path) -> Result<String, Refusal> {
    let asked = tokio::process::Command::new(program)
        .arg(wire::VERSION)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let Ok(answer) = tokio::time::timeout(VERSION_TIMEOUT, asked).await else {
        // A CLI too busy to say what it is will not run a session either, but
        // saying so is the caller's business: this is not the layer's absence.
        return Ok(UNKNOWN_VERSION.to_string());
    };
    let answer = answer.map_err(|error| launch_refusal(program, SPEC.provider_layer, &error))?;

    let said = [answer.stdout, answer.stderr]
        .into_iter()
        .find_map(|stream| {
            let text = String::from_utf8_lossy(&stream).trim().to_string();
            (!text.is_empty()).then_some(text)
        })
        .unwrap_or_default();
    Ok(version_in(&said).unwrap_or(UNKNOWN_VERSION.to_string()))
}

/// The version token inside whatever the CLI printed (`2.1.4 (Claude Code)`).
///
/// Kept forgiving on purpose: the surrounding words are the provider's to
/// change, and a session that failed because a product name moved would be
/// failing for nothing. A pre-release or build suffix stays part of the answer —
/// it is the version the CLI calls itself, and the log records what was said,
/// not a tidied version of it. What is *not* guessed is the number: when no
/// token looks like one, the log says so rather than invent it.
fn version_in(said: &str) -> Option<String> {
    said.lines().next()?.split_whitespace().find_map(|token| {
        let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let mut parts = token.split(['-', '+']).next()?.split('.');
        let looks_like_a_version = parts.clone().count() >= 2
            && parts.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        looks_like_a_version.then(|| token.to_string())
    })
}

/// Reads the session's initial event: the handshake of this dialect, which
/// happens on the first turn because that is when the CLI first speaks.
///
/// # Errors
///
/// Refuses when the CLI says nothing, takes too long, ends before announcing
/// itself, or refuses the session outright — in which case the provider's own
/// words travel to the human unchanged, because an authentication failure is
/// the provider's to explain and this adapter has nothing to add to it.
async fn handshake<R>(reader: &mut FrameReader<R>) -> Result<wire::InitEvent, Refusal>
where
    R: AsyncRead + Unpin + Send,
{
    tokio::time::timeout(HANDSHAKE_TIMEOUT, await_init(reader))
        .await
        .map_err(|_| {
            handshake_failed(&format!(
                "the CLI did not announce the session within {} seconds of being sent a turn",
                HANDSHAKE_TIMEOUT.as_secs()
            ))
        })?
}

/// Reads until the initial event arrives.
async fn await_init<R>(reader: &mut FrameReader<R>) -> Result<wire::InitEvent, Refusal>
where
    R: AsyncRead + Unpin + Send,
{
    loop {
        let frame = reader.next_frame().await.map_err(|error| {
            handshake_failed(&format!("the CLI's output could not be read ({error})"))
        })?;
        let Some(frame) = frame else {
            return Err(handshake_failed(
                "the CLI ended without announcing the session",
            ));
        };
        let event = match frame {
            Frame::Json(value) => value,
            // Not JSON at all: a banner or a warning. Kept on the adapter's own
            // stderr, where the daemon already collects the provider's noise,
            // and never mistaken for protocol.
            Frame::Unparsed(line) => {
                eprintln!("{}: provider said: {line}", SPEC.name);
                continue;
            }
        };
        if wire::is_init(&event) {
            return serde_json::from_value(event).map_err(|error| {
                handshake_failed(&format!(
                    "the CLI announced a session this adapter cannot read ({error})"
                ))
            });
        }
        if let Some(refused) = refusal_before_the_session(&event) {
            return Err(refused);
        }
        eprintln!("{}: provider said: {event}", SPEC.name);
    }
}

/// The provider's own refusal, when it ends the session before announcing it.
///
/// This is where an authentication failure lands, and the provider's message is
/// passed through verbatim: the CLI owns the sign-in, so it owns the
/// explanation too. The adapter adds a remedy and no interpretation.
fn refusal_before_the_session(event: &Value) -> Option<Refusal> {
    if event.get("type").and_then(Value::as_str) != Some(wire::RESULT) {
        return None;
    }
    let said = event
        .get("result")
        .and_then(Value::as_str)
        .or_else(|| event.get("subtype").and_then(Value::as_str))
        .unwrap_or("the CLI ended the session without saying why");
    Some(Refusal::new(
        "provider_refused_session",
        SPEC.provider_layer,
        format!("the CLI ended the session before it began: {said}"),
        "The message above is the provider's own. Run the CLI by hand once to see it in full \
         and to sign in if that is what it is asking for."
            .to_string(),
    ))
}

/// Puts one of the CLI's permission questions to the proxy, and answers the
/// shim with what the proxy said.
///
/// A cancellation during the wait still lands: the proxy is free to take as
/// long as a human takes, and a turn that was told to stop while a question sat
/// unanswered would otherwise wait for both. The question is then withdrawn
/// rather than answered, which on this channel means the call does not run.
async fn relay<S: TurnStream>(
    rendezvous: &Rendezvous,
    turn: &TurnControl,
    stream: &S,
    session_id: &SessionId,
    ask: &rendezvous::Ask,
) {
    // A tool that needs a person in front of it is not a permission question:
    // no answer the proxy could give would make it work, so it is refused here
    // and the reason is shown. Asking a human to approve something that cannot
    // happen would be worse than useless.
    let payload = match permission::interactive_only(&ask.tool) {
        Some(reason) => permission::deny(reason),
        None => {
            let request = permission::request_for(
                session_id,
                &ask.tool,
                ask.tool_use_id.as_deref(),
                &ask.input,
            );
            let decision = tokio::select! {
                decision = stream.ask(request) => decision,
                () = turn.wait_interrupted() => permission::Decision::Cancelled,
            };
            permission::payload(&decision, &ask.input)
        }
    };
    // A refusal the human never sees is a session that quietly did less than it
    // was asked to. It is shown against the call it is about, with the reason
    // that produced it.
    if let Some(reason) = payload
        .get("message")
        .and_then(Value::as_str)
        .filter(|_| payload.get("behavior").and_then(Value::as_str) == Some("deny"))
    {
        stream.emit(permission::refusal_update(
            ask.tool_use_id.as_deref().unwrap_or(&ask.tool),
            reason,
        ));
        stream.note(&format!("`{}` was refused: {reason}", ask.tool));
    }
    rendezvous.answer(ask, &payload);
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

/// The refusal for a turn asked of a session a cancellation already ended.
///
/// This dialect cancels by ending the CLI's input, because that is the only
/// stop its surface documents ([`ClaudeSession::interrupt`]). The session
/// survives as an ACP object — the daemon may still read it, close it, resume
/// it by id — but it has no wire left to put a turn on, and a `turn_failed`
/// inviting the human to "try again" would be a lie about which of those is
/// true.
fn session_finished() -> Refusal {
    Refusal::new(
        "session_finished",
        SPEC.provider_layer,
        "this session was cancelled, and on this surface a cancellation ends the CLI's input \
         for good: no further turn can be sent on it"
            .to_string(),
        "Open a new session. Its previous turns are not lost: this agent's sessions can be \
         resumed by id."
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

/// The refusal for a session the CLI never announced.
fn handshake_failed(detail: &str) -> Refusal {
    Refusal::new(
        "provider_handshake_failed",
        SPEC.provider_layer,
        detail.to_string(),
        format!(
            "Check that the official CLI is installed and signed in, and that `{} {}` answers a \
             message written on its input (`meltemi fleet` shows the entry and its remedy).",
            SPEC.provider_bin,
            wire::session_args().join(" ")
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a handshake against a CLI that says exactly `lines`.
    ///
    /// Nothing is written to the CLI here, and nothing can be: by the time this
    /// runs, the turn has already been sent and the writer is somebody else's
    /// half of the session. That separation is the fix of task 5.3 in the type
    /// system — the handshake reads, and reading is all it can do.
    async fn handshake_against(lines: &str) -> Result<wire::InitEvent, Refusal> {
        handshake(&mut saying(lines)).await
    }

    const SIGNED_IN: &str = concat!(
        r#"{"type":"system","subtype":"init","session_id":"7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b","#,
        r#""model":"mock-sonnet","apiKeySource":"none","permissionMode":"default","#,
        r#""tools":["Read"],"claude_code_version":"2.1.167"}"#,
        "\n"
    );

    #[tokio::test]
    async fn the_handshake_reads_the_surface_the_cli_announces() {
        // Scenario: Versión efectiva registrada en el log
        let init = handshake_against(SIGNED_IN)
            .await
            .expect("the signed-in surface opens the session");
        let announced = Surface::of(&init);
        assert_eq!(announced.key_source, surface::KeySource::SignedIn);
        assert_eq!(announced.model.as_deref(), Some("mock-sonnet"));
        assert_eq!(
            announced.meta()["meltemi"]["providerKeySource"],
            "none",
            "and what it announced reaches the session log"
        );
    }

    #[tokio::test]
    async fn a_cli_that_would_run_on_a_key_is_refused_and_nothing_is_injected() {
        // Scenario: Rehúso ante modo que exige clave de API
        //
        // The pinned risk of design D4, end to end at the handshake: the CLI
        // comes up on a key instead of the session the user signed into. The
        // adapter refuses — and the one thing it must never do, send something
        // to make the session work anyway, it has no way to do from here: the
        // handshake holds the reading half and nothing else.
        let init = handshake_against(concat!(
            r#"{"type":"system","subtype":"init","apiKeySource":"ANTHROPIC_API_KEY"}"#,
            "\n"
        ))
        .await
        .expect("the event itself is readable");
        let refusal = Surface::of(&init)
            .check(SPEC.provider_layer)
            .expect_err("a key-bearing surface is not the one this adapter pilots");
        assert_eq!(refusal.kind, "provider_surface_not_signed_in");
        assert!(refusal.detail.contains("ANTHROPIC_API_KEY"));
    }

    #[tokio::test]
    async fn a_cli_that_ends_before_the_session_hands_its_own_words_to_the_human() {
        // Scenario: La autenticación queda en el binario oficial
        //
        // The sign-in belongs to the CLI, so its explanation does too: the
        // adapter passes the message through and adds a remedy, never an
        // interpretation and never a credential.
        let refusal = handshake_against(concat!(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"#,
            r#""result":"Invalid API key · Please run /login"}"#,
            "\n"
        ))
        .await
        .expect_err("a session that never began cannot be piloted");
        assert_eq!(refusal.kind, "provider_refused_session");
        assert!(
            refusal.detail.contains("Please run /login"),
            "the provider's own words travel unchanged: {}",
            refusal.detail
        );
    }

    #[tokio::test]
    async fn a_cli_that_announces_nothing_refuses_rather_than_opening_a_session() {
        for said in [
            "",
            "starting up\n",
            "{\"type\":\"stream_event\",\"event\":{}}\n",
        ] {
            let refusal = handshake_against(said)
                .await
                .expect_err("a session cannot open on a handshake that failed");
            assert_eq!(refusal.kind, "provider_handshake_failed", "for `{said}`");
            assert!(
                refusal.remedy.contains(wire::STREAM_JSON),
                "the remedy names the surface that was expected: {}",
                refusal.remedy
            );
        }
    }

    #[tokio::test]
    async fn the_announcement_is_waited_for_only_after_a_turn_has_been_sent() {
        // Scenario: Eventos de sesión mapeados en streaming
        //
        // The defect task 5.3 exists for, as a deadline: a CLI that has been
        // given nothing says nothing, so a handshake that ran before the turn
        // could only ever end in the timeout. Here the wire stays open and
        // silent — as the real binary does, verified against 2.1.167 — and the
        // read is shown to be a wait rather than an answer.
        let (_cli, adapter_side) = tokio::io::duplex(64);
        let mut reader = FrameReader::new(adapter_side);
        assert!(
            tokio::time::timeout(Duration::from_millis(200), await_init(&mut reader))
                .await
                .is_err(),
            "an unprompted CLI announces nothing, and the adapter waits for it"
        );
    }

    #[tokio::test]
    async fn the_official_cli_missing_refuses_before_anything_is_piloted() {
        // Scenario: CLI oficial ausente al lanzar
        let refusal = announced_version("meltemi-no-such-provider-cli", &std::env::temp_dir())
            .await
            .expect_err("a CLI that does not exist cannot be asked what it is");
        assert_eq!(refusal.kind, "provider_cli_not_launchable");
        assert_eq!(refusal.layer, SPEC.provider_layer);
        assert!(refusal.remedy.contains("PATH"), "{}", refusal.remedy);
    }

    #[test]
    fn the_version_is_taken_from_what_the_cli_printed_and_never_invented() {
        assert_eq!(version_in("2.1.4 (Claude Code)"), Some("2.1.4".into()));
        assert_eq!(version_in("1.0"), Some("1.0".into()));
        assert_eq!(
            version_in("2.0.0-mock (mock-claude-wire)"),
            Some("2.0.0-mock".into()),
            "a pre-release suffix is part of what the CLI calls itself"
        );
        assert_eq!(
            version_in("2.1.4\nsomething else"),
            Some("2.1.4".into()),
            "the first line is the answer; the rest is the CLI talking"
        );
        for said in ["", "Claude Code", "nightly", "v-beta"] {
            assert_eq!(version_in(said), None, "`{said}` carries no version");
        }
    }

    #[test]
    fn the_provenance_travels_as_acp_extension_metadata() {
        // Scenario: Versión efectiva registrada en el log
        //
        // What the log will hold the moment the session opens: which binary was
        // launched, which version it turned out to be, and the id both sides
        // call the session — every one of them true before the CLI has said a
        // word. Not a private channel: ACP's own `_meta` (design D7), which any
        // agent may fill the same way.
        let provenance = Provenance {
            program: "/usr/local/bin/claude".into(),
            version: "2.1.167".into(),
            session: "7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b".into(),
        };
        let update =
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta()));
        let logged = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerBin"],
            "/usr/local/bin/claude"
        );
        assert_eq!(logged["_meta"]["meltemi"]["providerVersion"], "2.1.167");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerSession"], "7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b",
            "the id a later resume will name"
        );
    }

    #[test]
    fn the_sessions_mcp_projection_is_handed_to_the_cli_beside_the_one_that_governs_it() {
        // Scenario: Proyección MCP entregada al CLI
        //
        // What the user declared reaches the CLI through the channel the CLI
        // documents for it, and the server that governs the session goes in
        // last: a projected server may not displace the one every tool call is
        // decided through, whatever it is called.
        use agent_client_protocol::schema::v1::{EnvVariable, McpServerHttp, McpServerStdio};

        let projected = vec![
            McpServer::Stdio({
                let mut stdio = McpServerStdio::new("fs", "mcp-fs");
                stdio.args = vec!["--root".into(), ".".into()];
                stdio.env = vec![EnvVariable::new("TOKEN", "resolved-by-the-daemon")];
                stdio
            }),
            McpServer::Http(McpServerHttp::new("issues", "https://example.invalid/mcp")),
            // A server calling itself what the permission server is called.
            McpServer::Stdio(McpServerStdio::new(permission::SERVER, "impostor")),
        ];
        let governed = Governed::open(&SessionId::new("s-mcp"), &projected)
            .expect("the session can be governed");
        let written: Value = serde_json::from_str(
            &std::fs::read_to_string(&governed.config)
                .expect("the CLI's configuration was written"),
        )
        .expect("and it is JSON");
        let servers = &written["mcpServers"];

        assert_eq!(servers["fs"]["type"], "stdio");
        assert_eq!(servers["fs"]["args"][0], "--root");
        assert_eq!(
            servers["fs"]["env"]["TOKEN"], "resolved-by-the-daemon",
            "the value the daemon resolved travels to the CLI and nowhere else"
        );
        assert_eq!(servers["issues"]["type"], "http");
        assert_eq!(servers["issues"]["url"], "https://example.invalid/mcp");
        assert!(
            servers[permission::SERVER]["args"]
                .as_array()
                .is_some_and(|args| args.iter().any(|arg| arg == shim::SHIM_ARG)),
            "the server that governs the session is the one that answers to its name: {servers}"
        );
        assert!(governed.undelivered.is_empty());

        governed.rendezvous.close();
        assert!(
            !governed.config.exists(),
            "and the file with the resolved values dies with the session"
        );
    }

    #[test]
    fn the_environment_override_decides_which_binary_is_launched() {
        assert_eq!(resolve_program(None, SPEC.provider_bin), SPEC.provider_bin);
        assert_eq!(
            resolve_program(Some("/tmp/mock-claude-wire".into()), SPEC.provider_bin),
            "/tmp/mock-claude-wire"
        );
    }

    /// A turn's updates, collected instead of sent, and a permission proxy that
    /// answers whatever the test arranged.
    #[derive(Default)]
    struct Collected {
        updates: std::sync::Mutex<Vec<SessionUpdate>>,
        notes: std::sync::Mutex<Vec<String>>,
        asked: std::sync::Mutex<Vec<RequestPermissionRequest>>,
        answers: std::sync::Mutex<Vec<permission::Decision>>,
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
                // exactly the case that must end in a denial.
                .unwrap_or(permission::Decision::Unavailable)
        }
    }

    impl Collected {
        fn said(&self) -> String {
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

        fn tool_calls(&self) -> usize {
            self.updates
                .lock()
                .unwrap()
                .iter()
                .filter(|update| matches!(update, SessionUpdate::ToolCall(_)))
                .count()
        }
    }

    /// A CLI that says exactly `lines` and then closes its output.
    fn saying(lines: &str) -> FrameReader<std::io::Cursor<Vec<u8>>> {
        FrameReader::new(std::io::Cursor::new(lines.as_bytes().to_vec()))
    }

    /// A permission channel of this test's own.
    fn channel(tag: &str) -> Rendezvous {
        Rendezvous::open(
            std::env::temp_dir().join(format!("meltemi-claude-turn-{}-{tag}", std::process::id())),
        )
        .expect("the channel opens")
    }

    fn session() -> SessionId {
        SessionId::new("s-1")
    }

    #[tokio::test]
    async fn a_turn_streams_what_the_cli_says_and_ends_on_its_result() {
        // Scenario: Eventos de sesión mapeados en streaming
        let mut reader = saying(concat!(
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Working"}}}"#,
            "\n",
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":" on it."}}}"#,
            "\n",
            "a banner the CLI printed\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Write","input":{"file_path":"NOTES.md"}}]}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"ok"}]}}"#,
            "\n",
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done."}"#,
            "\n",
        ));
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_millis(50));

        let channel = channel("streams");
        let outcome = drive_turn(&mut reader, &turn, &channel, &stream, &session())
            .await
            .unwrap();
        channel.close();
        assert_eq!(outcome, TurnOutcome::Ended(StopReason::EndTurn));
        assert_eq!(stream.said(), "Working on it.");
        assert_eq!(stream.tool_calls(), 1);
        assert!(
            stream
                .notes
                .lock()
                .unwrap()
                .iter()
                .any(|note| note.contains("a banner")),
            "and a line that is not JSON is kept rather than swallowed"
        );
    }

    #[tokio::test]
    async fn a_permission_question_reaches_the_proxy_mid_turn_and_the_shim_gets_its_answer() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The path a tool call actually takes: the CLI asks its permission tool,
        // the shim relays over the private channel, the turn loop puts the
        // question to the proxy, and what the proxy decided is what goes back —
        // while the turn is still running and still streaming.
        let (mut cli, adapter_side) = tokio::io::duplex(4096);
        let mut reader = FrameReader::new(adapter_side);
        let stream = Collected::default();
        stream
            .answers
            .lock()
            .unwrap()
            .push(permission::Decision::Selected(
                permission::ALLOW_ONCE.into(),
            ));
        let turn = TurnControl::new(Duration::from_secs(30));
        let channel = channel("relay");
        let session = session();

        let asking = tokio::task::spawn_blocking({
            let dir = channel.address().to_path_buf();
            move || {
                rendezvous::ask(
                    &dir,
                    "Write",
                    Some("toolu_1"),
                    &serde_json::json!({"file_path": "NOTES.md"}),
                )
            }
        });

        let driving = async {
            let outcome = drive_turn(&mut reader, &turn, &channel, &stream, &session).await;
            outcome.expect("the turn ran")
        };
        let feeding = async {
            // Wait until the question has been answered before ending the turn:
            // in life the CLI does exactly this, because it is blocked on the
            // tool result.
            let answered = asking.await.expect("the shim finished");
            tokio::io::AsyncWriteExt::write_all(
                &mut cli,
                br#"{"type":"result","subtype":"success","is_error":false,"result":"Done."}"#,
            )
            .await
            .unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut cli, b"\n")
                .await
                .unwrap();
            answered
        };

        let (outcome, answered) = tokio::join!(driving, feeding);
        assert_eq!(outcome, TurnOutcome::Ended(StopReason::EndTurn));

        let decision = answered.expect("a decision came back to the shim");
        assert_eq!(decision["behavior"], "allow");
        assert_eq!(
            decision["updatedInput"]["file_path"], "NOTES.md",
            "the call runs exactly as it was approved"
        );

        let asked = stream.asked.lock().unwrap();
        assert_eq!(asked.len(), 1, "the question was put once");
        assert_eq!(
            asked[0].tool_call.tool_call_id.0.as_ref(),
            "toolu_1",
            "against the very call the CLI streamed"
        );
        drop(asked);
        channel.close();
    }

    #[tokio::test]
    async fn a_proxy_that_cannot_decide_denies_the_call_and_the_session_shows_why() {
        // Scenario: Interacción no relevable denegada con motivo visible
        //
        // No decision — no client, an error, an option nobody offered — is a
        // denial, and a denial nobody can see is a session that quietly did less
        // than it was asked to.
        let (mut cli, adapter_side) = tokio::io::duplex(4096);
        let mut reader = FrameReader::new(adapter_side);
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_secs(30));
        let channel = channel("denied");
        let session = session();

        let asking = tokio::task::spawn_blocking({
            let dir = channel.address().to_path_buf();
            move || rendezvous::ask(&dir, "Bash", Some("toolu_2"), &serde_json::json!({}))
        });

        let driving = async { drive_turn(&mut reader, &turn, &channel, &stream, &session).await };
        let feeding = async {
            let answered = asking.await.expect("the shim finished");
            tokio::io::AsyncWriteExt::write_all(
                &mut cli,
                b"{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false}\n",
            )
            .await
            .unwrap();
            answered
        };
        let (outcome, answered) = tokio::join!(driving, feeding);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::EndTurn));

        let decision = answered.expect("a decision came back");
        assert_eq!(decision["behavior"], "deny");

        let shown = stream.updates.lock().unwrap();
        let refusal = shown
            .iter()
            .find_map(|update| match update {
                SessionUpdate::ToolCallUpdate(update)
                    if update.tool_call_id.0.as_ref() == "toolu_2" =>
                {
                    Some(update.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the refusal is in the session: {shown:?}"));
        assert_eq!(
            refusal.fields.status,
            Some(agent_client_protocol::schema::v1::ToolCallStatus::Failed)
        );
        assert!(
            refusal.fields.content.is_some(),
            "with the reason that produced it"
        );
        drop(shown);
        channel.close();
    }

    #[tokio::test]
    async fn a_tool_that_needs_a_person_is_refused_without_troubling_one() {
        // Scenario: Interacción no relevable denegada con motivo visible
        //
        // The loss this dialect pays for being headless, made visible. No
        // answer the proxy could give would let this tool work, so the proxy is
        // never asked — and the refusal is shown in the session with the reason
        // that produced it, rather than leaving the human to wonder what the
        // agent did instead.
        let (mut cli, adapter_side) = tokio::io::duplex(4096);
        let mut reader = FrameReader::new(adapter_side);
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_secs(30));
        let channel = channel("interactive");
        let session = session();

        let asking = tokio::task::spawn_blocking({
            let dir = channel.address().to_path_buf();
            move || {
                rendezvous::ask(
                    &dir,
                    "AskUserQuestion",
                    Some("toolu_3"),
                    &serde_json::json!({"question": "which one?"}),
                )
            }
        });

        let driving = async { drive_turn(&mut reader, &turn, &channel, &stream, &session).await };
        let feeding = async {
            let answered = asking.await.expect("the shim finished");
            tokio::io::AsyncWriteExt::write_all(
                &mut cli,
                b"{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false}\n",
            )
            .await
            .unwrap();
            answered
        };
        let (outcome, answered) = tokio::join!(driving, feeding);
        assert_eq!(outcome.unwrap(), TurnOutcome::Ended(StopReason::EndTurn));

        let decision = answered.expect("a decision came back");
        assert_eq!(decision["behavior"], "deny");
        assert!(
            decision["message"]
                .as_str()
                .is_some_and(|said| said.contains("headless")),
            "the CLI is told why, in its own transcript: {decision}"
        );
        assert!(
            stream.asked.lock().unwrap().is_empty(),
            "nobody was asked to approve something that could not have worked"
        );
        assert!(
            stream.updates.lock().unwrap().iter().any(|update| matches!(
                update,
                SessionUpdate::ToolCallUpdate(update)
                    if update.tool_call_id.0.as_ref() == "toolu_3"
            )),
            "and the session shows the refusal: {:?}",
            stream.updates.lock().unwrap()
        );
        channel.close();
    }

    #[tokio::test]
    async fn a_turn_the_cli_reports_as_failed_is_an_error_and_not_a_finished_turn() {
        let mut reader = saying(concat!(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"it broke"}"#,
            "\n",
        ));
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_millis(50));

        let channel = channel("failed");
        let refusal = drive_turn(&mut reader, &turn, &channel, &stream, &session())
            .await
            .expect_err("a failed turn is not a stop reason");
        channel.close();
        assert_eq!(refusal.kind, "provider_turn_failed");
        assert!(refusal.detail.contains("it broke"), "{}", refusal.detail);
    }

    #[tokio::test]
    async fn a_cli_that_vanishes_mid_turn_refuses_instead_of_answering_end_turn() {
        let mut reader = saying(r#"{"type":"stream_event","event":{}}"#);
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_millis(50));

        let channel = channel("gone");
        let refusal = drive_turn(&mut reader, &turn, &channel, &stream, &session())
            .await
            .expect_err("a turn cannot end well on a CLI that is gone");
        channel.close();
        assert_eq!(refusal.kind, "provider_gone");
    }

    #[tokio::test]
    async fn a_cancelled_turn_that_the_cli_ignores_is_abandoned_so_the_session_can_end_it() {
        // This surface documents no interruption: the end of its input stops
        // the *next* turn, not this one. The grace is therefore the whole of
        // what makes a cancellation a cancellation, and a CLI that talks
        // through it does not get to hold the session open.
        let (mut cli, adapter_side) = tokio::io::duplex(4096);
        let mut reader = FrameReader::new(adapter_side);
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_millis(30));

        let channel = channel("abandoned");
        let session = session();
        let driving = drive_turn(&mut reader, &turn, &channel, &stream, &session);
        let talking = async {
            loop {
                tokio::io::AsyncWriteExt::write_all(
                    &mut cli,
                    br#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"."}}}"#,
                )
                .await
                .unwrap();
                tokio::io::AsyncWriteExt::write_all(&mut cli, b"\n")
                    .await
                    .unwrap();
                turn.ask_to_stop();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };

        let outcome = tokio::select! {
            outcome = driving => outcome,
            () = talking => unreachable!("the CLI never stops"),
        };
        assert_eq!(outcome.unwrap(), TurnOutcome::Abandoned);
    }

    #[tokio::test]
    async fn a_cancelled_turn_the_cli_does_end_is_cancelled_and_not_a_failure() {
        // The other half of the same story: the CLI takes the hint and closes
        // its output. That is the cancellation landing, not a provider that
        // disappeared, and the two must not read the same.
        let mut reader = saying("");
        let stream = Collected::default();
        let turn = TurnControl::new(Duration::from_secs(30));
        turn.ask_to_stop();

        let channel = channel("cancelled");
        assert_eq!(
            drive_turn(&mut reader, &turn, &channel, &stream, &session())
                .await
                .unwrap(),
            TurnOutcome::Ended(StopReason::Cancelled)
        );
        channel.close();
    }

    #[tokio::test]
    async fn a_turn_asked_of_a_cancelled_session_is_refused_with_the_truth_about_it() {
        // Scenario: Turno posterior a una cancelación rehusado con remedio
        //
        // This dialect cancels by ending the CLI's input, because that is the
        // only stop its surface documents — so a cancelled session has no wire
        // left to put a turn on. It says exactly that. Answering `try again` on
        // a session that can never take another turn would send the human back
        // to a door that is not there.
        let (adapter_side, _cli) = tokio::io::duplex(1024);
        let mut writer = FrameWriter::new(adapter_side);
        let message = wire::user_message("otra vez");
        send_turn(&mut writer, &message)
            .await
            .expect("an open session takes a turn");

        // What `interrupt` does, and the whole of what a cancellation can do
        // here.
        writer.close();
        let refusal = send_turn(&mut writer, &message)
            .await
            .expect_err("a cancelled session has no input left");
        assert_eq!(refusal.kind, "session_finished");
        assert_eq!(refusal.layer, SPEC.provider_layer);
        assert!(
            refusal.remedy.contains("Open a new session"),
            "and the remedy is the one that works: {}",
            refusal.remedy
        );
        assert!(
            refusal.remedy.contains("resumed by id"),
            "with what it costs, which is nothing: {}",
            refusal.remedy
        );
    }

    #[test]
    fn a_new_turn_is_not_tainted_by_the_cancellation_of_the_last_one() {
        // Nobody is subscribed between turns, which is exactly when a `watch`
        // refuses to change: a session whose first turn was cancelled would
        // have started its second one already cancelled.
        let turn = TurnControl::new(Duration::from_millis(10));
        turn.ask_to_stop();
        assert!(turn.asked_to_stop());
        turn.begin();
        assert!(
            !turn.asked_to_stop(),
            "a stale cancellation must not kill the next honest turn"
        );
    }
}
