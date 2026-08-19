// SPDX-License-Identifier: Apache-2.0

//! Scripted ACP agent for Meltemi's automated end-to-end test (task 4.6).
//!
//! This is NOT a real coding agent: it plays a fixed script that exercises
//! the whole ACP path meltemid drives — `initialize`, `session/new`, a
//! streamed agent message, one permission request, and (when the client
//! grants it) a file write — then ends the turn. The daemon's e2e test
//! asserts each of these steps.
//!
//! Contract with meltemid's propose flow: the prompt text carries a line
//! `PROPOSAL_PATH: <absolute path>`. When the permission is granted, the
//! agent writes a marker into that file, standing in for a real agent
//! filling in the proposal. Real agents ignore the marker line.

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionConfigBoolean, SessionConfigKind,
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigOptionValue,
    SessionConfigSelect, SessionConfigSelectOption, SessionId, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, StopReason, TextContent,
    ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Result, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Fixed session id the scripted agent hands back from `session/new`.
const SESSION_ID: &str = "mock-session";
/// Option id the agent offers (and checks for) to allow the file write.
const ALLOW_OPTION: &str = "allow";

/// The two config options the mock announces under `--config-options`: a
/// selector and a toggle, because the two ACP kinds take different paths on the
/// daemon side and a mock that only had one would leave the other untested.
const MODEL_OPTION: &str = "model";
const THINKING_OPTION: &str = "thinking";

/// What the announced options currently hold. Real state, not a canned answer:
/// the point of the flag is that `session/set_config_option` actually changes
/// what the next announcement says, which is what the daemon reads back.
static CURRENT_MODEL: Mutex<String> = Mutex::new(String::new());
static CURRENT_THINKING: AtomicBool = AtomicBool::new(false);

/// The options as they stand right now.
fn announced_options() -> Vec<SessionConfigOption> {
    let current = CURRENT_MODEL.lock().expect("mock config state").clone();
    let current = if current.is_empty() {
        "fast".to_string()
    } else {
        current
    };
    vec![
        SessionConfigOption::new(
            MODEL_OPTION,
            "Model",
            SessionConfigKind::Select(SessionConfigSelect::new(
                current,
                vec![
                    SessionConfigSelectOption::new("fast", "Fast"),
                    SessionConfigSelectOption::new("slow", "Slow").description("The careful one"),
                ],
            )),
        )
        .category(SessionConfigOptionCategory::Model),
        SessionConfigOption::new(
            THINKING_OPTION,
            "Extended thinking",
            SessionConfigKind::Boolean(SessionConfigBoolean::new(
                CURRENT_THINKING.load(Ordering::SeqCst),
            )),
        )
        .category(SessionConfigOptionCategory::ThoughtLevel),
    ]
}

