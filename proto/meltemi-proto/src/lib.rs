// SPDX-License-Identifier: Apache-2.0

//! Serde types for the Meltemi daemon<->client protocol.
//!
//! The JSON Schemas under `proto/schemas/` are the language-neutral source
//! of truth for this contract; the types in this crate must serialize in
//! conformance with them (validated by `tests/conformance.rs`). On any
//! discrepancy, the schema wins.

use std::collections::BTreeMap;

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
    /// Request (client -> daemon): start a governed free session on a project
    /// root with an instruction — no change, no task, no spec gate. The door
    /// the method's verbs never had: `session/direct` steers a session that
    /// already exists, `propose` and `sdd/*` are verbs of the method, and
    /// `worktree/dispatch` demands a change and a task. The government is the
    /// same as any other session's (lanzador-conversacional D1).
    pub const SESSION_START: &str = "session/start";
    /// Notification (client -> daemon): cancel an active session.
    pub const SESSION_CANCEL: &str = "session/cancel";
    /// Request (client -> daemon): direct an instruction to an existing session
    /// — queued as the next turn of an active session, or resuming a terminated
    /// but resumable one (control-remoto-asistido). The third remote verb.
    pub const SESSION_DIRECT: &str = "session/direct";
    /// Notification (daemon -> client): streamed session event.
    pub const SESSION_EVENT: &str = "session/event";
    /// Request (client -> daemon): declare whether this connection watches a
    /// session's event stream. The connection that started a session receives
    /// its stream without asking; any other must declare it
    /// (eventos-para-tardios).
    pub const SESSION_WATCH: &str = "session/watch";
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
    /// Request: run one competitor's turn over its assignment worktree with
    /// that competitor's own resolved binary — the composable race primitive
    /// (flota-multiproveedor). Never ticks `tasks.md`.
    pub const WORKTREE_DISPATCH: &str = "worktree/dispatch";
    /// Request: apply a human edit to a file of the project root or of a
    /// managed worktree, through the daemon — traceable as a `human_edit`
    /// event and governed by the soft-lock concurrency policy (edit-surface,
    /// gui-tauri-paridad design D4/D5).
    pub const WORKTREE_APPLY_EDIT: &str = "worktree/apply-edit";
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
    /// Request: deploy agents over a change's `tasks.md`, task by task, with
    /// the full composed cycle (checkpoint → turn → commit → tick).
    pub const SDD_IMPLEMENT: &str = "sdd/implement";
    /// Request: list the method's changes (active and archived) with aggregated
    /// state (artifacts, tasks, review, verify). Read-only (navegacion-del-metodo).
    pub const CHANGE_LIST: &str = "change/list";
    /// Request: show a change — its artifacts and its deltas per capability.
    pub const CHANGE_SHOW: &str = "change/show";
    /// Request: list the living-truth capabilities with requirement/scenario counts.
    pub const SPEC_LIST: &str = "spec/list";
    /// Request: show a living-truth capability, its requirements and scenarios.
    pub const SPEC_SHOW: &str = "spec/show";
    /// Request: validate a change (engine + dry-run merge) or the whole living
    /// truth, without archiving; findings are a result, not an error.
    pub const SDD_VALIDATE: &str = "sdd/validate";
    /// Request: the projects this user has pointed Meltemi at, most recently
    /// seen first — the catalog the surfaces build their project tree from
    /// (multiproyecto-suscripciones). Read-only; nothing is discovered by
    /// walking the disk.
    pub const PROJECT_LIST: &str = "project/list";
    /// Request: add a project root to the registry explicitly — the path the
    /// client hands over, validated and canonicalized. An explicit entry, never
    /// a discovery: the daemon opens no window and walks no disk
    /// (lanzador-conversacional D6).
    pub const PROJECT_REGISTER: &str = "project/register";
    /// Request: drop a project from the registry's listing, and from nothing
    /// else. Appends a forget line to the same append-only registry that the
    /// last-wins fold resolves; no file, no session and no log is ever touched
    /// (lanzador-conversacional D6).
    pub const PROJECT_FORGET: &str = "project/forget";

    /// Request: local usage accounting, aggregated by the daemon over the
    /// session records it already keeps (analitica-consumo-local). Reads local
    /// records only; opens no network connection, ever.
    pub const ANALYTICS_USAGE: &str = "analytics/usage";
    /// Request: link a named subscription of a catalog agent — a launch
    /// profile whose env pins the provider's auth-context variable to a fresh
    /// directory — answering with the composed login gesture Meltemi never
    /// runs (vincular-suscripciones).
    pub const SUBSCRIPTION_LINK: &str = "subscription/link";
    /// Request: unlink a linked subscription. Removes the profile from the
    /// daemon-owned store only; the auth-context directory is never deleted.
    pub const SUBSCRIPTION_UNLINK: &str = "subscription/unlink";
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
    /// A subscription link/unlink was refused: the entry declares no
    /// auth-context variable, the name is invalid or already linked, or the
    /// profile lives in hand-written configuration. The detail says which and
    /// the remedy says what to do instead (vincular-suscripciones).
    pub const SUBSCRIPTION_REFUSED: i64 = 2005;
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
    /// A `change/show`, `spec/show` or `sdd/validate` named a change or
    /// capability that does not exist under `.meltemi/`. Nothing is read further.
    pub const ARTIFACT_NOT_FOUND: i64 = 4006;
    /// An `analytics/usage` query is unusable as written (inverted range,
    /// unparseable bound, empty limit). It is refused with a remedy rather than
    /// silently replaced by a default period.
    pub const USAGE_QUERY_INVALID: i64 = 5000;
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
    /// The fleet agents detected on this system, when the error is a refusal
    /// to resolve one (2000/2001). A surface can then offer a choice instead of
    /// transcribing a lament (lanzador-conversacional D7). An empty list is
    /// itself an answer — the fleet was consulted and nothing was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<AgentCandidate>>,
}

