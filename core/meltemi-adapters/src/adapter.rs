// SPDX-License-Identifier: Apache-2.0

//! The ACP surface both adapters share (adaptadores-propios-acp tasks 1.1,
//! 2.2).
//!
//! Toward meltemid an adapter is an ordinary ACP agent over stdio: the daemon
//! pilots it exactly as it pilots a native level-1 agent, with the same
//! permission proxy, the same worktrees and the same session log. Nothing here
//! knows the daemon — that is the point of D7's governance rule: no private
//! channel, no capability hanging off an id.
//!
//! This module owns the four entry points the daemon uses — `initialize`,
//! `session/new`, `session/prompt`, `session/cancel` — and the session
//! bookkeeping around them. What it deliberately does **not** own is the
//! provider wire: that is a [`ProviderDialect`], and each binary supplies its
//! own. The seam is narrow on purpose — open a session, run a turn, interrupt
//! it, end it — because everything either dialect shares should live here and
//! everything specific to one provider should be impossible to leak in.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, SessionId, StopReason,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use tokio::sync::Mutex;

use crate::bridge::{CancelAction, CloseAction, LifecycleError, SessionLifecycle, TurnEnd};
use crate::diagnostic::Refusal;
use crate::supervisor::ShutdownPolicy;

/// Which provider surface an adapter binary translates to ACP.
///
/// The names are the neutral ones the living spec uses; which concrete CLI and
/// which concrete flags each dialect means is data of the fleet registry and of
/// this change's design, never of the specs (design D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// A headless session of newline-delimited JSON events over stdio.
    HeadlessSession,
    /// A JSON-RPC 2.0 server with newline delimitation over stdio.
    JsonRpcServer,
}

/// One adapter binary's identity: what it announces over ACP, and which
/// official CLI it pilots.
#[derive(Debug, Clone, Copy)]
pub struct AdapterSpec {
    /// The adapter's name in the ACP handshake.
    pub name: &'static str,
    /// The provider layer named in every refusal, worded as the fleet catalog
    /// words it — the human must recognize the piece being talked about.
    pub provider_layer: &'static str,
    /// The official CLI this adapter pilots, as it is looked up on the PATH.
    pub provider_bin: &'static str,
    /// The provider surface this adapter translates.
    pub dialect: Dialect,
}

/// The provider side of one ACP session, once a dialect has opened it.
///
/// Every method takes `&self`: a cancellation arrives while a turn is in
/// flight, and a session that had to be locked to be interrupted could not be
/// interrupted at all.
pub trait ProviderSession: Send + Sync + 'static {
    /// Prepares the dialect for the turn the bridge has just accepted:
    /// whatever the last turn left behind — a cancellation asked for, an
    /// interruption still owed — goes now, before this one exists.
    ///
    /// Synchronous, and called on the ACP dispatch loop rather than from inside
    /// [`ProviderSession::run_turn`], which is the whole reason it is a method
    /// of its own. `session/cancel` is delivered on that same loop, so the
    /// order between the two is decided there and nowhere else: a cancellation
    /// either arrives before the prompt was accepted, and there is no turn to
    /// stop, or after this reset, and it stands. A dialect that reset itself
    /// when the turn *began* — which is later, in a task the loop spawned — had
    /// a window between the two where a cancellation was recorded and then
    /// erased by the turn it was meant for: nothing was sent to the provider,
    /// the whole turn ran, and the session still answered `cancelled`.
    fn begin_turn(&self);

    /// Runs one turn and answers with the ACP stop reason it earned.
    ///
    /// Streaming updates travel over the connection the session was opened
    /// with; what comes back here is only how the turn ended.
    fn run_turn(
        &self,
        prompt: PromptRequest,
    ) -> impl Future<Output = Result<StopReason, Refusal>> + Send;

    /// Interrupts the turn in flight, in the provider's own terms.
    fn interrupt(&self) -> impl Future<Output = ()> + Send;

    /// Ends the provider process.
    fn shutdown(&self, policy: ShutdownPolicy) -> impl Future<Output = ()> + Send;
}

