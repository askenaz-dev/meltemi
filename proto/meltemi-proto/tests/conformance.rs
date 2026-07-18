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
        SessionEventKind::TurnCompleted {
            stop_reason: TurnStatus::Completed,
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