/// Set when the client sends `session/cancel` AND `--honor-cancel` was passed.
///
/// Off by default on purpose: a mock that honoured cancel unconditionally would
/// change the stop reason every existing cancellation test reads, and those
/// tests assert what the daemon does when an agent *ignores* the cancel — which
/// is the case the daemon has to survive (redirigir-turno design D6).
static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() -> Result<()> {
    // `--load-session` makes the mock announce session-load support and handle
    // `session/load`, so the resume path can be exercised end to end.
    let supports_load = std::env::args().any(|a| a == "--load-session");
    // `--mcp` makes the mock announce MCP support, so the passthrough injection
    // path can be exercised end to end.
    let supports_mcp = std::env::args().any(|a| a == "--mcp");
    let honors_cancel = std::env::args().any(|a| a == "--honor-cancel");
    // `--cancel-turn` makes the mock end its turn `Cancelled` with nobody having
    // asked: the agent giving up on its own, which the daemon must keep treating
    // as the end of the session (redirigir-turno).
    let cancels_itself = std::env::args().any(|a| a == "--cancel-turn");
    // `--config-options` makes the mock announce session config options and
    // honour `session/set_config_option`, so the live-change path can be
    // exercised without any provider (modelo-y-esfuerzo design D2).
    //
    // Off by default like every other mock switch, and for the same reason:
    // announcing options unconditionally would make every existing session
    // start emit a `config_options_announced` event that the tests reading the
    // log were never written to expect.
    let announces_config = std::env::args().any(|a| a == "--config-options");

    Agent
        .builder()
        .name("mock-agent")
        .on_receive_notification(
            async move |cancel: CancelNotification, _cx| {
                let _ = cancel;
                if honors_cancel {
                    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _cx| {
                let mut capabilities = AgentCapabilities::new();
                capabilities.load_session = supports_load;
                capabilities.mcp_capabilities.http = supports_mcp;
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version)
                        .agent_capabilities(capabilities),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_new_session: NewSessionRequest, responder, _cx| {
                let response = NewSessionResponse::new(SessionId::new(SESSION_ID));
                responder.respond(if announces_config {
                    response.config_options(announced_options())
                } else {
                    response
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // Loading a prior session succeeds (the mock keeps no real state);
            // its presence lets the daemon exercise the resume path.
            async move |_load: LoadSessionRequest, responder, _cx| {
                let response = LoadSessionResponse::new();
                responder.respond(if announces_config {
                    response.config_options(announced_options())
                } else {
                    response
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // The live change. Values the announcement does not cover are
            // refused here too: the daemon checks first, and an agent that
            // accepted anything anyway would let a bug on that side pass
            // unnoticed.
            async move |set: SetSessionConfigOptionRequest, responder, _cx| {
                if !announces_config {
                    return responder.respond(SetSessionConfigOptionResponse::new(Vec::new()));
                }
                match (set.config_id.0.as_ref(), &set.value) {
                    (MODEL_OPTION, SessionConfigOptionValue::ValueId { value })
                        if matches!(value.0.as_ref(), "fast" | "slow") =>
                    {
                        *CURRENT_MODEL.lock().expect("mock config state") = value.0.to_string();
                    }
                    (THINKING_OPTION, SessionConfigOptionValue::Boolean { value }) => {
                        CURRENT_THINKING.store(*value, Ordering::SeqCst);
                    }
                    _ => {}
                }
                responder.respond(SetSessionConfigOptionResponse::new(announced_options()))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |prompt: PromptRequest, responder, cx: ConnectionTo<Client>| {
                // Run the turn on a spawned task: it sends a permission request
                // and awaits it, which must NOT happen on the dispatch loop —
                // the loop has to stay free to deliver that very response
                // (a `block_task` inside a handler deadlocks by design).
                cx.spawn({
                    let cx = cx.clone();
                    async move {
                        run_scripted_turn(&prompt, &cx).await;
                        // The stop reason is read AFTER the turn, and reset, so a
                        // session that keeps going reports the next turn on its
                        // own merits rather than inheriting this one's ending.
                        let stop =
                            if cancels_itself || CANCEL_REQUESTED.swap(false, Ordering::SeqCst) {
                                StopReason::Cancelled
                            } else {
                                StopReason::EndTurn
                            };
                        responder.respond(PromptResponse::new(stop))
                    }
                })?;
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}

/// Plays the fixed script: stream a message, ask one permission, and — if
/// granted — write the proposal marker into the path from the prompt.
async fn run_scripted_turn(prompt: &PromptRequest, cx: &ConnectionTo<Client>) {
    let session_id = prompt.session_id.clone();

    // `--turn-delay-ms <N>` holds each turn open for N milliseconds before doing
    // any work, so a test can race a `session/direct` into an active turn
    // (control-remoto-asistido). Default: no delay.
    if let Some(ms) = turn_delay_ms() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if CANCEL_REQUESTED.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    // 0. `--think` streams a thought chunk before speaking, so the surfaces
    // that show an agent's reasoning can be exercised without a real agent
    // (pensamiento-a-la-vista). Off by default: a mock that thinks by default
    // would change what every existing transcript test reads.
    if thinks() {
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Reading the proposal before writing it."),
            ))),
        ));
    }

    // 1. Stream a chunk of the agent's "response".
    let _ = cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Filling in the proposal.",
        )))),
    ));

    // 1b. With `--ask`, put a real question first: the agent's own options,
    // one of them recommended in its label, exactly as a provider would send
    // it. The answer comes back as the selected option id.
    if asks() {
        let question = RequestPermissionRequest::new(
            session_id.clone(),
            ToolCallUpdate::new("ask-route", ToolCallUpdateFields::new()),
            vec![
                PermissionOption::new(
                    "option-0",
                    "Rewrite it (recommended)",
                    PermissionOptionKind::AllowOnce,
                ),
                PermissionOption::new("option-1", "Patch it", PermissionOptionKind::AllowOnce),
            ],
        );
        let chosen = cx.send_request(question).block_task().await;
        // The choice is echoed into the transcript, so a test can assert the
        // turn actually continued with the answer rather than merely receiving
        // one.
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(format!("answering with {}", chosen_option(&chosen))),
            ))),
        ));
    }

    // 2. Ask the client for permission to write the file.
    // The path this write would touch rides with the request, because a real
    // agent's tool call carries it (`rawInput.path` or `locations[0].path`, ACP
    // v1) and the daemon's rule matchers read it. A mock that omitted it was
    // under-modelling the protocol, and any posture bounded BY PATH would have
    // looked broken against it (modos-de-autonomia design D5).
    let mut fields = ToolCallUpdateFields::new();
    if let Some(path) = proposal_path(prompt).or_else(|| sdd_artifact_path(prompt)) {
        fields = fields.raw_input(serde_json::json!({ "path": path }));
    }
    let request = RequestPermissionRequest::new(
        session_id,
        ToolCallUpdate::new("write-proposal", fields),
        vec![
            PermissionOption::new(ALLOW_OPTION, "Allow", PermissionOptionKind::AllowOnce),
            PermissionOption::new("reject", "Reject", PermissionOptionKind::RejectOnce),
        ],
    );
    let decision = cx.send_request(request).block_task().await;

    // 3. If explicitly allowed, write the requested artifact(s).
    if granted(&decision) {
        if let Some(path) = proposal_path(prompt) {
            // Echo the profile name AND a marker env var, so a launch profile's
            // env overlay is observable end-to-end (flota-multiproveedor): the
            // marker proves which auth context the subprocess ran under.
            let marker = std::env::var("MELTEMI_MOCK_MARKER").unwrap_or_default();
            let _ = std::fs::write(
                &path,
                format!(
                    "# Proposal ({profile})\n\nGenerated by the scripted end-to-end agent \
                     with the `{profile}` profile.\nmarker={marker}\n",
                    profile = profile()
                ),
            );
        }
        write_sdd_artifact(prompt);
    }
}

