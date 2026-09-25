// SPDX-License-Identifier: Apache-2.0

//! Between the server's conversation and the ACP session
//! (adaptadores-propios-acp task 2.3).
//!
//! The server streams a turn as a thread of items — a message arriving in
//! chunks, a command running, files changing — and ACP streams a turn as
//! session updates. This module is the whole of that translation, and it is
//! deliberately free of I/O: it takes a notification and answers with what the
//! session should show and whether the turn is over, which is what lets every
//! rule below be tested without a process, a pipe or a clock.
//!
//! Three rules earn their keep here:
//!
//! - **Nothing is said twice.** A message that arrived as chunks is not
//!   repeated whole when its item completes; a message that never streamed a
//!   chunk *is*, because otherwise it would be lost.
//! - **Nothing is dropped.** An item of a kind this adapter does not know still
//!   appears, by the name the provider gave it. An unknown item that vanished
//!   would erase evidence exactly when a session is going wrong.
//! - **Nothing is invented.** The provider hands a unified diff; ACP's diff
//!   update wants the file before and after. Rather than fabricate those, the
//!   diff travels as what it is — text — with the path alongside it so clients
//!   can still follow along.

use std::collections::HashSet;

use agent_client_protocol::schema::v1::{
    Content, ContentBlock, ContentChunk, MessageId, PromptRequest, SessionUpdate, TextContent,
    ToolCall, ToolCallContent, ToolCallLocation, ToolCallStatus, ToolCallUpdate,
    ToolCallUpdateFields, ToolKind,
};
use serde_json::Value;

use super::wire::{self, ItemStatus, ThreadItem};

/// What a notification says about the turn itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// The turn began, and this is the id an interruption must name.
    TurnStarted(String),
    /// The turn is over, this is how it ended.
    TurnEnded(wire::TurnStatus),
    /// The turn failed and the server is not going to retry.
    TurnFailed(String),
    /// The CLI reports a remote control that is not off, in this state: another
    /// client could answer this thread's approvals outside the permission proxy
    /// (codex-al-dia design D5).
    RemoteControl(String),
}

/// One notification, translated.
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

/// Translates one turn's notifications, remembering what has already been said.
#[derive(Debug, Default)]
pub struct Mapper {
    /// Items whose text already reached the session as chunks.
    streamed: HashSet<String>,
    /// MCP servers of the CLI's own configuration already named in the
    /// session. The mapper lives as long as the session, so each is named once.
    announced_servers: HashSet<String>,
    /// The CLI's remote control, when the last report said it is not off.
    remote_control: Option<String>,
}

impl Mapper {
    /// A mapper for a fresh session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What one server notification becomes.
    ///
    /// A method this adapter does not map answers with nothing at all: the
    /// server says a great deal that an ACP session has no place for, and
    /// inventing a place for it would be worse than silence.
    pub fn map(&mut self, method: &str, params: &Value) -> Mapped {
        match method {
            wire::TURN_STARTED => match parse::<wire::TurnNotification>(params) {
                Some(turn) => Mapped::signal(Signal::TurnStarted(turn.turn.id)),
                None => Mapped::noted(format!("unreadable `{method}`: {params}")),
            },
            wire::TURN_COMPLETED => match parse::<wire::TurnNotification>(params) {
                Some(turn) => Mapped::signal(Signal::TurnEnded(turn.turn.status)),
                None => Mapped::noted(format!("unreadable `{method}`: {params}")),
            },
            wire::ERROR => self.error(params),
            wire::AGENT_MESSAGE_DELTA => self.chunk(params, false),
            wire::REASONING_TEXT_DELTA | wire::REASONING_SUMMARY_TEXT_DELTA => {
                self.chunk(params, true)
            }
            wire::ITEM_STARTED => self.item(params, false),
            wire::ITEM_COMPLETED => self.item(params, true),
            wire::MCP_SERVER_STARTUP_STATUS => self.mcp_server(params),
            wire::REMOTE_CONTROL_STATUS_CHANGED => self.remote_control_changed(params),
            _ => Mapped::default(),
        }
    }

