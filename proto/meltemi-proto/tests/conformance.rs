// SPDX-License-Identifier: Apache-2.0

//! Conformance suite: the serde types of `meltemi-proto` must serialize in
//! accordance with the JSON Schemas under `proto/schemas/v1/`, which are the
//! source of truth of the contract.

use meltemi_proto::*;
use serde_json::{Value, json};

fn schema_doc(file: &str) -> Value {
    let path = format!(
        "{}/../schemas/v1/{file}.schema.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read schema {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("schema {path} is not valid JSON: {e}"))
}

/// Builds a validator for one `$defs` entry of a schema file. The `$defs`
/// section is grafted onto a synthetic root so internal `#/$defs/...`
/// references keep resolving.
fn validator_for_def(file: &str, def: &str) -> jsonschema::Validator {
    let doc = schema_doc(file);
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{def}"),
        "$defs": doc["$defs"].clone(),
    });
    jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap_or_else(|e| panic!("schema {file}#/$defs/{def} does not compile: {e}"))
}

#[track_caller]
fn assert_conforms<T: serde::Serialize>(file: &str, def: &str, value: &T) {
    let instance = serde_json::to_value(value).expect("serialization failed");
    let validator = validator_for_def(file, def);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| format!("{} (at {})", e, e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "instance does not conform to {file}#/$defs/{def}:\n{instance:#}\nerrors:\n{}",
        errors.join("\n")
    );
}

#[track_caller]
fn assert_rejected(file: &str, def: &str, instance: &Value) {
    let validator = validator_for_def(file, def);
    assert!(
        validator.validate(instance).is_err(),
        "instance unexpectedly conforms to {file}#/$defs/{def}:\n{instance:#}"
    );
}

fn peer() -> PeerInfo {
    PeerInfo {
        name: "meltemi-devclient".into(),
        version: "0.1.0".into(),
    }
}

const TS: &str = "2026-07-11T12:00:00Z";

fn event(kind: SessionEventKind) -> SessionEvent {
    SessionEvent {
        v: SESSION_EVENT_VERSION,
        ts: TS.into(),
        kind,
    }
}

#[test]
fn initialize_conforms() {
    assert_conforms(
        "initialize",
        "params",
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: peer(),
        },
    );
    assert_conforms(
        "initialize",
        "result",
        &InitializeResult {
            protocol_version: PROTOCOL_VERSION,
            daemon: PeerInfo {
                name: "meltemid".into(),
                version: "0.1.0".into(),
            },
        },
    );
}

#[test]
fn status_conforms() {
    let states = [
        SessionState::Starting,
        SessionState::Active,
        SessionState::WaitingPermission,
        SessionState::Ended,
    ];
    assert_conforms(
        "status",
        "result",
        &StatusResult {
            daemon_version: "0.1.0".into(),
            uptime_seconds: 42,
            sessions: states
                .into_iter()
                .map(|state| SessionSummary {
                    session_id: "sess-1".into(),
                    agent_command: vec!["mock-agent".into(), "--flag".into()],
                    state,
                })
                .collect(),
        },
    );
    // Empty session list is valid too.
    assert_conforms(
        "status",
        "result",
        &StatusResult {
            daemon_version: "0.1.0".into(),
            uptime_seconds: 0,
            sessions: vec![],
        },
    );
}

#[test]
fn propose_conforms() {
    assert_conforms(
        "propose",
        "params",
        &ProposeParams {
            idea: "add dark mode to the settings page".into(),
            project_root: "C:\\repos\\fixture".into(),
        },
    );
    let statuses = [
        TurnStatus::Completed,
        TurnStatus::Cancelled,
        TurnStatus::Refused,
        TurnStatus::MaxTokens,
        TurnStatus::MaxTurnRequests,
    ];
    for status in statuses {
        assert_conforms(
            "propose",
            "result",
            &ProposeResult {
                change_name: "add-dark-mode".into(),
                proposal_path: "C:\\repos\\fixture\\.meltemi\\changes\\add-dark-mode\\proposal.md"
                    .into(),
                status,
                denied_permissions: 0,
            },
        );
    }
    // A turn that suffered denials declares the count (honesty of result).
    assert_conforms(
        "propose",
        "result",
        &ProposeResult {
            change_name: "add-dark-mode".into(),
            proposal_path: "/repo/.meltemi/changes/add-dark-mode/proposal.md".into(),
            status: TurnStatus::Completed,
            denied_permissions: 3,
        },
    );
}