/// Which option a permission answer selected, or `none` when it selected
/// nothing. Written for a transcript a test reads, never for a protocol.
fn chosen_option(
    decision: &std::result::Result<
        agent_client_protocol::schema::v1::RequestPermissionResponse,
        agent_client_protocol::Error,
    >,
) -> String {
    match decision {
        Ok(response) => match &response.outcome {
            RequestPermissionOutcome::Selected(selected) => selected.option_id.0.to_string(),
            RequestPermissionOutcome::Cancelled => "cancelled".to_string(),
            _ => "unknown".to_string(),
        },
        Err(_) => "none".to_string(),
    }
}

/// Reads the optional `--turn-delay-ms <N>` argument (milliseconds to hold each
/// turn open before scripting it).
/// Whether `--think` was passed: the mock streams a thought chunk before it
/// speaks.
fn thinks() -> bool {
    std::env::args().any(|a| a == "--think")
}

/// Whether `--ask` was passed: the mock puts a question with options before it
/// works, so the answering flow is exercisable without a real agent and without
/// the network. Off by default, like every other knob here — a mock that asked
/// on its own would change what every existing permission test reads
/// (preguntas-del-agente design D6).
fn asks() -> bool {
    std::env::args().any(|a| a == "--ask")
}

fn turn_delay_ms() -> Option<u64> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--turn-delay-ms" {
            return args.next().and_then(|v| v.parse().ok());
        }
    }
    None
}

/// The mock's profile name (`--profile <name>`, default `mock`). Two instances
/// with distinct profiles produce distinguishable output, so provider
/// parallelism is observable in the acceptance script (hito-v01-aceptacion).
fn profile() -> String {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            return args.next().unwrap_or_else(|| "mock-agent".to_string());
        }
    }
    "mock-agent".to_string()
}