/// A session that was opened, and the id it answers to.
///
/// The id is the dialect's to choose because resuming is: a CLI that keeps its
/// own sessions is asked to resume one **by its own id**, so a dialect that can
/// resume names its ACP sessions after the provider's, and the daemon's later
/// `session/load` needs no memory anywhere to translate. A dialect that cannot
/// resume keeps the id it was handed.
pub struct Opened<S> {
    /// The id this session answers to from here on.
    pub id: SessionId,
    /// The provider side of it.
    pub session: S,
}

/// How one adapter binary pilots its provider.
pub trait ProviderDialect: Send + Sync + 'static {
    /// The provider side of a session in this dialect.
    type Session: ProviderSession;

    /// What this binary announces over ACP and which CLI it pilots.
    fn spec(&self) -> AdapterSpec;

    /// What this dialect announces in the ACP handshake.
    ///
    /// Announced honestly and never earlier than true: a capability claimed
    /// here makes the daemon offer something — a resume, a set of MCP servers —
    /// that the dialect must then actually be able to do.
    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities::new()
    }

    /// Launches the official CLI for a new ACP session and hands back the
    /// session that pilots it.
    ///
    /// # Errors
    ///
    /// Refuses — never falls back — when the CLI cannot be launched or does not
    /// offer the surface the adapter requires.
    fn open(
        &self,
        session_id: SessionId,
        request: NewSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> impl Future<Output = Result<Opened<Self::Session>, Refusal>> + Send;

    /// Reopens a session the CLI itself kept, so that a turn continues where
    /// the last one left off.
    ///
    /// # Errors
    ///
    /// Refuses when the dialect cannot resume at all — which is the default,
    /// and which is also why such a dialect must not announce that it can.
    fn load(
        &self,
        request: LoadSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> impl Future<Output = Result<Self::Session, Refusal>> + Send {
        let spec = self.spec();
        let _ = (request, cx);
        async move {
            Err(Refusal::new(
                "session_load_unsupported",
                spec.provider_layer,
                "this adapter cannot reopen a previous session of the piloted CLI".to_string(),
                "Start a new session; this agent's sessions are not resumable through Meltemi."
                    .to_string(),
            ))
        }
    }
}

/// A session the daemon opened on this adapter.
pub struct Session<S> {
    /// The lifecycle rules that keep the ACP session and the provider process
    /// in step. Held under a plain lock and never across an await: the
    /// transitions are decisions, not work.
    lifecycle: StdMutex<SessionLifecycle>,
    /// The provider process this session supervises. Behind an `Arc` so a turn
    /// and a cancellation can hold it at once — which is the whole point of a
    /// cancellation.
    provider: Mutex<Option<Arc<S>>>,
}

impl<S: ProviderSession> Session<S> {
    /// A session over an already-opened provider.
    fn new(provider: S) -> Self {
        let mut lifecycle = SessionLifecycle::new();
        lifecycle
            .provider_started()
            .expect("a fresh lifecycle has no provider yet");
        Self {
            lifecycle: StdMutex::new(lifecycle),
            provider: Mutex::new(Some(Arc::new(provider))),
        }
    }

    /// Runs a lifecycle transition under the lock.
    fn with_lifecycle<T>(&self, transition: impl FnOnce(&mut SessionLifecycle) -> T) -> T {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        transition(&mut lifecycle)
    }

    /// The provider, if the session still has one.
    async fn provider(&self) -> Option<Arc<S>> {
        self.provider.lock().await.clone()
    }

    /// Closes the session and ends its provider process.
    async fn close(&self, policy: ShutdownPolicy) {
        if self.with_lifecycle(SessionLifecycle::close) == CloseAction::ShutDownProvider
            && let Some(provider) = self.provider.lock().await.take()
        {
            // A provider that will not go politely is killed: the session is
            // over, and nothing of it may outlive the worktree it ran in.
            provider.shutdown(policy).await;
        }
    }
}

/// The adapter's live sessions.
struct Sessions<S> {
    entries: Mutex<HashMap<String, Arc<Session<S>>>>,
    counter: AtomicU64,
}

impl<S> Default for Sessions<S> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(0),
        }
    }
}