#[test]
fn permission_rules_and_queue_conform() {
    let rule = PermissionRule {
        effect: PermissionRuleEffect::Allow,
        tool: Some("edit".into()),
        command_prefix: None,
        path_prefix: Some("/repo/src".into()),
        scope: PermissionRuleScope::Project,
    };
    assert_conforms("permission", "rule", &rule);
    // A bare deny rule (matches anything of its scope) is valid.
    assert_conforms(
        "permission",
        "rule",
        &PermissionRule {
            effect: PermissionRuleEffect::Deny,
            tool: None,
            command_prefix: None,
            path_prefix: None,
            scope: PermissionRuleScope::Global,
        },
    );

    let pending = PendingPermission {
        request_id: "req-1".into(),
        session_id: "sess-1".into(),
        tool: "write-proposal".into(),
        summary: "Write proposal.md".into(),
        options: vec![PermissionOption {
            option_id: "allow".into(),
            name: "Allow".into(),
            kind: PermissionOptionKind::AllowOnce,
        }],
        waiting_seconds: 12,
        expires_in_seconds: 108,
        expired: false,
        suggested_rule: Some(rule.clone()),
    };
    assert_conforms("permission", "pendingPermission", &pending);
    // An expired entry (negative remaining) is still valid and shown.
    assert_conforms(
        "permission",
        "pendingPermission",
        &PendingPermission {
            expires_in_seconds: -4,
            expired: true,
            suggested_rule: None,
            ..pending.clone()
        },
    );

    assert_conforms(
        "permission",
        "pendingResult",
        &PermissionPendingResult {
            pending: vec![pending.clone()],
        },
    );
    assert_conforms(
        "permission",
        "changedParams",
        &PermissionChangedParams {
            pending: vec![pending],
        },
    );

    assert_conforms(
        "permission",
        "decideParams",
        &PermissionDecideParams {
            request_id: "req-1".into(),
            option_id: Some("allow".into()),
            persist_rule: Some(rule),
        },
    );
    // A cancel (deny) carries no option id.
    assert_conforms(
        "permission",
        "decideParams",
        &PermissionDecideParams {
            request_id: "req-1".into(),
            option_id: None,
            persist_rule: None,
        },
    );
    for status in [
        PermissionDecideStatus::Applied,
        PermissionDecideStatus::AlreadyResolved,
    ] {
        assert_conforms(
            "permission",
            "decideResult",
            &PermissionDecideResult { status },
        );
    }
}

#[test]
fn fleet_conforms() {
    assert_conforms("fleet", "params", &FleetListParams { project_root: None });
    assert_conforms(
        "fleet",
        "params",
        &FleetListParams {
            project_root: Some("C:\\repos\\fixture".into()),
        },
    );
    assert_conforms(
        "fleet",
        "result",
        &FleetListResult {
            registry_version: "2026-07-09".into(),
            agents: vec![
                FleetAgent {
                    id: "native-agent".into(),
                    display_name: "Native Agent".into(),
                    source: FleetAgentSource::Registry,
                    integration_level: 1,
                    verified_level: None,
                    verified_at: None,
                    mcp_support: false,
                    detected: true,
                    binary_path: Some("C:\\bin\\native-agent.exe".into()),
                    configured: true,
                    underlying_agent: None,
                    layers: Vec::new(),
                    install_state: None,
                    remedy: None,
                    remedy_command: None,
                    legal_status: None,
                    legal_note: None,
                },
                FleetAgent {
                    id: "absent-agent".into(),
                    display_name: "Absent Agent".into(),
                    source: FleetAgentSource::Registry,
                    integration_level: 4,
                    verified_level: None,
                    verified_at: None,
                    mcp_support: false,
                    detected: false,
                    binary_path: None,
                    configured: false,
                    underlying_agent: None,
                    layers: Vec::new(),
                    install_state: None,
                    remedy: None,
                    remedy_command: None,
                    legal_status: None,
                    legal_note: None,
                },
                FleetAgent {
                    id: "my-agent".into(),
                    display_name: "My Agent".into(),
                    source: FleetAgentSource::Custom,
                    integration_level: 1,
                    verified_level: None,
                    verified_at: None,
                    mcp_support: false,
                    detected: false,
                    binary_path: None,
                    configured: false,
                    underlying_agent: None,
                    layers: Vec::new(),
                    install_state: None,
                    remedy: None,
                    remedy_command: None,
                    legal_status: None,
                    legal_note: None,
                },
                // A launch profile row: a catalog agent under a selected auth
                // context (flota-multiproveedor).
                FleetAgent {
                    id: "work".into(),
                    display_name: "work".into(),
                    source: FleetAgentSource::Profile,
                    integration_level: 1,
                    verified_level: None,
                    verified_at: None,
                    mcp_support: false,
                    detected: true,
                    binary_path: Some("C:\\bin\\native-agent.exe".into()),
                    configured: false,
                    underlying_agent: Some("native-agent".into()),
                    layers: Vec::new(),
                    install_state: None,
                    remedy: None,
                    remedy_command: None,
                    legal_status: None,
                    legal_note: None,
                },
            ],
        },
    );
    // An empty catalog (substituted registry) is valid too.
    assert_conforms(
        "fleet",
        "result",
        &FleetListResult {
            registry_version: "fixture-1".into(),
            agents: vec![],
        },
    );
}

