// SPDX-License-Identifier: Apache-2.0

//! The provider's headless session wire, in the shape this adapter speaks it
//! (adaptadores-propios-acp task 3.1, design D4).
//!
//! This provider's CLI has no schema to dump, so the authority here is its own
//! headless documentation plus the wire as observed and frozen in
//! `core/mock-provider/scripts/`. Where a name could not be taken from an
//! official dump, the fixture says so in its own header and the manual run
//! against the real binary (task 5.2) is what re-anchors it. Nothing below was
//! written from a blog post.
//!
//! Three decisions worth naming, because they are the ones that would rot
//! quietly:
//!
//! - **The launch surface is one line and it is here.** [`session_args`] is the
//!   whole of what this adapter asks the official binary to be: the documented
//!   headless session, with the account the user already signed into. `--bare`
//!   is not there and will never be added — it skips the sign-in and demands a
//!   key, which is the one degradation design D4 forbids.
//! - **The session's identity is dictated, never learned.** [`session_id_args`]
//!   hands the CLI the id this session will answer to on both sides, so nothing
//!   about opening a session has to wait for the CLI to speak — which it does
//!   not do until it has been given a turn to run (task 5.3).
//! - **Incoming types are permissive.** No struct denies unknown fields and
//!   every field the adapter does not require is optional: this wire grows
//!   between CLI releases, and a session that refused to open because a new
//!   field appeared would be refusing for the provider's success, not for its
//!   failure.

use serde::Deserialize;

// --- The launch surface --------------------------------------------------

/// Runs one non-interactive session and exits: the documented headless mode.
pub const PRINT: &str = "-p";
/// The adapter writes the user's turns as newline-delimited JSON events.
pub const INPUT_FORMAT: &str = "--input-format";
/// The CLI writes the session as newline-delimited JSON events.
pub const OUTPUT_FORMAT: &str = "--output-format";
/// The value both of the format flags take.
pub const STREAM_JSON: &str = "stream-json";
/// Token-level deltas and subagent transcripts, which is what buys this dialect
/// near-parity of streaming with a natively-ACP agent.
pub const INCLUDE_PARTIAL_MESSAGES: &str = "--include-partial-messages";
/// The CLI requires this alongside the streamed output format in headless mode.
/// It makes the CLI chattier on **stderr**, where the daemon already collects
/// what the provider says and where nothing is ever parsed as protocol.
pub const VERBOSE: &str = "--verbose";
/// Asks the CLI for its own version and nothing else.
pub const VERSION: &str = "--version";
/// Hands the CLI a set of MCP servers to run for this session.
pub const MCP_CONFIG: &str = "--mcp-config";
/// Names the tool the CLI must ask before running anything its own static rules
/// did not already decide.
pub const PERMISSION_PROMPT_TOOL: &str = "--permission-prompt-tool";
/// Hands the CLI the settings this session runs under, which is where the hook
/// that gates every tool call is installed.
pub const SETTINGS: &str = "--settings";
/// Tells the CLI which id this session has. It takes a UUID and uses it
/// verbatim, which is what lets the adapter name a session before the CLI has
/// said anything at all.
pub const SESSION_ID: &str = "--session-id";