    /// An MCP server the CLI starts from its own configuration: named in the
    /// session the first time it is reported, and never again (codex-al-dia
    /// design D5). It runs outside the CLI's sandbox, and the calls the CLI
    /// does not submit for approval never reach the proxy; saying so is what
    /// keeps that loss visible.
    fn mcp_server(&mut self, params: &Value) -> Mapped {
        let Some(server) = parse::<wire::McpServerStartupStatus>(params) else {
            return Mapped::noted(format!("unreadable MCP server status: {params}"));
        };
        if !self.announced_servers.insert(server.name.clone()) {
            return Mapped::default();
        }
        Mapped::updates(vec![SessionUpdate::ToolCall(
            ToolCall::new(
                format!("meltemi:mcp-server:{}", server.name),
                format!(
                    "MCP server `{}` from the Codex configuration: outside the permission proxy",
                    server.name
                ),
            )
            .kind(ToolKind::Other)
            .status(ToolCallStatus::Completed)
            .content(vec![text_content(
                "Codex starts this server from its own configuration. It runs outside Codex's \
                 sandbox, and the calls Codex does not submit for approval run without Meltemi's \
                 permission proxy deciding.",
            )]),
        )])
    }

    /// The CLI's remote control changing state (codex-al-dia design D5).
    ///
    /// Off is the only state a session runs under. Anything else — including
    /// a report this adapter cannot read — is remembered and signalled, so the
    /// check fails closed; a later report that it is off clears it.
    fn remote_control_changed(&mut self, params: &Value) -> Mapped {
        let Some(report) = parse::<wire::RemoteControlStatus>(params) else {
            self.remote_control = Some("unreadable".to_string());
            return Mapped {
                signal: Some(Signal::RemoteControl("unreadable".to_string())),
                noted: Some(format!("unreadable remote control status: {params}")),
                ..Mapped::default()
            };
        };
        if report.status == wire::REMOTE_CONTROL_DISABLED {
            self.remote_control = None;
            return Mapped::default();
        }
        self.remote_control = Some(report.status.clone());
        Mapped::signal(Signal::RemoteControl(report.status))
    }

    /// The CLI's remote control, if the last report said it is not off.
    #[must_use]
    pub fn remote_control(&self) -> Option<&str> {
        self.remote_control.as_deref()
    }

    /// A streamed chunk of an item's text.
    fn chunk(&mut self, params: &Value, thought: bool) -> Mapped {
        let Some(delta) = parse::<wire::DeltaNotification>(params) else {
            return Mapped::noted(format!("unreadable delta: {params}"));
        };
        // Remembered so the item's completion does not say the same thing
        // again, whole, under the message it already streamed.
        self.streamed.insert(delta.item_id.clone());
        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(delta.delta)))
            .message_id(MessageId::new(delta.item_id));
        Mapped::updates(vec![if thought {
            SessionUpdate::AgentThoughtChunk(chunk)
        } else {
            SessionUpdate::AgentMessageChunk(chunk)
        }])
    }

    /// An item appearing or reaching its final form.
    fn item(&mut self, params: &Value, completed: bool) -> Mapped {
        let Some(notification) = parse::<wire::ItemNotification>(params) else {
            return Mapped::noted(format!("unreadable item: {params}"));
        };
        let raw = notification.item;
        let Some(item) = serde_json::from_value::<ThreadItem>(raw.clone()).ok() else {
            return Mapped::noted(format!("unreadable item: {raw}"));
        };

        // A message is text, not a tool call. It reaches the session as chunks
        // while it streams; whole, and only once, if it never did.
        if let ThreadItem::AgentMessage { id, text } = &item {
            if !completed || self.streamed.contains(id) || text.is_empty() {
                return Mapped::default();
            }
            return Mapped::updates(vec![SessionUpdate::AgentMessageChunk(
                ContentChunk::new(ContentBlock::Text(TextContent::new(text.clone())))
                    .message_id(MessageId::new(id.clone())),
            )]);
        }
        // Reasoning arrives as chunks too, and its completed form carries a
        // structure this adapter does not map. Repeating it would be noise.
        //
        // The user's message comes back from the server as an item of the turn.
        // What the user said is already on the record — the daemon wrote the
        // prompt before the turn began — and shown here it would be a tool call
        // that is really the prompt, once per turn.
        if matches!(
            item,
            ThreadItem::Reasoning { .. } | ThreadItem::UserMessage { .. }
        ) {
            return Mapped::default();
        }

        let Some(rendered) = render(&item, &raw, completed) else {
            return Mapped::default();
        };
        Mapped::updates(vec![if completed {
            SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                rendered.id,
                ToolCallUpdateFields::new()
                    .kind(rendered.kind)
                    .status(rendered.status)
                    .title(rendered.title)
                    .content(rendered.content)
                    .locations(rendered.locations),
            ))
        } else {
            SessionUpdate::ToolCall(
                ToolCall::new(rendered.id, rendered.title)
                    .kind(rendered.kind)
                    .status(rendered.status)
                    .content(rendered.content)
                    .locations(rendered.locations),
            )
        }])
    }

    /// The server hit an error.
    ///
    /// When it says it will retry, the turn is **not** over and closing it here
    /// would cut the agent off mid-thought and report it as done. The error
    /// still goes on the record — on stderr, where the daemon already collects
    /// what the provider says — rather than being dressed up as something the
    /// agent told the user.
    fn error(&mut self, params: &Value) -> Mapped {
        let Some(error) = parse::<wire::ErrorNotification>(params) else {
            return Mapped::noted(format!("unreadable error: {params}"));
        };
        if error.will_retry {
            return Mapped::noted(format!(
                "the provider hit an error and will retry: {}",
                error.error.message
            ));
        }
        Mapped::signal(Signal::TurnFailed(error.error.message))
    }
}

