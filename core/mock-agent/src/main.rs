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

    Agent
        .builder()
        .name("mock-agent")
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _cx| {
                let mut capabilities = AgentCapabilities::new();
                capabilities.load_session = supports_load;
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

    // 3. If explicitly allowed, write the marker into the proposal path.
    if granted(&decision)
        && let Some(path) = proposal_path(prompt)
    {
        let _ = std::fs::write(
            &path,
            "# Proposal (mock-agent)\n\nGenerated by the scripted end-to-end agent.\n",
        );
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