impl<S: ProviderSession> Sessions<S> {
    /// The id the next session will be addressed by. Minted before the provider
    /// is launched because the dialect needs it: a session's very first update
    /// — the provenance of the binary it just launched — already belongs to it.
    fn next_id(&self, prefix: &str) -> SessionId {
        SessionId::new(format!(
            "{prefix}-{}",
            self.counter.fetch_add(1, Ordering::Relaxed) + 1
        ))
    }

    /// Registers an opened session under the id it was minted with.
    async fn register(&self, id: &SessionId, provider: S) {
        self.entries
            .lock()
            .await
            .insert(session_key(id), Arc::new(Session::new(provider)));
    }

    async fn get(&self, id: &str) -> Option<Arc<Session<S>>> {
        self.entries.lock().await.get(id).cloned()
    }

    /// Closes every live session. Called when the ACP connection ends: the
    /// daemon is gone, so nothing this adapter launched has a reason to live.
    ///
    /// This is the orderly path, and it is not the one that usually runs: the
    /// crate's stdio connection does not end when the daemon closes its side
    /// (verified against this binary and against `mock-agent`, which behaves
    /// identically), so in practice the daemon kills the adapter — and a killed
    /// process runs no cleanup. `kill_on_drop` does not survive that either: it
    /// fires when the child handle is dropped, which needs Rust code to run.
    /// Ending the provider with certainty when the adapter is killed takes a
    /// job object on Windows and a process group elsewhere; that is where the
    /// stronger form belongs, with `sandbox-propio`.
    async fn close_all(&self, policy: ShutdownPolicy) {
        let live: Vec<Arc<Session<S>>> =
            self.entries.lock().await.drain().map(|(_, s)| s).collect();
        for session in live {
            session.close(policy).await;
        }
    }
}

/// Renders an ACP session id as the plain string it serializes as.
fn session_key(id: &SessionId) -> String {
    id.0.as_ref().to_string()
}