/// The flags that put the official CLI in the documented headless session, with
/// the account the user already signed into.
///
/// Everything this adapter asks of the provider's launch surface is here, in
/// one list, so that what it asks for can be read in one place and argued with.
/// What is deliberately absent: any flag that selects a mode demanding an API
/// key, and any flag that loosens the CLI's own permission gate — the hard gate
/// is configured through the documented settings channel, never by asking the
/// CLI to stop checking.
#[must_use]
pub fn session_args() -> Vec<String> {
    [
        PRINT,
        INPUT_FORMAT,
        STREAM_JSON,
        OUTPUT_FORMAT,
        STREAM_JSON,
        INCLUDE_PARTIAL_MESSAGES,
        VERBOSE,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

// --- Event types the CLI emits -------------------------------------------

/// The envelope of everything the CLI says about the session itself.
pub const SYSTEM: &str = "system";
/// The subtype of the first event of a session: the handshake this adapter
/// reads its surface out of.
pub const INIT: &str = "init";
/// A raw model stream event, which is where the token-level deltas live.
pub const STREAM_EVENT: &str = "stream_event";
/// A whole assistant message, tool calls included.
pub const ASSISTANT: &str = "assistant";
/// A message on the user's side of the transcript — which is how tool results
/// travel on this wire.
pub const USER: &str = "user";
/// The event that closes a turn.
pub const RESULT: &str = "result";

/// What the CLI announces as the source of the credentials it will use.
///
/// The signed-in session announces `none`: no key anywhere, the account the
/// user logged into. Anything else is the surface design D4 refuses.
pub const SIGNED_IN_KEY_SOURCE: &str = "none";

// --- Inside a streamed model event ---------------------------------------

/// A piece of a block arriving: the token-level delta itself.
pub const CONTENT_BLOCK_DELTA: &str = "content_block_delta";
/// A delta of the assistant's words.
pub const TEXT_DELTA: &str = "text_delta";
/// A delta of the assistant's thinking.
pub const THINKING_DELTA: &str = "thinking_delta";

// --- Message content blocks ----------------------------------------------

/// Words.
pub const TEXT: &str = "text";
/// Thinking.
pub const THINKING: &str = "thinking";
/// A tool the assistant wants to run.
pub const TOOL_USE: &str = "tool_use";
/// What a tool answered.
pub const TOOL_RESULT: &str = "tool_result";

/// The field that marks an event as belonging to a subagent's transcript: the
/// id of the tool call that spawned it.
pub const PARENT_TOOL_USE_ID: &str = "parent_tool_use_id";

// --- How a turn ends ------------------------------------------------------

/// The turn ran to its end.
pub const SUCCESS: &str = "success";
/// The turn ran out of the CLI's own turn budget. Not a failure: a limit, and
/// ACP has a stop reason that says exactly that.
pub const ERROR_MAX_TURNS: &str = "error_max_turns";

/// The flags that hand the CLI its MCP servers, tell it where to ask, and
/// install the gate that no mode of the CLI can skip.
///
/// Separate from [`session_args`] because these are the session's own — they
/// carry paths that exist for one session and disappear with it — while the
/// launch surface is the same for every session this adapter ever opens.
#[must_use]
pub fn permission_args(
    config: &std::path::Path,
    prompt_tool: &str,
    settings: &std::path::Path,
) -> Vec<String> {
    vec![
        MCP_CONFIG.to_string(),
        config.display().to_string(),
        PERMISSION_PROMPT_TOOL.to_string(),
        prompt_tool.to_string(),
        SETTINGS.to_string(),
        settings.display().to_string(),
    ]
}

/// The MCP configuration document, in the shape the CLI's flag reads.
#[must_use]
pub fn mcp_config(servers: serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "mcpServers": servers })
}

/// One MCP server of the session's projection, in the shape the CLI's
/// configuration takes it.
///
/// `None` for a transport this adapter cannot hand over — which it never
/// silently drops: the caller says so in the session, because a tool the user
/// declared and never got is exactly the kind of absence that reads as a bug in
/// the agent.
#[must_use]
pub fn server_entry(
    server: &agent_client_protocol::schema::v1::McpServer,
) -> Option<(String, serde_json::Value)> {
    use agent_client_protocol::schema::v1::McpServer;
    match server {
        McpServer::Stdio(stdio) => Some((
            stdio.name.clone(),
            serde_json::json!({
                "type": "stdio",
                "command": stdio.command.display().to_string(),
                "args": stdio.args,
                "env": stdio.env.iter()
                    .map(|variable| (variable.name.clone(), serde_json::Value::from(variable.value.clone())))
                    .collect::<serde_json::Map<_, _>>(),
            }),
        )),
        McpServer::Http(http) => Some((
            http.name.clone(),
            serde_json::json!({
                "type": "http",
                "url": http.url,
                "headers": http.headers.iter()
                    .map(|header| (header.name.clone(), serde_json::Value::from(header.value.clone())))
                    .collect::<serde_json::Map<_, _>>(),
            }),
        )),
        McpServer::Sse(sse) => Some((
            sse.name.clone(),
            serde_json::json!({
                "type": "sse",
                "url": sse.url,
                "headers": sse.headers.iter()
                    .map(|header| (header.name.clone(), serde_json::Value::from(header.value.clone())))
                    .collect::<serde_json::Map<_, _>>(),
            }),
        )),
        _ => None,
    }
}

