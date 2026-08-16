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

/// Every state the contract declares, in declaration order.
///
/// A list, because Rust cannot iterate an enum — which is exactly the problem
/// this constant exists to bound. Adding a variant without adding it here is
/// caught by [`the_state_enum_is_the_same_in_rust_and_in_both_schemas`], not by
/// the compiler.
const ALL_SESSION_STATES: [SessionState; 6] = [
    SessionState::Starting,
    SessionState::Active,
    SessionState::WaitingPermission,
    SessionState::WaitingInstruction,
    SessionState::Ended,
    SessionState::Interrupted,
];

/// The session-state enum is written THREE times — once in Rust and once in
/// each of the two schemas that define it independently — and until now nothing
/// compared them. A state present in Rust and missing from a schema is a
/// conformance failure that only shows up when a real daemon sends a real
/// session; a state in one schema and not the other is a contract that
/// contradicts itself.
#[test]
fn the_state_enum_is_the_same_in_rust_and_in_both_schemas() {
    let in_rust: Vec<String> = ALL_SESSION_STATES
        .iter()
        .map(|state| {
            serde_json::to_value(state)
                .expect("a state serializes")
                .as_str()
                .expect("as a string")
                .to_string()
        })
        .collect();

    for schema in ["status", "session-list"] {
        let text = std::fs::read_to_string(format!("../schemas/v1/{schema}.schema.json"))
            .unwrap_or_else(|e| panic!("read {schema}.schema.json: {e}"));
        let doc: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        let in_schema: Vec<String> = doc["$defs"]["sessionState"]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("{schema} declares sessionState"))
            .iter()
            .map(|v| v.as_str().expect("a string").to_string())
            .collect();
        assert_eq!(
            in_rust, in_schema,
            "`{schema}.schema.json` and the Rust enum disagree about session states"
        );
    }
}