/// One agent offered as a way out of a resolution refusal. It carries the very
/// vocabulary `fleet/list` publishes, computed by the same detection path, so
/// the error and the Fleet view cannot disagree. Ids, detection and remedies
/// only: never an environment value, a credential path or anything shaped like
/// a secret (constitution §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCandidate {
    /// Stable catalog identifier, the same one `agent.id` selects.
    pub id: String,
    /// Whether this agent's pilot point was found on this system.
    pub detected: bool,
    /// The composed install state across its layers.
    pub install_state: FleetInstallState,
    /// Which layer is missing and what to do about it, in one sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
    /// The exact command that installs the missing layer. Data, never run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy_command: Option<String>,
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
    /// The catalog id the session's agent resolved to, when the resolution
    /// named one (multiproyecto-suscripciones design D4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// The launch profile — the "subscription" — the session ran under. The
    /// NAME only: no field ever carries the profile's env overlay (§2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// What the session is about, derived by the daemon from the instruction
    /// that opened it. Absent when no user sentence started the session — a
    /// dispatched race lane — and for sessions recorded before titles existed
    /// (titulo-de-sesion design D1, D2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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
    /// A launch profile (`[[fleet.profile]]`): a catalog agent under a selected
    /// auth context (flota-multiproveedor).
    Profile,
}

/// Where a per-session agent resolution came from (flota-multiproveedor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetResolutionSource {
    /// A launch profile matched by name.
    Profile,
    /// A catalog id matched by name.
    Catalog,
    /// The project-configured agent (the free-label fallback).
    Configured,
}

/// The period grain of a usage aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageGranularity {
    Day,
    Week,
    Month,
    /// One cell per dimension combination for the whole range (the default).
    #[default]
    Total,
}

/// Parameters of `analytics/usage` (analitica-consumo-local D5).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsUsageParams {
    /// The project to account for; absent aggregates every project with
    /// records in the user's data directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    /// Inclusive lower bound (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    /// Exclusive upper bound (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Period grain; absent means `total`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granularity: Option<UsageGranularity>,
    /// Keep only cells whose effective binary matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Keep only cells of this launch profile (subscription).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Maximum cells to return; the response declares when it truncated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// The closed, declared set of activity metrics of a cell (design D2). Adding
/// a fact here is a future delta, never a silent extension.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageActivity {
    pub sessions: u32,
    /// Sessions with a recorded end.
    pub sessions_closed: u32,
    /// Sessions with no recorded end (interrupted or still running).
    pub sessions_open: u32,
    /// Seconds of closed sessions only: an open session is never extrapolated.
    pub active_seconds: u64,
    pub prompts: u32,
    /// Completed turns by stop reason (the reason is the key).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub turns_by_stop_reason: BTreeMap<String, u32>,
    pub permissions_requested: u32,
    pub permissions_approved: u32,
    pub permissions_denied: u32,
    pub permissions_expired: u32,
    pub human_edits: u32,
    pub commits: u32,
    pub checkpoints: u32,
    pub errors: u32,
}