/// Resumes the CLI's own session of this name.
///
/// Only `--resume`, never the flag that forks the resumed session into a new
/// one: the daemon asked for *this* session to continue, and a fork would
/// quietly answer with a different one whose id nobody holds.
pub const RESUME: &str = "--resume";

/// The flags that continue a session the CLI kept.
#[must_use]
pub fn resume_args(provider_session: &str) -> Vec<String> {
    vec![RESUME.to_string(), provider_session.to_string()]
}

/// The flag that names a new session, so both sides call it the same thing from
/// the first instant.
///
/// This is the other half of not waiting for the CLI to introduce itself: the
/// adapter used to take the session's id out of the initial event, which meant
/// no session could be opened until that event arrived — and it does not arrive
/// until a turn has been sent. Dictating the id unties the two, and the id a
/// later resume names is the one written here.
///
/// It never travels beside [`resume_args`]: a resume already names the session,
/// and asking for two identities in one launch is a question the CLI should
/// never have to answer.
#[must_use]
pub fn session_id_args(session: &str) -> Vec<String> {
    vec![SESSION_ID.to_string(), session.to_string()]
}

/// The user's turn, in the shape this wire takes it.
///
/// The content is a list of blocks rather than a bare string: that is the shape
/// the provider's own message format is defined in, and the one a future
/// non-text block could join without changing anything here.
#[must_use]
pub fn user_message(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": USER,
        "message": {
            "role": USER,
            "content": [{"type": TEXT, "text": text}],
        },
    })
}

// --- The initial event ---------------------------------------------------

/// The first event of a session — which arrives **after** the first turn is
/// sent, because this CLI announces nothing until it has been given something
/// to do.
///
/// The fields below are the ones the adapter reads, and they are the ones the
/// binary was seen to emit. What the whole event carried, observed against
/// `claude` 2.1.167 on 2026-07-28 (task 5.3, and the run in
/// `docs/conformidad-manual.md`): `type`, `subtype`, `cwd`, `session_id`,
/// `tools`, `mcp_servers` as `{name, status}` objects, `model`,
/// `permissionMode`, `slash_commands`, `apiKeySource`, `claude_code_version`,
/// `output_style`, `agents`, `skills`, `plugins`, `uuid`, `memory_paths` and a
/// handful of flags. What it did **not** carry is a `capabilities` array: the
/// design assumed one and the binary settled it, so nothing here hangs off a
/// field nobody emits.
///
/// Field names on this wire are not uniform — some are snake_case and some are
/// camelCase — so each one is spelled out rather than renamed wholesale. Being
/// clever there would silently drop exactly the field the surface check reads.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct InitEvent {
    /// The model the session will run.
    #[serde(default)]
    pub model: Option<String>,
    /// Where the credentials come from. `none` is the signed-in session.
    #[serde(default, rename = "apiKeySource")]
    pub api_key_source: Option<String>,
    /// The permission mode the CLI came up in, which the session log records:
    /// the adapter's own hard gate decides every call regardless, and knowing
    /// which mode ran underneath it is the difference between reading a session
    /// afterwards and guessing at it.
    #[serde(default, rename = "permissionMode")]
    pub permission_mode: Option<String>,
}