/// The row that opens every session: what Codex still runs without asking the
/// permission proxy under the strictest gate it offers (codex-al-dia design
/// D5). The hard-gate requirement promises visible losses; this is the one this
/// provider cannot close from the adapter.
#[must_use]
pub fn uncovered_by_gate() -> SessionUpdate {
    SessionUpdate::ToolCall(
        ToolCall::new(
            "meltemi:gate",
            "Codex runs some things without asking the permission proxy",
        )
        .kind(ToolKind::Other)
        .status(ToolCallStatus::Completed)
        .content(vec![text_content(
            "Under the strictest gate Codex offers (approval policy `untrusted`, a read-only \
             sandbox, approvals sent to Meltemi), these still run without a decision from the \
             permission proxy: commands Codex trusts as read-only, inside its read-only sandbox; \
             the provider's own web search; and calls to MCP servers from Codex's configuration \
             that Codex does not submit for approval, which run outside its sandbox.",
        )]),
    )
}

/// The row that declares a CLI newer than the version this adapter was
/// verified against (codex-al-dia design D2): the session opens, and says so
/// before the first turn instead of presenting itself as verified.
#[must_use]
pub fn beyond_verified(effective: &str, verified: &str) -> SessionUpdate {
    SessionUpdate::ToolCall(
        ToolCall::new(
            "meltemi:provider-version",
            format!(
                "Codex {effective} is newer than {verified}, the version this adapter was verified \
                 against"
            ),
        )
        .kind(ToolKind::Other)
        .status(ToolCallStatus::Completed)
        .content(vec![text_content(&format!(
            "The session opens under the same gate, which is still checked on every thread and \
             refuses any value it does not know. What changed in the wire after {verified} has not \
             been verified; if something here looks wrong, say that Codex {effective} answered."
        ))]),
    )
}

/// The text of a prompt, as the wire takes it.
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

/// An item as a tool call: everything ACP needs to show it.
struct Rendered {
    id: String,
    title: String,
    kind: ToolKind,
    status: ToolCallStatus,
    content: Vec<ToolCallContent>,
    locations: Vec<ToolCallLocation>,
}

