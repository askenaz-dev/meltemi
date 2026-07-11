// SPDX-License-Identifier: Apache-2.0

//! Serde types for the Meltemi daemon<->client protocol.
//!
//! The JSON Schemas under `proto/schemas/` are the language-neutral source
//! of truth for this contract; the types in this crate must serialize in
//! conformance with them (validated by `tests/conformance.rs`). On any
//! discrepancy, the schema wins.

use serde::{Deserialize, Serialize};

/// Current daemon<->client contract version (see `proto/README.md`).
pub const PROTOCOL_VERSION: u32 = 1;

/// Current session event envelope version (independent from
/// [`PROTOCOL_VERSION`]; see `session-event.schema.json`).
pub const SESSION_EVENT_VERSION: u32 = 1;

/// JSON-RPC method and notification names of the v1 contract.
pub mod methods {
    /// Request: version negotiation; first message on every connection.
    pub const INITIALIZE: &str = "initialize";
    /// Request: daemon version, uptime and active sessions.
    pub const STATUS: &str = "status";
    /// Request: orderly daemon termination.
    pub const SHUTDOWN: &str = "shutdown";
    /// Request: scaffold a change proposal and delegate it to the agent.
    pub const PROPOSE: &str = "propose";
    /// Notification (client -> daemon): cancel an active session.
    pub const SESSION_CANCEL: &str = "session/cancel";
    /// Notification (daemon -> client): streamed session event.
    pub const SESSION_EVENT: &str = "session/event";
    /// Request (daemon -> client): permission passthrough from the agent.
    pub const PERMISSION_REQUEST: &str = "permission/request";
    /// Notification (daemon -> client): a permission request timed out.
    pub const PERMISSION_TIMEOUT: &str = "permission/timeout";
}

/// Application error codes, outside the JSON-RPC reserved range and grouped
/// by domain: 1xxx daemon, 2xxx ACP session, 3xxx propose. Catalog:
/// `error.schema.json`.
pub mod error_codes {
    /// The client declared an unsupported contract version.
    pub const PROTOCOL_VERSION_UNSUPPORTED: i64 = 1000;
    /// A method was called before a successful `initialize`.
    pub const NOT_INITIALIZED: i64 = 1001;
    /// The daemon is shutting down and no longer accepts work.
    pub const SHUTTING_DOWN: i64 = 1002;
    /// No `agent.command` is configured.
    pub const AGENT_COMMAND_NOT_CONFIGURED: i64 = 2000;
    /// The configured agent command does not exist on this system.
    pub const AGENT_COMMAND_NOT_FOUND: i64 = 2001;
    /// The agent subprocess could not be started.
    pub const AGENT_SPAWN_FAILED: i64 = 2002;
    /// The ACP initialize negotiation with the agent failed.
    pub const AGENT_HANDSHAKE_FAILED: i64 = 2003;
    /// The given session id does not correspond to an active session.
    pub const SESSION_NOT_FOUND: i64 = 2004;
    /// The derived change name already exists under `.meltemi/changes/`.
    pub const CHANGE_ALREADY_EXISTS: i64 = 3000;
    /// No change name can be derived from the given idea.
    pub const INVALID_IDEA: i64 = 3001;
    /// The project root does not exist or is not a directory.
    pub const PROJECT_ROOT_INVALID: i64 = 3002;
}

/// Structured `error.data` payload: `{ kind, detail, remedy }` (D11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorData {
    /// Machine-readable error kind, stable across daemon versions.
    pub kind: String,
    /// Human-readable English detail of what went wrong.
    pub detail: String,
    /// Actionable English suggestion for how to fix it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

/// Identification of one side of the connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    /// Peer name (e.g. `meltemid`, `meltemi-devclient`).
    pub name: String,
    /// Peer version string.
    pub version: String,
}

/// Params of `initialize`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// The contract version the client speaks.
    pub protocol_version: u32,
    /// Client identification.
    pub client: PeerInfo,
}

/// Result of `initialize`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The negotiated contract version.
    pub protocol_version: u32,
    /// Daemon identification.
    pub daemon: PeerInfo,
}

/// Lifecycle state of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// The agent subprocess is being launched / handshaked.
    Starting,
    /// The session is running a turn.
    Active,
    /// The session is blocked on a permission decision.
    WaitingPermission,
    /// The session has finished.
    Ended,
}

/// One active session, as reported by `status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// Meltemi session identifier.
    pub session_id: String,
    /// The official agent binary and arguments (program first).
    pub agent_command: Vec<String>,
    /// Current lifecycle state.
    pub state: SessionState,
}

/// Result of `status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResult {
    /// Daemon version string.
    pub daemon_version: String,
    /// Seconds since the daemon started.
    pub uptime_seconds: u64,
    /// Active sessions.
    pub sessions: Vec<SessionSummary>,
}