/// Measured token counters. Every field is optional on purpose: a counter the
/// official output did not declare stays absent and MUST NOT become zero.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTokens {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl UsageTokens {
    /// Whether no counter at all was measured (so the field is reported absent).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.input.is_none()
            && self.output.is_none()
            && self.cached_input.is_none()
            && self.reasoning.is_none()
            && self.total.is_none()
    }
}

/// Why a session contributed no measured tokens. Stable set (design D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageUnreportedKind {
    /// The session ran over ACP, whose protocol does not carry usage.
    ProtocolCarriesNoUsage,
    /// The integration level runs no process (level 4, artifacts only).
    LevelRunsNoProcess,
    /// A level-3 run whose structured output declared no counters.
    StreamDeclaredNoCounters,
}

/// One reason with how many sessions it accounts for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageUnreportedReason {
    pub kind: UsageUnreportedKind,
    pub sessions: u32,
}

/// How much of the activity a token figure was computed over (design D4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCoverage {
    /// Sessions that contributed measured counters.
    pub measured_sessions: u32,
    /// Sessions with no usage data at all.
    pub unreported_sessions: u32,
    /// Why each unreported session has no data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<UsageUnreportedReason>,
}

/// One aggregation cell: project × agent × profile × period (design D2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCell {
    pub project_key: String,
    pub project_root: String,
    /// The effective binary that ran — what happened, not what config promised.
    pub agent: String,
    /// The catalog id, when the resolution named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// The launch profile (subscription) by name, when the resolution used one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The integration level, an attribute of the cell rather than a dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// The period label: `2026-07-24`, `2026-W30`, `2026-07` or `total`.
    pub period: String,
    pub activity: UsageActivity,
    /// Measured counters; absent when nothing was measured (never zeroed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<UsageTokens>,
    pub coverage: UsageCoverage,
}

/// Facts that could not be attributed to an agent or a profile. They live in
/// their own bucket and are never spread over the attributed cells (design D2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageUnattributed {
    pub project_key: String,
    pub project_root: String,
    pub period: String,
    /// Human edits applied with no session active on the tree.
    pub human_edits: u32,
}

/// A stable disclosure key. The daemon emits keys; each surface renders the
/// text from its own ES/EN catalog (design D6) — the daemon is no translator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageDisclosure {
    /// Activity is folded from the local session logs and index.
    ActivityFromLocalRecords,
    /// Tokens come only from counters the official output reported.
    TokensOnlyWhenOfficialOutputReports,
    /// Quota, balance and billing of the provider account are not visible.
    NoQuotaBalanceOrBilling,
    /// Nothing is estimated: an absent counter stays absent.
    NothingIsEstimated,
    /// No data leaves this machine.
    NothingLeavesThisMachine,
}

/// Result of `analytics/usage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsUsageResult {
    pub cells: Vec<UsageCell>,
    /// Unattributed facts, per project and period.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unattributed: Vec<UsageUnattributed>,
    /// Sum of the returned cells, with the coverage of its token figures.
    pub totals: UsageTotals,
    /// True when `limit` cut the cell list — declared, never silent.
    pub truncated: bool,
    /// The honesty disclosure, as stable keys.
    pub disclosure: Vec<UsageDisclosure>,
}

/// Totals over the returned cells.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTotals {
    pub cells: u32,
    pub activity: UsageActivity,
    /// Measured tokens only; absent when nothing was measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<UsageTokens>,
    pub coverage: UsageCoverage,
}

/// Parameters of `project/list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListParams {
    /// Keep only the projects whose root still exists on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub existing_only: Option<bool>,
}

/// One registered project (multiproyecto-suscripciones design D4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// The stable key the daemon derives from the canonical root.
    pub project_key: String,
    /// Absolute repository root, as it was used.
    pub root: String,
    /// Whether that root still exists on this machine.
    pub exists: bool,
    pub first_seen_at: String,
    pub last_seen_at: String,
    /// Sessions recorded for this project (historical included).
    pub sessions_total: u32,
    /// Sessions of the project running right now (starting, active, or waiting
    /// on a permission). Zero is a fact, not an omission: a project with no live
    /// session is still listed (multiproyecto-suscripciones).
    #[serde(default)]
    pub active_sessions: u32,
    /// How many of them can be resumed.
    pub resumable_sessions: u32,
}

/// Result of `project/list`, most recently seen first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResult {
    pub projects: Vec<ProjectInfo>,
}