/// Runs one adapter binary: ACP over stdio until the daemon closes it.
///
/// # Errors
///
/// Returns the ACP connection error when the stdio connection fails.
pub async fn run<D: ProviderDialect>(dialect: D) -> agent_client_protocol::Result<()> {
    let dialect = Arc::new(dialect);
    let spec = dialect.spec();
    let sessions: Arc<Sessions<D::Session>> = Arc::new(Sessions::default());
    let shutdown = ShutdownPolicy::default();

    let announced = dialect.capabilities();
    let served = Agent
        .builder()
        .name(spec.name)
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _cx| {
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version)
                        .agent_capabilities(announced.clone()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let sessions = sessions.clone();
                let dialect = dialect.clone();
                async move |request: NewSessionRequest, responder, cx: ConnectionTo<Client>| {
                    let sessions = sessions.clone();
                    let dialect = dialect.clone();
                    let id = sessions.next_id(spec.name);
                    // Launching the CLI and shaking hands with it is work, and
                    // work does not belong on the dispatch loop: a provider
                    // that took its time would freeze every other session of
                    // this adapter while it did.
                    cx.spawn({
                        let cx = cx.clone();
                        async move {
                            match dialect.open(id, request, cx).await {
                                Ok(opened) => {
                                    // Under the id the dialect chose, which is
                                    // the provider's own wherever resuming
                                    // means naming it later.
                                    sessions.register(&opened.id, opened.session).await;
                                    responder.respond(NewSessionResponse::new(opened.id))
                                }
                                Err(refusal) => responder.respond_with_error(refusal.into()),
                            }
                        }
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let sessions = sessions.clone();
                let dialect = dialect.clone();
                async move |request: LoadSessionRequest, responder, cx: ConnectionTo<Client>| {
                    let sessions = sessions.clone();
                    let dialect = dialect.clone();
                    let id = request.session_id.clone();
                    cx.spawn({
                        let cx = cx.clone();
                        async move {
                            match dialect.load(request, cx).await {
                                Ok(session) => {
                                    sessions.register(&id, session).await;
                                    responder.respond(LoadSessionResponse::new())
                                }
                                Err(refusal) => responder.respond_with_error(refusal.into()),
                            }
                        }
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let sessions = sessions.clone();
                async move |prompt: PromptRequest, responder, cx: ConnectionTo<Client>| {
                    let Some(session) = sessions.get(&session_key(&prompt.session_id)).await else {
                        return responder.respond_with_error(unknown_session(&spec).into());
                    };
                    // The turn guard comes before any dialect work: two prompts
                    // racing on one provider would interleave on a wire that
                    // cannot tell them apart.
                    if let Err(error) = session.with_lifecycle(SessionLifecycle::turn_started) {
                        return responder.respond_with_error(turn_refused(&spec, error).into());
                    }
                    let Some(provider) = session.provider().await else {
                        session.with_lifecycle(SessionLifecycle::turn_ended);
                        return responder.respond_with_error(session_closed(&spec).into());
                    };
                    // Here and not inside the turn: the dialect's own turn
                    // control is reset on this loop, beside the lifecycle guard
                    // it belongs to, so that a `session/cancel` delivered on the
                    // same loop can no longer be wiped by the turn it names.
                    provider.begin_turn();
                    // Same reason as the guard above, plus one that is not
                    // optional here: the turn awaits
                    // `session/request_permission`, whose answer arrives on this
                    // very loop.
                    cx.spawn(async move {
                        let outcome = provider.run_turn(prompt).await;
                        let end = session.with_lifecycle(SessionLifecycle::turn_ended);
                        match prompt_answer(&spec, outcome, end) {
                            Ok(stop) => responder.respond(PromptResponse::new(stop)),
                            Err(refusal) => responder.respond_with_error(refusal.into()),
                        }
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let sessions = sessions.clone();
                async move |cancel: CancelNotification, cx: ConnectionTo<Client>| {
                    if let Some(session) = sessions.get(&session_key(&cancel.session_id)).await
                        && session.with_lifecycle(SessionLifecycle::cancel_requested)
                            == CancelAction::InterruptTurn
                        && let Some(provider) = session.provider().await
                    {
                        // Interrupting can wait on the provider, so it does not
                        // happen here: this loop must stay free to deliver the
                        // turn's own answer.
                        cx.spawn(async move {
                            provider.interrupt().await;
                            Ok(())
                        })?;
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await;

    // The daemon is gone: nothing this adapter launched has a reason to live.
    sessions.close_all(shutdown).await;
    served
}

/// What the daemon is answered with, given how the turn came back and what the
/// session's lifecycle says happened to it.
///
/// The lifecycle has the last word on both halves, for the same reason: it is
/// the only piece that knows a `session/cancel` arrived for *this* turn.
///
/// - A turn that drained after a cancellation ended **cancelled**, whatever the
///   provider called it. ACP requires that answer and it is also the true one.
/// - A turn that *failed* after a cancellation ended cancelled too. On a
///   surface whose only documented stop is the end of its input, cancelling is
///   precisely what makes the turn stop working — reporting that as a
///   malfunction would tell the human their stop broke something, when it did
///   exactly what they asked. The provider's own words are not lost: they go to
///   this adapter's stderr, which the daemon already collects into the session
///   log.
fn prompt_answer(
    spec: &AdapterSpec,
    outcome: Result<StopReason, Refusal>,
    end: TurnEnd,
) -> Result<StopReason, Refusal> {
    match (outcome, end) {
        (Ok(stop), TurnEnd::Completed) => Ok(stop),
        (Ok(_), TurnEnd::Cancelled) => Ok(StopReason::Cancelled),
        (Err(refusal), TurnEnd::Cancelled) => {
            eprintln!(
                "{}: the cancelled turn ended with `{}`: {}",
                spec.name, refusal.kind, refusal.detail
            );
            Ok(StopReason::Cancelled)
        }
        (Err(refusal), TurnEnd::Completed) => Err(refusal),
    }
}

/// The refusal for a prompt the session's lifecycle will not accept.
fn turn_refused(spec: &AdapterSpec, error: LifecycleError) -> Refusal {
    Refusal::new(
        "turn_refused",
        spec.name,
        error.to_string(),
        "Wait for the turn in flight to end, or open a new session.",
    )
}

/// A prompt for a session this adapter never opened (or already closed).
fn unknown_session(spec: &AdapterSpec) -> Refusal {
    Refusal::new(
        "unknown_session",
        spec.name,
        "the prompt names a session this adapter did not open".to_string(),
        "Open the session with `session/new` before prompting.".to_string(),
    )
}

/// A prompt for a session whose provider has already been shut down.
fn session_closed(spec: &AdapterSpec) -> Refusal {
    Refusal::new(
        "session_closed",
        spec.name,
        "this session's provider process has already been shut down".to_string(),
        "Open a new session.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A provider session that pilots nothing and writes down what it was
    /// asked, in the order it was asked.
    ///
    /// What the registry's rules are about is bookkeeping — one turn at a time,
    /// a cancellation reaching the turn it names, a closed session ending its
    /// provider — and a real dialect underneath would only add a wire to keep
    /// quiet about. The log exists for the one rule that is about *order*
    /// rather than about state.
    #[derive(Default)]
    struct StubSession {
        calls: StdMutex<Vec<&'static str>>,
    }

    impl StubSession {
        fn note(&self, what: &'static str) {
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(what);
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    impl ProviderSession for StubSession {
        fn begin_turn(&self) {
            self.note("begin_turn");
        }

        async fn run_turn(&self, _prompt: PromptRequest) -> Result<StopReason, Refusal> {
            self.note("run_turn");
            Ok(StopReason::EndTurn)
        }

        async fn interrupt(&self) {
            self.note("interrupt");
        }

        async fn shutdown(&self, _policy: ShutdownPolicy) {
            self.note("shutdown");
        }
    }

    fn pending() -> StubSession {
        StubSession::default()
    }

    #[tokio::test]
    async fn a_session_is_addressable_by_the_id_it_was_opened_with() {
        // The four entry points share one registry: `session/cancel` must reach
        // the very lifecycle `session/new` created, or a cancellation lands
        // nowhere and the turn runs on. And it must reach only that one.
        let sessions = Sessions::default();
        let first = sessions.next_id("adapter");
        let second = sessions.next_id("adapter");
        assert_ne!(first, second, "each session gets its own id");
        sessions.register(&first, pending()).await;
        sessions.register(&second, pending()).await;

        let session = sessions
            .get(&session_key(&first))
            .await
            .expect("the session is live");
        session
            .with_lifecycle(SessionLifecycle::turn_started)
            .unwrap();
        assert_eq!(
            sessions
                .get(&session_key(&first))
                .await
                .expect("still live")
                .with_lifecycle(SessionLifecycle::cancel_requested),
            CancelAction::InterruptTurn,
            "the cancellation reaches the turn in flight, not a copy of it"
        );
        assert_eq!(
            sessions
                .get(&session_key(&second))
                .await
                .expect("still live")
                .with_lifecycle(SessionLifecycle::cancel_requested),
            CancelAction::Nothing,
            "cancelling one session leaves the others running"
        );
        assert!(sessions.get("never-opened").await.is_none());
    }

    /// A spec to put in a refusal; which dialect it names decides nothing here.
    const TEST_SPEC: AdapterSpec = AdapterSpec {
        name: "meltemi-test-acp",
        provider_layer: "the official test CLI",
        provider_bin: "test-cli",
        dialect: Dialect::HeadlessSession,
    };

    fn a_prompt(id: &SessionId) -> PromptRequest {
        PromptRequest::new(id.clone(), Vec::new())
    }

    #[tokio::test]
    async fn a_cancellation_between_the_prompt_and_its_turn_is_not_erased_by_it() {
        // Scenario: Cancelación temprana no perdida entre la aceptación y el turno
        //
        // The ordering the ACP dispatch loop guarantees, replayed here because
        // the window it closes is microseconds wide and no end-to-end test can
        // be aimed at it. The loop is serial: it accepts `session/prompt`,
        // takes the turn guard, resets the dialect, and only *then* spawns the
        // turn — so a `session/cancel` delivered next is handled whole before
        // the turn's first instruction runs.
        //
        // Reset the dialect from inside the spawned turn instead, which is
        // where it used to happen, and the same three messages produce a
        // different sequence: the cancellation is recorded and then wiped by
        // the very turn it was meant for. Nothing reaches the provider, the
        // whole turn runs — commands and file changes included — and the
        // lifecycle still reports it cancelled. The work happens and the human
        // is told it did not.
        let sessions = Sessions::default();
        let id = sessions.next_id("adapter");
        sessions.register(&id, pending()).await;
        let session = sessions.get(&session_key(&id)).await.expect("live");
        let provider = session.provider().await.expect("the session has one");

        // `session/prompt` arrives: guard, then reset, on the loop.
        session
            .with_lifecycle(SessionLifecycle::turn_started)
            .unwrap();
        provider.begin_turn();

        // `session/cancel` arrives on that same loop, before the spawned turn
        // has had a chance to run.
        assert_eq!(
            session.with_lifecycle(SessionLifecycle::cancel_requested),
            CancelAction::InterruptTurn
        );
        provider.interrupt().await;

        // And only now the turn itself.
        let outcome = provider.run_turn(a_prompt(&id)).await;
        let end = session.with_lifecycle(SessionLifecycle::turn_ended);

        assert_eq!(
            provider.calls(),
            vec!["begin_turn", "interrupt", "run_turn"],
            "the dialect is reset before a cancellation can reach it, never after"
        );
        assert_eq!(end, TurnEnd::Cancelled);
        assert_eq!(
            prompt_answer(&TEST_SPEC, outcome, end).expect("a cancelled turn is not an error"),
            StopReason::Cancelled
        );
    }

    #[test]
    fn a_turn_a_cancellation_broke_is_answered_cancelled_and_not_as_a_malfunction() {
        // Scenario: Turno cancelado reportado como cancelado
        //
        // On a surface whose only documented stop is the end of its input,
        // cancelling is exactly what makes the turn stop working. Answering the
        // daemon with that failure would tell the human their stop broke
        // something, when it did the one thing they asked for — and ACP wants
        // `cancelled` for a cancelled prompt in any case.
        let broke = Refusal::new(
            "session_finished",
            TEST_SPEC.provider_layer,
            "the input was closed by the cancellation".to_string(),
            "Open a new session.".to_string(),
        );
        assert_eq!(
            prompt_answer(&TEST_SPEC, Err(broke), TurnEnd::Cancelled)
                .expect("a cancelled turn is answered, not refused"),
            StopReason::Cancelled
        );

        // And a turn nobody cancelled still reports its failure as a failure: a
        // rule that swallowed those would hide every real one.
        let failed = Refusal::new(
            "provider_turn_failed",
            TEST_SPEC.provider_layer,
            "it broke".to_string(),
            "Try again.".to_string(),
        );
        assert_eq!(
            prompt_answer(&TEST_SPEC, Err(failed), TurnEnd::Completed)
                .expect_err("a failure nobody asked for is still a failure")
                .kind,
            "provider_turn_failed"
        );
        assert_eq!(
            prompt_answer(&TEST_SPEC, Ok(StopReason::EndTurn), TurnEnd::Completed).unwrap(),
            StopReason::EndTurn
        );
    }

    #[tokio::test]
    async fn closing_the_connection_closes_every_session_it_opened() {
        // When the daemon goes, the adapter's sessions go with it: a closed
        // session takes no further turns, and its provider is shut down on this
        // very path.
        let sessions = Sessions::default();
        let id = sessions.next_id("adapter");
        sessions.register(&id, pending()).await;
        let session = sessions
            .get(&session_key(&id))
            .await
            .expect("the session is live");

        sessions.close_all(ShutdownPolicy::default()).await;
        assert!(
            sessions.get(&session_key(&id)).await.is_none(),
            "the registry keeps no closed sessions"
        );
        assert_eq!(
            session.with_lifecycle(SessionLifecycle::turn_started),
            Err(LifecycleError::SessionClosed)
        );
        assert!(
            session.provider().await.is_none(),
            "and the provider went with it"
        );
    }
}
