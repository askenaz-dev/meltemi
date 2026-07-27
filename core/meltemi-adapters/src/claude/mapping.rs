// SPDX-License-Identifier: Apache-2.0

//! Between the CLI's session events and the ACP session
//! (adaptadores-propios-acp task 3.2).
//!
//! The CLI narrates a turn twice over: once token by token, as raw model
//! stream events, and once in whole messages as each one completes. ACP wants
//! one narration. This module is the whole of that translation and is
//! deliberately free of I/O — it takes an event and answers with what the
//! session should show and whether the turn is over, which is what lets every
//! rule below be argued with one at a time, without a process, a pipe or a
//! clock.
//!
//! Four rules earn their keep here:
//!
//! - **Nothing is said twice.** Text that arrived as deltas is not repeated
//!   when the whole message lands; text that never streamed *is* shown, because
//!   otherwise it would be lost. The comparison is against what was actually
//!   streamed, not against a flag: this wire may stream part of a message and
//!   not the rest, and a flag would drop the rest.
//! - **Nothing is dropped.** A tool this adapter has no kind for still appears,
//!   under the name the provider gave it, and a subagent's whole transcript is
//!   kept under the tool call that spawned it rather than thrown away for being
//!   awkward to place.
//! - **Nothing is invented.** A tool call carries the input the provider sent,
//!   verbatim, as its evidence. Where ACP wants something this wire does not
//!   send, the session shows what was sent instead of a fabrication.
//! - **A limit is not a failure.** Running out of the CLI's own turn budget has
//!   a stop reason of its own in ACP; every other error ends the turn as an
//!   error, with the provider's own words.

use agent_client_protocol::schema::v1::PromptRequest;
use agent_client_protocol::schema::v1::{
    Content, ContentBlock, ContentChunk, SessionUpdate, StopReason, TextContent, ToolCall,
    ToolCallContent, ToolCallLocation, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    ToolKind,
};
use serde_json::Value;

use super::wire;

/// What an event says about the turn itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// The turn is over, and this is how it ended.
    TurnEnded(StopReason),
    /// The turn ended badly, in the provider's own words.
    TurnFailed(String),
}

/// One event, translated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mapped {
    /// What the session should show.
    pub updates: Vec<SessionUpdate>,
    /// What it means for the turn, if anything.
    pub signal: Option<Signal>,
    /// A line worth keeping on stderr, where the daemon already collects the
    /// provider's noise — never shown as if the agent had said it.
    pub noted: Option<String>,
}

impl Mapped {
    fn updates(updates: Vec<SessionUpdate>) -> Self {
        Self {
            updates,
            ..Self::default()
        }
    }

    fn signal(signal: Signal) -> Self {
        Self {
            signal: Some(signal),
            ..Self::default()
        }
    }

    fn noted(note: String) -> Self {
        Self {
            noted: Some(note),
            ..Self::default()
        }
    }
}

/// Translates one turn's events, remembering what has already been said.
#[derive(Debug, Default)]
pub struct Mapper {
    /// Everything that has reached the session as deltas since the last whole
    /// message. What a completed message repeats of it is not shown again.
    streamed: String,
}

impl Mapper {
    /// A mapper for a fresh turn.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What one session event becomes.
    ///
    /// An event of a kind this adapter does not map answers with nothing at
    /// all: this wire says a great deal an ACP session has no place for, and
    /// inventing a place for it would be worse than silence.
    pub fn map(&mut self, event: &Value) -> Mapped {
        match kind(event, "type") {
            Some(wire::STREAM_EVENT) => self.delta(event.get("event").unwrap_or(&Value::Null)),
            Some(wire::ASSISTANT) => self.assistant(event),
            Some(wire::USER) => self.tool_results(event),
            Some(wire::RESULT) => result(event),
            // The initial event was consumed by the handshake; a later one is
            // the CLI talking about itself mid-session (a compaction, a
            // warning), which belongs on the record and not in the transcript.
            Some(wire::SYSTEM) => Mapped::noted(format!("provider said: {event}")),
            _ => Mapped::default(),
        }
    }