/// Params of `project/register`: point Meltemi at a directory explicitly,
/// before anything has ever run in it (lanzador-conversacional D6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegisterParams {
    /// The directory to register. It must exist and be a directory; the daemon
    /// canonicalizes it before storing, creates nothing inside it, and does not
    /// require it to hold `.meltemi/` — registering is aiming the tool at a
    /// folder, not initializing it as a project.
    pub root: String,
}

/// Result of `project/register`: the registered project in the very shape
/// `project/list` reports, so a surface can show the row without asking again.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegisterResult {
    /// The project as the registry now holds it, with its canonical root.
    pub project: ProjectInfo,
}

/// Params of `project/forget`: drop a project from the registry's listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectForgetParams {
    /// The project root to forget. It need not exist on disk — a root that
    /// vanished is precisely the one worth forgetting, so a registry that
    /// demanded a canonicalizable path would make it unforgettable.
    pub root: String,
}

/// Result of `project/forget`. Nothing on disk is deleted either way: the
/// project's sessions, its logs and its analytics are all still there, and it
/// reappears in the listing the moment it is used or registered again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectForgetResult {
    /// Whether the registry was listing that root and no longer does. `false`
    /// says it was not listed to begin with.
    pub forgotten: bool,
}

/// Params of `subscription/link`: link a named subscription of a catalog
/// agent whose registry entry declares its auth-context variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionLinkParams {
    /// The catalog id of the agent the subscription runs (e.g. a level-2
    /// entry). Free labels and profiles are not linkable targets.
    pub agent: String,
    /// The name of the link — it becomes the profile name and the context
    /// directory's name, so it must be a safe path component (kebab-case).
    pub name: String,
}

/// The composed authentication gesture: everything the human needs to log the
/// provider's own binary into the linked context. Meltemi composes it and
/// never runs it — the binary authenticates itself (§2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginGesture {
    /// The environment variable that redirects the provider's auth context.
    pub var: String,
    /// The absolute path of the linked context directory (the value).
    pub value: String,
    /// The provider's documented login gesture, as registry data.
    pub hint: String,
    /// The gesture as one POSIX shell line (`VAR=value <hint>`).
    pub posix: String,
    /// The gesture as PowerShell lines (`$env:VAR = "value"; <hint>`).
    pub powershell: String,
}

/// Result of `subscription/link`: the profile now listed by the catalog plus
/// the login gesture. The context directory was created empty; Meltemi never
/// reads what the provider stores in it afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionLinkResult {
    /// The profile name the resolver now honors.
    pub profile: String,
    /// The underlying catalog agent the profile launches.
    pub agent: String,
    /// The composed authentication gesture for the human to run.
    pub gesture: LoginGesture,
}

/// Params of `subscription/unlink`: undo a linked subscription by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUnlinkParams {
    /// The linked profile name to remove from the daemon-owned store.
    pub name: String,
}

/// Result of `subscription/unlink`. The auth-context directory is NEVER
/// deleted — whatever the provider stored there is not Meltemi's to destroy —
/// and the response names the path left behind so the human can decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUnlinkResult {
    /// The unlinked profile name.
    pub profile: String,
    /// The context directory left behind, untouched.
    pub context_dir: String,
}

/// Which layer of an entry a detection result describes: the provider's own
/// official CLI, or the ACP adapter Meltemi can actually pilot
/// (flota-deteccion-guia design D1/D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetLayerKind {
    /// The provider's official CLI, as the user installs it.
    Cli,
    /// The ACP adapter that puts a level-2 agent on the protocol.
    Adapter,
}

/// Where a detected layer's binary came from (adaptadores-propios-acp design
/// D8). Reported beside the absolute path, so a find is never anonymous: the
/// same binary name can exist in several places, and which one won decides what
/// a launch will execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetLayerSource {
    /// Found by name on the user's `PATH`.
    Path,
    /// Found at one of the candidate paths the registry declares.
    CandidatePath,
    /// Found beside the running daemon: the layer travels in Meltemi's own
    /// installers rather than being installed separately.
    Bundled,
}

/// One detected (or missing) layer of a fleet entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetLayer {
    pub kind: FleetLayerKind,
    /// The binary name the registry declares for this layer.
    pub bin: String,
    /// Whether the layer was found on this system.
    pub detected: bool,
    /// Absolute path of the found binary; present only when detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    /// The find is evidence of an installation but not a launchable target
    /// (a Windows `.ps1` shim): honest, never handed to a launch.
    #[serde(default)]
    pub evidence_only: bool,
    /// The install command the registry declares for this layer, shown to the
    /// user and never executed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
    /// Whether the registry declares this layer as travelling in Meltemi's own
    /// installers. Such a layer has no third-party install command: its remedy
    /// is to reinstall or repair Meltemi.
    #[serde(default)]
    pub bundled: bool,
    /// Where the find came from; present only when the layer was detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FleetLayerSource>,
}

