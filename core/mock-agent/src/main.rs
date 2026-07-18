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
    AgentCapabilities, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse,
    PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionId, SessionNotification,
    SessionUpdate, StopReason, TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Result, Stdio};

/// Fixed session id the scripted agent hands back from `session/new`.
const SESSION_ID: &str = "mock-session";
/// Option id the agent offers (and checks for) to allow the file write.
const ALLOW_OPTION: &str = "allow";

#[tokio::main]
async fn main() -> Result<()> {
    // `--load-session` makes the mock announce session-load support and handle
    // `session/load`, so the resume path can be exercised end to end.
    let supports_load = std::env::args().any(|a| a == "--load-session");
    // `--mcp` makes the mock announce MCP support, so the passthrough injection
    // path can be exercised end to end.
    let supports_mcp = std::env::args().any(|a| a == "--mcp");

    Agent
        .builder()
        .name("mock-agent")
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
                responder.respond(NewSessionResponse::new(SessionId::new(SESSION_ID)))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // Loading a prior session succeeds (the mock keeps no real state);
            // its presence lets the daemon exercise the resume path.
            async move |_load: LoadSessionRequest, responder, _cx| {
                responder.respond(LoadSessionResponse::new())
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
                        responder.respond(PromptResponse::new(StopReason::EndTurn))
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

    // 1. Stream a chunk of the agent's "response".
    let _ = cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Filling in the proposal.",
        )))),
    ));

    // 2. Ask the client for permission to write the file.
    let request = RequestPermissionRequest::new(
        session_id,
        ToolCallUpdate::new("write-proposal", ToolCallUpdateFields::new()),
        vec![
            PermissionOption::new(ALLOW_OPTION, "Allow", PermissionOptionKind::AllowOnce),
            PermissionOption::new("reject", "Reject", PermissionOptionKind::RejectOnce),
        ],
    );
    let decision = cx.send_request(request).block_task().await;

    // 3. If explicitly allowed, write the requested artifact(s).
    if granted(&decision) {
        if let Some(path) = proposal_path(prompt) {
            let _ = std::fs::write(
                &path,
                format!(
                    "# Proposal ({profile})\n\nGenerated by the scripted end-to-end agent \
                     with the `{profile}` profile.\n",
                    profile = profile()
                ),
            );
        }
        write_sdd_artifact(prompt);
    }
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
