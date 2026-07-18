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
    /// Request: the fleet catalog (known agents crossed with local detection).
    pub const FLEET_LIST: &str = "fleet/list";
    /// Request: (re)project the compiled context into the declared targets.
    pub const CONTEXT_PROJECT: &str = "context/project";
    /// Request: list sessions (active and historical) for a project.
    pub const SESSION_LIST: &str = "session/list";
    /// Request: read a session's JSONL log, paginated by line range.
    pub const SESSION_LOG: &str = "session/log";
    /// Request: the repository tree honoring gitignore, with sizes.
    pub const REPO_MAP: &str = "repo/map";
    /// Notification (client -> daemon): cancel an active session.
    pub const SESSION_CANCEL: &str = "session/cancel";
    /// Notification (daemon -> client): streamed session event.
    pub const SESSION_EVENT: &str = "session/event";
    /// Request (daemon -> client): permission passthrough from the agent.
    pub const PERMISSION_REQUEST: &str = "permission/request";
    /// Notification (daemon -> client): a permission request timed out.
    pub const PERMISSION_TIMEOUT: &str = "permission/timeout";
    /// Request (client -> daemon): enumerate the pending permission queue.
    pub const PERMISSION_PENDING: &str = "permission/pending";
    /// Request (client -> daemon): resolve a pending permission by id.
    pub const PERMISSION_DECIDE: &str = "permission/decide";
    /// Notification (daemon -> client): the pending permission queue changed.
    pub const PERMISSION_CHANGED: &str = "permission/changed";
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
    /// No `agent.command` (nor `agent.id`) is configured.
    pub const AGENT_COMMAND_NOT_CONFIGURED: i64 = 2000;
    /// The selected agent is not present on this system: the configured
    /// catalog id is unknown, or its binary was not detected.
    pub const AGENT_NOT_DETECTED: i64 = 2001;
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
    /// The session had no recorded end (the daemon stopped mid-session): it is
    /// inspectable, and resumable only if the agent supports session load.
    Interrupted,
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

/// Serde default for an integration level field (level 1).
fn one() -> u8 {
    1
}

/// One agent's persisted conformance result: the verified level and per-level
/// criteria outcomes, stamped with the run date and the agent version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConformanceResult {
    /// The catalog id the run verified.
    pub agent_id: String,
    /// The highest level whose criteria all passed.
    pub verified_level: u8,
    /// The agent version reported at handshake, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
    /// When the run concluded (RFC 3339, UTC).
    pub run_at: String,
    /// Per-criterion pass/fail, keyed by a stable criterion name.
    pub criteria: Vec<ConformanceCriterion>,
}

/// One conformance criterion outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConformanceCriterion {
    /// The integration level this criterion belongs to.
    pub level: u8,
    /// A stable criterion name (from the scenario it verifies).
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
}

/// Params of `session/list`: filters over the sessions of a project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListParams {
    /// Restrict to one project root; when absent, list across projects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    /// Restrict to one lifecycle state (e.g. only `ended` or `interrupted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<SessionState>,
    /// Cap the number returned (most recent first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One session (active or historical) as reported by `session/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// Meltemi session identifier.
    pub session_id: String,
    /// The agent binary and arguments (program first).
    pub agent_command: Vec<String>,
    /// Absolute repository root the session ran in.
    pub project_root: String,
    /// Current or final lifecycle state.
    pub state: SessionState,
    /// The integration level the session ran at (1-4).
    #[serde(default = "one")]
    pub level: u8,
    /// The final turn status, when the session ended cleanly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_status: Option<TurnStatus>,
    /// Start timestamp (RFC 3339, UTC).
    pub started_at: String,
    /// End timestamp, when the session ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// Whether the session can be resumed (the agent announced session load
    /// and the agent session id is known).
    pub resumable: bool,
}

/// Result of `session/list`, most recent first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResult {
    /// The sessions matching the filters.
    pub sessions: Vec<SessionInfo>,
}