/// The composed install state of an entry across its layers (design D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetInstallState {
    /// The pilot point is present (and the official CLI too, when declared).
    Ready,
    /// The official CLI is installed; the ACP adapter is missing.
    AdapterMissing,
    /// The adapter is installed; the official CLI is not.
    CliMissing,
    /// Nothing was found for this entry.
    NotDetected,
    /// Something is installed, but no launchable target exists.
    NotLaunchable,
}

/// How sanctioned the declared integration path is (design D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetLegalStatus {
    /// The provider documents and supports this path.
    Sanctioned,
    /// Not documented, not forbidden.
    Tolerated,
    /// Known restrictions apply: the note says which.
    Grey,
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
    /// For a profile row, the catalog id of the underlying agent it launches;
    /// absent on registry/custom rows (flota-multiproveedor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_agent: Option<String>,
    /// The entry's layers with their per-layer detection (design D3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<FleetLayer>,
    /// The composed install state across those layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_state: Option<FleetInstallState>,
    /// Which layer is missing and what to do about it, in one sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
    /// The exact command that installs the missing layer. Data, never run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy_command: Option<String>,
    /// How sanctioned this integration path is, when the registry declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_status: Option<FleetLegalStatus>,
    /// The short note behind that status, shown verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_note: Option<String>,
    /// The env variable that redirects this entry's authentication context,
    /// when the registry snapshot declares it. Its presence is what makes the
    /// entry a linkable subscription target (vincular-suscripciones D3); the
    /// variable NAME only — never a value, never a credential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_context_var: Option<String>,
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
    /// The fleet agent to deliberate with: a launch profile name or a catalog
    /// id, resolved in the fleet's existing order. Choosing one relaxes
    /// nothing: `sdd/explore` still writes no artifact and no project file,
    /// whichever agent runs (lanzador-conversacional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
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
    /// The fleet agent to draft the proposal: a launch profile name or a
    /// catalog id, resolved in the fleet's existing order. Absent behaves
    /// exactly as before this field existed — the project's configured agent —
    /// so no client that never sends it sees any change
    /// (lanzador-conversacional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
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

/// Params of `session/watch`: declare (or drop) interest in a session's live
/// event stream for this connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWatchParams {
    /// The session whose stream to watch.
    pub session_id: String,
    /// `true` to receive its events on this connection, `false` to stop.
    pub watch: bool,
}

/// Result of `session/watch`: what this connection now does with that session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWatchResult {
    /// The session echoed back.
    pub session_id: String,
    /// Whether this connection is now watching it.
    pub watching: bool,
}

/// Params of `session/start`: an instruction and the project to run it on.
/// No change, no task, no specification — the free session's whole point
/// (lanzador-conversacional D1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartParams {
    /// Absolute path to the root of the repository the session works in. The
    /// free session operates on that root, like every other human-attended
    /// path; it creates no worktree (design D2).
    pub project_root: String,
    /// The first instruction, verbatim: it becomes the session's first prompt.
    pub instruction: String,
    /// The fleet agent to run: a launch profile name or a catalog id, resolved
    /// in the fleet's existing order. Absent uses the project's configured
    /// agent, exactly as before this method existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// Why a free session got no restore point. The two causes take different
/// remedies and telling them apart is the whole point: `git init` on a
/// repository that already exists is not advice, it is noise (design D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointUnavailable {
    /// The project root is not a git repository at all.
    NotAGitRepo,
    /// It is a git repository with nothing committed yet: no history to
    /// snapshot, so there is nothing to come back to.
    NoHistory,
}

/// Result of `session/start`, final for the scriptable client that listens to
/// no notifications: the session's id, how its turn ended and how many
/// permissions were denied. A client that does listen already has the id long
/// before this — `session_started` reaches the connection that started the
/// session ahead of the agent's first token, which is what lets a surface
/// navigate into the conversation instead of waiting out the turn (design D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartResult {
    /// Meltemi session identifier.
    pub session_id: String,
    /// The official agent binary and arguments (program first).
    pub agent_command: Vec<String>,
    /// Final status of the agent turn.
    pub status: TurnStatus,
    /// How many of the agent's permission requests were denied during the turn
    /// (by rule, by the human, or by timeout/default). Greater than zero means
    /// the work may be incomplete (honesty of result).
    #[serde(default)]
    pub denied_permissions: u32,
    /// The restore point taken before the first turn, as a git ref. Absent
    /// means none was taken — and the two fields below say why and what to do,
    /// because promising a restore point and not creating one would be worse
    /// than not promising it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<String>,
    /// Why there is no restore point; present only when `checkpointRef` is
    /// absent. The session started regardless: this is never a refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_unavailable: Option<CheckpointUnavailable>,
    /// The English remedy matching that cause, for surfaces that show prose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_remedy: Option<String>,
}

