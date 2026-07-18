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
    /// Request: create/edit the project constitution with a final gate.
    pub const SDD_CONSTITUTION: &str = "sdd/constitution";
    /// Request: deliberate with the agent, never writing (streaming).
    pub const SDD_EXPLORE: &str = "sdd/explore";
    /// Request: start the SDD authoring cycle for a change.
    pub const SDD_PROPOSE: &str = "sdd/propose";
    /// Request: refine design and sequence tasks by dependencies.
    pub const SDD_PLAN: &str = "sdd/plan";
    /// Request: decide a pending authoring gate (approve/comment/abort).
    pub const SDD_GATE: &str = "sdd/gate";
    /// Request: the review checklist of a change's spec deltas.
    pub const SDD_REVIEW: &str = "sdd/review";
    /// Request: decide one review checklist item (approve/comment/reject).
    pub const SDD_REVIEW_DECIDE: &str = "sdd/review-decide";
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
    /// Request: plan and create the N×M worktree assignment for a set of
    /// tasks and agents (parallel batches, serialized overlaps, races).
    pub const WORKTREE_ASSIGN: &str = "worktree/assign";
    /// Request: list the worktrees the daemon manages for a project.
    pub const WORKTREE_LIST: &str = "worktree/list";
    /// Request: remove a managed worktree (safe cleanup, dirty needs confirm).
    pub const WORKTREE_REMOVE: &str = "worktree/remove";
    /// Request: the diff of each competitor of a task against the common base.
    pub const WORKTREE_DIFF: &str = "worktree/diff";
    /// Request: apply one file from a source worktree into a target worktree
    /// (assisted merge; each application is an explicit human decision).
    pub const WORKTREE_MERGE_FILE: &str = "worktree/merge-file";
    /// Request: create the pre-task checkpoint of a worktree (technical ref).
    pub const CHECKPOINT_CREATE: &str = "checkpoint/create";
    /// Request: list checkpoints by change and task (ref, moment, worktree).
    pub const CHECKPOINT_LIST: &str = "checkpoint/list";
    /// Request: revert a task's worktree to its checkpoint (needs confirm).
    pub const CHECKPOINT_REVERT: &str = "checkpoint/revert";
    /// Request: record an approved out-of-tree operation against a task, so
    /// reversion can declare it irreversible (honest scope).
    pub const CHECKPOINT_RECORD_OP: &str = "checkpoint/record-op";
    /// Request: propose or apply the atomic per-task commit with traceability
    /// trailers, in supervised (preview) or autonomous (apply) mode.
    pub const COMMIT_TASK: &str = "commit/task";
    /// Request: the per-requirement verification checklist of a change (each
    /// scenario linked to a test, marked manually, or unverified).
    pub const SDD_VERIFY: &str = "sdd/verify";
    /// Request: record a manual verification of a scenario with a note.
    pub const SDD_VERIFY_MARK: &str = "sdd/verify-mark";
    /// Request: fold a change's deltas into the living truth atomically and
    /// preserve it in the dated history (gated by verification).
    pub const SDD_ARCHIVE: &str = "sdd/archive";
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
    /// Worktree orchestration was requested on a directory that is not a git
    /// repository (honest degradation: refuse with remedy).
    pub const WORKTREE_UNAVAILABLE: i64 = 4000;
    /// A managed-worktree or checkpoint operation was refused: the target is
    /// not one the daemon created, it has uncommitted changes, or the required
    /// confirmation was not given.
    pub const WORKTREE_REFUSED: i64 = 4001;
    /// No checkpoint exists for the requested change/task/agent.
    pub const CHECKPOINT_NOT_FOUND: i64 = 4002;
    /// The per-task commit could not be created — most often a user git hook
    /// rejected it. The hook output is surfaced verbatim; hooks are never
    /// bypassed. The task stays completed-without-commit (a visible state).
    pub const GIT_COMMIT_FAILED: i64 = 4003;
    /// Archiving was blocked: a requirement is neither verified nor excepted.
    /// The detail lists what is missing; nothing is folded.
    pub const VERIFY_INCOMPLETE: i64 = 4004;
    /// Archiving was blocked: applying the change's deltas to the living truth
    /// raised conflict diagnostics. Nothing is folded — the truth is intact.
    pub const SPEC_MERGE_CONFLICT: i64 = 4005;
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

/// Params of `sdd/propose`: start the authoring cycle for a change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddProposeParams {
    /// Free-form description of the change.
    pub idea: String,
    /// Absolute path to the project root.
    pub project_root: String,
    /// Force a mode against the eligibility criterion (`spec_full` or
    /// `fast_forward`); absent lets the daemon choose by the criterion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_mode: Option<String>,
}

