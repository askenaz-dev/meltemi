// SPDX-License-Identifier: Apache-2.0

//! Conformance of the server dialect against the provider's own dumped schema
//! (adaptadores-propios-acp task 2.1, design D6).
//!
//! The official CLI dumps the schema of the wire it speaks, per version. That
//! dump is vendored verbatim under
//! `core/mock-provider/schemas/codex-app-server/` and it is the authority here:
//! this suite validates, field by field, that
//!
//! 1. every message the adapter **sends** conforms to it;
//! 2. every line the scripted fixture **emits** conforms to it — so the mock
//!    wire cannot drift away from the real contract while every test stays
//!    green, which is exactly how a fixture stops being evidence;
//! 3. the adapter's types **parse** what that wire says;
//! 4. a divergence fails naming the field, not with "invalid".
//!
//! No provider binary runs here, and none needs to: that is the whole point of
//! vendoring the dump (constitution §5, design D10).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use meltemi_adapters::codex::wire::{
    self, ApprovalDecision, ApprovalResponse, ClientInfo, ErrorNotification, InitializeParams,
    InitializeResult, ItemNotification, ItemStatus, ThreadItem, ThreadStartParams,
    ThreadStartResult, TurnInterruptParams, TurnNotification, TurnStartParams, TurnStartResult,
    TurnStatus, UserInput,
};

/// Where the vendored dump lives (see its `PROVENANCE.md`).
fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("mock-provider")
        .join("schemas")
        .join("codex-app-server")
}

/// The scripted wire whose every line must conform to the same dump.
fn transcript() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("mock-provider")
        .join("scripts")
        .join("codex-app-server.ndjson");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn schema_doc(file: &str) -> Value {
    let path = schema_dir().join(file);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read the vendored schema {}: {e}. Re-anchor the dump as PROVENANCE.md says.",
            path.display()
        )
    });
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{file} is not valid JSON: {e}"))
}

fn validator_for(file: &str) -> jsonschema::Validator {
    jsonschema::options()
        .build(&schema_doc(file))
        .unwrap_or_else(|e| panic!("the vendored schema {file} does not compile: {e}"))
}

/// Every way `instance` diverges from `file`, each naming its own field.
fn divergences(file: &str, instance: &Value) -> Vec<String> {
    validator_for(file)
        .iter_errors(instance)
        .map(|e| format!("{} (at {})", e, e.instance_path()))
        .collect()
}

#[track_caller]
fn assert_conforms(file: &str, what: &str, instance: &Value) {
    let errors = divergences(file, instance);
    assert!(
        errors.is_empty(),
        "{what} does not conform to the provider's dumped {file}:\n{instance:#}\n{}",
        errors.join("\n")
    );
}

/// The dumped schema of a method's parameters, for every method this adapter
/// speaks or is spoken to with.
///
/// Keeping this table beside the constants is what makes a mistyped method name
/// a failure here rather than a mystery at runtime: a method with no schema in
/// the dump is a method the provider does not have.
fn params_schema() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (wire::INITIALIZE, "InitializeParams.json"),
        (wire::THREAD_START, "ThreadStartParams.json"),
        (wire::TURN_START, "TurnStartParams.json"),
        (wire::TURN_INTERRUPT, "TurnInterruptParams.json"),
        (wire::TURN_STARTED, "TurnStartedNotification.json"),
        (wire::TURN_COMPLETED, "TurnCompletedNotification.json"),
        (wire::ITEM_STARTED, "ItemStartedNotification.json"),
        (wire::ITEM_COMPLETED, "ItemCompletedNotification.json"),
        (
            wire::AGENT_MESSAGE_DELTA,
            "AgentMessageDeltaNotification.json",
        ),
        (
            wire::REASONING_TEXT_DELTA,
            "ReasoningTextDeltaNotification.json",
        ),
        (
            wire::REASONING_SUMMARY_TEXT_DELTA,
            "ReasoningSummaryTextDeltaNotification.json",
        ),
        (wire::ERROR, "ErrorNotification.json"),
        (
            wire::COMMAND_EXECUTION_APPROVAL,
            "CommandExecutionRequestApprovalParams.json",
        ),
        (
            wire::FILE_CHANGE_APPROVAL,
            "FileChangeRequestApprovalParams.json",
        ),
    ])
}

/// The dumped schema of a method's result, for the methods that have one.
fn result_schema() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (wire::INITIALIZE, "InitializeResponse.json"),
        (wire::THREAD_START, "ThreadStartResponse.json"),
        (wire::TURN_START, "TurnStartResponse.json"),
        (wire::TURN_INTERRUPT, "TurnInterruptResponse.json"),
        (
            wire::COMMAND_EXECUTION_APPROVAL,
            "CommandExecutionRequestApprovalResponse.json",
        ),
        (
            wire::FILE_CHANGE_APPROVAL,
            "FileChangeRequestApprovalResponse.json",
        ),
    ])
}

/// The transcript's directives, comments and blank lines dropped.
fn transcript_steps() -> Vec<Value> {
    transcript()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("script line is not JSON: {e}"))
        })
        .collect()
}