/// Params of `session/direct`: an instruction aimed at an existing session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDirectParams {
    /// The session to direct (its Meltemi id).
    pub session_id: String,
    /// The instruction to enqueue or resume with, verbatim.
    pub instruction: String,
    /// The project the session belongs to (locates its log and, on resume,
    /// the repository the agent runs in). Absent means the daemon's default
    /// project resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
}

/// How `session/direct` handled the instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectDisposition {
    /// The session was active: the instruction was queued and will be dispatched
    /// as the next turn of the same agent session when the current turn ends —
    /// unless the session is cancelled first, in which case it is dropped
    /// undispatched (the log shows `instruction_queued` with no `prompt_sent`, so
    /// the loss is auditable, not silent). Observe the outcome via `session/log`.
    Queued,
    /// The session was terminated but resumable: it was resumed with the
    /// instruction as the prompt, as a new session linked to the original.
    Resumed,
}

/// Result of `session/direct`. A non-existent or non-resumable session is a
/// `SESSION_NOT_FOUND` error, not a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDirectResult {
    /// How the instruction was handled.
    pub disposition: DirectDisposition,
    /// The session the instruction landed on: the same id when queued, or the
    /// new resumed session's id.
    pub session_id: String,
    /// When resumed, the original session this one continues.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumed_from: Option<String>,
    /// 1-based position of the instruction in the queue, when queued.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<u32>,
    /// The final turn status, when the instruction ran a turn (on resume).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TurnStatus>,
    /// How many permission requests were denied during the resumed turn (0 when
    /// merely queued). Greater than zero means the result may be incomplete.
    #[serde(default)]
    pub denied_permissions: u32,
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
    /// Seconds until it expires; negative once expired (snapshot). Absent
    /// when the wait policy imposes no deadline (waiting for the human).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
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
        /// What the session is about. Travels in the event, not only in the
        /// index, so a client that opens a tab on this event can name it at
        /// once, and so the index can be rebuilt from the log alone
        /// (titulo-de-sesion design D3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
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
        /// Whether the decision DENIED the request. Recorded explicitly because
        /// the outcome alone cannot say: selecting a reject option and selecting
        /// an allow option have the same shape (analitica-consumo-local D2).
        /// Absent in logs written before this field existed — read as unknown,
        /// never as an approval.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        denied: Option<bool>,
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
    /// A session's agent was resolved from the fleet (flota-multiproveedor):
    /// the effective binary and how the name resolved. Carries the binary and
    /// source ONLY — never the profile's env values (fair play §2).
    AgentResolved {
        /// The effective binary (program) that will run.
        binary: String,
        /// How the name resolved.
        source: FleetResolutionSource,
        /// The profile name, when resolved via a launch profile.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
        /// The catalog id the name resolved to, when it named one — so a
        /// rebuild from the log recovers which agent ran
        /// (multiproyecto-suscripciones D5).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        /// The integration level of the resolved agent.
        level: u8,
    },
    /// A task's deployment turn began (comando-implement progress). Emitted
    /// before the agent runs in the task's worktree.
    TaskStarted {
        /// The change the task belongs to.
        change: String,
        /// The task label starting.
        task: String,
        /// The agent deployed on the task.
        agent: String,
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
    /// A directed instruction was accepted into a session's queue
    /// (session/direct). It is dispatched as a later `PromptSent` when the turn
    /// in progress concludes; recording it before dispatch makes remote steering
    /// auditable even if the daemon dies before the queue drains.
    InstructionQueued {
        /// The instruction text that will become the session's next prompt.
        instruction: String,
    },
    /// A human edit applied through the daemon (edit-surface traceability):
    /// the file relative to its tree, and the session active on the tree at
    /// apply time, when any.
    HumanEdit {
        /// The edited file, relative to its tree.
        file: String,
        /// The session active on the tree, when any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
    /// Usage counters an official level-3 structured output reported
    /// (analitica-consumo-local D3). Carries counters, their source and the
    /// model when the output names it — never credentials, headers, cookies or
    /// any account identifier (fair play §2). A counter the output does not
    /// declare stays ABSENT; it is never recorded as zero.
    UsageReported {
        /// Which official structured output the counters were read from.
        source: String,
        /// The model the output named, when it named one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        /// Tokens the output declared as input/prompt.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        /// Tokens the output declared as output/completion.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
        /// Input tokens the output declared as served from cache.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cached_input_tokens: Option<u64>,
        /// Reasoning tokens, when the output breaks them out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_tokens: Option<u64>,
        /// The total the output declared. Never computed here: a sum of
        /// heterogeneous partial counters would be an invented number.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total_tokens: Option<u64>,
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

/// One competitor's result in a race: its diff against the common base, plus
/// the provenance of the last turn dispatched over that lane
/// (tablero-de-carrera design D1). Every provenance field is additive and
/// omissible: a lane with no dispatch on record states nothing rather than
/// guessing, and omitting them all serializes exactly as before they existed.
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
    /// How the last dispatch over this lane resolved its binary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<FleetResolutionSource>,
    /// The launch profile — the subscription — that ran the lane: the NAME
    /// only, never its env overlay (§2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The integration level the lane's dispatch ran at (1-4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// The session that ran the lane's last dispatch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Whether the lane's branch carries a commit over its own base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed: Option<bool>,
    /// The lane's head commit SHA, when it committed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// The base revision THIS lane was created from. Lanes of one task
    /// normally share it; when they do not, each keeps its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_rev: Option<String>,
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

/// Params of `worktree/dispatch`. `agent` is a profile, catalog id, or free
/// label (resolved in that order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDispatchParams {
    pub project_root: String,
    pub change: String,
    pub task: String,
    pub agent: String,
}

/// How a dispatch resolved its competitor's binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchResolution {
    /// The effective binary (program).
    pub binary: String,
    /// How the name resolved.
    pub source: FleetResolutionSource,
    /// The integration level of the resolved agent.
    pub level: u8,
    /// The profile name, when resolved via a launch profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// Result of `worktree/dispatch`: one competitor's turn + traceability commit.
/// `task_ticked` is always `false` — a competitor does not own the task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeDispatchResult {
    pub change: String,
    pub task: String,
    pub agent: String,
    /// How the competitor's binary was resolved.
    pub resolution: DispatchResolution,
    /// The worktree the turn ran in.
    pub worktree: String,
    /// Whether the turn produced a commit.
    pub committed: bool,
    /// The commit SHA, when committed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// The files changed against the common base.
    pub changed_files: Vec<String>,
    /// The mapped ACP turn status.
    pub status: TurnStatus,
    /// Always `false`: a dispatch never marks the task.
    pub task_ticked: bool,
    /// The session the dispatch opened, so whoever dispatched can correlate the
    /// lane with the session that ran it (tablero-de-carrera design D2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Parameters of `worktree/apply-edit` (gui-tauri-paridad design D5): the
/// target tree is the project root, or a managed worktree when the
/// change/task/agent triple is given. `content` replaces the file whole; the
/// write is refused unless `confirm` acknowledges an active session or an
/// in-flight turn on the tree (soft lock, never a hard lock).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeApplyEditParams {
    /// Absolute path of the repository root.
    pub project_root: String,
    /// With `task` and `agent`, addresses a managed worktree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The file to write, relative to the target tree.
    pub file: String,
    /// The full new content of the file (UTF-8).
    pub content: String,
    /// Acknowledges the tree's activity when it is not free.
    #[serde(default)]
    pub confirm: bool,
}

/// The tree activity observed by `worktree/apply-edit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeEditState {
    /// No session runs on the tree: the edit applies without friction.
    Free,
    /// A session is registered on the tree but no turn is in flight:
    /// simple confirmation required.
    SessionActive,
    /// The agent is inside a turn on the tree: reinforced confirmation
    /// required.
    TurnInFlight,
}