#[test]
fn status_conforms() {
    let states = ALL_SESSION_STATES;
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
            agent: None,
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

/// The additive agent selector: naming one conforms, and omitting it produces
/// the very bytes a client sent before the field existed.
#[test]
fn propose_and_explore_take_an_optional_agent() {
    let named = ProposeParams {
        idea: "add dark mode to the settings page".into(),
        project_root: "C:\\repos\\fixture".into(),
        agent: Some("claude-code".into()),
    };
    assert_conforms("propose", "params", &named);
    let omitted = ProposeParams {
        agent: None,
        ..named.clone()
    };
    assert_conforms("propose", "params", &omitted);
    assert_eq!(
        serde_json::to_value(&omitted).unwrap(),
        json!({ "idea": "add dark mode to the settings page", "projectRoot": "C:\\repos\\fixture" }),
        "omitting the agent must serialize exactly as before the field existed"
    );

    let explore = SddExploreParams {
        project_root: "C:\\repos\\fixture".into(),
        topic: "how should the launcher work".into(),
        agent: Some("subscription-profile".into()),
    };
    assert_conforms("sdd", "exploreParams", &explore);
    assert_conforms(
        "sdd",
        "exploreParams",
        &SddExploreParams {
            agent: None,
            ..explore.clone()
        },
    );
    // An empty name is not a name: it would resolve to nothing in silence.
    assert_rejected(
        "propose",
        "params",
        &json!({ "idea": "i", "projectRoot": "/r", "agent": "" }),
    );
    assert_rejected(
        "sdd",
        "exploreParams",
        &json!({ "projectRoot": "/r", "agent": "" }),
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
        expires_in_seconds: Some(108),
        expired: false,
        suggested_rule: Some(rule.clone()),
    };
    assert_conforms("permission", "pendingPermission", &pending);
    // An expired entry (negative remaining) is still valid and shown.
    assert_conforms(
        "permission",
        "pendingPermission",
        &PendingPermission {
            expires_in_seconds: Some(-4),
            expired: true,
            suggested_rule: None,
            ..pending.clone()
        },
    );
    // A deadline-free entry (waiting for the human) omits the field.
    assert_conforms(
        "permission",
        "pendingPermission",
        &PendingPermission {
            expires_in_seconds: None,
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
                    auth_context_var: None,
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
                    auth_context_var: None,
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
                    auth_context_var: None,
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
                    auth_context_var: None,
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
        // The additive resolution fields (multiproyecto-suscripciones D4).
        agent_id: Some("opencode".into()),
        profile: Some("work".into()),
        // And what the session is about (titulo-de-sesion D3).
        title: Some("Corregir el login del portal".into()),
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
    // Three ways for the additive title, the discipline tablero-de-carrera
    // fixed in writing: present, omitted, and the omitted form byte-identical
    // to what a client that never heard of titles would send.
    let untitled = SessionInfo {
        title: None,
        ..info.clone()
    };
    assert_conforms("session-list", "sessionInfo", &untitled);
    let encoded = serde_json::to_value(&untitled).expect("untitled encodes");
    assert!(
        encoded.get("title").is_none(),
        "an absent title is omitted from the wire, never sent as null: {encoded}"
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
fn session_watch_conforms() {
    assert_conforms(
        "session-watch",
        "params",
        &SessionWatchParams {
            session_id: "sess-1".into(),
            watch: true,
        },
    );
    // Dropping interest travels the same shape.
    assert_conforms(
        "session-watch",
        "params",
        &SessionWatchParams {
            session_id: "sess-1".into(),
            watch: false,
        },
    );
    assert_conforms(
        "session-watch",
        "result",
        &SessionWatchResult {
            session_id: "sess-1".into(),
            watching: true,
        },
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
            interrupt: false,
            detach: false,
        },
    );
    // The additive flag, the three ways tablero-de-carrera fixed in writing:
    // present, omitted, and the omitted shape byte-identical to what callers
    // sent before it existed.
    let asking = SessionDirectParams {
        session_id: "sess-1".into(),
        instruction: "no sigas por ahi".into(),
        project_root: None,
        interrupt: true,
        detach: false,
    };
    assert_conforms("session-direct", "params", &asking);
    let plain = SessionDirectParams {
        interrupt: false,
        detach: false,
        ..asking.clone()
    };
    assert_conforms("session-direct", "params", &plain);
    assert!(
        !serde_json::to_string(&plain).unwrap().contains("interrupt"),
        "a caller that does not ask sends exactly what it sent before"
    );
    assert!(
        serde_json::to_string(&asking)
            .unwrap()
            .contains("\"interrupt\":true"),
        "and one that asks says so"
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
fn session_start_conforms() {
    assert_conforms(
        "session-start",
        "params",
        &SessionStartParams {
            project_root: "C:\\repos\\fixture".into(),
            instruction: "find out why the build is slow".into(),
            agent: Some("claude-code".into()),
            detach: false,
        },
    );
    // No agent named: the project's configured one, exactly as everywhere else.
    assert_conforms(
        "session-start",
        "params",
        &SessionStartParams {
            project_root: "/repos/fixture".into(),
            instruction: "find out why the build is slow".into(),
            agent: None,
            detach: false,
        },
    );

    // The additive flag, three ways. Present is valid; omitted is valid; and the
    // omitted shape is BYTE-IDENTICAL to what callers sent before the field
    // existed — the only one of the three that can actually break somebody, and
    // therefore the only one worth asserting on bytes.
    let staying = SessionStartParams {
        project_root: "/repos/fixture".into(),
        instruction: "keep me alive".into(),
        agent: None,
        detach: true,
    };
    assert_conforms("session-start", "params", &staying);
    assert_eq!(
        serde_json::to_value(&staying).expect("serializes")["detach"],
        serde_json::json!(true),
        "asking to stay travels"
    );
    let one_shot = SessionStartParams {
        detach: false,
        ..staying.clone()
    };
    assert_eq!(
        serde_json::to_string(&one_shot).expect("serializes"),
        r#"{"projectRoot":"/repos/fixture","instruction":"keep me alive"}"#,
        "not asking is not saying no: the field vanishes and the wire is what it always was"
    );
    // And a caller that never heard of the field still parses, with the
    // behaviour it had.
    let legacy: SessionStartParams =
        serde_json::from_str(r#"{"projectRoot":"/repos/fixture","instruction":"keep me alive"}"#)
            .expect("a pre-existing caller still parses");
    assert!(!legacy.detach);

    let started = SessionStartResult {
        session_id: "sess-1".into(),
        agent_command: vec!["mock-agent".into(), "--acp".into()],
        status: Some(TurnStatus::Completed),
        denied_permissions: 0,
        checkpoint_ref: Some("refs/meltemi/checkpoints/free/sess-1-mock".into()),
        checkpoint_unavailable: None,
        checkpoint_remedy: None,
    };
    assert_conforms("session-start", "result", &started);
    // Every turn outcome the contract knows, including a cancelled one.
    for status in [
        TurnStatus::Completed,
        TurnStatus::Cancelled,
        TurnStatus::Refused,
        TurnStatus::MaxTokens,
        TurnStatus::MaxTurnRequests,
    ] {
        assert_conforms(
            "session-start",
            "result",
            &SessionStartResult {
                status: Some(status),
                denied_permissions: 2,
                ..started.clone()
            },
        );
    }
    // No restore point: the session started anyway, and the result says which
    // of the two causes it was — the remedies are not interchangeable.
    assert_conforms(
        "session-start",
        "result",
        &SessionStartResult {
            checkpoint_ref: None,
            checkpoint_unavailable: Some(CheckpointUnavailable::NotAGitRepo),
            checkpoint_remedy: Some(
                "Run `git init` in this directory to get restore points.".into(),
            ),
            ..started.clone()
        },
    );
    assert_conforms(
        "session-start",
        "result",
        &SessionStartResult {
            checkpoint_ref: None,
            checkpoint_unavailable: Some(CheckpointUnavailable::NoHistory),
            checkpoint_remedy: Some(
                "Make the first commit in this repository to get restore points.".into(),
            ),
            ..started.clone()
        },
    );

    // An instruction is what a free session is made of; there is no default.
    assert_rejected(
        "session-start",
        "params",
        &json!({ "projectRoot": "C:\\repos\\fixture" }),
    );
    assert_rejected(
        "session-start",
        "params",
        &json!({ "projectRoot": "C:\\repos\\fixture", "instruction": "" }),
    );
    // Which binary ran is never ambiguous: an empty command would say nothing.
    assert_rejected(
        "session-start",
        "result",
        &json!({
            "sessionId": "sess-1",
            "agentCommand": [],
            "status": "completed",
            "deniedPermissions": 0
        }),
    );
    // The denial count is part of the honest result, not an optional extra.
    assert_rejected(
        "session-start",
        "result",
        &json!({ "sessionId": "sess-1", "agentCommand": ["mock-agent"], "status": "completed" }),
    );
    // A third cause would need a remedy of its own before it could exist.
    assert_rejected("session-start", "checkpointUnavailable", &json!("no_git"));
}

// The registry's own two verbs (lanzador-conversacional D6).
#[test]
fn project_registry_conforms() {
    assert_conforms(
        "project-registry",
        "projectRegisterParams",
        &ProjectRegisterParams {
            root: "C:\\repos\\fixture".into(),
        },
    );
    let project = ProjectInfo {
        project_key: "a1b2c3d4e5f60718".into(),
        root: "C:\\repos\\fixture".into(),
        exists: true,
        first_seen_at: TS.into(),
        last_seen_at: TS.into(),
        sessions_total: 0,
        active_sessions: 0,
        resumable_sessions: 0,
    };
    // A folder registered before anything ever ran in it: zeros all the way
    // down, and listed all the same.
    assert_conforms(
        "project-registry",
        "projectRegisterResult",
        &ProjectRegisterResult { project },
    );
    assert_conforms(
        "project-registry",
        "projectForgetParams",
        &ProjectForgetParams {
            root: "/repos/vanished".into(),
        },
    );
    assert_conforms(
        "project-registry",
        "projectForgetResult",
        &ProjectForgetResult { forgotten: true },
    );
    // Nothing was listed under that root: an answer, not a failure.
    assert_conforms(
        "project-registry",
        "projectForgetResult",
        &ProjectForgetResult { forgotten: false },
    );

    assert_rejected("project-registry", "projectRegisterParams", &json!({}));
    assert_rejected(
        "project-registry",
        "projectRegisterParams",
        &json!({ "root": "" }),
    );
    assert_rejected(
        "project-registry",
        "projectForgetParams",
        &json!({ "root": "" }),
    );
    assert_rejected("project-registry", "projectForgetResult", &json!({}));
}

/// `projectInfo` is defined twice — the registry file cannot `$ref` across
/// files, since the harness grafts one document's `$defs` onto a synthetic
/// root. Copies drift unless something watches them, so this does.
#[test]
fn the_two_project_info_definitions_are_the_same_definition() {
    assert_eq!(
        schema_doc("project-list")["$defs"]["projectInfo"],
        schema_doc("project-registry")["$defs"]["projectInfo"],
        "project-list and project-registry describe the same project differently"
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
            title: Some("Corregir el login del portal".into()),
        },
        // The same event without a title: what a dispatched lane writes, and
        // what every session recorded before titles existed looks like.
        SessionEventKind::SessionStarted {
            session_id: "sess-2".into(),
            agent_command: vec!["mock-agent".into()],
            project_root: "C:\\repos\\fixture".into(),
            title: None,
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
            denied: Some(false),
            rule: None,
        },
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "cancelled"}),
            decided_by: PermissionDecidedBy::DefaultDeny,
            denied: Some(true),
            rule: None,
        },
        // Selecting a REJECT option has the same shape as selecting an allow
        // one, which is exactly why the denial is recorded as a fact.
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "selected", "optionId": "reject"}),
            decided_by: PermissionDecidedBy::Client,
            denied: Some(true),
            rule: None,
        },
        // A log written before the field omits it: unknown, never an approval.
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "cancelled"}),
            decided_by: PermissionDecidedBy::Timeout,
            denied: None,
            rule: None,
        },
        // A rule-resolved decision carries the rule for provenance (audit).
        SessionEventKind::PermissionDecided {
            outcome: json!({"outcome": "selected", "optionId": "allow"}),
            decided_by: PermissionDecidedBy::Rule,
            denied: Some(false),
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
            agent_id: Some("claude-code".into()),
            level: 1,
        },
        // Usage counters an official structured output reported: only the
        // declared ones, and never any account identity.
        SessionEventKind::UsageReported {
            source: "usage".into(),
            model: Some("mock-1".into()),
            input_tokens: Some(120),
            output_tokens: Some(34),
            cached_input_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
        },
        SessionEventKind::UsageReported {
            source: "msg.info.total_token_usage".into(),
            model: None,
            input_tokens: Some(7),
            output_tokens: None,
            cached_input_tokens: Some(2),
            reasoning_tokens: Some(1),
            total_tokens: Some(10),
        },
        // The same agent under no profile: the id is still recorded.
        SessionEventKind::AgentResolved {
            binary: "native-agent".into(),
            source: FleetResolutionSource::Catalog,
            profile: None,
            agent_id: Some("claude-code".into()),
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

// Local usage accounting (analitica-consumo-local D5).
#[test]
fn analytics_usage_conforms() {
    assert_conforms("analytics", "params", &AnalyticsUsageParams::default());
    for grain in [
        UsageGranularity::Day,
        UsageGranularity::Week,
        UsageGranularity::Month,
        UsageGranularity::Total,
    ] {
        assert_conforms(
            "analytics",
            "params",
            &AnalyticsUsageParams {
                project_root: Some("C:\\repos\\fixture".into()),
                since: Some(TS.into()),
                until: Some("2026-08-11T12:00:00Z".into()),
                granularity: Some(grain),
                agent: Some("claude".into()),
                profile: Some("work".into()),
                limit: Some(500),
            },
        );
    }

    let activity = UsageActivity {
        sessions: 3,
        sessions_closed: 2,
        sessions_open: 1,
        active_seconds: 420,
        prompts: 9,
        turns_by_stop_reason: [("completed".to_string(), 2u32)].into_iter().collect(),
        permissions_requested: 4,
        permissions_approved: 3,
        permissions_denied: 1,
        permissions_expired: 0,
        human_edits: 2,
        commits: 1,
        checkpoints: 1,
        errors: 0,
    };
    let coverage = UsageCoverage {
        measured_sessions: 1,
        unreported_sessions: 2,
        reasons: vec![UsageUnreportedReason {
            kind: UsageUnreportedKind::ProtocolCarriesNoUsage,
            sessions: 2,
        }],
    };
    assert_conforms("analytics", "activity", &activity);
    assert_conforms("analytics", "coverage", &coverage);

    // Measured counters: a declared subset only — absence stays absent.
    let tokens = UsageTokens {
        input: Some(1200),
        output: Some(340),
        ..UsageTokens::default()
    };
    assert_conforms("analytics", "tokens", &tokens);
    // An all-absent token object is NOT a valid measurement: the cell omits it.
    assert_rejected("analytics", "tokens", &json!({}));
    // A counter is never negative, and never a string.
    assert_rejected("analytics", "tokens", &json!({ "input": -1 }));

    let cell = UsageCell {
        project_key: "a1b2c3d4e5f60718".into(),
        project_root: "C:\\repos\\fixture".into(),
        agent: "claude".into(),
        agent_id: Some("claude-code".into()),
        profile: Some("work".into()),
        level: Some(3),
        period: "2026-07".into(),
        activity: activity.clone(),
        tokens: Some(tokens.clone()),
        coverage: coverage.clone(),
    };
    assert_conforms("analytics", "cell", &cell);
    // A cell with nothing measured omits tokens entirely (never zeros).
    assert_conforms(
        "analytics",
        "cell",
        &UsageCell {
            tokens: None,
            agent_id: None,
            profile: None,
            level: None,
            ..cell.clone()
        },
    );

    assert_conforms(
        "analytics",
        "unattributed",
        &UsageUnattributed {
            project_key: "a1b2c3d4e5f60718".into(),
            project_root: "C:\\repos\\fixture".into(),
            period: "2026-07".into(),
            human_edits: 2,
        },
    );

    let result = AnalyticsUsageResult {
        cells: vec![cell],
        unattributed: vec![],
        totals: UsageTotals {
            cells: 1,
            activity,
            tokens: Some(tokens),
            coverage,
        },
        truncated: false,
        disclosure: vec![
            UsageDisclosure::ActivityFromLocalRecords,
            UsageDisclosure::TokensOnlyWhenOfficialOutputReports,
            UsageDisclosure::NoQuotaBalanceOrBilling,
            UsageDisclosure::NothingIsEstimated,
            UsageDisclosure::NothingLeavesThisMachine,
        ],
    };
    assert_conforms("analytics", "result", &result);
    // An empty answer is valid and honest: zero cells, no fabricated rows.
    assert_conforms(
        "analytics",
        "result",
        &AnalyticsUsageResult {
            cells: vec![],
            unattributed: vec![],
            totals: UsageTotals::default(),
            truncated: false,
            disclosure: vec![UsageDisclosure::NothingLeavesThisMachine],
        },
    );
    // The disclosure never travels empty: the numbers always carry it.
    assert_rejected(
        "analytics",
        "result",
        &json!({
            "cells": [],
            "totals": { "cells": 0, "activity": {}, "coverage": {} },
            "truncated": false,
            "disclosure": []
        }),
    );
}

// The project registry catalog (multiproyecto-suscripciones D4).
#[test]
fn project_list_conforms() {
    assert_conforms("project-list", "params", &ProjectListParams::default());
    assert_conforms(
        "project-list",
        "params",
        &ProjectListParams {
            existing_only: Some(true),
        },
    );
    let project = ProjectInfo {
        project_key: "a1b2c3d4e5f60718".into(),
        root: "C:\\repos\\fixture".into(),
        exists: true,
        first_seen_at: TS.into(),
        last_seen_at: TS.into(),
        sessions_total: 7,
        active_sessions: 1,
        resumable_sessions: 2,
    };
    assert_conforms("project-list", "projectInfo", &project);
    // No live session is a fact, not an omission: the project is still listed.
    assert_conforms(
        "project-list",
        "projectInfo",
        &ProjectInfo {
            active_sessions: 0,
            ..project.clone()
        },
    );
    // A vanished root is reported honestly, never dropped.
    assert_conforms(
        "project-list",
        "projectInfo",
        &ProjectInfo {
            exists: false,
            ..project.clone()
        },
    );
    assert_conforms(
        "project-list",
        "result",
        &ProjectListResult {
            projects: vec![project],
        },
    );
    assert_conforms(
        "project-list",
        "result",
        &ProjectListResult { projects: vec![] },
    );
    assert_rejected(
        "project-list",
        "projectInfo",
        &json!({ "projectKey": "", "root": "/r", "exists": true }),
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
            bundled: false,
            source: None,
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
            bundled: false,
            source: Some(FleetLayerSource::CandidatePath),
        },
    );
    // A layer that travels in Meltemi's own installers: found beside the
    // daemon, declared bundled, and carrying no install command at all
    // (adaptadores-propios-acp design D8).
    assert_conforms(
        "fleet",
        "fleetLayer",
        &FleetLayer {
            kind: FleetLayerKind::Adapter,
            bin: "meltemi-codex-acp".into(),
            detected: true,
            binary_path: Some("/opt/meltemi/meltemi-codex-acp".into()),
            evidence_only: false,
            install: None,
            bundled: true,
            source: Some(FleetLayerSource::Bundled),
        },
    );
    for source in [
        FleetLayerSource::Path,
        FleetLayerSource::CandidatePath,
        FleetLayerSource::Bundled,
    ] {
        assert_conforms("fleet", "fleetLayerSource", &source);
    }
    assert_rejected("fleet", "fleetLayerSource", &json!("somewhere"));
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
                source: None,
                profile: None,
                level: None,
                session_id: None,
                committed: None,
                sha: None,
                base_rev: None,
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
            session_id: Some("6f1f0a2e-2b0e-4a1c-9f4d-3b1c2d5e6f70".into()),
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

/// The per-lane provenance of a race (tablero-de-carrera design D1), checked
/// three ways for every field it adds: present and conforming, omitted and
/// conforming, and — the one that matters to a client built before this change
/// — omitted producing the very bytes the previous shape produced.
// Scenario: Los campos aditivos no rompen al cliente anterior
#[test]
fn a_race_lane_declares_its_provenance_without_breaking_the_previous_shape() {
    let bare = WorktreeCompetitorDiff {
        agent: "claude".into(),
        path: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-claude".into(),
        changed_files: vec!["src/a.rs".into()],
        diff: "diff --git a/src/a.rs b/src/a.rs\n".into(),
        source: None,
        profile: None,
        level: None,
        session_id: None,
        committed: None,
        sha: None,
        base_rev: None,
    };

    // Every field present: the lane states who ran it, under which
    // subscription, at which level, in which session, and how it ended.
    let full = WorktreeCompetitorDiff {
        source: Some(FleetResolutionSource::Profile),
        profile: Some("work".into()),
        level: Some(2),
        session_id: Some("6f1f0a2e-2b0e-4a1c-9f4d-3b1c2d5e6f70".into()),
        committed: Some(true),
        sha: Some("b".repeat(40)),
        base_rev: Some("a".repeat(40)),
        ..bare.clone()
    };
    assert_conforms("worktree", "competitorDiff", &full);

    // Every field omitted: still conforming, because none of them is required.
    assert_conforms("worktree", "competitorDiff", &bare);

    // And byte for byte what the shape produced before the fields existed.
    assert_eq!(
        serde_json::to_value(&bare).unwrap(),
        json!({
            "agent": "claude",
            "path": "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-claude",
            "changedFiles": ["src/a.rs"],
            "diff": "diff --git a/src/a.rs b/src/a.rs\n"
        }),
        "a lane with no dispatch on record serializes exactly as before"
    );

    // One field at a time, so a field that silently stopped being skipped
    // cannot hide behind the six that still are.
    for (name, lane) in [
        (
            "source",
            WorktreeCompetitorDiff {
                source: Some(FleetResolutionSource::Catalog),
                ..bare.clone()
            },
        ),
        (
            "profile",
            WorktreeCompetitorDiff {
                profile: Some("work".into()),
                ..bare.clone()
            },
        ),
        (
            "level",
            WorktreeCompetitorDiff {
                level: Some(1),
                ..bare.clone()
            },
        ),
        (
            "sessionId",
            WorktreeCompetitorDiff {
                session_id: Some("s-1".into()),
                ..bare.clone()
            },
        ),
        (
            "committed",
            WorktreeCompetitorDiff {
                committed: Some(false),
                ..bare.clone()
            },
        ),
        (
            "sha",
            WorktreeCompetitorDiff {
                sha: Some("c".repeat(40)),
                ..bare.clone()
            },
        ),
        (
            "baseRev",
            WorktreeCompetitorDiff {
                base_rev: Some("a".repeat(40)),
                ..bare.clone()
            },
        ),
    ] {
        assert_conforms("worktree", "competitorDiff", &lane);
        let value = serde_json::to_value(&lane).unwrap();
        assert!(
            value.get(name).is_some(),
            "`{name}` must reach the wire when it is known"
        );
        assert_eq!(
            value.as_object().unwrap().len(),
            5,
            "`{name}` alone is added; the other six stay omitted: {value}"
        );
    }

    // Degenerates are refused rather than shown as an empty provenance: an
    // empty profile or session id is not "unknown", it is a broken record.
    for degenerate in ["profile", "sessionId", "sha", "baseRev"] {
        let mut instance = serde_json::to_value(&bare).unwrap();
        instance[degenerate] = json!("");
        assert_rejected("worktree", "competitorDiff", &instance);
    }
    // The level is the integration level, so the range is the contract's.
    for out_of_range in [0, 5] {
        let mut instance = serde_json::to_value(&bare).unwrap();
        instance["level"] = json!(out_of_range);
        assert_rejected("worktree", "competitorDiff", &instance);
    }
}

/// The dispatch names the session it opened — the other half of the
/// correlation, for whoever dispatched rather than whoever polls the diff.
#[test]
fn a_dispatch_result_names_its_session_and_omits_it_compatibly() {
    let result = WorktreeDispatchResult {
        change: "add-thing".into(),
        task: "1.1".into(),
        agent: "work".into(),
        resolution: DispatchResolution {
            binary: "native-agent".into(),
            source: FleetResolutionSource::Catalog,
            level: 1,
            profile: None,
        },
        worktree: "C:\\repos\\fixture\\.meltemi\\worktrees\\add-thing\\1-1-work".into(),
        committed: false,
        sha: None,
        changed_files: Vec::new(),
        status: TurnStatus::Completed,
        task_ticked: false,
        session_id: None,
    };
    assert_conforms("worktree", "dispatchResult", &result);
    let before = serde_json::to_value(&result).unwrap();
    assert!(
        before.get("sessionId").is_none(),
        "omitted is omitted: {before}"
    );

    let named = WorktreeDispatchResult {
        session_id: Some("6f1f0a2e-2b0e-4a1c-9f4d-3b1c2d5e6f70".into()),
        ..result.clone()
    };
    assert_conforms("worktree", "dispatchResult", &named);
    let after = serde_json::to_value(&named).unwrap();
    assert_eq!(
        after.as_object().unwrap().len(),
        before.as_object().unwrap().len() + 1,
        "naming the session adds one key and disturbs nothing else"
    );

    // An empty session id names nothing and is refused as such.
    let mut degenerate = after.clone();
    degenerate["sessionId"] = json!("");
    assert_rejected("worktree", "dispatchResult", &degenerate);
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
        gate_pending: false,
        gate_artifact: None,
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
    // A change whose authoring gate awaits names the artifact it is about.
    assert_conforms(
        "change",
        "changeInfo",
        &ChangeInfo {
            gate_pending: true,
            gate_artifact: Some("specs".into()),
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
            candidates: None,
        },
    );
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "not_initialized".into(),
            detail: "Call initialize first.".into(),
            remedy: None,
            candidates: None,
        },
    );
}