    /// A token-level delta of the message being written.
    fn delta(&mut self, event: &Value) -> Mapped {
        if kind(event, "type") != Some(wire::CONTENT_BLOCK_DELTA) {
            // Block starts and stops, message envelopes: the shape of the
            // stream, not its content. What they carry arrives again, whole, in
            // the completed message.
            return Mapped::default();
        }
        let delta = event.get("delta").unwrap_or(&Value::Null);
        match kind(delta, "type") {
            Some(wire::TEXT_DELTA) => {
                let text = text_of(delta, "text");
                self.streamed.push_str(&text);
                Mapped::updates(vec![SessionUpdate::AgentMessageChunk(chunk(&text))])
            }
            Some(wire::THINKING_DELTA) => {
                let text = text_of(delta, "thinking");
                self.streamed.push_str(&text);
                Mapped::updates(vec![SessionUpdate::AgentThoughtChunk(chunk(&text))])
            }
            // A tool's arguments arriving character by character. The whole call
            // lands in the completed message, and half a JSON object is nothing
            // a session can show.
            _ => Mapped::default(),
        }
    }

    /// A whole assistant message: words, thinking and the tools it wants to run.
    fn assistant(&mut self, event: &Value) -> Mapped {
        if let Some(parent) = parent_tool_use(event) {
            return self.subagent(&parent, event);
        }
        let mut updates = Vec::new();
        for block in blocks(event) {
            match kind(block, "type") {
                Some(wire::TEXT) => {
                    let text = text_of(block, "text");
                    if !text.is_empty() && !self.already_said(&text) {
                        updates.push(SessionUpdate::AgentMessageChunk(chunk(&text)));
                    }
                }
                Some(wire::THINKING) => {
                    let text = text_of(block, "thinking");
                    if !text.is_empty() && !self.already_said(&text) {
                        updates.push(SessionUpdate::AgentThoughtChunk(chunk(&text)));
                    }
                }
                Some(wire::TOOL_USE) => updates.push(tool_call(block)),
                _ => {}
            }
        }
        // The message is complete: what streamed of it has been accounted for,
        // and the next one starts from silence.
        self.streamed.clear();
        Mapped::updates(updates)
    }

    /// Everything a subagent said, kept under the tool call that spawned it.
    ///
    /// ACP has no notion of a session inside a session, and inventing top-level
    /// tool calls for work the human never asked for directly would misdescribe
    /// the turn. The transcript is not dropped either — it becomes content of
    /// the call it belongs to, which is where somebody reading the session will
    /// look for it.
    fn subagent(&mut self, parent: &str, event: &Value) -> Mapped {
        let transcript: Vec<String> = blocks(event)
            .iter()
            .filter_map(|block| match kind(block, "type") {
                Some(wire::TEXT) => Some(text_of(block, "text")),
                Some(wire::THINKING) => Some(text_of(block, "thinking")),
                Some(wire::TOOL_USE) => Some(format!("[{}]", title_of(block))),
                _ => None,
            })
            .filter(|line| !line.is_empty())
            .collect();
        if transcript.is_empty() {
            return Mapped::default();
        }
        Mapped::updates(vec![SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
            parent.to_string(),
            ToolCallUpdateFields::new().content(vec![text_content(&transcript.join("\n"))]),
        ))])
    }

    /// What a tool answered, on the call it answered.
    fn tool_results(&mut self, event: &Value) -> Mapped {
        let updates = blocks(event)
            .iter()
            .filter(|block| kind(block, "type") == Some(wire::TOOL_RESULT))
            .filter_map(|block| {
                let id = block.get("tool_use_id").and_then(Value::as_str)?;
                let failed = block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let said = result_text(block.get("content").unwrap_or(&Value::Null));
                let mut fields = ToolCallUpdateFields::new().status(if failed {
                    ToolCallStatus::Failed
                } else {
                    ToolCallStatus::Completed
                });
                if !said.is_empty() {
                    fields = fields.content(vec![text_content(&said)]);
                }
                Some(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                    id.to_string(),
                    fields,
                )))
            })
            .collect();
        Mapped::updates(updates)
    }

    /// Whether this text already reached the session as deltas.
    fn already_said(&self, text: &str) -> bool {
        self.streamed.contains(text)
    }
}