/// Where the `human_edit` event was appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditLogDestination {
    /// The active session's JSONL log.
    Session,
    /// The project-scoped edits log (no session was active on the tree).
    Project,
}

/// Result of `worktree/apply-edit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeApplyEditResult {
    /// The file written, relative to its tree.
    pub file: String,
    /// Bytes written.
    pub bytes_written: u64,
    /// The activity observed at apply time.
    pub tree_state: TreeEditState,
    /// Where the `human_edit` event landed.
    pub logged_to: EditLogDestination,
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

/// Params of `sdd/implement`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddImplementParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// The change whose `tasks.md` to deploy.
    pub change: String,
    /// The agent to deploy on each task.
    pub agent: String,
    /// Plan only: return the eligible sequence without touching anything.
    #[serde(default)]
    pub plan_only: bool,
    /// Request autonomous mode (direct commits within permission rules). Without
    /// applicable rules the deployment degrades to supervised, with a notice.
    #[serde(default)]
    pub autonomous: bool,
}

/// One task's outcome in a deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementTask {
    /// The task id.
    pub id: String,
    /// The task description.
    pub description: String,
    /// `planned` (plan mode), `committed` (done this run), `already-done`
    /// (ticked before), or `pending` (not reached — e.g. after interruption).
    pub status: String,
    /// The commit SHA, when the task was committed this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