#[test]
fn every_method_the_adapter_speaks_exists_in_the_dumped_schema() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // A constant is a claim about the provider's wire. Compiling proves nothing
    // about it; the dump does.
    for (method, file) in params_schema() {
        assert!(
            schema_dir().join(file).exists(),
            "`{method}` names {file}, which is not in the vendored dump"
        );
        // And it compiles, so a later validation failure is about the instance.
        let _ = validator_for(file);
    }
    for file in result_schema().values() {
        let _ = validator_for(file);
    }
}

#[test]
fn what_the_adapter_sends_conforms_to_the_dumped_schema() {
    // Scenario: Tipos validados contra el esquema volcado
    let initialize = InitializeParams {
        client_info: ClientInfo {
            name: "meltemi-codex-acp".into(),
            version: "0.1.0".into(),
        },
    };
    assert_conforms(
        "InitializeParams.json",
        "the handshake",
        &serde_json::to_value(&initialize).unwrap(),
    );

    let thread_start = ThreadStartParams {
        // The provider's schema accepts it here; the conformance proves the shape.
        model: Some("a-model-name".into()),
        cwd: "/project/worktree".into(),
    };
    assert_conforms(
        "ThreadStartParams.json",
        "the thread the session maps onto",
        &serde_json::to_value(&thread_start).unwrap(),
    );

    let turn_start = TurnStartParams {
        // And accepts effort HERE and only here.
        effort: Some("high".into()),
        thread_id: "thread-1".into(),
        input: vec![UserInput::Text {
            text: "write the proposal".into(),
        }],
    };
    assert_conforms(
        "TurnStartParams.json",
        "the turn",
        &serde_json::to_value(&turn_start).unwrap(),
    );

    let interrupt = TurnInterruptParams {
        thread_id: "thread-1".into(),
        turn_id: "turn-1".into(),
    };
    assert_conforms(
        "TurnInterruptParams.json",
        "the interruption",
        &serde_json::to_value(&interrupt).unwrap(),
    );

    // Every decision the proxy can hand back must be one the provider accepts:
    // an adapter that answered an approval with a word the CLI does not know
    // would strand the turn on the very path the human just decided.
    for decision in [
        ApprovalDecision::Accept,
        ApprovalDecision::AcceptForSession,
        ApprovalDecision::Decline,
        ApprovalDecision::Cancel,
    ] {
        let answer = serde_json::to_value(ApprovalResponse::new(decision)).unwrap();
        for file in [
            "CommandExecutionRequestApprovalResponse.json",
            "FileChangeRequestApprovalResponse.json",
        ] {
            assert_conforms(file, &format!("the decision {decision:?}"), &answer);
        }
    }
}

#[test]
fn the_scripted_wire_conforms_to_the_dumped_schema() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // The fixture is only evidence while it says what the provider says. This
    // is what stops it from quietly becoming fiction again.
    let params = params_schema();
    let results = result_schema();
    let mut awaited: Option<String> = None;
    let mut checked = 0;

    for step in transcript_steps() {
        let directive = step["mock"].as_str().expect("every line is a directive");
        let method = step["method"].as_str().map(str::to_string);
        match directive {
            "await-request" => awaited = method,
            "respond" => {
                let method = awaited
                    .as_deref()
                    .expect("the script answers a request it awaited");
                let file = results
                    .get(method)
                    .unwrap_or_else(|| panic!("`{method}` has no result in the vendored dump"));
                assert_conforms(file, &format!("the answer to `{method}`"), &step["result"]);
                checked += 1;
            }
            "notify" | "request" => {
                let method = method.expect("a notification or request names its method");
                let file = params
                    .get(method.as_str())
                    .unwrap_or_else(|| panic!("`{method}` is not in the vendored dump"));
                assert_conforms(file, &format!("the params of `{method}`"), &step["params"]);
                checked += 1;
            }
            other => panic!("unknown directive `{other}`"),
        }
    }

    assert!(
        checked >= 10,
        "the transcript should exercise the wire, not a handshake: {checked} messages checked"
    );
}