#[test]
fn session_list_and_log_conform() {
    assert_conforms(
        "session-list",
        "params",
        &SessionListParams {
            project_root: Some("C:\\repos\\fixture".into()),
            state: Some(SessionState::Interrupted),
            limit: Some(50),
        },
    );
    assert_conforms("session-list", "params", &SessionListParams::default());

    let info = SessionInfo {
        session_id: "sess-1".into(),
        agent_command: vec!["mock-agent".into()],
        project_root: "C:\\repos\\fixture".into(),
        state: SessionState::Ended,
        level: 1,
        final_status: Some(TurnStatus::Completed),
        started_at: "2026-07-11T12:00:00Z".into(),
        ended_at: Some("2026-07-11T12:05:00Z".into()),
        resumable: true,
    };
    assert_conforms("session-list", "sessionInfo", &info);
    // An interrupted session has no end and is not resumable.
    assert_conforms(
        "session-list",
        "sessionInfo",
        &SessionInfo {
            state: SessionState::Interrupted,
            final_status: None,
            ended_at: None,
            resumable: false,
            ..info.clone()
        },
    );
    assert_conforms(
        "session-list",
        "result",
        &SessionListResult {
            sessions: vec![info],
        },
    );

    assert_conforms(
        "session-log",
        "params",
        &SessionLogParams {
            project_root: "C:\\repos\\fixture".into(),
            session_id: "sess-1".into(),
            offset: Some(20),
            limit: Some(100),
        },
    );
    assert_conforms(
        "session-log",
        "result",
        &SessionLogResult {
            session_id: "sess-1".into(),
            total: 42,
            offset: 20,
            lines: vec!["{\"v\":1}".into(), "{\"v\":1}".into()],
        },
    );
}

#[test]
fn repo_map_conforms() {
    assert_conforms(
        "repo-map",
        "params",
        &RepoMapParams {
            project_root: "C:\\repo".into(),
            depth: Some(2),
            limit: Some(500),
        },
    );
    assert_conforms(
        "repo-map",
        "result",
        &RepoMapResult {
            entries: vec![
                RepoEntry {
                    path: "src".into(),
                    is_dir: true,
                    size: 0,
                },
                RepoEntry {
                    path: "src/lib.rs".into(),
                    is_dir: false,
                    size: 1234,
                },
            ],
            truncated: true,
            omitted: 12,
        },
    );
}

#[test]
fn context_project_conforms() {
    assert_conforms(
        "context",
        "params",
        &ContextProjectParams {
            project_root: "C:\\repos\\fixture".into(),
        },
    );
    assert_conforms(
        "context",
        "result",
        &ContextProjectResult {
            targets: vec![
                ContextTarget {
                    path: "AGENTS.md".into(),
                    fingerprint: "a".repeat(64),
                    written: true,
                },
                ContextTarget {
                    path: "CLAUDE.md".into(),
                    fingerprint: "0".repeat(64),
                    written: false,
                },
            ],
        },
    );
    // A non-hex or wrong-length fingerprint is rejected.
    assert_rejected(
        "context",
        "target",
        &json!({ "path": "AGENTS.md", "fingerprint": "nothex", "written": true }),
    );
}

#[test]
fn session_cancel_conforms() {
    assert_conforms(
        "session-cancel",
        "params",
        &SessionCancelParams {
            session_id: "sess-1".into(),
        },
    );
}

#[test]
fn session_direct_conforms() {
    assert_conforms(
        "session-direct",
        "params",
        &SessionDirectParams {
            session_id: "sess-1".into(),
            instruction: "also add a dark theme".into(),
            project_root: Some("C:\\repos\\fixture".into()),
        },
    );
    // Queued: an active session accepted the instruction as its next turn.
    assert_conforms(
        "session-direct",
        "result",
        &SessionDirectResult {
            disposition: DirectDisposition::Queued,
            session_id: "sess-1".into(),
            resumed_from: None,
            queue_position: Some(1),
            status: None,
            denied_permissions: 0,
        },
    );
    // Resumed: a terminated resumable session was resumed with the instruction.
    assert_conforms(
        "session-direct",
        "result",
        &SessionDirectResult {
            disposition: DirectDisposition::Resumed,
            session_id: "sess-2".into(),
            resumed_from: Some("sess-1".into()),
            queue_position: None,
            status: Some(TurnStatus::Completed),
            denied_permissions: 2,
        },
    );
}

#[test]
fn permission_conforms() {
    let kinds = [
        PermissionOptionKind::AllowOnce,
        PermissionOptionKind::AllowAlways,
        PermissionOptionKind::RejectOnce,
        PermissionOptionKind::RejectAlways,
    ];
    assert_conforms(
        "permission",
        "requestParams",
        &PermissionRequestParams {
            session_id: "sess-1".into(),
            tool_call: json!({
                "toolCallId": "call-1",
                "title": "Write proposal.md",
                "kind": "edit",
                "rawInput": {"path": "proposal.md"},
            }),
            options: kinds
                .into_iter()
                .enumerate()
                .map(|(i, kind)| PermissionOption {
                    option_id: format!("opt-{i}"),
                    name: "Allow".into(),
                    kind,
                })
                .collect(),
        },
    );
    assert_conforms(
        "permission",
        "result",
        &PermissionRequestResult {
            outcome: PermissionOutcome::Selected {
                option_id: "opt-0".into(),
            },
        },
    );
    assert_conforms(
        "permission",
        "result",
        &PermissionRequestResult {
            outcome: PermissionOutcome::Cancelled,
        },
    );
    assert_conforms(
        "permission",
        "timeoutParams",
        &PermissionTimeoutParams {
            session_id: "sess-1".into(),
            tool_call_id: Some("call-1".into()),
        },
    );
    assert_conforms(
        "permission",
        "timeoutParams",
        &PermissionTimeoutParams {
            session_id: "sess-1".into(),
            tool_call_id: None,
        },
    );
}