/// What an item looks like in the session, or `None` when it is not something
/// the session shows.
fn render(item: &ThreadItem, raw: &Value, completed: bool) -> Option<Rendered> {
    let default_status = if completed {
        ToolCallStatus::Completed
    } else {
        ToolCallStatus::InProgress
    };
    Some(match item {
        ThreadItem::CommandExecution {
            id,
            command,
            status,
            aggregated_output,
            exit_code,
        } => Rendered {
            id: id.clone(),
            title: command.clone(),
            kind: ToolKind::Execute,
            status: tool_status(*status),
            content: aggregated_output
                .as_ref()
                .filter(|output| !output.is_empty())
                .map(|output| vec![text_content(output)])
                .unwrap_or_default(),
            locations: exit_code.map(|_| Vec::new()).unwrap_or_default(),
        },
        ThreadItem::FileChange {
            id,
            status,
            changes,
        } => Rendered {
            id: id.clone(),
            title: file_change_title(changes),
            kind: ToolKind::Edit,
            status: tool_status(*status),
            content: changes
                .iter()
                .map(|change| text_content(&format!("{}\n{}", change.path, change.diff)))
                .collect(),
            locations: changes
                .iter()
                .map(|change| ToolCallLocation::new(change.path.clone()))
                .collect(),
        },
        ThreadItem::McpToolCall {
            id,
            server,
            tool,
            status,
        } => Rendered {
            id: id.clone(),
            title: format!("{server}/{tool}"),
            kind: ToolKind::Other,
            status: tool_status(*status),
            content: Vec::new(),
            locations: Vec::new(),
        },
        ThreadItem::WebSearch { id, query } => Rendered {
            id: id.clone(),
            title: query.clone(),
            kind: ToolKind::Fetch,
            status: default_status,
            content: Vec::new(),
            locations: Vec::new(),
        },
        // A kind this adapter does not map yet still appears, under the name
        // the provider gave it.
        ThreadItem::Unrecognised => Rendered {
            id: raw.get("id").and_then(Value::as_str)?.to_string(),
            title: raw
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            kind: ToolKind::Other,
            status: default_status,
            content: Vec::new(),
            locations: Vec::new(),
        },
        // Handled before this point: they are text, not tool calls.
        ThreadItem::AgentMessage { .. }
        | ThreadItem::Reasoning { .. }
        | ThreadItem::UserMessage { .. } => return None,
    })
}

/// How the provider's status reads in ACP's vocabulary.
///
/// `declined` becomes `failed` for want of anything truer: ACP has no status
/// for "the human said no", and the item did not run. What actually happened is
/// never hidden — the denial is the proxy's own decision, recorded where
/// decisions are recorded.
fn tool_status(status: ItemStatus) -> ToolCallStatus {
    match status {
        ItemStatus::InProgress => ToolCallStatus::InProgress,
        ItemStatus::Completed => ToolCallStatus::Completed,
        ItemStatus::Failed | ItemStatus::Declined => ToolCallStatus::Failed,
    }
}

fn text_content(text: &str) -> ToolCallContent {
    ToolCallContent::Content(Content::new(ContentBlock::Text(TextContent::new(text))))
}

fn file_change_title(changes: &[wire::FileUpdateChange]) -> String {
    match changes {
        [] => "Change files".to_string(),
        [one] => one.path.clone(),
        many => format!("{} and {} more", many[0].path, many.len() - 1),
    }
}