/// The text of a prompt, as this wire takes it.
///
/// Only text blocks: the adapter announces no capability for images or embedded
/// resources, so nothing else can arrive, and silently dropping something it
/// had promised to carry is the failure this shape prevents.
#[must_use]
pub fn prompt_text(prompt: &PromptRequest) -> String {
    prompt
        .prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// How a turn ended.
fn result(event: &Value) -> Mapped {
    let subtype = kind(event, "subtype").unwrap_or_default();
    let failed = event
        .get("is_error")
        .and_then(Value::as_bool)
        .unwrap_or(subtype != wire::SUCCESS);
    if !failed {
        return Mapped::signal(Signal::TurnEnded(StopReason::EndTurn));
    }
    if subtype == wire::ERROR_MAX_TURNS {
        // A budget the CLI itself enforces, and a stop reason ACP has a name
        // for. Reporting it as a failure would send somebody looking for a bug
        // that is not there.
        return Mapped::signal(Signal::TurnEnded(StopReason::MaxTurnRequests));
    }
    let said = event
        .get("result")
        .and_then(Value::as_str)
        .filter(|said| !said.is_empty())
        .map_or_else(|| format!("the turn ended as `{subtype}`"), str::to_string);
    Mapped::signal(Signal::TurnFailed(said))
}

/// A tool call as the session shows it.
fn tool_call(block: &Value) -> SessionUpdate {
    let id = block
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let input = block.get("input").unwrap_or(&Value::Null);
    let name = kind(block, "name").unwrap_or("tool");
    SessionUpdate::ToolCall(
        ToolCall::new(id, title_of(block))
            .kind(tool_kind(name))
            .status(ToolCallStatus::InProgress)
            // The arguments, exactly as the provider sent them: the evidence a
            // human decides on, and nothing added to it.
            .content(if input.is_null() {
                Vec::new()
            } else {
                vec![text_content(&input.to_string())]
            })
            .locations(
                path_in(input)
                    .map(|path| vec![ToolCallLocation::new(path)])
                    .unwrap_or_default(),
            ),
    )
}

/// What a tool call is called in the session: the tool's own name, plus the one
/// argument that says what it is about when there is one.
fn title_of(block: &Value) -> String {
    let name = kind(block, "name").unwrap_or("tool");
    let input = block.get("input").unwrap_or(&Value::Null);
    match ["command", "file_path", "path", "pattern", "url", "prompt"]
        .iter()
        .find_map(|field| input.get(*field).and_then(Value::as_str))
    {
        Some(about) if !about.is_empty() => format!("{name}: {}", first_line(about)),
        _ => name.to_string(),
    }
}

/// The file a tool call is about, when it names one.
fn path_in(input: &Value) -> Option<String> {
    ["file_path", "path", "notebook_path"]
        .iter()
        .find_map(|field| input.get(*field).and_then(Value::as_str))
        .map(str::to_string)
}

/// How a tool's name reads in ACP's vocabulary.
///
/// The names are the provider's, and the list is knowingly incomplete: a tool
/// this table does not know — an MCP tool, or one the CLI grew last week — is
/// shown as what it is rather than guessed at, because a wrong icon is worse
/// than no icon.
fn tool_kind(name: &str) -> ToolKind {
    match name {
        "Read" | "NotebookRead" => ToolKind::Read,
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => ToolKind::Edit,
        "Bash" | "BashOutput" | "KillShell" | "KillBash" => ToolKind::Execute,
        "Glob" | "Grep" => ToolKind::Search,
        "WebFetch" | "WebSearch" => ToolKind::Fetch,
        _ => ToolKind::Other,
    }
}

/// What a tool result said, whether it came as text or as blocks.
fn result_text(content: &Value) -> String {
    match content {
        Value::String(said) => said.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The content blocks of an event's message.
fn blocks(event: &Value) -> Vec<&Value> {
    event
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .map(|blocks| blocks.iter().collect())
        .unwrap_or_default()
}

/// The tool call a subagent's event belongs to.
fn parent_tool_use(event: &Value) -> Option<String> {
    event
        .get(wire::PARENT_TOOL_USE_ID)
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

fn kind<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn text_of(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_string()
}

fn chunk(text: &str) -> ContentChunk {
    ContentChunk::new(ContentBlock::Text(TextContent::new(text)))
}

fn text_content(text: &str) -> ToolCallContent {
    ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new(text))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn said(updates: &[SessionUpdate]) -> Vec<(String, bool)> {
        updates
            .iter()
            .filter_map(|update| match update {
                SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                    ContentBlock::Text(text) => Some((text.text.clone(), false)),
                    _ => None,
                },
                SessionUpdate::AgentThoughtChunk(chunk) => match &chunk.content {
                    ContentBlock::Text(text) => Some((text.text.clone(), true)),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_turn_streams_token_by_token_and_says_nothing_twice() {
        // Scenario: Eventos de sesión mapeados en streaming
        //
        // The wire narrates the same message twice — deltas first, the whole
        // thing after — and a client that showed both would read as the agent
        // repeating itself.
        let mut mapper = Mapper::new();
        for piece in ["Working", " on it."] {
            let mapped = mapper.map(&json!({
                "type": "stream_event",
                "event": {"type": "content_block_delta", "index": 0,
                          "delta": {"type": "text_delta", "text": piece}},
            }));
            assert_eq!(said(&mapped.updates), vec![(piece.to_string(), false)]);
        }

        let whole = mapper.map(&json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": "Working on it."}]},
        }));
        assert!(
            said(&whole.updates).is_empty(),
            "what streamed is not repeated whole: {:?}",
            whole.updates
        );

        // And the other way round: a message that never streamed must not be
        // lost because the provider chose not to chunk it.
        let silent = mapper.map(&json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": "Done."}]},
        }));
        assert_eq!(said(&silent.updates), vec![("Done.".to_string(), false)]);
    }

    #[test]
    fn thinking_is_shown_as_thinking_and_not_as_speech() {
        // Scenario: Eventos de sesión mapeados en streaming
        let mut mapper = Mapper::new();
        let mapped = mapper.map(&json!({
            "type": "stream_event",
            "event": {"type": "content_block_delta", "index": 0,
                      "delta": {"type": "thinking_delta", "thinking": "hmm"}},
        }));
        assert_eq!(said(&mapped.updates), vec![("hmm".to_string(), true)]);

        // A tool's arguments arriving character by character are not something a
        // session can show: half a JSON object is not evidence.
        let partial = mapper.map(&json!({
            "type": "stream_event",
            "event": {"type": "content_block_delta", "index": 1,
                      "delta": {"type": "input_json_delta", "partial_json": "{\"fi"}},
        }));
        assert!(partial.updates.is_empty());
    }

    #[test]
    fn a_tool_call_carries_the_input_it_was_given_and_its_result_updates_it() {
        // Scenario: Eventos de sesión mapeados en streaming
        let mut mapper = Mapper::new();
        let call = mapper.map(&json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "Write",
                 "input": {"file_path": "NOTES.md", "content": "hola"}}
            ]},
        }));
        let SessionUpdate::ToolCall(shown) = &call.updates[0] else {
            panic!("a tool use is a tool call: {call:?}");
        };
        assert_eq!(shown.tool_call_id.0.as_ref(), "toolu_1");
        assert_eq!(shown.kind, ToolKind::Edit);
        assert!(
            shown.title.contains("NOTES.md"),
            "the title says what it is about: {}",
            shown.title
        );
        assert_eq!(
            shown.locations.len(),
            1,
            "so a client can follow along to the file"
        );
        assert!(
            !shown.content.is_empty(),
            "and the arguments travel as the evidence they are"
        );

        let answered = mapper.map(&json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "ok"}
            ]},
        }));
        let SessionUpdate::ToolCallUpdate(update) = &answered.updates[0] else {
            panic!("its result updates the same call: {answered:?}");
        };
        assert_eq!(update.tool_call_id.0.as_ref(), "toolu_1");
        assert_eq!(update.fields.status, Some(ToolCallStatus::Completed));

        let failed = mapper.map(&json!({
            "type": "user",
            "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1",
                 "content": [{"type": "text", "text": "no such file"}], "is_error": true}
            ]},
        }));
        let SessionUpdate::ToolCallUpdate(update) = &failed.updates[0] else {
            panic!("a failed result is still an update: {failed:?}");
        };
        assert_eq!(update.fields.status, Some(ToolCallStatus::Failed));
    }

    #[test]
    fn a_tool_this_adapter_has_no_kind_for_still_appears_by_its_own_name() {
        // The provider grows tools, and MCP servers bring their own. One that
        // vanished would erase evidence exactly when a session is going wrong.
        let mut mapper = Mapper::new();
        let call = mapper.map(&json!({
            "type": "assistant",
            "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_9", "name": "mcp__fs__list", "input": {}}
            ]},
        }));
        let SessionUpdate::ToolCall(shown) = &call.updates[0] else {
            panic!("the unknown tool is shown, not dropped: {call:?}");
        };
        assert_eq!(shown.title, "mcp__fs__list");
        assert_eq!(shown.kind, ToolKind::Other);
    }

    #[test]
    fn a_subagents_transcript_is_kept_under_the_call_that_spawned_it() {
        // Scenario: Eventos de sesión mapeados en streaming
        //
        // ACP has no session inside a session. Inventing top-level tool calls
        // for work nobody asked for directly would misdescribe the turn; losing
        // the transcript would lose the part of the turn that explains it.
        let mut mapper = Mapper::new();
        let mapped = mapper.map(&json!({
            "type": "assistant",
            "parent_tool_use_id": "toolu_task_1",
            "message": {"role": "assistant", "content": [
                {"type": "text", "text": "Looking at the tests."},
                {"type": "tool_use", "id": "toolu_sub", "name": "Grep", "input": {"pattern": "fn main"}}
            ]},
        }));
        let SessionUpdate::ToolCallUpdate(update) = &mapped.updates[0] else {
            panic!("the subagent's words belong to its parent call: {mapped:?}");
        };
        assert_eq!(update.tool_call_id.0.as_ref(), "toolu_task_1");
        let content = update.fields.content.as_ref().expect("the transcript");
        assert_eq!(content.len(), 1);
        assert!(
            format!("{:?}", content[0]).contains("Looking at the tests."),
            "the subagent's words are kept: {content:?}"
        );
    }

    #[test]
    fn a_turn_ends_on_the_result_and_a_limit_is_not_a_failure() {
        let mut mapper = Mapper::new();
        assert_eq!(
            mapper
                .map(
                    &json!({"type": "result", "subtype": "success", "is_error": false,
                             "result": "Done."})
                )
                .signal,
            Some(Signal::TurnEnded(StopReason::EndTurn))
        );
        assert_eq!(
            mapper
                .map(&json!({"type": "result", "subtype": "error_max_turns", "is_error": true}))
                .signal,
            Some(Signal::TurnEnded(StopReason::MaxTurnRequests)),
            "a budget the CLI enforces has a stop reason of its own"
        );
        let failed = mapper.map(&json!({
            "type": "result", "subtype": "error_during_execution", "is_error": true,
            "result": "the tool crashed"
        }));
        assert_eq!(
            failed.signal,
            Some(Signal::TurnFailed("the tool crashed".into())),
            "and every other error ends the turn in the provider's own words"
        );
    }

    #[test]
    fn an_event_this_adapter_does_not_map_changes_nothing() {
        let mut mapper = Mapper::new();
        assert_eq!(
            mapper.map(&json!({"type": "something_new"})),
            Mapped::default()
        );
        assert!(
            mapper
                .map(&json!({"type": "system", "subtype": "compact_boundary"}))
                .noted
                .is_some(),
            "but the CLI talking about itself stays on the record"
        );
    }
}