#[test]
fn session_events_conform() {
    let events = [
        SessionEventKind::SessionStarted {
            session_id: "sess-1".into(),
            agent_command: vec!["mock-agent".into()],
            project_root: "C:\\repos\\fixture".into(),
        },
        SessionEventKind::PromptSent {
            text: "Complete the proposal".into(),
        },
        SessionEventKind::RefsExpanded {
            expansions: vec![RefExpansion {
                path: "src/lib.rs".into(),
                bytes: 512,
                not_found: false,
                truncated: false,
            }],
        },
        SessionEventKind::AgentUpdate {
            update: json!({
                "sessionUpdate": "agent_message_chunk",
                "content": {"type": "text", "text": "Working on it"},
            }),
        },
        SessionEventKind::PermissionRequested {
            request: json!({
                "sessionId": "sess-1",
                "toolCall": {"toolCallId": "call-1"},
                "options": [{"optionId": "opt-0", "name": "Allow", "kind": "allow_once"}],
            }),
        },
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "selected", "optionId": "opt-0"}),
            decided_by: PermissionDecidedBy::Client,
            rule: None,
        },
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "cancelled"}),
            decided_by: PermissionDecidedBy::DefaultDeny,
            rule: None,
        },
        // A rule-resolved decision carries the rule for provenance (audit).
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "selected", "optionId": "allow"}),
            decided_by: PermissionDecidedBy::Rule,
            rule: Some(PermissionRule {
                effect: PermissionRuleEffect::Allow,
                tool: Some("edit".into()),
                command_prefix: None,
                path_prefix: Some("/repo".into()),
                scope: PermissionRuleScope::Project,
            }),
        },
        SessionEventKind::McpInjected {
            servers: vec!["fs".into(), "search".into()],
        },
        SessionEventKind::McpNotDelivered {
            reason: "the agent does not announce MCP support".into(),
        },
        SessionEventKind::CheckpointCreated {
            git_ref: "refs/meltemi/checkpoints/add-thing/1-1-claude".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
        },
        SessionEventKind::CheckpointRestored {
            git_ref: "refs/meltemi/checkpoints/add-thing/1-1-claude".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
            irreversible: vec!["ran command: npm publish".into()],
        },
        SessionEventKind::AgentResolved {
            binary: "native-agent".into(),
            source: FleetResolutionSource::Profile,
            profile: Some("work".into()),
            level: 1,
        },
        SessionEventKind::TaskStarted {
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
        },
        SessionEventKind::TaskCommitted {
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
            sha: "a".repeat(40),
            requirements: vec!["worktree-orchestration/managed-worktrees".into()],
        },
        SessionEventKind::TurnCompleted {
            stop_reason: TurnStatus::Completed,
        },
        SessionEventKind::InstructionQueued {
            instruction: "also add a dark theme".into(),
        },
        SessionEventKind::HumanEdit {
            file: "src/lib.rs".into(),
            session_id: Some("sess-1".into()),
        },
        SessionEventKind::HumanEdit {
            file: ".meltemi/specs/x/spec.md".into(),
            session_id: None,
        },
        SessionEventKind::SessionCancelled {},
        SessionEventKind::SessionEnded {
            reason: "shutdown".into(),
        },
        SessionEventKind::Error {
            kind: "agent_spawn_failed".into(),
            detail: "No such file or directory".into(),
        },
    ];
    for kind in events {
        assert_conforms("session-event", "sessionEvent", &event(kind));
    }
    assert_conforms(
        "session-event",
        "notificationParams",
        &SessionEventParams {
            session_id: "sess-1".into(),
            event: event(SessionEventKind::PromptSent {
                text: "hello".into(),
            }),
        },
    );
}

// The additive two-layer detection fields (flota-deteccion-guia D3).
#[test]
fn fleet_layers_conform() {
    assert_conforms(
        "fleet",
        "fleetLayer",
        &FleetLayer {
            kind: FleetLayerKind::Adapter,
            bin: "codex-acp".into(),
            detected: false,
            binary_path: None,
            evidence_only: false,
            install: Some("cargo install codex-acp".into()),
        },
    );
    assert_conforms(
        "fleet",
        "fleetLayer",
        &FleetLayer {
            kind: FleetLayerKind::Cli,
            bin: "codex".into(),
            detected: true,
            binary_path: Some("C:/shims/codex.cmd".into()),
            evidence_only: true,
            install: None,
        },
    );
    // The composed state and the legal status are closed enumerations.
    for state in [
        FleetInstallState::Ready,
        FleetInstallState::AdapterMissing,
        FleetInstallState::CliMissing,
        FleetInstallState::NotDetected,
        FleetInstallState::NotLaunchable,
    ] {
        assert_conforms("fleet", "fleetInstallState", &state);
    }
    for status in [
        FleetLegalStatus::Sanctioned,
        FleetLegalStatus::Tolerated,
        FleetLegalStatus::Grey,
    ] {
        assert_conforms("fleet", "fleetLegalStatus", &status);
    }
    assert_rejected("fleet", "fleetInstallState", &json!("maybe"));
    assert_rejected("fleet", "fleetLayerKind", &json!("headless"));
    // A full agent row carrying the additive fields still conforms.
    assert_conforms(
        "fleet",
        "fleetAgent",
        &json!({
            "id": "codex-cli",
            "displayName": "Codex CLI",
            "source": "registry",
            "integrationLevel": 2,
            "mcpSupport": false,
            "detected": false,
            "configured": false,
            "layers": [
                { "kind": "cli", "bin": "codex", "detected": true },
                { "kind": "adapter", "bin": "codex-acp", "detected": false }
            ],
            "installState": "adapter_missing",
            "remedy": "the ACP adapter is missing",
            "remedyCommand": "cargo install codex-acp",
            "legalStatus": "tolerated",
            "legalNote": "the adapter wraps the official app-server mode"
        }),
    );
}