/// Whether a frame is the initial event this adapter shakes hands on.
#[must_use]
pub fn is_init(value: &serde_json::Value) -> bool {
    value.get("type").and_then(serde_json::Value::as_str) == Some(SYSTEM)
        && value.get("subtype").and_then(serde_json::Value::as_str) == Some(INIT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_launch_surface_is_the_signed_in_session_and_nothing_else() {
        // The one list that decides what the official binary is asked to be.
        // `--bare` skips the sign-in and demands a key: it is not here, and a
        // test says so out loud so that adding it takes deleting this line.
        let args = session_args();
        assert!(args.contains(&PRINT.to_string()));
        assert_eq!(
            args.iter().filter(|arg| *arg == STREAM_JSON).count(),
            2,
            "both directions of the wire are the JSON event dialect: {args:?}"
        );
        assert!(args.contains(&INCLUDE_PARTIAL_MESSAGES.to_string()));
        assert!(
            !args.iter().any(|arg| arg.contains("bare")),
            "the mode that demands an API key is never asked for: {args:?}"
        );
        assert!(
            !args.iter().any(|arg| arg.contains("dangerously")),
            "and neither is any flag that asks the CLI to stop checking: {args:?}"
        );
    }

    #[test]
    fn the_initial_event_is_read_field_by_field_and_survives_new_ones() {
        // The wire mixes snake_case and camelCase and grows between releases.
        // A session that refused to open because a field appeared would refuse
        // for the provider's success. The event below is the shape the real
        // binary emitted, abridged; every field it carries that this adapter
        // does not read must pass through without complaint.
        let init: InitEvent = serde_json::from_value(json!({
            "type": "system", "subtype": "init", "cwd": "/p",
            "session_id": "7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b",
            "tools": ["Read"], "mcp_servers": [{"name": "fs", "status": "connected"}],
            "model": "sonnet", "permissionMode": "default",
            "slash_commands": [], "apiKeySource": "none",
            "claude_code_version": "2.1.167", "agents": [], "skills": [],
            "somethingBrandNew": {"nested": true}
        }))
        .expect("an initial event with an unknown field still parses");
        assert_eq!(init.api_key_source.as_deref(), Some("none"));
        assert_eq!(init.model.as_deref(), Some("sonnet"));
        assert_eq!(init.permission_mode.as_deref(), Some("default"));

        // And an event that announces none of it is still an event: the surface
        // check decides what its silence means, not the parser.
        let bare: InitEvent = serde_json::from_value(json!({"type": "system", "subtype": "init"}))
            .expect("an initial event that announces nothing still parses");
        assert!(bare.api_key_source.is_none());
        assert!(bare.model.is_none());
    }

    #[test]
    fn a_resume_names_the_session_the_cli_kept_and_never_forks_it() {
        // Scenario: Reanudación mapeada a la sesión ACP
        //
        // The daemon asked for *this* session to continue. Forking would give
        // it a different one, whose id nobody holds and whose work nobody asked
        // for.
        let args = resume_args("abc-123");
        assert_eq!(args, vec![RESUME.to_string(), "abc-123".to_string()]);
        assert!(!args.iter().any(|arg| arg.contains("fork")), "{args:?}");
    }

    #[test]
    fn a_new_session_is_named_by_the_adapter_and_a_resumed_one_by_what_it_resumes() {
        // Scenario: Reanudación mapeada a la sesión ACP
        //
        // One identity per launch, and the same string on both sides of it: the
        // id dictated here is the id the daemon will hand back to `session/load`
        // later, so the resume needs nothing remembered anywhere.
        let fresh = session_id_args("7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b");
        assert_eq!(
            fresh,
            vec![
                SESSION_ID.to_string(),
                "7d8c9c45-4a39-4503-ac3d-fdbb94f8f05b".to_string()
            ]
        );
        assert!(
            !session_args()
                .iter()
                .chain(fresh.iter())
                .any(|arg| arg == RESUME),
            "a session being named is not a session being resumed"
        );
        assert!(
            !resume_args("abc-123").iter().any(|arg| arg == SESSION_ID),
            "and a resume names the session it continues, not a second one"
        );
    }

    #[test]
    fn only_the_initial_event_is_mistaken_for_the_handshake() {
        assert!(is_init(
            &json!({"type": "system", "subtype": "init", "apiKeySource": "none"})
        ));
        assert!(!is_init(&json!({"type": "system", "subtype": "compact"})));
        assert!(!is_init(&json!({"type": "result", "subtype": "success"})));
        assert!(!is_init(&json!({"type": "stream_event"})));
    }
}
