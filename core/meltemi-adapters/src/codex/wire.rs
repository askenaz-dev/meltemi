// SPDX-License-Identifier: Apache-2.0

//! The provider's server wire, in the shape this adapter speaks it
//! (adaptadores-propios-acp task 2.1).
//!
//! The authority for every name and every field below is the schema the
//! official CLI dumps for its own version, vendored verbatim under
//! `core/mock-provider/schemas/codex-app-server/` and validated against these
//! types by `tests/codex_conformance.rs`. Nothing here was written from memory
//! or from a blog post; where a fixture and the dump disagree, the dump wins.
//!
//! Three decisions worth naming, because they are the ones that would rot
//! quietly:
//!
//! - **The surface is the thread/turn/item generation.** The same server also
//!   answers an older conversation-shaped generation; this adapter speaks one
//!   surface, the one whose primitives design D6 maps onto an ACP session, and
//!   another surface would be another change.
//! - **Incoming types are permissive, outgoing types are exact.** No incoming
//!   struct denies unknown fields: a provider that adds one must not break a
//!   session. What the adapter *sends* is validated field by field against the
//!   dump, because that is the direction where being wrong means being refused.
//! - **The supported range is declared, not assumed.** [`SUPPORTED_FLOOR`] and
//!   [`SUPPORTED_CEILING`] are what the handshake refuses outside of. They
//!   move when someone re-anchors the vendored dump, and never before.

use serde::{Deserialize, Serialize};

/// The CLI version whose dumped schema is vendored, and against which every
/// type in this module was verified.
pub const VENDORED_SCHEMA_VERSION: &str = "0.77.0";

/// Lowest CLI version this adapter accepts, as `(major, minor, patch)`.
///
/// It is the version the vendored dump came from: below it, nobody has checked
/// that the thread/turn/item surface is the one described there, and the
/// adapter refuses rather than assume (design D6).
pub const SUPPORTED_FLOOR: (u64, u64, u64) = (0, 77, 0);

/// First CLI version this adapter refuses, as `(major, minor, patch)`.
///
/// A major bump is where a wire is allowed to change out from under a client,
/// so that is where the refusal sits. Within the current major the surface has
/// grown additively, and the incoming types above are permissive on purpose so
/// that growth costs nothing — but the claim is only ever as fresh as the last
/// re-anchoring of the vendored dump, which is what task 5.2's manual run
/// against a real CLI exists to check.
pub const SUPPORTED_CEILING: (u64, u64, u64) = (1, 0, 0);

/// The token that prefixes the CLI's own version inside the user agent string
/// the handshake returns (`codex_cli_rs/0.77.0 (…)`).
pub const USER_AGENT_PRODUCT: &str = "codex_cli_rs";

/// The argument that puts the official CLI in its server mode.
pub const SERVER_MODE_ARG: &str = "app-server";

// --- Methods the adapter calls ------------------------------------------

/// Handshake; answers with the server's user agent.
pub const INITIALIZE: &str = "initialize";
/// Sent once after the handshake, as the wire expects.
pub const INITIALIZED: &str = "initialized";
/// Opens the thread an ACP session maps onto.
pub const THREAD_START: &str = "thread/start";
/// Starts a turn on a thread.
pub const TURN_START: &str = "turn/start";
/// Interrupts the turn in flight.
pub const TURN_INTERRUPT: &str = "turn/interrupt";

// --- Notifications the server sends -------------------------------------

/// A turn began.
pub const TURN_STARTED: &str = "turn/started";
/// A turn ended; carries the status that becomes the ACP stop reason.
pub const TURN_COMPLETED: &str = "turn/completed";
/// An item of the turn appeared.
pub const ITEM_STARTED: &str = "item/started";
/// An item of the turn reached its final form.
pub const ITEM_COMPLETED: &str = "item/completed";
/// A chunk of the agent's message.
pub const AGENT_MESSAGE_DELTA: &str = "item/agentMessage/delta";
/// A chunk of the agent's reasoning.
pub const REASONING_TEXT_DELTA: &str = "item/reasoning/textDelta";
/// A chunk of the agent's reasoning summary.
pub const REASONING_SUMMARY_TEXT_DELTA: &str = "item/reasoning/summaryTextDelta";
/// The turn hit an error; `willRetry` says whether the server carries on.
pub const ERROR: &str = "error";

// --- Requests the server makes of the adapter ---------------------------