/// Params of `sdd/explore` and `sdd/constitution`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddExploreParams {
    /// Absolute path to the project root.
    pub project_root: String,
    /// The topic/question to deliberate (explore) or guidance (constitution).
    #[serde(default)]
    pub topic: String,
}

/// Params of `sdd/gate`: decide a pending authoring gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddGateParams {
    /// Absolute path to the project root.
    pub project_root: String,
    /// The change whose gate is being decided.
    pub change_name: String,
    /// `approve` | `comment` | `abort`.
    pub decision: String,
    /// The rework comment, when `decision` is `comment`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Params of `sdd/review`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddReviewParams {
    pub project_root: String,
    pub change_name: String,
}

/// Params of `sdd/review-decide`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddReviewDecideParams {
    pub project_root: String,
    pub change_name: String,
    /// The capability the requirement belongs to.
    pub capability: String,
    /// The requirement name being decided.
    pub requirement: String,
    /// `approve` | `comment` | `reject`.
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// One review checklist item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub capability: String,
    pub requirement: String,
    /// `pending` | `approved` | `commented` | `rejected`.
    pub state: String,
    /// Diagnostics anchored to this requirement.
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Result of a review step: the checklist and whether it can close.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddReviewResult {
    pub change_name: String,
    pub items: Vec<ReviewItem>,
    /// How many items are still `pending`.
    pub pending: u32,
    /// Whether closing the review is allowed (all items decided).
    pub can_close: bool,
}

/// Params of `sdd/plan`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddPlanParams {
    /// Absolute path to the project root.
    pub project_root: String,
    /// The change to plan.
    pub change_name: String,
}

/// Result of an SDD cycle step: where the cycle now stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddResult {
    /// The change name.
    pub change_name: String,
    /// The cycle phase: `gate_pending` | `completed` | `aborted` | `invalid` |
    /// `explored`.
    pub phase: String,
    /// The artifact awaiting a gate (when `phase` is `gate_pending`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    /// The authoring mode in effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Validation diagnostics returned to the agent (when `phase` is `invalid`).
    #[serde(default)]
    pub diagnostics: Vec<String>,
    /// How to decide the pending gate (scriptable guidance, never a hang).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_hint: Option<String>,
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
    /// A pre-task checkpoint of the worktree was created (checkpoints-rollback).
    CheckpointCreated {
        /// The technical ref under `refs/meltemi/checkpoints/`.
        git_ref: String,
        /// The change the task belongs to.
        change: String,
        /// The task label the checkpoint precedes.
        task: String,
        /// The agent whose worktree was snapshotted.
        agent: String,
    },
    /// A task's worktree was restored to its checkpoint. `irreversible` lists
    /// the approved out-of-tree operations the restore could not undo — the
    /// reversion is never presented as total when it is non-empty.
    CheckpointRestored {
        /// The technical ref restored from.
        git_ref: String,
        /// The change the task belongs to.
        change: String,
        /// The task label restored.
        task: String,
        /// The agent whose worktree was restored.
        agent: String,
        /// Approved out-of-tree operations that remain in effect.
        irreversible: Vec<String>,
    },
    /// An atomic per-task commit was applied with traceability trailers
    /// (git-commit-por-tarea). Never carries co-authorship — the author is the
    /// user's own git identity.
    TaskCommitted {
        /// The change the task belongs to.
        change: String,
        /// The task label committed.
        task: String,
        /// The agent whose worktree was committed.
        agent: String,
        /// The commit SHA.
        sha: String,
        /// The `<capability>/<requirement>` pairs recorded in `Meltemi-Req`.
        requirements: Vec<String>,
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

/// One task to assign, with the agents that will work it and the files it
/// declares it touches. More than one agent on the same task is a race.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeTask {
    /// The change the task belongs to.
    pub change: String,
    /// The task label (e.g. `1.2`).
    pub task: String,
    /// The agents assigned to this task (>1 = a race).
    pub agents: Vec<String>,
    /// The files the task declares it touches (for overlap serialization).
    #[serde(default)]
    pub files: Vec<String>,
}

/// Params of `worktree/assign`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeAssignParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The tasks to assign.
    pub tasks: Vec<WorktreeTask>,
}

/// One worktree the daemon manages, as reported over the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    /// The change the worktree serves.
    pub change: String,
    /// The task label.
    pub task: String,
    /// The agent that owns this worktree.
    pub agent: String,
    /// Absolute path of the worktree on disk.
    pub path: String,
    /// The managed branch checked out in the worktree.
    pub branch: String,
    /// The base revision the worktree was created from.
    pub base_rev: String,
    /// Whether this worktree competes with others on the same task (a race).
    pub competitor: bool,
}