/// Params of `session/log`: a paginated slice of a session's JSONL log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogParams {
    /// The project the session belongs to (locates the log on disk).
    pub project_root: String,
    /// The session whose log to read.
    pub session_id: String,
    /// First line to return (0-based); defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Maximum lines to return; defaults to a daemon-chosen page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Result of `session/log`: raw JSONL lines plus paging metadata, so a client
/// can page backward through a long transcript without reading the daemon's
/// disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLogResult {
    /// The session id echoed back.
    pub session_id: String,
    /// Total number of lines in the log.
    pub total: u32,
    /// Offset of the first returned line.
    pub offset: u32,
    /// The raw JSONL lines in `[offset, offset + lines.len())`.
    pub lines: Vec<String>,
}

/// Params of `fleet/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetListParams {
    /// Absolute path of a project root; when given, the response marks the
    /// agent that project's configuration selects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
}

/// Where a fleet catalog entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetAgentSource {
    /// The bundled registry snapshot.
    Registry,
    /// A user-declared agent (`[[fleet.custom]]` in config).
    Custom,
}

/// One agent of the fleet catalog, as reported by `fleet/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetAgent {
    /// Stable catalog identifier (selectable via `agent.id` in config).
    pub id: String,
    /// Human-readable agent name.
    pub display_name: String,
    /// Where this entry comes from.
    pub source: FleetAgentSource,
    /// Declared Meltemi integration level (1 native ACP, 2 adapter,
    /// 3 structured headless, 4 artifacts).
    pub integration_level: u8,
    /// The level verified by the last conformance run, when one is recorded.
    /// Absent means declared-but-unverified (shown distinctly in surfaces).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_level: Option<u8>,
    /// The date (RFC 3339) of the conformance run behind `verified_level`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    /// Whether the agent declares MCP passthrough support in the registry.
    #[serde(default)]
    pub mcp_support: bool,
    /// Whether the agent's binary was found on this system.
    pub detected: bool,
    /// Absolute path of the detected binary; present only when detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    /// Whether the project at `projectRoot` selects this agent (always
    /// `false` when the request carried no `projectRoot`).
    pub configured: bool,
}

/// Result of `fleet/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetListResult {
    /// Version of the registry snapshot the catalog was built from.
    pub registry_version: String,
    /// The catalog, in registry order with custom entries appended.
    pub agents: Vec<FleetAgent>,
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

/// Params of `repo/map`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoMapParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// Maximum directory depth to descend (0 = the root's entries only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    /// Maximum entries to return before declaring truncation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// One entry of the repository map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    /// Path relative to the repository root (forward slashes).
    pub path: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
}

/// Result of `repo/map`. `truncated`/`omitted` declare any budget cutoff —
/// never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoMapResult {
    /// The entries honoring nested `.gitignore`, sorted by path.
    pub entries: Vec<RepoEntry>,
    /// Whether the limit cut the listing short.
    pub truncated: bool,
    /// How many entries were omitted by the cutoff.
    pub omitted: u32,
}

/// Params of `context/project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProjectParams {
    /// Absolute path to the root of the target repository.
    pub project_root: String,
}

/// One target file the projection wrote (or found already current).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTarget {
    /// Destination path, relative to the project root (`AGENTS.md`, ...).
    pub path: String,
    /// Hex SHA-256 of the compiled content written into the managed block.
    pub fingerprint: String,
    /// Whether the file was actually rewritten (false when already current).
    pub written: bool,
}

/// Result of `context/project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProjectResult {
    /// Every declared target with its fingerprint and whether it changed.
    pub targets: Vec<ContextTarget>,
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
    /// How many of the agent's permission requests were denied during the
    /// turn (by rule, by the human, or by timeout/default). When greater than
    /// zero the artifact may be incomplete (honesty of result, H1/H4/H5).
    #[serde(default)]
    pub denied_permissions: u32,
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
    /// A persistent rule resolved it before escalation.
    Rule,
}

/// The effect of a permission rule: grant or refuse the matched request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionRuleEffect {
    /// Grant the request (select the agent's allow option).
    Allow,
    /// Refuse the request (select a reject option, or cancel).
    Deny,
}