#[test]
fn the_adapters_types_parse_what_that_wire_says() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // Conformance in the other direction: the dump says what may arrive, and
    // the types must read it without losing what the mapping needs.
    let steps = transcript_steps();
    let result_of = |method: &str| -> Value {
        let mut awaited = None;
        for step in &steps {
            match step["mock"].as_str() {
                Some("await-request") => awaited = step["method"].as_str(),
                Some("respond") if awaited == Some(method) => return step["result"].clone(),
                _ => {}
            }
        }
        panic!("the transcript answers `{method}`");
    };
    let params_of = |method: &str| -> Vec<Value> {
        steps
            .iter()
            .filter(|s| s["method"].as_str() == Some(method))
            .filter(|s| s["params"].is_object())
            .map(|s| s["params"].clone())
            .collect()
    };

    let handshake: InitializeResult = serde_json::from_value(result_of(wire::INITIALIZE)).unwrap();
    assert!(
        handshake
            .user_agent
            .starts_with(&format!("{}/", wire::USER_AGENT_PRODUCT)),
        "the user agent carries the CLI's version, which the skew check reads: {}",
        handshake.user_agent
    );

    let thread: ThreadStartResult = serde_json::from_value(result_of(wire::THREAD_START)).unwrap();
    assert_eq!(thread.thread.id, "mock-thread-1");

    let turn: TurnStartResult = serde_json::from_value(result_of(wire::TURN_START)).unwrap();
    assert_eq!(turn.turn.status, TurnStatus::Completed);

    let started: TurnNotification =
        serde_json::from_value(params_of(wire::TURN_STARTED)[0].clone()).unwrap();
    assert_eq!(started.turn.status, TurnStatus::InProgress);
    let completed: TurnNotification =
        serde_json::from_value(params_of(wire::TURN_COMPLETED)[0].clone()).unwrap();
    assert_eq!(completed.turn.status, TurnStatus::Completed);
    assert_eq!(completed.turn.id, started.turn.id);

    let chunks: Vec<String> = params_of(wire::AGENT_MESSAGE_DELTA)
        .into_iter()
        .map(|p| {
            serde_json::from_value::<wire::DeltaNotification>(p)
                .unwrap()
                .delta
        })
        .collect();
    assert_eq!(chunks.concat(), "Working on it.");

    let items: Vec<ThreadItem> = params_of(wire::ITEM_COMPLETED)
        .into_iter()
        .map(|p| {
            let notification: ItemNotification = serde_json::from_value(p).unwrap();
            serde_json::from_value(notification.item).unwrap()
        })
        .collect();
    assert!(
        items.iter().any(|item| matches!(
            item,
            ThreadItem::AgentMessage { text, .. } if text == "Working on it."
        )),
        "the whole message arrives as an item too: {items:?}"
    );
    assert!(
        items.iter().any(|item| matches!(
            item,
            ThreadItem::FileChange { status, changes, .. }
                if *status == ItemStatus::Completed && changes[0].path == "NOTES.md"
        )),
        "and so does the file change the approval was about: {items:?}"
    );
}

#[test]
fn a_divergence_fails_naming_the_field() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // "Invalid" is not a diagnosis. When the provider's wire moves, whoever
    // re-anchors the dump must be told which field moved.
    let missing_thread = json!({"input": [{"type": "text", "text": "hola"}]});
    let errors = divergences("TurnStartParams.json", &missing_thread);
    assert!(
        errors.iter().any(|e| e.contains("threadId")),
        "the failure names the missing field: {errors:?}"
    );

    let wrong_input = json!({"threadId": "t", "input": [{"type": "text"}]});
    let errors = divergences("TurnStartParams.json", &wrong_input);
    assert!(
        errors.iter().any(|e| e.contains("/input/0")),
        "and points at where inside the message it went wrong: {errors:?}"
    );

    let unknown_decision = json!({"decision": "approved"});
    let errors = divergences("FileChangeRequestApprovalResponse.json", &unknown_decision);
    assert!(
        errors.iter().any(|e| e.contains("decision")),
        "a decision word the provider does not know is a divergence, not a detail: {errors:?}"
    );
}

#[test]
fn the_supported_range_is_declared_and_anchored_to_the_vendored_dump() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // The floor is not a taste: it is the version somebody actually verified,
    // and it must be the one the vendored dump came from, or the range is a
    // guess wearing a constant's clothes.
    let floor = format!(
        "{}.{}.{}",
        wire::SUPPORTED_FLOOR.0,
        wire::SUPPORTED_FLOOR.1,
        wire::SUPPORTED_FLOOR.2
    );
    assert_eq!(
        floor,
        wire::VENDORED_SCHEMA_VERSION,
        "the supported floor is the version the vendored dump came from"
    );
    assert!(wire::SUPPORTED_FLOOR < wire::SUPPORTED_CEILING);

    let provenance = schema_dir().join("PROVENANCE.md");
    let text = std::fs::read_to_string(&provenance).expect("the dump declares where it came from");
    assert!(
        text.contains(wire::VENDORED_SCHEMA_VERSION),
        "and PROVENANCE.md names that same version"
    );
}

#[test]
fn an_error_notification_says_whether_the_turn_is_over() {
    // Scenario: Tipos validados contra el esquema volcado
    //
    // The provider retries on its own. A turn closed on a retriable error would
    // cut the agent off mid-thought and report it as done.
    let retriable: ErrorNotification = serde_json::from_value(json!({
        "threadId": "t", "turnId": "turn-1", "willRetry": true,
        "error": {"message": "upstream hiccup", "codexErrorInfo": "internalServerError"}
    }))
    .unwrap();
    assert!(retriable.will_retry);
    assert_eq!(retriable.error.message, "upstream hiccup");
    assert_conforms(
        "ErrorNotification.json",
        "the error the adapter reads",
        &json!({
            "threadId": "t", "turnId": "turn-1", "willRetry": true,
            "error": {"message": "upstream hiccup", "codexErrorInfo": "internalServerError"}
        }),
    );
}