/// Concatenates the prompt's text blocks.
fn prompt_text(prompt: &PromptRequest) -> String {
    let mut text = String::new();
    for block in &prompt.prompt {
        if let ContentBlock::Text(t) = block {
            text.push_str(&t.text);
            text.push('\n');
        }
    }
    text
}

/// Reads a `KEY: value` line from the prompt.
fn value_after(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix(key).map(|r| r.trim().to_string()))
}

/// Writes the SDD artifact the authoring prompt asked for, with valid content
/// (or, with `--sdd-invalid`, an intentionally invalid delta spec so the
/// engine's validation-as-gate can be exercised).
/// The path an SDD authoring turn is about to write, read from the same prompt
/// keys `write_sdd_artifact` uses. It rides with the permission request so the
/// daemon's path matchers have something to match, exactly as a real agent's
/// tool call would carry it.
fn sdd_artifact_path(prompt: &PromptRequest) -> Option<std::path::PathBuf> {
    let text = prompt_text(prompt);
    for key in [
        "CONSTITUTION_PATH:",
        "TASKS_PATH:",
        "ARTIFACT_PATH:",
        "DESIGN_PATH:",
    ] {
        if let Some(path) = value_after(&text, key) {
            return Some(std::path::PathBuf::from(path));
        }
    }
    None
}

fn write_sdd_artifact(prompt: &PromptRequest) {
    let text = prompt_text(prompt);
    let invalid = std::env::args().any(|a| a == "--sdd-invalid");

    if let Some(path) = value_after(&text, "CONSTITUTION_PATH:") {
        let _ = std::fs::write(
            path,
            "# Constitution (mock)\n\n## Principles\n\n1. Spec-first.\n",
        );
        return;
    }
    if let Some(path) = value_after(&text, "TASKS_PATH:") {
        let _ = std::fs::write(path, "## 1. Work\n\n- [ ] 1.1 Do the thing (dep: none)\n");
        return;
    }
    let Some(artifact) = value_after(&text, "ARTIFACT:") else {
        return;
    };
    let Some(path) = value_after(&text, "ARTIFACT_PATH:") else {
        return;
    };
    match artifact.as_str() {
        "proposal" => {
            let _ = std::fs::write(
                &path,
                "## Why\n\nBecause.\n\n## What Changes\n\n## Impact\n",
            );
        }
        "specs" => {
            // ARTIFACT_PATH is the specs directory; write one capability delta.
            let cap_dir = std::path::Path::new(&path).join("example-capability");
            let _ = std::fs::create_dir_all(&cap_dir);
            let spec = if invalid {
                // A requirement with no scenario: the engine must reject this.
                "## ADDED Requirements\n\n### Requirement: Broken\nThe system SHALL do a thing.\n"
            } else {
                "## ADDED Requirements\n\n### Requirement: Example\n\
                 The system SHALL do a thing.\n\n#### Scenario: It works\n\
                 - **WHEN** invoked\n- **THEN** it responds\n"
            };
            let _ = std::fs::write(cap_dir.join("spec.md"), spec);
        }
        "design" => {
            let _ = std::fs::write(&path, "## Context\n\n## Decisions\n\n### D1\nA choice.\n");
        }
        "tasks" => {
            let _ = std::fs::write(&path, "## 1. Work\n\n- [ ] 1.1 Do the thing\n");
        }
        _ => {}
    }
}

/// Whether the client granted the write by selecting the allow option.
fn granted(
    decision: &std::result::Result<
        agent_client_protocol::schema::v1::RequestPermissionResponse,
        agent_client_protocol::Error,
    >,
) -> bool {
    match decision {
        Ok(response) => match &response.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                selected.option_id.0.as_ref() == ALLOW_OPTION
            }
            _ => false,
        },
        Err(_) => false,
    }
}

/// Extracts the `PROPOSAL_PATH:` line from the prompt's text content.
fn proposal_path(prompt: &PromptRequest) -> Option<std::path::PathBuf> {
    for block in &prompt.prompt {
        if let ContentBlock::Text(text) = block {
            for line in text.text.lines() {
                if let Some(rest) = line.strip_prefix("PROPOSAL_PATH:") {
                    return Some(std::path::PathBuf::from(rest.trim()));
                }
            }
        }
    }
    None
}