/// Where a permission rule persists and how far it reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionRuleScope {
    /// The user's global rules file.
    Global,
    /// The project's `.meltemi/permissions.toml`.
    Project,
}

/// A persistent permission rule, evaluated in the daemon before a request is
/// escalated to the human. The matchers are ANDed; an omitted matcher matches
/// anything on that dimension. A rule MUST NOT grant an option the agent did
/// not offer — it only decides among the offered options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    /// Whether the rule allows or denies.
    pub effect: PermissionRuleEffect,
    /// Match on the tool/operation kind (exact).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Match when the request's command starts with this prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_prefix: Option<String>,
    /// Match when the affected path starts with this prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_prefix: Option<String>,
    /// Where the rule lives.
    pub scope: PermissionRuleScope,
}

/// One pending permission request as seen in a queue snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPermission {
    /// Stable id of this pending request (used by `permission/decide`).
    pub request_id: String,
    /// The Meltemi session it belongs to.
    pub session_id: String,
    /// Short tool/operation label (from the ACP tool call).
    pub tool: String,
    /// One-line human summary of what is being authorized.
    pub summary: String,
    /// The options the agent offered.
    pub options: Vec<PermissionOption>,
    /// Seconds the request has been waiting (snapshot at query time).
    pub waiting_seconds: u64,
    /// Seconds until it expires; negative once expired (snapshot).
    pub expires_in_seconds: i64,
    /// Whether it has already expired but is still shown (never dropped
    /// silently).
    pub expired: bool,
    /// A rule suggested to end repeated identical approvals (anti-fatigue);
    /// present once the human has approved the same shape enough times.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_rule: Option<PermissionRule>,
}

/// Result of `permission/pending`: the current queue snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionPendingResult {
    /// The pending requests, oldest first.
    pub pending: Vec<PendingPermission>,
}

/// Params of the `permission/changed` notification: the full current queue,
/// so every client reconciles to one snapshot without a round-trip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionChangedParams {
    /// The pending requests after the change, oldest first.
    pub pending: Vec<PendingPermission>,
}

/// Params of `permission/decide`: resolve a pending request by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDecideParams {
    /// The pending request to resolve.
    pub request_id: String,
    /// The chosen option id; `None` cancels (denies) the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_id: Option<String>,
    /// A rule to persist alongside this decision ("allow/deny always").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persist_rule: Option<PermissionRule>,
}

/// Whether a `permission/decide` call applied or lost the race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecideStatus {
    /// This call resolved the request.
    Applied,
    /// The request was already resolved by another path; nothing changed.
    AlreadyResolved,
}

/// Result of `permission/decide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDecideResult {
    /// The reconciliation outcome (first-wins).
    pub status: PermissionDecideStatus,
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
    /// `@` references in a prompt were expanded; records the paths and injected
    /// byte counts so the delivered context is reconstructable
    /// (gestion-contexto-repo).
    RefsExpanded {
        /// One entry per expanded reference.
        expansions: Vec<RefExpansion>,
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
        /// The rule that resolved it, when `decided_by` is `rule` — its scope
        /// and content, so every grant is traceable to what took it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rule: Option<PermissionRule>,
    },
    /// MCP servers were injected into the session at creation. Only the server
    /// names are recorded — never resolved env values or credentialed URLs
    /// (mcp-passthrough D3).
    McpInjected {
        /// The names of the injected servers.
        servers: Vec<String>,
    },
    /// Declared MCP servers were not delivered (the agent announced no MCP
    /// support); recorded so the omission is visible, never silent.
    McpNotDelivered {
        /// Why they were not delivered.
        reason: String,
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

/// One expanded `@` reference: the path and how many bytes it injected (0 when
/// not found or truncated to nothing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefExpansion {
    /// The referenced path.
    pub path: String,
    /// Bytes injected into the prompt for this reference.
    pub bytes: u64,
    /// Whether the reference could not be found.
    pub not_found: bool,
    /// Whether the injected content was truncated by a limit.
    pub truncated: bool,
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