fn parse<T: serde::de::DeserializeOwned>(params: &Value) -> Option<T> {
    serde_json::from_value(params.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text_of(update: &SessionUpdate) -> Option<(&str, bool)> {
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                ContentBlock::Text(text) => Some((text.text.as_str(), false)),
                _ => None,
            },
            SessionUpdate::AgentThoughtChunk(chunk) => match &chunk.content {
                ContentBlock::Text(text) => Some((text.text.as_str(), true)),
                _ => None,
            },
            _ => None,
        }
    }

    #[test]
    fn a_turn_streams_its_chunks_and_closes_on_the_status_the_server_gives_it() {
        // Scenario: Conversación del servidor mapeada en streaming
        let mut mapper = Mapper::new();

        assert_eq!(
            mapper
                .map(
                    wire::TURN_STARTED,
                    &json!({"threadId": "t", "turn": {"id": "turn-1", "status": "inProgress", "items": []}})
                )
                .signal,
            Some(Signal::TurnStarted("turn-1".into())),
            "the turn id an interruption will have to name"
        );

        let streamed = mapper.map(
            wire::AGENT_MESSAGE_DELTA,
            &json!({"threadId": "t", "turnId": "turn-1", "itemId": "item-1", "delta": "Working "}),
        );
        assert_eq!(text_of(&streamed.updates[0]), Some(("Working ", false)));

        let thinking = mapper.map(
            wire::REASONING_TEXT_DELTA,
            &json!({"threadId": "t", "turnId": "turn-1", "itemId": "item-9", "contentIndex": 0, "delta": "hmm"}),
        );
        assert_eq!(
            text_of(&thinking.updates[0]),
            Some(("hmm", true)),
            "reasoning is the agent thinking, not the agent speaking"
        );

        assert_eq!(
            mapper
                .map(
                    wire::TURN_COMPLETED,
                    &json!({"threadId": "t", "turn": {"id": "turn-1", "status": "completed", "items": []}})
                )
                .signal,
            Some(Signal::TurnEnded(wire::TurnStatus::Completed))
        );
    }

    #[test]
    fn a_message_is_said_once_whether_or_not_it_streamed() {
        // The trap this rule exists for: a client that shows the chunks and
        // then the whole message again reads as the agent repeating itself.
        let mut mapper = Mapper::new();
        let item = json!({
            "threadId": "t", "turnId": "turn-1",
            "item": {"type": "agentMessage", "id": "item-1", "text": "Working on it."}
        });

        mapper.map(
            wire::AGENT_MESSAGE_DELTA,
            &json!({"threadId": "t", "turnId": "turn-1", "itemId": "item-1", "delta": "Working on it."}),
        );
        assert!(
            mapper.map(wire::ITEM_COMPLETED, &item).updates.is_empty(),
            "what streamed is not repeated whole"
        );

        // And the other way round: a message that never streamed must not be
        // lost just because the provider chose not to chunk it.
        let mut silent = Mapper::new();
        assert!(silent.map(wire::ITEM_STARTED, &item).updates.is_empty());
        let completed = silent.map(wire::ITEM_COMPLETED, &item);
        assert_eq!(
            text_of(&completed.updates[0]),
            Some(("Working on it.", false))
        );
    }

    #[test]
    fn items_become_tool_calls_and_their_completion_updates_them() {
        // Scenario: Conversación del servidor mapeada en streaming
        let mut mapper = Mapper::new();
        let running = mapper.map(
            wire::ITEM_STARTED,
            &json!({"threadId": "t", "turnId": "turn-1", "item": {
                "type": "commandExecution", "id": "item-2", "command": "cargo test",
                "cwd": "/p", "commandActions": [], "status": "inProgress"
            }}),
        );
        let SessionUpdate::ToolCall(call) = &running.updates[0] else {
            panic!("a command is a tool call: {running:?}");
        };
        assert_eq!(call.title, "cargo test");
        assert_eq!(call.kind, ToolKind::Execute);
        assert_eq!(call.status, ToolCallStatus::InProgress);

        let done = mapper.map(
            wire::ITEM_COMPLETED,
            &json!({"threadId": "t", "turnId": "turn-1", "item": {
                "type": "commandExecution", "id": "item-2", "command": "cargo test",
                "cwd": "/p", "commandActions": [], "status": "completed",
                "aggregatedOutput": "ok\n", "exitCode": 0
            }}),
        );
        let SessionUpdate::ToolCallUpdate(update) = &done.updates[0] else {
            panic!("its completion updates the same call: {done:?}");
        };
        assert_eq!(update.tool_call_id.0.as_ref(), "item-2");
        assert_eq!(update.fields.status, Some(ToolCallStatus::Completed));

        // A file change carries its diff as text and its path as a location:
        // ACP's diff update wants the file before and after, which the provider
        // never sends, and fabricating them would misdescribe the change.
        let edit = mapper.map(
            wire::ITEM_COMPLETED,
            &json!({"threadId": "t", "turnId": "turn-1", "item": {
                "type": "fileChange", "id": "item-3", "status": "completed",
                "changes": [{"path": "NOTES.md", "kind": {"type": "add"}, "diff": "@@\n+hola\n"}]
            }}),
        );
        let SessionUpdate::ToolCallUpdate(update) = &edit.updates[0] else {
            panic!("a file change is a tool call: {edit:?}");
        };
        assert_eq!(update.fields.kind, Some(ToolKind::Edit));
        assert_eq!(
            update.fields.locations.as_ref().map(Vec::len),
            Some(1),
            "so a client can follow along to the file"
        );
    }

    #[test]
    fn what_the_gate_does_not_cover_is_said_before_anything_else() {
        // Scenario: Lo que la compuerta del CLI no cubre se declara al abrir
        //
        // The row the session opens with (the adapter sends it before the
        // first turn; the daemon's end-to-end test checks the order).
        let SessionUpdate::ToolCall(row) = uncovered_by_gate() else {
            panic!("the losses are a row the surfaces already draw");
        };
        assert_eq!(row.tool_call_id.0.as_ref(), "meltemi:gate");
        assert_eq!(row.kind, ToolKind::Other);
        assert_eq!(row.status, ToolCallStatus::Completed);
        let said = format!("{:?}", row.content);
        for loss in ["read-only", "web search", "MCP servers"] {
            assert!(said.contains(loss), "the row names `{loss}`: {said}");
        }
    }

    #[test]
    fn an_mcp_server_the_cli_starts_is_named_once_per_session() {
        // Scenario: Un servidor de herramientas que el CLI arranca se nombra
        //
        // As 0.156.1 reports it (recorded 2026-09-24): starting, then ready, and
        // again in a later turn. One row, the first time.
        let mut mapper = Mapper::new();
        let report = |status: &str| json!({"threadId": "t", "name": "node_repl", "status": status, "error": null, "failureReason": null});
        let first = mapper.map(wire::MCP_SERVER_STARTUP_STATUS, &report("starting"));
        let [SessionUpdate::ToolCall(row)] = first.updates.as_slice() else {
            panic!("the server is named: {first:?}");
        };
        assert_eq!(row.tool_call_id.0.as_ref(), "meltemi:mcp-server:node_repl");
        assert!(row.title.contains("node_repl") && row.title.contains("permission proxy"));
        for status in ["ready", "ready"] {
            let again = mapper.map(wire::MCP_SERVER_STARTUP_STATUS, &report(status));
            assert!(again.updates.is_empty(), "never twice: {again:?}");
        }
        let other = mapper.map(
            wire::MCP_SERVER_STARTUP_STATUS,
            &json!({"threadId": "t", "name": "codex_apps", "status": "starting"}),
        );
        assert_eq!(other.updates.len(), 1, "another server is another row");
    }

    #[test]
    fn a_newer_cli_is_declared_with_both_versions() {
        // Scenario: Versión posterior a la verificada declarada, no rehusada
        let SessionUpdate::ToolCall(row) = beyond_verified("0.157.0", "0.156.1") else {
            panic!("a visible row");
        };
        assert_eq!(row.tool_call_id.0.as_ref(), "meltemi:provider-version");
        assert!(row.title.contains("0.157.0") && row.title.contains("0.156.1"));
        assert!(!row.title.contains("  "), "{}", row.title);
    }

    #[test]
    fn a_remote_control_that_is_not_off_is_signalled_and_remembered() {
        // Scenario: Un control remoto activo rehúsa la sesión
        //
        // As every 0.156.1 connection reports it (recorded 2026-09-24): off,
        // and then nothing to say.
        let mut mapper = Mapper::new();
        let report = |status: &str| json!({"status": status, "serverName": "m", "installationId": "i", "environmentId": null});
        let off = mapper.map(wire::REMOTE_CONTROL_STATUS_CHANGED, &report("disabled"));
        assert_eq!(off, Mapped::default());
        assert_eq!(mapper.remote_control(), None);

        // Every other state the dump declares, and one it does not yet.
        for status in ["connecting", "connected", "errored", "a-state-added-later"] {
            let on = mapper.map(wire::REMOTE_CONTROL_STATUS_CHANGED, &report(status));
            assert_eq!(on.signal, Some(Signal::RemoteControl(status.to_string())));
            assert!(on.updates.is_empty(), "the refusal says it, not a row");
            assert_eq!(mapper.remote_control(), Some(status));
        }

        // Off again, and the session may carry on.
        mapper.map(wire::REMOTE_CONTROL_STATUS_CHANGED, &report("disabled"));
        assert_eq!(mapper.remote_control(), None);

        // A report that cannot be read fails closed.
        let unreadable = mapper.map(
            wire::REMOTE_CONTROL_STATUS_CHANGED,
            &json!({"serverName": "m"}),
        );
        assert!(matches!(unreadable.signal, Some(Signal::RemoteControl(_))));
        assert!(unreadable.noted.is_some());
        assert!(mapper.remote_control().is_some());
    }

    #[test]
    fn the_echo_of_the_users_own_message_is_not_shown() {
        // Scenario: El eco del mensaje del usuario no se muestra
        //
        // Exactly as 0.156.1 emits it, measured on 2026-09-24: at the start of
        // every turn, started and completed, with the prompt as content.
        let mut mapper = Mapper::new();
        let item = json!({
            "type": "userMessage", "id": "item-0", "clientId": null,
            "content": [{"type": "text", "text": "write the proposal", "text_elements": []}]
        });
        for (method, stamp) in [
            (wire::ITEM_STARTED, "startedAtMs"),
            (wire::ITEM_COMPLETED, "completedAtMs"),
        ] {
            let mapped = mapper.map(
                method,
                &json!({"threadId": "t", "turnId": "turn-1", "item": item, stamp: 1_790_000_001_100_u64}),
            );
            assert!(
                mapped.updates.is_empty(),
                "the prompt is already on the record, not a tool call: {mapped:?}"
            );
            assert!(mapped.noted.is_none() && mapped.signal.is_none());
        }
    }

    #[test]
    fn an_item_of_an_unknown_kind_still_appears_by_the_name_the_provider_gave_it() {
        // The provider grows item kinds. One that vanished would erase evidence
        // exactly when a session is going wrong.
        let mut mapper = Mapper::new();
        let mapped = mapper.map(
            wire::ITEM_STARTED,
            &json!({"threadId": "t", "turnId": "turn-1",
                    "item": {"type": "somethingNew", "id": "item-7"}}),
        );
        let SessionUpdate::ToolCall(call) = &mapped.updates[0] else {
            panic!("the unknown item is shown, not dropped: {mapped:?}");
        };
        assert_eq!(call.title, "somethingNew");
        assert_eq!(call.tool_call_id.0.as_ref(), "item-7");
    }

    #[test]
    fn a_retriable_error_does_not_end_the_turn_and_is_not_dressed_up_as_speech() {
        // The provider retries on its own. Closing the turn here would cut the
        // agent off mid-thought and report it as done.
        let mut mapper = Mapper::new();
        let retrying = mapper.map(
            wire::ERROR,
            &json!({"threadId": "t", "turnId": "turn-1", "willRetry": true,
                    "error": {"message": "upstream hiccup"}}),
        );
        assert_eq!(retrying.signal, None);
        assert!(retrying.updates.is_empty());
        assert!(
            retrying.noted.unwrap().contains("upstream hiccup"),
            "but it is on the record"
        );

        let fatal = mapper.map(
            wire::ERROR,
            &json!({"threadId": "t", "turnId": "turn-1", "willRetry": false,
                    "error": {"message": "context window exceeded"}}),
        );
        assert_eq!(
            fatal.signal,
            Some(Signal::TurnFailed("context window exceeded".into()))
        );
    }

    #[test]
    fn a_method_this_adapter_does_not_map_changes_nothing() {
        let mut mapper = Mapper::new();
        assert_eq!(
            mapper.map(
                "thread/tokenUsage/updated",
                &json!({"threadId": "t", "usage": {}})
            ),
            Mapped::default()
        );
    }
}