/// Final status of an agent turn, mapped from the ACP stop reason
/// (`end_turn` -> `completed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    /// The turn ended successfully.
    Completed,
    /// The turn was cancelled by the client.
    Cancelled,
    /// The agent refused to continue.
    Refused,
    /// The agent hit its token limit.
    MaxTokens,
    /// The agent hit its per-turn request limit.
    MaxTurnRequests,
}

/// Params of `propose`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeParams {
    /// Free-form description of the change to propose.
    pub idea: String,
    /// Absolute path to the root of the target repository.
    pub project_root: String,
}

/// Result of `propose`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeResult {
    /// Kebab-case change name derived from the idea.
    pub change_name: String,
    /// Absolute path of the created `proposal.md`.
    pub proposal_path: String,
    /// Final status of the agent turn.
    pub status: TurnStatus,
}

/// Params of the `session/cancel` notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCancelParams {
    /// The session to cancel.
    pub session_id: String,
}

/// Hint about the nature of a permission option (mirrors ACP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    /// Allow this operation only this time.
    AllowOnce,
    /// Allow this operation and remember the choice.
    AllowAlways,
    /// Reject this operation only this time.
    RejectOnce,
    /// Reject this operation and remember the choice.
    RejectAlways,
}

/// An option the client presents to the user for a permission request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    /// Unique identifier for this option.
    pub option_id: String,
    /// Human-readable label to display to the user.
    pub name: String,
    /// Hint about the nature of this option.
    pub kind: PermissionOptionKind,
}

/// Params of `permission/request` (daemon -> client).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestParams {
    /// The Meltemi session this request belongs to.
    pub session_id: String,
    /// ACP `ToolCallUpdate` forwarded verbatim: the tool or operation, the
    /// affected command or path, and the external-effect classification when
    /// the agent provides it.
    pub tool_call: serde_json::Value,
    /// Options for the user to choose from.
    pub options: Vec<PermissionOption>,
}

/// The decision on a permission request, mirroring ACP
/// `RequestPermissionOutcome`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PermissionOutcome {
    /// The turn was cancelled before the user responded.
    Cancelled,
    /// The user selected one of the provided options.
    #[serde(rename_all = "camelCase")]
    Selected {
        /// The id of the selected option.
        option_id: String,
    },
}

/// Result of `permission/request`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestResult {
    /// The client's decision.
    pub outcome: PermissionOutcome,
}

/// Params of the `permission/timeout` notification (daemon -> client).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionTimeoutParams {
    /// The session whose permission request expired.
    pub session_id: String,
    /// The tool call the expired request referred to, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Who resolved a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecidedBy {
    /// The connected client answered.
    Client,
    /// No client was connected to the session; denied by default.
    DefaultDeny,
    /// The client did not answer within the configured timeout.
    Timeout,
}

/// Versioned session event envelope (D12). Appended as one JSON line per
/// event to the session's JSONL log, and streamed to the client via the
/// `session/event` notification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Envelope version ([`SESSION_EVENT_VERSION`]).
    pub v: u32,
    /// Event timestamp, RFC 3339, UTC.
    pub ts: String,
    /// Event type and payload.
    #[serde(flatten)]
    pub kind: SessionEventKind,
}

/// Session event types and their payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SessionEventKind {
    /// The agent subprocess was launched and the session created.
    SessionStarted {
        /// Meltemi session identifier.
        session_id: String,
        /// The agent binary and arguments (program first).
        agent_command: Vec<String>,
        /// The repository root the session works in.
        project_root: String,
    },
    /// A prompt was sent to the agent.
    PromptSent {
        /// The prompt text.
        text: String,
    },
    /// The agent emitted a session update (ACP `SessionUpdate`, verbatim).
    AgentUpdate {
        /// The forwarded update.
        update: serde_json::Value,
    },
    /// The agent requested a permission.
    PermissionRequested {
        /// The forwarded request (see `permission.schema.json`).
        request: serde_json::Value,
    },
    /// A permission request was resolved.
    PermissionDecided {
        /// The decision returned to the agent.
        outcome: serde_json::Value,
        /// Who resolved it.
        decided_by: PermissionDecidedBy,
    },
    /// The agent turn finished.
    TurnCompleted {
        /// Mapped ACP stop reason.
        stop_reason: TurnStatus,
    },
    /// The client cancelled the session.
    SessionCancelled {},
    /// The session ended and its log is complete.
    SessionEnded {
        /// Why the session ended.
        reason: String,
    },
    /// An error occurred within the session.
    Error {
        /// Machine-readable error kind.
        kind: String,
        /// Human-readable English detail.
        detail: String,
    },
}

/// Params of the `session/event` notification (daemon -> client).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventParams {
    /// The session the event belongs to.
    pub session_id: String,
    /// The event.
    pub event: SessionEvent,
}
