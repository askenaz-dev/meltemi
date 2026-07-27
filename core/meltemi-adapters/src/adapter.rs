// SPDX-License-Identifier: Apache-2.0

//! The ACP surface both adapters share (adaptadores-propios-acp task 1.1).
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
//! provider wire: translating a turn into the provider's dialect is the work of
//! the JSON-RPC server dialect (tasks 2.2-2.4) and of the headless session
//! dialect (tasks 3.1-3.5). Until a dialect is wired, a prompt **refuses**: an
//! adapter that answered `end_turn` without piloting anything would be lying
//! about work it never did.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, InitializeRequest, InitializeResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, SessionId,
};
use agent_client_protocol::{Agent, Stdio};
use tokio::sync::Mutex;

use crate::diagnostic::Refusal;

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

impl Dialect {
    /// The tasks that wire this dialect's turn mapping, named in the refusal a
    /// skeleton prompt returns — so nobody has to guess whether the silence is
    /// a bug or unbuilt scaffolding.
    fn pending_tasks(self) -> &'static str {
        match self {
            Dialect::HeadlessSession => "adaptadores-propios-acp 3.1-3.5",
            Dialect::JsonRpcServer => "adaptadores-propios-acp 2.2-2.4",
        }
    }
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

/// A session the daemon opened on this adapter.
struct Session {
    /// Raised by `session/cancel`. The turn in flight reads it to stop, and a
    /// turn that starts after it reads it too: a cancellation that arrives
    /// between turns must not be forgotten.
    cancelled: AtomicBool,
}

/// The adapter's live sessions.
#[derive(Default)]
struct Sessions {
    entries: Mutex<HashMap<String, Arc<Session>>>,
    counter: AtomicU64,
}

impl Sessions {
    /// Registers a new session and returns its id.
    async fn open(&self, prefix: &str) -> String {
        let id = format!(
            "{prefix}-{}",
            self.counter.fetch_add(1, Ordering::Relaxed) + 1
        );
        self.entries.lock().await.insert(
            id.clone(),
            Arc::new(Session {
                cancelled: AtomicBool::new(false),
            }),
        );
        id
    }

    async fn get(&self, id: &str) -> Option<Arc<Session>> {
        self.entries.lock().await.get(id).cloned()
    }
}

/// The capabilities this adapter announces.
///
/// Announced honestly and no earlier than true: session load stays off until
/// resume is mapped (task 3.5), and MCP passthrough until the projection is
/// handed to the CLI (task 3.5). An adapter that announced them now would make
/// the daemon offer a resume that cannot happen.
fn capabilities() -> AgentCapabilities {
    AgentCapabilities::new()
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
pub async fn run(spec: AdapterSpec) -> agent_client_protocol::Result<()> {
    let sessions = Arc::new(Sessions::default());

    Agent
        .builder()
        .name(spec.name)
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _cx| {
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version)
                        .agent_capabilities(capabilities()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let sessions = sessions.clone();
                async move |_new_session: NewSessionRequest, responder, _cx| {
                    let id = sessions.open(spec.name).await;
                    responder.respond(NewSessionResponse::new(SessionId::new(id)))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let sessions = sessions.clone();
                async move |prompt: PromptRequest, responder, _cx| match sessions
                    .get(&session_key(&prompt.session_id))
                    .await
                {
                    Some(_session) => responder.respond_with_error(dialect_not_wired(&spec).into()),
                    None => responder.respond_with_error(unknown_session(&spec).into()),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let sessions = sessions.clone();
                async move |cancel: CancelNotification, _cx| {
                    if let Some(session) = sessions.get(&session_key(&cancel.session_id)).await {
                        session.cancelled.store(true, Ordering::SeqCst);
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}

/// The refusal a skeleton prompt returns: honest scaffolding, never a fake
/// turn. It disappears when the dialect it names lands.
fn dialect_not_wired(spec: &AdapterSpec) -> Refusal {
    Refusal::new(
        "dialect_not_wired",
        spec.name,
        format!(
            "this adapter does not yet translate turns to the {} of `{}`",
            match spec.dialect {
                Dialect::HeadlessSession => "headless session dialect",
                Dialect::JsonRpcServer => "JSON-RPC server dialect",
            },
            spec.provider_bin
        ),
        format!(
            "The turn mapping lands in {}; until then pilot the agent with another entry of the fleet.",
            spec.dialect.pending_tasks()
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> AdapterSpec {
        AdapterSpec {
            name: "meltemi-test-acp",
            provider_layer: "the official test CLI",
            provider_bin: "test-cli",
            dialect: Dialect::HeadlessSession,
        }
    }

    #[tokio::test]
    async fn a_session_is_addressable_by_the_id_it_was_opened_with() {
        // The four entry points share one registry: `session/cancel` must find
        // the very session `session/new` created, or a cancellation lands
        // nowhere and the turn runs on.
        let sessions = Sessions::default();
        let first = sessions.open("adapter").await;
        let second = sessions.open("adapter").await;
        assert_ne!(first, second, "each session gets its own id");

        let session = sessions.get(&first).await.expect("the session is live");
        assert!(!session.cancelled.load(Ordering::SeqCst));
        session.cancelled.store(true, Ordering::SeqCst);
        assert!(
            sessions
                .get(&first)
                .await
                .expect("still live")
                .cancelled
                .load(Ordering::SeqCst),
            "the cancellation is visible to the turn, not to a copy of it"
        );
        assert!(
            !sessions
                .get(&second)
                .await
                .expect("still live")
                .cancelled
                .load(Ordering::SeqCst),
            "cancelling one session leaves the others running"
        );
        assert!(sessions.get("never-opened").await.is_none());
    }

    #[test]
    fn a_skeleton_prompt_refuses_naming_the_task_that_wires_it() {
        // The refusal is scaffolding, and says so: an adapter that answered a
        // prompt with `end_turn` would claim work it never did.
        let refusal = dialect_not_wired(&spec());
        assert_eq!(refusal.kind, "dialect_not_wired");
        assert!(refusal.detail.contains("test-cli"));
        assert!(refusal.remedy.contains("3.1-3.5"));
    }
}