/// One batch of the assignment plan: its tasks run in parallel; batches run in
/// sequence. `serializedReason` explains why a batch was split off (overlap).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeBatch {
    /// The task labels running in parallel in this batch.
    pub tasks: Vec<String>,
    /// Why this batch was serialized after the previous one, if it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialized_reason: Option<String>,
}

/// Result of `worktree/assign`: the parallel/serial plan plus every worktree
/// created (one per agent per task, all from the same fixed base).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeAssignResult {
    /// The common base revision fixed for the whole assignment.
    pub base_rev: String,
    /// The parallel/serial batching of tasks.
    pub batches: Vec<WorktreeBatch>,
    /// Every worktree created for the assignment.
    pub worktrees: Vec<Worktree>,
}

/// Params of `worktree/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeListParams {
    /// Absolute path to the repository root.
    pub project_root: String,
}

/// Result of `worktree/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeListResult {
    /// The worktrees the daemon manages for this project (its own only).
    pub worktrees: Vec<Worktree>,
}

/// Params of `worktree/remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeRemoveParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// Absolute path of the managed worktree to remove.
    pub path: String,
    /// Confirmation to remove a worktree with uncommitted changes.
    #[serde(default)]
    pub force: bool,
}

/// Result of `worktree/remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeRemoveResult {
    /// Whether the worktree was removed.
    pub removed: bool,
}

/// Params of `worktree/diff`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDiffParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change whose task competitors to compare.
    pub change: String,
    /// The task label whose competitors to compare.
    pub task: String,
}

/// One competitor's result in a race: its diff against the common base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeCompetitorDiff {
    /// The agent that produced this result.
    pub agent: String,
    /// The worktree path.
    pub path: String,
    /// The files changed against the common base.
    pub changed_files: Vec<String>,
    /// The unified diff against the common base.
    pub diff: String,
}

/// Result of `worktree/diff`: every competitor of the task, side by side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDiffResult {
    /// The common base revision the diffs are taken against.
    pub base_rev: String,
    /// One entry per competing agent.
    pub competitors: Vec<WorktreeCompetitorDiff>,
}

/// Params of `worktree/merge-file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeMergeFileParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// Absolute path of the worktree chosen as the integration base.
    pub target: String,
    /// Absolute path of the worktree to take the file from.
    pub source: String,
    /// The file (relative to the worktree root) to apply.
    pub file: String,
    /// Explicit human confirmation: nothing is applied without it.
    #[serde(default)]
    pub confirm: bool,
}

/// Result of `worktree/merge-file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeMergeFileResult {
    /// Whether the file was applied into the target worktree.
    pub applied: bool,
}

/// Identifies one task's worktree checkpoint by change, task and agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointCreateParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change the task belongs to.
    pub change: String,
    /// The task label the checkpoint precedes.
    pub task: String,
    /// The agent whose worktree is snapshotted.
    pub agent: String,
}

/// One pre-task checkpoint of a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    /// The change the task belongs to.
    pub change: String,
    /// The task label the checkpoint precedes.
    pub task: String,
    /// The agent whose worktree was snapshotted.
    pub agent: String,
    /// The technical ref under `refs/meltemi/checkpoints/`.
    pub git_ref: String,
    /// Absolute path of the worktree the checkpoint restores.
    pub worktree: String,
    /// RFC 3339 UTC moment the checkpoint was created.
    pub created_at: String,
    /// Approved out-of-tree operations accumulated for this task (irreversible).
    #[serde(default)]
    pub irreversible: Vec<String>,
}

/// Result of `checkpoint/create`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointCreateResult {
    /// The checkpoint that was created.
    pub checkpoint: Checkpoint,
}

/// Params of `checkpoint/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointListParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// Optional change filter; omitted lists every checkpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
}

/// Result of `checkpoint/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointListResult {
    /// The checkpoints, most recent first.
    pub checkpoints: Vec<Checkpoint>,
}

/// Params of `checkpoint/revert`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRevertParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change the task belongs to.
    pub change: String,
    /// The task label to revert.
    pub task: String,
    /// The agent whose worktree to restore.
    pub agent: String,
    /// Explicit human confirmation: nothing is reverted without it.
    #[serde(default)]
    pub confirm: bool,
}

/// The honest scope of a reversion: what the restore covered and what it could
/// not undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertScope {
    /// Whether the worktree was restored to the checkpoint (tracked state and
    /// removal of later untracked files).
    pub worktree_restored: bool,
    /// Whether the reversion is complete: the worktree was restored and no
    /// out-of-tree operation remains in effect.
    pub complete: bool,
    /// Approved out-of-tree operations that remain in effect (irreversible).
    #[serde(default)]
    pub irreversible: Vec<String>,
}