#[test]
fn worktree_conforms() {
    assert_conforms(
        "worktree",
        "assignParams",
        &WorktreeAssignParams {
            project_root: "C:\\repos\\fixture".into(),
            tasks: vec![
                WorktreeTask {
                    change: "add-thing".into(),
                    task: "1.1".into(),
                    agents: vec!["claude".into(), "gemini".into()],
                    files: vec!["src/a.rs".into()],
                },
                WorktreeTask {
                    change: "add-thing".into(),
                    task: "1.2".into(),
                    agents: vec!["claude".into()],
                    files: vec![],
                },
            ],
        },
    );

    let racer = Worktree {
        change: "add-thing".into(),
        task: "1.1".into(),
        agent: "claude".into(),
        path: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-claude".into(),
        branch: "meltemi/add-thing/1-1-claude".into(),
        base_rev: "a".repeat(40),
        competitor: true,
    };
    assert_conforms("worktree", "worktree", &racer);

    assert_conforms(
        "worktree",
        "assignResult",
        &WorktreeAssignResult {
            base_rev: "a".repeat(40),
            batches: vec![
                WorktreeBatch {
                    tasks: vec!["1.1".into()],
                    serialized_reason: None,
                },
                WorktreeBatch {
                    tasks: vec!["1.2".into()],
                    serialized_reason: Some("serialized: shares src/a.rs".into()),
                },
            ],
            worktrees: vec![racer.clone()],
        },
    );

    assert_conforms(
        "worktree",
        "listParams",
        &WorktreeListParams {
            project_root: "C:\\repos\\fixture".into(),
        },
    );
    assert_conforms(
        "worktree",
        "listResult",
        &WorktreeListResult {
            worktrees: vec![racer.clone()],
        },
    );
    assert_conforms("worktree", "listResult", &WorktreeListResult::default());

    assert_conforms(
        "worktree",
        "removeParams",
        &WorktreeRemoveParams {
            project_root: "C:\\repos\\fixture".into(),
            path: racer.path.clone(),
            force: true,
        },
    );
    assert_conforms(
        "worktree",
        "removeResult",
        &WorktreeRemoveResult { removed: true },
    );

    assert_conforms(
        "worktree",
        "diffParams",
        &WorktreeDiffParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
        },
    );
    assert_conforms(
        "worktree",
        "diffResult",
        &WorktreeDiffResult {
            base_rev: "a".repeat(40),
            competitors: vec![WorktreeCompetitorDiff {
                agent: "claude".into(),
                path: racer.path.clone(),
                changed_files: vec!["src/a.rs".into()],
                diff: "diff --git a/src/a.rs b/src/a.rs\n".into(),
            }],
        },
    );

    assert_conforms(
        "worktree",
        "mergeFileParams",
        &WorktreeMergeFileParams {
            project_root: "C:\\repos\\fixture".into(),
            target: racer.path.clone(),
            source: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-gemini".into(),
            file: "src/a.rs".into(),
            confirm: true,
        },
    );
    assert_conforms(
        "worktree",
        "mergeFileResult",
        &WorktreeMergeFileResult { applied: true },
    );

    // A task with no agents is rejected (nobody to assign).
    assert_rejected(
        "worktree",
        "task",
        &json!({ "change": "c", "task": "1.1", "agents": [] }),
    );

    // Dispatch: the race primitive (flota-multiproveedor).
    assert_conforms(
        "worktree",
        "dispatchParams",
        &WorktreeDispatchParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "work".into(),
        },
    );
    assert_conforms(
        "worktree",
        "dispatchResult",
        &WorktreeDispatchResult {
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "work".into(),
            resolution: DispatchResolution {
                binary: "native-agent".into(),
                source: FleetResolutionSource::Profile,
                level: 1,
                profile: Some("work".into()),
            },
            worktree: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-work".into(),
            committed: true,
            sha: Some("a".repeat(40)),
            changed_files: vec!["toggle.rs".into()],
            status: TurnStatus::Completed,
            task_ticked: false,
        },
    );
    // task_ticked MUST be false — a dispatch never marks the task.
    assert_rejected(
        "worktree",
        "dispatchResult",
        &json!({
            "change": "c", "task": "1.1", "agent": "a",
            "resolution": { "binary": "b", "source": "catalog", "level": 1 },
            "worktree": "/w", "committed": false, "changedFiles": [],
            "status": "completed", "taskTicked": true
        }),
    );

    // Apply-edit: the traceable human edit (gui-tauri-paridad D5).
    assert_conforms(
        "worktree",
        "applyEditParams",
        &WorktreeApplyEditParams {
            project_root: "C:\\repos\\fixture".into(),
            change: Some("add-thing".into()),
            task: Some("1.1".into()),
            agent: Some("work".into()),
            file: "src/lib.rs".into(),
            content: "pub fn edited() {}\n".into(),
            confirm: true,
        },
    );
    assert_conforms(
        "worktree",
        "applyEditParams",
        &WorktreeApplyEditParams {
            project_root: "/repo".into(),
            change: None,
            task: None,
            agent: None,
            file: ".meltemi/specs/x/spec.md".into(),
            content: String::new(),
            confirm: false,
        },
    );
    assert_conforms(
        "worktree",
        "applyEditResult",
        &WorktreeApplyEditResult {
            file: "src/lib.rs".into(),
            bytes_written: 19,
            tree_state: TreeEditState::TurnInFlight,
            logged_to: EditLogDestination::Session,
        },
    );
    // An empty file path is refused by the schema.
    assert_rejected(
        "worktree",
        "applyEditParams",
        &json!({ "projectRoot": "/repo", "file": "", "content": "x" }),
    );
}