/// A refusal to resolve an agent hands over the fleet instead of a sentence
/// (D7): 2000 and 2001 enriched, no new code.
#[test]
fn resolution_refusals_carry_the_fleet_candidates() {
    let states = [
        FleetInstallState::Ready,
        FleetInstallState::AdapterMissing,
        FleetInstallState::CliMissing,
        FleetInstallState::NotDetected,
        FleetInstallState::NotLaunchable,
    ];
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "agent_command_not_configured".into(),
            detail: "No agent is configured for this project.".into(),
            remedy: Some("Pick one of the detected agents, or set agent.id.".into()),
            candidates: Some(
                states
                    .into_iter()
                    .map(|install_state| AgentCandidate {
                        id: "claude-code".into(),
                        detected: install_state == FleetInstallState::Ready,
                        install_state,
                        remedy: Some("Install the official CLI.".into()),
                        remedy_command: Some("npm i -g @anthropic-ai/claude-code".into()),
                    })
                    .collect(),
            ),
        },
    );
    // A detected candidate needs no remedy at all.
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "agent_not_detected".into(),
            detail: "The agent `acme-agent` is not detected on this system.".into(),
            remedy: Some("Choose a detected agent.".into()),
            candidates: Some(vec![AgentCandidate {
                id: "mock-agent".into(),
                detected: true,
                install_state: FleetInstallState::Ready,
                remedy: None,
                remedy_command: None,
            }]),
        },
    );
    // The fleet was consulted and nothing was found: an answer, not an omission.
    assert_conforms(
        "error",
        "errorData",
        &ErrorData {
            kind: "agent_command_not_configured".into(),
            detail: "No agent is configured and none was detected.".into(),
            remedy: Some("Install one of the supported CLIs.".into()),
            candidates: Some(vec![]),
        },
    );

    // A candidate nobody can act on: no state to show, no choice to offer.
    assert_rejected(
        "error",
        "agentCandidate",
        &json!({ "id": "claude-code", "detected": false }),
    );
    assert_rejected(
        "error",
        "agentCandidate",
        &json!({ "id": "", "detected": true, "installState": "ready" }),
    );
    assert_rejected(
        "error",
        "agentCandidate",
        &json!({ "id": "x", "detected": true, "installState": "probably" }),
    );
}