/// Result of `checkpoint/revert`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRevertResult {
    /// Whether the worktree was reverted.
    pub reverted: bool,
    /// The honest scope of what the reversion covered.
    pub scope: RevertScope,
}

/// Params of `checkpoint/record-op`: record one approved out-of-tree operation
/// against a task, so its reversion declares the operation irreversible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecordOpParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change the task belongs to.
    pub change: String,
    /// The task label the operation ran under.
    pub task: String,
    /// The agent that ran the operation.
    pub agent: String,
    /// A human-readable description of the out-of-tree operation.
    pub operation: String,
}

/// Result of `checkpoint/record-op`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecordOpResult {
    /// Whether the operation was recorded.
    pub recorded: bool,
}

/// One requirement a task implements, for the `Meltemi-Req` trailer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRequirement {
    /// The capability (spec) the requirement lives in.
    pub capability: String,
    /// The requirement name (free text; slugged into the trailer).
    pub requirement: String,
}

/// Params of `commit/task`. With `confirm` false the daemon returns the
/// proposed message and the diff summary without committing (supervised
/// preview); with `confirm` true it applies the commit (autonomous, or the
/// human's approval). The title/body are the editable inputs — the daemon
/// guarantees the message form and trailers either way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitTaskParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change the task belongs to.
    pub change: String,
    /// The task label being committed.
    pub task: String,
    /// The agent whose worktree is committed.
    pub agent: String,
    /// The imperative English title (the daemon enforces the convention).
    pub title: String,
    /// The optional body (what/why); the agent may propose it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The requirements this task implements (one `Meltemi-Req` each).
    #[serde(default)]
    pub requirements: Vec<TaskRequirement>,
    /// The files the task declared it touches, to verify the commit's scope.
    #[serde(default)]
    pub declared_files: Vec<String>,
    /// Whether to apply the commit (true) or only preview it (false).
    #[serde(default)]
    pub confirm: bool,
}

/// Result of `commit/task`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitTaskResult {
    /// Whether a commit was applied (false for a preview).
    pub committed: bool,
    /// The final, convention-guaranteed commit message (with trailers).
    pub message: String,
    /// The commit SHA, when applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// The files the commit changed against the checkpoint base.
    pub changed_files: Vec<String>,
    /// Committed paths outside the task's declared scope — reported, never
    /// hidden (empty when the scope matches or nothing was declared).
    pub deviations: Vec<String>,
    /// Whether the worktree is clean after the commit (atomicity).
    pub tree_clean: bool,
}

/// Params of `sdd/verify`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddVerifyParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change whose requirements to verify.
    pub change: String,
}

/// One scenario's verification status in the checklist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyScenario {
    /// The capability (spec) the scenario belongs to.
    pub capability: String,
    /// The requirement the scenario belongs to.
    pub requirement: String,
    /// The scenario name.
    pub scenario: String,
    /// `linked` (a test names it), `manual` (marked with a note), or
    /// `unverified`.
    pub status: String,
    /// The note recorded with a manual verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Result of `sdd/verify`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddVerifyResult {
    /// One entry per scenario of the change's deltas.
    pub scenarios: Vec<VerifyScenario>,
    /// How many scenarios are verified (linked or manual).
    pub verified: u32,
    /// The total number of scenarios.
    pub total: u32,
    /// Whether every scenario is verified.
    pub complete: bool,
}

/// Params of `sdd/verify-mark`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddVerifyMarkParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change the scenario belongs to.
    pub change: String,
    /// The scenario name to mark verified.
    pub scenario: String,
    /// The note justifying the manual verification.
    pub note: String,
}

/// Result of `sdd/verify-mark`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddVerifyMarkResult {
    /// Whether the manual mark was recorded.
    pub marked: bool,
}

/// One requirement exception for archiving: a scenario allowed through the gate
/// without verification, with a justification recorded in the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveException {
    /// The scenario being excepted.
    pub scenario: String,
    /// Why it is allowed through unverified.
    pub justification: String,
}

/// Params of `sdd/archive`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddArchiveParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change to archive.
    pub change: String,
    /// Confirmation to proceed when the living specs tree is dirty.
    #[serde(default)]
    pub confirm: bool,
    /// Explicit exceptions for unverified requirements (recorded).
    #[serde(default)]
    pub exceptions: Vec<ArchiveException>,
}

/// Result of `sdd/archive`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddArchiveResult {
    /// The capabilities whose living specs were folded.
    pub capabilities: Vec<String>,
    /// Where the change was preserved in the dated history.
    pub archived_to: String,
    /// Whether the projected context was regenerated after the fold.
    pub projection_regenerated: bool,
    /// The scenarios that passed the gate as recorded exceptions.
    #[serde(default)]
    pub excepted: Vec<String>,
}