#[test]
fn checkpoint_conforms() {
    let cp = Checkpoint {
        change: "add-thing".into(),
        task: "1.1".into(),
        agent: "claude".into(),
        git_ref: "refs/meltemi/checkpoints/add-thing/1-1-claude".into(),
        worktree: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-claude".into(),
        created_at: TS.into(),
        irreversible: vec!["ran command: npm publish".into()],
    };
    assert_conforms("checkpoint", "checkpoint", &cp);

    assert_conforms(
        "checkpoint",
        "createParams",
        &CheckpointCreateParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
        },
    );
    assert_conforms(
        "checkpoint",
        "createResult",
        &CheckpointCreateResult {
            checkpoint: cp.clone(),
        },
    );

    assert_conforms(
        "checkpoint",
        "listParams",
        &CheckpointListParams {
            project_root: "C:\\repos\\fixture".into(),
            change: Some("add-thing".into()),
        },
    );
    assert_conforms(
        "checkpoint",
        "listResult",
        &CheckpointListResult {
            checkpoints: vec![cp.clone()],
        },
    );
    assert_conforms("checkpoint", "listResult", &CheckpointListResult::default());

    assert_conforms(
        "checkpoint",
        "revertParams",
        &CheckpointRevertParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
            confirm: true,
        },
    );
    // A clean reversion: complete, no irreversibles.
    assert_conforms(
        "checkpoint",
        "revertResult",
        &CheckpointRevertResult {
            reverted: true,
            scope: RevertScope {
                worktree_restored: true,
                complete: true,
                irreversible: vec![],
            },
        },
    );
    // A reversion with an irreversible operation is never complete.
    assert_conforms(
        "checkpoint",
        "revertResult",
        &CheckpointRevertResult {
            reverted: true,
            scope: RevertScope {
                worktree_restored: true,
                complete: false,
                irreversible: vec!["ran command: npm publish".into()],
            },
        },
    );

    assert_conforms(
        "checkpoint",
        "recordOpParams",
        &CheckpointRecordOpParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "1.1".into(),
            agent: "claude".into(),
            operation: "ran command: npm publish".into(),
        },
    );
    assert_conforms(
        "checkpoint",
        "recordOpResult",
        &CheckpointRecordOpResult { recorded: true },
    );

    // A ref outside the technical namespace is rejected (never a user branch).
    assert_rejected(
        "checkpoint",
        "checkpoint",
        &json!({
            "change": "c", "task": "1.1", "agent": "claude",
            "gitRef": "refs/heads/main", "worktree": "/w", "createdAt": TS,
        }),
    );
}

#[test]
fn commit_task_conforms() {
    assert_conforms(
        "commit",
        "params",
        &CommitTaskParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "2.1".into(),
            agent: "claude".into(),
            title: "Add the thing".into(),
            body: Some("Implements the thing because the spec asks for it.".into()),
            requirements: vec![TaskRequirement {
                capability: "git-per-task".into(),
                requirement: "Commit atómico por tarea completada".into(),
            }],
            declared_files: vec!["src/thing.rs".into()],
            confirm: true,
        },
    );
    // A preview with no body and no requirements is valid too.
    assert_conforms(
        "commit",
        "params",
        &CommitTaskParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            task: "2.1".into(),
            agent: "claude".into(),
            title: "Add the thing".into(),
            body: None,
            requirements: vec![],
            declared_files: vec![],
            confirm: false,
        },
    );
    // An applied commit carries its sha.
    assert_conforms(
        "commit",
        "result",
        &CommitTaskResult {
            committed: true,
            message: "Add the thing\n\n(add-thing 2.1)\n\nMeltemi-Task: add-thing/2.1".into(),
            sha: Some("a".repeat(40)),
            changed_files: vec!["src/thing.rs".into()],
            deviations: vec![],
            tree_clean: true,
        },
    );
    // A preview declares no sha and reports predicted deviations.
    assert_conforms(
        "commit",
        "result",
        &CommitTaskResult {
            committed: false,
            message: "Add the thing\n\n(add-thing 2.1)\n\nMeltemi-Task: add-thing/2.1".into(),
            sha: None,
            changed_files: vec!["src/thing.rs".into(), "src/other.rs".into()],
            deviations: vec!["src/other.rs".into()],
            tree_clean: false,
        },
    );
}