/// Result of `sdd/implement`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddImplementResult {
    /// The deployment mode actually run: `plan` or `act`.
    pub mode: String,
    /// Whether the run was autonomous (false when supervised or degraded).
    pub autonomous: bool,
    /// Why autonomy was degraded to supervised, when it was (e.g. no rules).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degraded: Option<String>,
    /// The tasks and their outcomes, in order.
    pub tasks: Vec<ImplementTask>,
    /// The ids committed this run.
    pub committed: Vec<String>,
}

/// Params of `change/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeListParams {
    /// Absolute path to the repository root.
    pub project_root: String,
    /// Maximum entries to return (active first, then archived).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Which of a change's artifacts are present on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeArtifacts {
    pub proposal: bool,
    pub design: bool,
    pub tasks: bool,
    pub specs: bool,
}

/// One change in the listing, with its aggregated state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeInfo {
    /// The change name (kebab-case).
    pub name: String,
    /// Whether it is archived history.
    pub archived: bool,
    /// The archive date, when archived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    /// Which artifacts are present.
    pub artifacts: ChangeArtifacts,
    /// Tasks ticked / total.
    pub tasks_done: u32,
    pub tasks_total: u32,
    /// Review items decided / total.
    pub review_decided: u32,
    pub review_total: u32,
    /// Verify scenarios verified / total.
    pub verified: u32,
    pub verify_total: u32,
    /// Whether an authoring gate awaits a human decision on this change.
    pub gate_pending: bool,
    /// The artifact the pending gate is about (`proposal`, `specs`, `design`,
    /// `tasks`, `constitution`); absent when no gate is pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_artifact: Option<String>,
}

/// Result of `change/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeListResult {
    /// Active changes first (name asc), then archived (most recent first).
    pub changes: Vec<ChangeInfo>,
}

/// Params of `change/show`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeShowParams {
    pub project_root: String,
    pub change: String,
}

/// One artifact of a change, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeArtifact {
    /// `proposal` | `design` | `tasks`.
    pub name: String,
    pub content: String,
}

/// One capability delta of a change, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeDelta {
    pub capability: String,
    pub content: String,
}

/// Result of `change/show`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeShowResult {
    pub name: String,
    pub artifacts: Vec<ChangeArtifact>,
    pub deltas: Vec<ChangeDelta>,
}

/// Params of `spec/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecListParams {
    pub project_root: String,
}

/// One living-truth capability with its counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecInfo {
    pub capability: String,
    pub requirements: u32,
    pub scenarios: u32,
}

/// Result of `spec/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecListResult {
    pub specs: Vec<SpecInfo>,
}

/// Params of `spec/show`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecShowParams {
    pub project_root: String,
    pub capability: String,
}

/// One EARS step of a scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecStep {
    /// The classified marker word (`when`, `then`, …).
    pub marker: String,
    pub text: String,
}

/// One scenario of a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecScenario {
    pub name: String,
    pub steps: Vec<SpecStep>,
}

/// One requirement of a living-truth capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecRequirement {
    pub name: String,
    pub description: String,
    pub scenarios: Vec<SpecScenario>,
}

/// Result of `spec/show`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecShowResult {
    pub capability: String,
    pub requirements: Vec<SpecRequirement>,
}

/// Params of `sdd/validate`. With no `change`, the whole living truth is
/// validated structurally.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddValidateParams {
    pub project_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateDiagnostic {
    pub capability: String,
    /// `file:line` anchor of the finding.
    pub location: String,
    pub message: String,
}

/// Result of `sdd/validate`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SddValidateResult {
    /// `change` or `living-truth`.
    pub scope: String,
    /// The change validated, when scope is `change`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Whether the validation is clean (no diagnostics).
    pub clean: bool,
    pub diagnostics: Vec<ValidateDiagnostic>,
}