/// The server asks to run a command.
pub const COMMAND_EXECUTION_APPROVAL: &str = "item/commandExecution/requestApproval";
/// The server asks to change files.
pub const FILE_CHANGE_APPROVAL: &str = "item/fileChange/requestApproval";

// --- Handshake -----------------------------------------------------------

/// Who is calling. The provider's CLI echoes it back inside its user agent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// The adapter's name.
    pub name: String,
    /// The adapter's version.
    pub version: String,
}

/// `initialize` parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Who is calling.
    pub client_info: ClientInfo,
}

/// `initialize` result.
///
/// The user agent is the only version the handshake offers, and it is the one
/// the skew check reads.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The server's user agent, e.g. `codex_cli_rs/0.77.0 (…)`.
    pub user_agent: String,
}

// --- Threads and turns ---------------------------------------------------

/// `thread/start` parameters.
///
/// Only the working directory is sent. Model, sandbox and approval policy are
/// deliberately left to the CLI's own configuration: choosing them here would
/// make Meltemi assume a provider's semantics, which §5 forbids the core to do
/// and which the adapter has no better information to do either.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
    /// The session's working directory: the project root or one of its
    /// worktrees.
    pub cwd: String,
}

/// A thread, of which the adapter reads only its identity.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// The thread id every later call carries.
    pub id: String,
}

/// `thread/start` result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResult {
    /// The thread that was opened.
    pub thread: Thread,
}

/// One piece of the user's message.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UserInput {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
}

/// `turn/start` parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
    /// The thread this turn runs on.
    pub thread_id: String,
    /// The user's message.
    pub input: Vec<UserInput>,
}

/// How a turn ended, in the server's own terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnStatus {
    /// The turn ran to its end.
    Completed,
    /// The turn was interrupted.
    Interrupted,
    /// The turn failed.
    Failed,
    /// The turn is still running.
    InProgress,
}

/// A turn, of which the adapter reads its identity and how it ended.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    /// The turn id, which an interruption must name.
    pub id: String,
    /// How the turn ended.
    pub status: TurnStatus,
}

/// `turn/start` result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResult {
    /// The turn that was started.
    pub turn: Turn,
}

/// `turn/interrupt` parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptParams {
    /// The thread the turn runs on.
    pub thread_id: String,
    /// The turn to interrupt.
    pub turn_id: String,
}

/// `turn/started` and `turn/completed` notifications.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnNotification {
    /// The thread the turn belongs to.
    pub thread_id: String,
    /// The turn.
    pub turn: Turn,
}

/// `item/started` and `item/completed` notifications.
///
/// The item stays a raw value: it is parsed into [`ThreadItem`] for the kinds
/// the mapping knows, and an item of a kind it does not know still has an id
/// and a type to show. An unrecognised item that vanished would erase evidence
/// exactly when a session is going wrong.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemNotification {
    /// The thread the item belongs to.
    pub thread_id: String,
    /// The turn the item belongs to.
    pub turn_id: String,
    /// The item, verbatim.
    pub item: serde_json::Value,
}

/// A streamed chunk of an item's text.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaNotification {
    /// The thread the item belongs to.
    pub thread_id: String,
    /// The turn the item belongs to.
    pub turn_id: String,
    /// The item being streamed.
    pub item_id: String,
    /// The chunk.
    pub delta: String,
}

/// What went wrong with a turn.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnError {
    /// The message, as the provider words it.
    pub message: String,
}

/// The `error` notification.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorNotification {
    /// The turn that failed.
    pub turn_id: String,
    /// What went wrong.
    pub error: TurnError,
    /// Whether the server will try again on its own — in which case the turn
    /// is not over and the adapter must not close it.
    pub will_retry: bool,
}

// --- Items ---------------------------------------------------------------

/// How a command execution or a file change is going.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// Still running.
    InProgress,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Refused — by the permission proxy, through this very adapter.
    Declined,
}

/// One file the agent proposes to change.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdateChange {
    /// The file's path.
    pub path: String,
    /// The change, as a unified diff.
    pub diff: String,
}