#[test]
fn verify_archive_conforms() {
    assert_conforms(
        "verify-archive",
        "verifyParams",
        &SddVerifyParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
        },
    );
    assert_conforms(
        "verify-archive",
        "verifyResult",
        &SddVerifyResult {
            scenarios: vec![
                VerifyScenario {
                    capability: "verify-archive".into(),
                    requirement: "Archivado con fusión atómica".into(),
                    scenario: "Fusión total o nada".into(),
                    status: "linked".into(),
                    note: None,
                },
                VerifyScenario {
                    capability: "verify-archive".into(),
                    requirement: "Archivado con fusión atómica".into(),
                    scenario: "Histórico y proyección".into(),
                    status: "manual".into(),
                    note: Some("verified by hand on staging".into()),
                },
            ],
            verified: 2,
            total: 2,
            complete: true,
        },
    );
    assert_conforms(
        "verify-archive",
        "verifyResult",
        &SddVerifyResult::default(),
    );

    assert_conforms(
        "verify-archive",
        "verifyMarkParams",
        &SddVerifyMarkParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            scenario: "Fusión total o nada".into(),
            note: "checked manually".into(),
        },
    );
    assert_conforms(
        "verify-archive",
        "verifyMarkResult",
        &SddVerifyMarkResult { marked: true },
    );

    assert_conforms(
        "verify-archive",
        "archiveParams",
        &SddArchiveParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            confirm: true,
            exceptions: vec![ArchiveException {
                scenario: "Some edge case".into(),
                justification: "covered by manual QA, test pending".into(),
            }],
        },
    );
    assert_conforms(
        "verify-archive",
        "archiveResult",
        &SddArchiveResult {
            capabilities: vec!["verify-archive".into()],
            archived_to: "C:\\repos\\fixture\\.meltemi\\changes\\archive\\2026-07-18-add-thing"
                .into(),
            projection_regenerated: true,
            excepted: vec!["Some edge case".into()],
        },
    );

    // An unknown verification status is rejected.
    assert_rejected(
        "verify-archive",
        "verifyScenario",
        &json!({ "capability": "c", "requirement": "r", "scenario": "s", "status": "maybe" }),
    );
}

#[test]
fn implement_conforms() {
    assert_conforms(
        "implement",
        "params",
        &SddImplementParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "add-thing".into(),
            agent: "claude".into(),
            plan_only: false,
            autonomous: true,
        },
    );
    assert_conforms(
        "implement",
        "result",
        &SddImplementResult {
            mode: "act".into(),
            autonomous: false,
            degraded: Some("no permission rules apply; running supervised".into()),
            tasks: vec![
                ImplementTask {
                    id: "1.1".into(),
                    description: "First task".into(),
                    status: "committed".into(),
                    sha: Some("a".repeat(40)),
                },
                ImplementTask {
                    id: "1.2".into(),
                    description: "Second".into(),
                    status: "already-done".into(),
                    sha: None,
                },
            ],
            committed: vec!["1.1".into()],
        },
    );

    // An unknown task status is rejected.
    assert_rejected(
        "implement",
        "task",
        &json!({ "id": "1.1", "description": "x", "status": "exploded" }),
    );
}

#[test]
fn navigation_conforms() {
    assert_conforms(
        "change",
        "listParams",
        &ChangeListParams {
            project_root: "C:\\repos\\fixture".into(),
            limit: Some(50),
        },
    );
    let info = ChangeInfo {
        name: "flota-multiproveedor".into(),
        archived: false,
        archived_at: None,
        artifacts: ChangeArtifacts {
            proposal: true,
            design: true,
            tasks: true,
            specs: true,
        },
        tasks_done: 3,
        tasks_total: 11,
        review_decided: 0,
        review_total: 4,
        verified: 0,
        verify_total: 8,
    };
    assert_conforms("change", "changeInfo", &info);
    // An archived change carries its date.
    assert_conforms(
        "change",
        "changeInfo",
        &ChangeInfo {
            archived: true,
            archived_at: Some("2026-07-18".into()),
            ..info.clone()
        },
    );
    assert_conforms(
        "change",
        "listResult",
        &ChangeListResult {
            changes: vec![info],
        },
    );
    assert_conforms("change", "listResult", &ChangeListResult::default());

    assert_conforms(
        "change",
        "showParams",
        &ChangeShowParams {
            project_root: "C:\\repos\\fixture".into(),
            change: "flota-multiproveedor".into(),
        },
    );
    assert_conforms(
        "change",
        "showResult",
        &ChangeShowResult {
            name: "flota-multiproveedor".into(),
            artifacts: vec![ChangeArtifact {
                name: "proposal".into(),
                content: "## Why\n...".into(),
            }],
            deltas: vec![ChangeDelta {
                capability: "fleet-catalog".into(),
                content: "## ADDED Requirements\n...".into(),
            }],
        },
    );

    assert_conforms(
        "spec",
        "listResult",
        &SpecListResult {
            specs: vec![SpecInfo {
                capability: "fleet-catalog".into(),
                requirements: 5,
                scenarios: 12,
            }],
        },
    );
    assert_conforms(
        "spec",
        "showResult",
        &SpecShowResult {
            capability: "fleet-catalog".into(),
            requirements: vec![SpecRequirement {
                name: "Resolución de agente por sesión".into(),
                description: "El daemon SHALL...".into(),
                scenarios: vec![SpecScenario {
                    name: "Sesión lanza el binario de su id".into(),
                    steps: vec![SpecStep {
                        marker: "when".into(),
                        text: "una sesión se lanza".into(),
                    }],
                }],
            }],
        },
    );
    // An unknown step marker is rejected.
    assert_rejected(
        "spec",
        "specStep",
        &json!({ "marker": "unless", "text": "x" }),
    );

    assert_conforms(
        "validate",
        "params",
        &SddValidateParams {
            project_root: "C:\\repos\\fixture".into(),
            change: Some("flota-multiproveedor".into()),
        },
    );
    assert_conforms(
        "validate",
        "result",
        &SddValidateResult {
            scope: "change".into(),
            target: Some("flota-multiproveedor".into()),
            clean: false,
            diagnostics: vec![ValidateDiagnostic {
                capability: "fleet-catalog".into(),
                location: "spec.md:12".into(),
                message: "modified requirement `X` does not exist".into(),
            }],
        },
    );
    assert_conforms(
        "validate",
        "result",
        &SddValidateResult {
            scope: "living-truth".into(),
            target: None,
            clean: true,
            diagnostics: vec![],
        },
    );
    // An unknown scope is rejected.
    assert_rejected(
        "validate",
        "result",
        &json!({ "scope": "everything", "clean": true, "diagnostics": [] }),
    );
}