/// The candidate's install state is the fleet's install state, spelled twice
/// because the harness resolves no cross-file `$ref`. A candidate whose states
/// drifted from `fleet/list` would make the error and the Fleet view disagree,
/// which is exactly what D7 forbids.
#[test]
fn candidate_install_states_are_the_fleets_install_states() {
    assert_eq!(
        schema_doc("fleet")["$defs"]["fleetInstallState"]["enum"],
        schema_doc("error")["$defs"]["agentCandidate"]["properties"]["installState"]["enum"],
        "the error's install states and the fleet's have diverged"
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
        error_codes::USAGE_QUERY_INVALID,
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

/// The catalog row's linkability datum (vincular-suscripciones D3): additive,
/// omissible, and byte-identical when omitted — the variable NAME only.
#[test]
fn a_fleet_row_may_declare_its_auth_context_variable() {
    let bare = FleetAgent {
        id: "provider-a".into(),
        display_name: "Provider A".into(),
        source: FleetAgentSource::Registry,
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
        auth_context_var: None,
    };
    let declared = FleetAgent {
        auth_context_var: Some("PROVIDER_CONTEXT_DIR".into()),
        ..bare.clone()
    };
    assert_conforms("fleet", "fleetAgent", &declared);
    assert_conforms("fleet", "fleetAgent", &bare);
    // Omitted serializes exactly as before the field existed.
    let value = serde_json::to_value(&bare).unwrap();
    assert!(
        value.get("authContextVar").is_none(),
        "omission must leave no key behind: {value:#}"
    );
    // An empty variable name is a broken record, not an unknown.
    let mut broken = serde_json::to_value(&declared).unwrap();
    broken["authContextVar"] = serde_json::json!("");
    assert_rejected("fleet", "fleetAgent", &broken);
}

/// The subscription link/unlink contract (vincular-suscripciones design D5):
/// the composed gesture travels whole, refusal shapes are the error contract's
/// concern, and the kebab name rule is the schema's own word.
#[test]
fn subscription_link_and_unlink_conform() {
    assert_conforms(
        "subscription",
        "linkParams",
        &SubscriptionLinkParams {
            agent: "provider-a".into(),
            name: "work".into(),
        },
    );
    let gesture = LoginGesture {
        var: "PROVIDER_CONTEXT_DIR".into(),
        value: r"C:\Users\u\AppData\Roaming\meltemi\data\subscriptions\work".into(),
        hint: "provider login".into(),
        posix:
            "PROVIDER_CONTEXT_DIR=/home/u/.local/share/meltemi/subscriptions/work provider login"
                .into(),
        powershell: r#"$env:PROVIDER_CONTEXT_DIR = "C:\...\work"; provider login"#.into(),
    };
    assert_conforms(
        "subscription",
        "linkResult",
        &SubscriptionLinkResult {
            profile: "work".into(),
            agent: "provider-a".into(),
            gesture,
        },
    );
    assert_conforms(
        "subscription",
        "unlinkParams",
        &SubscriptionUnlinkParams {
            name: "work".into(),
        },
    );
    assert_conforms(
        "subscription",
        "unlinkResult",
        &SubscriptionUnlinkResult {
            profile: "work".into(),
            context_dir: r"C:\Users\u\AppData\Roaming\meltemi\data\subscriptions\work".into(),
        },
    );

    // The name is a directory-safe kebab component, by schema: no separators,
    // no uppercase, no spaces — the daemon refuses them before any directory
    // exists, and the schema says so first.
    for bad in ["Work", "wo rk", "wo/rk", r"wo\rk", "..", "-work", "work-"] {
        assert_rejected(
            "subscription",
            "linkParams",
            &serde_json::json!({ "agent": "provider-a", "name": bad }),
        );
    }
    // A gesture with an empty variable is a broken record, not an unknown.
    assert_rejected(
        "subscription",
        "linkResult",
        &serde_json::json!({
            "profile": "work",
            "agent": "provider-a",
            "gesture": { "var": "", "value": "x", "hint": "h", "posix": "p", "powershell": "w" }
        }),
    );
}