/// The items a turn is made of, in the kinds this adapter maps.
///
/// Deliberately not exhaustive of the provider's list: an unknown `type`
/// deserializes as [`ThreadItem::Unrecognised`], and the mapping shows it by
/// its raw type rather than dropping it.
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ThreadItem {
    /// Something the agent said.
    AgentMessage {
        /// The item id, shared with its streamed chunks.
        id: String,
        /// The whole text of the message.
        text: String,
    },
    /// Something the agent thought.
    Reasoning {
        /// The item id.
        id: String,
    },
    /// A command the agent ran.
    CommandExecution {
        /// The item id.
        id: String,
        /// The command line.
        command: String,
        /// How it is going.
        status: ItemStatus,
        /// Everything it printed, once it is over.
        #[serde(default)]
        aggregated_output: Option<String>,
        /// Its exit code, once it is over.
        #[serde(default)]
        exit_code: Option<i64>,
    },
    /// Files the agent changed.
    FileChange {
        /// The item id.
        id: String,
        /// How it is going.
        status: ItemStatus,
        /// The files and their diffs.
        changes: Vec<FileUpdateChange>,
    },
    /// A tool call on an MCP server.
    McpToolCall {
        /// The item id.
        id: String,
        /// The server the tool lives on.
        server: String,
        /// The tool.
        tool: String,
        /// How it is going.
        status: ItemStatus,
    },
    /// A web search the agent ran.
    WebSearch {
        /// The item id.
        id: String,
        /// What it searched for.
        query: String,
    },
    /// An item of a kind this adapter does not map yet.
    #[serde(other)]
    Unrecognised,
}

// --- Approvals -----------------------------------------------------------

/// What the server asks approval for, in the shape both approval requests
/// share: the item it is about, and an optional reason.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalParams {
    /// The thread the item belongs to.
    pub thread_id: String,
    /// The turn the item belongs to.
    pub turn_id: String,
    /// The item awaiting a decision.
    pub item_id: String,
    /// Why the server is asking, when it says.
    #[serde(default)]
    pub reason: Option<String>,
}

/// The decision the adapter hands back — always the permission proxy's, never
/// its own.
///
/// The provider also accepts an "accept with an execpolicy amendment" object;
/// it is not offered, because the adapter has no policy of its own to amend
/// and inventing one would be deciding on the human's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalDecision {
    /// Go ahead, this once.
    Accept,
    /// Go ahead, and stop asking for the rest of the session.
    AcceptForSession,
    /// No.
    Decline,
    /// The turn is being cancelled; do not proceed and do not ask again.
    Cancel,
}

/// The answer to either approval request.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResponse {
    /// The decision.
    pub decision: ApprovalDecision,
}

impl ApprovalResponse {
    /// The answer carrying this decision.
    #[must_use]
    pub fn new(decision: ApprovalDecision) -> Self {
        Self { decision }
    }
}

impl ThreadItem {
    /// The item's id, whatever its kind — `None` only for a kind this adapter
    /// does not know, whose id the caller reads from the raw value.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            ThreadItem::AgentMessage { id, .. }
            | ThreadItem::Reasoning { id }
            | ThreadItem::CommandExecution { id, .. }
            | ThreadItem::FileChange { id, .. }
            | ThreadItem::McpToolCall { id, .. }
            | ThreadItem::WebSearch { id, .. } => Some(id),
            ThreadItem::Unrecognised => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_item_of_an_unknown_kind_is_kept_rather_than_refused() {
        // A provider that adds an item kind must not break a session, and the
        // item must not vanish either: it parses, and its raw form still
        // carries the id and the type the mapping shows.
        let raw = json!({"type": "somethingNew", "id": "item-9", "extra": 1});
        let item: ThreadItem = serde_json::from_value(raw.clone()).expect("unknown kinds parse");
        assert!(matches!(item, ThreadItem::Unrecognised));
        assert_eq!(item.id(), None);
        assert_eq!(raw["id"], "item-9");
    }

    #[test]
    fn a_known_item_keeps_the_fields_the_mapping_needs() {
        let item: ThreadItem = serde_json::from_value(json!({
            "type": "commandExecution",
            "id": "item-1",
            "command": "cargo test",
            "cwd": "/project",
            "commandActions": [],
            "status": "completed",
            "aggregatedOutput": "ok\n",
            "exitCode": 0,
            "durationMs": 12
        }))
        .expect("a command execution parses");
        let ThreadItem::CommandExecution {
            command,
            status,
            exit_code,
            ..
        } = item
        else {
            panic!("the item kept its kind");
        };
        assert_eq!(command, "cargo test");
        assert_eq!(status, ItemStatus::Completed);
        assert_eq!(exit_code, Some(0));
    }
}