#[test]
fn error_data_conforms() {
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "agent_not_detected".into(),
            detail: "The agent `acme-agent` is not detected on this system.".into(),
            remedy: Some("Set agent.command in your meltemi config.".into()),
        },
    );
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "not_initialized".into(),
            detail: "Call initialize first.".into(),
            remedy: None,
        },
    );
}

#[test]
fn error_codes_match_catalog() {
    let doc = schema_doc("error");
    let catalog: Vec<i64> = doc["$defs"]["errorCode"]["enum"]
        .as_array()
        .expect("errorCode enum")
        .iter()
        .map(|v| v.as_i64().expect("integer code"))
        .collect();
    let constants = [
        error_codes::PROTOCOL_VERSION_UNSUPPORTED,
        error_codes::NOT_INITIALIZED,
        error_codes::SHUTTING_DOWN,
        error_codes::AGENT_COMMAND_NOT_CONFIGURED,
        error_codes::AGENT_NOT_DETECTED,
        error_codes::AGENT_SPAWN_FAILED,
        error_codes::AGENT_HANDSHAKE_FAILED,
        error_codes::SESSION_NOT_FOUND,
        error_codes::CHANGE_ALREADY_EXISTS,
        error_codes::INVALID_IDEA,
        error_codes::PROJECT_ROOT_INVALID,
        error_codes::WORKTREE_UNAVAILABLE,
        error_codes::WORKTREE_REFUSED,
        error_codes::CHECKPOINT_NOT_FOUND,
        error_codes::GIT_COMMIT_FAILED,
        error_codes::VERIFY_INCOMPLETE,
        error_codes::SPEC_MERGE_CONFLICT,
        error_codes::ARTIFACT_NOT_FOUND,
    ];
    let mut sorted = constants.to_vec();
    sorted.sort_unstable();
    assert_eq!(
        sorted, catalog,
        "error code constants and schema catalog diverged"
    );
}

/// Negative controls: prove the harness actually rejects non-conforming data.
#[test]
fn harness_rejects_invalid_instances() {
    // Missing required field.
    assert_rejected("initialize", "params", &json!({"protocolVersion": 1}));
    // Wrong name style for the change (must be kebab-case).
    assert_rejected(
        "propose",
        "result",
        &json!({
            "changeName": "Add Dark Mode",
            "proposalPath": "x",
            "status": "completed",
        }),
    );
    // Unknown turn status.
    assert_rejected(
        "propose",
        "result",
        &json!({
            "changeName": "add-dark-mode",
            "proposalPath": "x",
            "status": "exploded",
        }),
    );
    // Bad timestamp format (formats are asserted, not annotated).
    assert_rejected(
        "session-event",
        "sessionEvent",
        &json!({
            "v": 1,
            "ts": "yesterday at noon",
            "type": "prompt_sent",
            "payload": {"text": "hi"},
        }),
    );
    // Unknown session event type.
    assert_rejected(
        "session-event",
        "sessionEvent",
        &json!({
            "v": 1,
            "ts": TS,
            "type": "coffee_break",
            "payload": {},
        }),
    );
    // Permission outcome must carry optionId when selected.
    assert_rejected(
        "permission",
        "result",
        &json!({"outcome": {"outcome": "selected"}}),
    );
    // Integration levels live in 1..=4 (declared scale).
    assert_rejected(
        "fleet",
        "fleetAgent",
        &json!({
            "id": "x",
            "displayName": "X",
            "source": "registry",
            "integrationLevel": 5,
            "detected": false,
            "configured": false,
        }),
    );
    // Unknown catalog source.
    assert_rejected(
        "fleet",
        "fleetAgent",
        &json!({
            "id": "x",
            "displayName": "X",
            "source": "marketplace",
            "integrationLevel": 1,
            "detected": false,
            "configured": false,
        }),
    );
    // The registry version is mandatory.
    assert_rejected("fleet", "result", &json!({ "agents": [] }));
}
