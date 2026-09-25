// SPDX-License-Identifier: Apache-2.0

//! The terminal approver (aprobador-en-el-cli): what a subcommand does with the
//! permission requests the daemon pushes to it.
//!
//! This module is the pure core — the question drawn from what the agent sent
//! (design D2), what each key means (D3), the rule remembering may write and
//! when it must not be offered (D5), whether a request still stands (D6), and
//! the choices it shares with the shell's tray — so every rule of it is
//! testable without a terminal or a daemon.

pub mod ask;
pub mod keys;

use std::collections::{HashMap, VecDeque};

use meltemi_proto::{
    PendingPermission, PermissionOption, PermissionOptionKind, PermissionRequestParams,
    PermissionRule, PermissionRuleEffect, PermissionRuleScope,
};
use serde_json::{Map, Value};

pub use ask::Approver;

/// A key as the approver reads it, already translated from whatever the
/// terminal sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character typed without Ctrl or Alt.
    Char(char),
    /// Escape.
    Esc,
    /// Enter, which never decides anything: there is no default option.
    Enter,
    /// Ctrl+C, which in raw mode arrives as a key rather than as a signal.
    CtrlC,
    /// Pasted text, which is never an answer.
    Paste,
    /// Anything else a terminal can send.
    Other,
}

/// What a key means for an open question (design D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Choose the agent's option with this id.
    Choose(String),
    /// Choose this option once a second key confirms it: with it the agent
    /// stops asking about similar requests, beyond Meltemi's proxy.
    ChooseConfirmed(String),
    /// Deny with the agent's reject option, or cancel when it offered none.
    Deny(Option<String>),
    /// Approve once and remember the decision as a rule, once a second key
    /// confirms it.
    Remember,
    /// Stop: cancel the session, and end the command with `13`.
    Stop,
    /// No effect: the question stays open.
    Nothing,
}

/// What the key after a confirmation prompt means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// `y`: go ahead.
    Confirm,
    /// Ctrl+C still stops the session.
    Stop,
    /// Any other key goes back to the question.
    Back,
}

/// The facts of a tool call as the daemon's rules read them: the tool is the
/// kind or else the title, the command is `rawInput.command`, the path is
/// `rawInput.path` or else the first location (`RequestFacts` in the daemon).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub tool: Option<String>,
    pub command: Option<String>,
    pub path: Option<String>,
}

impl Facts {
    /// Reads the facts of an ACP tool call, treating anything absent or empty
    /// as no fact on that dimension.
    #[must_use]
    pub fn of(call: &Value) -> Self {
        let raw_input = call.get("rawInput");
        Self {
            tool: text(call.get("kind")).or_else(|| text(call.get("title"))),
            command: text(raw_input.and_then(|input| input.get("command"))),
            path: text(raw_input.and_then(|input| input.get("path"))).or_else(|| {
                text(
                    call.get("locations")
                        .and_then(Value::as_array)
                        .and_then(|locations| locations.first())
                        .and_then(|location| location.get("path")),
                )
            }),
        }
    }
}

/// A non-empty string at `value`.
fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// One question: what the agent sent, and nothing more (design D2). Every
/// text in it is the agent's, carried as sent and cleaned only when printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// The Meltemi session that asks.
    pub session_id: String,
    /// The id the request holds in the daemon's queue, when the daemon sends
    /// it (a daemon older than this change does not).
    pub request_id: Option<String>,
    /// The agent's id for the tool call, when it gave one.
    pub tool_call_id: Option<String>,
    /// What the daemon's rules read from the push itself: the only facts a
    /// remembered rule may be built from.
    pub facts: Facts,
    /// What is shown: the push completed with what the session streamed for
    /// the same tool call.
    pub shown: Facts,
    /// The agent's title for the operation.
    pub title: Option<String>,
    /// The text of the call's content, line by line.
    pub content: Vec<String>,
    /// The agent's options, in its order.
    pub options: Vec<PermissionOption>,
}

impl Question {
    /// The question a push asks. `streamed` is the last `tool_call` or
    /// `tool_call_update` the session sent for the same id, when this
    /// connection saw one: some agents send the command only there.
    #[must_use]
    pub fn from_push(push: &PermissionRequestParams, streamed: Option<&Value>) -> Self {
        let call = merge_call(streamed, &push.tool_call);
        Self {
            session_id: push.session_id.clone(),
            request_id: push.request_id.clone(),
            tool_call_id: text(push.tool_call.get("toolCallId")),
            facts: Facts::of(&push.tool_call),
            shown: Facts::of(&call),
            title: text(call.get("title")),
            content: content_lines(&call),
            options: push.options.clone(),
        }
    }

    /// Whether anything says what the call would do beyond its tool.
    #[must_use]
    pub fn names_anything(&self) -> bool {
        self.title.is_some()
            || self.shown.command.is_some()
            || self.shown.path.is_some()
            || !self.content.is_empty()
    }

    /// The question as the terminal shows it, with what `r` would do.
    #[must_use]
    pub fn render(&self, remember: &Remember) -> String {
        let mut text = format!(
            "permission requested by session {}\n",
            clean(&self.session_id)
        );
        let tool = self
            .shown
            .tool
            .as_deref()
            .map_or_else(|| "(not named by the agent)".to_string(), clean);
        text.push_str(&format!("  tool     {tool}\n"));
        // The title only when it adds something: without a kind, the tool
        // already is the title.
        if let Some(title) = self
            .title
            .as_deref()
            .filter(|title| Some(*title) != self.shown.tool.as_deref())
        {
            text.push_str(&format!("  title    {}\n", clean(title)));
        }
        if let Some(command) = &self.shown.command {
            text.push_str(&format!("  command  {}\n", clean(command)));
        }
        if let Some(path) = &self.shown.path {
            text.push_str(&format!("  path     {}\n", clean(path)));
        }
        for (index, line) in self.content.iter().enumerate() {
            let label = if index == 0 { "content" } else { "" };
            text.push_str(&format!("  {label:<7}  {}\n", clean(line)));
        }
        if !self.names_anything() {
            text.push_str("  (the agent did not say what it would do)\n");
        }
        for (index, option) in self.options.iter().enumerate() {
            // Only the first nine answer to a digit; the rest are still shown,
            // so nothing the agent offered is hidden.
            let key = if index < 9 {
                (index + 1).to_string()
            } else {
                " ".to_string()
            };
            text.push_str(&format!(
                "  {key} {}  ({})\n",
                clean(&option.name),
                kind_words(option.kind)
            ));
        }
        let mut keys = vec!["d deny"];
        if matches!(remember, Remember::Offered(_)) {
            keys.push("r allow and remember");
        }
        keys.push("Ctrl+C stop the session");
        text.push_str(&format!("  {}\n", keys.join("   ")));
        if let Remember::NotOffered(reason) = remember {
            text.push_str(&format!("  (no rule to remember: {reason})\n"));
        }
        text
    }

    /// The name and class of the option with this id, for the line that says
    /// what was chosen.
    #[must_use]
    pub fn describe_option(&self, option_id: &str) -> String {
        match self.options.iter().find(|o| o.option_id == option_id) {
            Some(option) => format!("{} ({})", clean(&option.name), kind_words(option.kind)),
            None => clean(option_id),
        }
    }
}

/// The tool calls the sessions streamed to this connection, kept to complete a
/// question with what the agent already said about the same call (design
/// D2). Bounded: only the latest calls can be asked about.
#[derive(Debug, Default)]
pub struct StreamedCalls {
    known: HashMap<(String, String), Value>,
    order: VecDeque<(String, String)>,
}

impl StreamedCalls {
    /// How many calls are kept.
    const KEPT: usize = 256;

    /// Keeps the tool call a `session/event` carries, merged over what was
    /// known of it, and returns its session and id; `None` for every other
    /// event.
    pub fn observe(&mut self, params: &Value) -> Option<(String, String)> {
        let session = params.get("sessionId")?.as_str()?;
        let event = params.get("event")?;
        if event.get("type")?.as_str()? != "agent_update" {
            return None;
        }
        let update = event.get("payload")?.get("update")?;
        if !matches!(
            update.get("sessionUpdate")?.as_str()?,
            "tool_call" | "tool_call_update"
        ) {
            return None;
        }
        let key = (
            session.to_string(),
            update.get("toolCallId")?.as_str()?.to_string(),
        );
        let merged = merge_call(self.known.get(&key), update);
        if self.known.insert(key.clone(), merged).is_none() {
            self.order.push_back(key.clone());
            if self.order.len() > Self::KEPT
                && let Some(oldest) = self.order.pop_front()
            {
                self.known.remove(&oldest);
            }
        }
        Some(key)
    }

    /// What the session streamed for this call, if anything.
    #[must_use]
    pub fn get(&self, session_id: &str, tool_call_id: Option<&str>) -> Option<&Value> {
        self.known
            .get(&(session_id.to_string(), tool_call_id?.to_string()))
    }
}

/// A tool call completed with an update, the way ACP applies one: every field
/// the update carries replaces the known one, and the rest stay.
#[must_use]
pub fn merge_call(known: Option<&Value>, update: &Value) -> Value {
    let mut merged = known
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(Map::new);
    if let Some(fields) = update.as_object() {
        for (key, value) in fields {
            if !value.is_null() {
                merged.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(merged)
}

/// The text of a call's content, line by line: text blocks in full, and a
/// line naming each diff and terminal, which are not text to read here.
fn content_lines(call: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(items) = call.get("content").and_then(Value::as_array) else {
        return lines;
    };
    for item in items {
        match item.get("type").and_then(Value::as_str) {
            Some("content") => {
                let block = item.get("content");
                if block.and_then(|b| b.get("type")).and_then(Value::as_str) == Some("text")
                    && let Some(body) = block.and_then(|b| b.get("text")).and_then(Value::as_str)
                {
                    lines.extend(body.lines().map(str::to_string));
                }
            }
            Some("diff") => {
                let path = text(item.get("path")).unwrap_or_else(|| "a file".into());
                let after = item
                    .get("newText")
                    .and_then(Value::as_str)
                    .map_or(0, |body| body.lines().count());
                lines.push(format!("an edit of {path}: {after} lines after it"));
            }
            Some("terminal") => {
                let id = text(item.get("terminalId")).unwrap_or_default();
                lines.push(format!("the terminal {id}"));
            }
            _ => {}
        }
    }
    lines
}

/// What `r` would do for a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remember {
    /// Remembering writes this rule.
    Offered(PermissionRule),
    /// Remembering is not offered, and this says why.
    NotOffered(&'static str),
}

/// The rule `r` would write, or why it must not be offered (design D5).
///
/// The rule is the most specific one the facts the daemon reads from the push
/// allow: the tool with the exact command, else with the exact path, or the
/// daemon's suggestion when it is at least that specific. `entry` is the
/// request as the queue shows it; `root` is the project the command's session
/// works in, `None` when it may work in a task worktree, where a rule would
/// be written into the worktree and govern no next session.
#[must_use]
pub fn rule_to_remember(
    question: &Question,
    entry: Option<&PendingPermission>,
    root: Option<&str>,
) -> Remember {
    let Some(root) = root else {
        return Remember::NotOffered(
            "this command's sessions may work in a task worktree, where a rule would not govern the next session",
        );
    };
    if question.request_id.is_none() {
        return Remember::NotOffered("this daemon does not name its requests");
    }
    let Some(tool) = &question.facts.tool else {
        return Remember::NotOffered("the agent did not name the tool");
    };
    if reject_option(&question.options).is_none() {
        return Remember::NotOffered("this is the agent's question, not an approval");
    }
    if allow_once_option(&question.options).is_none() {
        return Remember::NotOffered("the agent offers no one-time approval");
    }
    let Some(entry) = entry else {
        return Remember::NotOffered("the daemon's queue could not be read");
    };
    let specific =
        |rule: &PermissionRule| rule.command_prefix.is_some() || rule.path_prefix.is_some();
    let rule = match &entry.suggested_rule {
        Some(suggested) if specific(suggested) => suggested.clone(),
        _ => {
            let rule = PermissionRule {
                effect: PermissionRuleEffect::Allow,
                tool: Some(tool.clone()),
                command_prefix: question.facts.command.clone(),
                path_prefix: None,
                scope: PermissionRuleScope::Project,
            };
            match (&question.facts.command, &question.facts.path) {
                (Some(_), _) => rule,
                (None, Some(path)) => PermissionRule {
                    path_prefix: Some(path.clone()),
                    ..rule
                },
                (None, None) => {
                    return Remember::NotOffered(
                        "the request names no command or path, and a rule on the tool alone would allow every such request",
                    );
                }
            }
        }
    };
    if rule
        .path_prefix
        .as_deref()
        .is_some_and(|prefix| reaches_meltemi(prefix, root))
    {
        return Remember::NotOffered("it would reach .meltemi/, where the rules live");
    }
    Remember::Offered(rule)
}

/// Whether a path prefix covers anything inside the project's `.meltemi/`:
/// a path through a `.meltemi` directory, or a prefix of the directory itself
/// (the project root, or any ancestor of it). Compared the way the daemon
/// matches — as text — after one spelling for separators and, where the file
/// system ignores it, for case.
fn reaches_meltemi(prefix: &str, root: &str) -> bool {
    let spell = |path: &str| {
        let mut path = path.replace('\\', "/");
        if let Some(rest) = path.strip_prefix("//?/") {
            path = rest.to_string();
        }
        if cfg!(windows) {
            path = path.to_lowercase();
        }
        path.trim_end_matches('/').to_string()
    };
    let prefix = spell(prefix);
    if prefix
        .split('/')
        .any(|part| part.eq_ignore_ascii_case(".meltemi"))
    {
        return true;
    }
    format!("{}/.meltemi", spell(root)).starts_with(&prefix)
}

/// What a key means for this question (design D3). `remembers` is whether `r`
/// was offered.
#[must_use]
pub fn answer_for(key: Key, question: &Question, remembers: bool) -> Answer {
    match key {
        Key::Char(digit @ '1'..='9') => {
            let index = digit as usize - '1' as usize;
            match question.options.get(index) {
                Some(option) if option.kind == PermissionOptionKind::AllowAlways => {
                    Answer::ChooseConfirmed(option.option_id.clone())
                }
                Some(option) => Answer::Choose(option.option_id.clone()),
                None => Answer::Nothing,
            }
        }
        Key::Char('d' | 'D') | Key::Esc => {
            Answer::Deny(reject_option(&question.options).map(|option| option.option_id.clone()))
        }
        Key::Char('r' | 'R') if remembers => Answer::Remember,
        Key::CtrlC => Answer::Stop,
        _ => Answer::Nothing,
    }
}

/// What the key after a confirmation prompt means: only `y` confirms.
#[must_use]
pub fn confirmation_for(key: Key) -> Confirmation {
    match key {
        Key::Char('y' | 'Y') => Confirmation::Confirm,
        Key::CtrlC => Confirmation::Stop,
        _ => Confirmation::Back,
    }
}

/// The agent's option that grants the request, if it offered one. Neither the
/// tray's approve nor the terminal ever invents one.
#[must_use]
pub fn allow_option(options: &[PermissionOption]) -> Option<&PermissionOption> {
    options.iter().find(|option| {
        matches!(
            option.kind,
            PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
        )
    })
}

/// The agent's option that grants the request this one time only.
#[must_use]
pub fn allow_once_option(options: &[PermissionOption]) -> Option<&PermissionOption> {
    options
        .iter()
        .find(|option| option.kind == PermissionOptionKind::AllowOnce)
}

/// The agent's option that refuses the request, or `None` when it offered
/// none — then denying cancels. The tray's deny, the terminal's `d` and the
/// denial without a terminal all choose this way: a rejection tells the agent
/// "refused", a cancellation tells it the turn ended before anyone decided.
#[must_use]
pub fn reject_option(options: &[PermissionOption]) -> Option<&PermissionOption> {
    options.iter().find(|option| {
        matches!(
            option.kind,
            PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
        )
    })
}

/// A rule in words, naming exactly what would persist and where.
#[must_use]
pub fn describe_rule(rule: &PermissionRule) -> String {
    let effect = match rule.effect {
        PermissionRuleEffect::Allow => "allow",
        PermissionRuleEffect::Deny => "deny",
    };
    let mut matchers = Vec::new();
    if let Some(tool) = &rule.tool {
        matchers.push(format!("tool {}", clean(tool)));
    }
    if let Some(prefix) = &rule.command_prefix {
        matchers.push(format!("commands starting with `{}`", clean(prefix)));
    }
    if let Some(prefix) = &rule.path_prefix {
        matchers.push(format!("paths starting with {}", clean(prefix)));
    }
    let matched = if matchers.is_empty() {
        "every request".to_string()
    } else {
        matchers.join(", ")
    };
    let scope = match rule.scope {
        PermissionRuleScope::Project => "in this project (.meltemi/permissions.toml)",
        PermissionRuleScope::Global => "in every project (your global permissions.toml)",
    };
    format!("{effect} {matched} {scope}")
}

/// An option's class in words.
#[must_use]
pub fn kind_words(kind: PermissionOptionKind) -> &'static str {
    match kind {
        PermissionOptionKind::AllowOnce => "allow once",
        PermissionOptionKind::AllowAlways => "allow always",
        PermissionOptionKind::RejectOnce => "reject once",
        PermissionOptionKind::RejectAlways => "reject always",
    }
}

/// Whether a request still stands, from what the queue says of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Still waiting for a decision.
    Pending,
    /// Expired under the daemon's wait policy.
    Expired,
    /// No longer in the queue: decided elsewhere, or its session stopped.
    Resolved,
}

/// The standing of a request from its queue entry — `None` when the queue no
/// longer lists it. Only an answer the daemon gave after the push was received
/// can say this: an earlier snapshot may predate the request (design D6).
#[must_use]
pub fn standing(entry: Option<&PendingPermission>) -> Standing {
    match entry {
        None => Standing::Resolved,
        Some(entry) if entry.expired => Standing::Expired,
        Some(_) => Standing::Pending,
    }
}

/// Whether a queue snapshot calls for checking a request again: it no longer
/// lists it, or lists it expired. Never a verdict by itself — a snapshot may
/// have been taken before the request existed.
#[must_use]
pub fn calls_for_check(snapshot: &[PendingPermission], request_id: &str) -> bool {
    snapshot
        .iter()
        .find(|entry| entry.request_id == request_id)
        .is_none_or(|entry| entry.expired)
}

/// Why a question left the terminal without an answer from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Withdrawal {
    /// The daemon's wait policy ran out.
    Expired,
    /// Another surface decided it, or its session stopped.
    Resolved,
    /// The command ended with the question open.
    CommandEnded,
    /// The connection to the daemon closed.
    ConnectionClosed,
}

impl Withdrawal {
    /// The withdrawal in words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Expired => "withdrawn: it expired under the daemon's wait policy",
            Self::Resolved => {
                "withdrawn: it was resolved elsewhere (decided on another surface, or its session stopped)"
            }
            Self::CommandEnded => "withdrawn: the command ended before an answer",
            Self::ConnectionClosed => "withdrawn: the connection to the daemon closed",
        }
    }
}

/// What stderr says when a request is denied because there is no terminal to
/// ask in (design D1): the truth, and the one way to grant without a human
/// that serves every verb.
#[must_use]
pub fn no_terminal_notice(question: &Question) -> String {
    let tool = question
        .shown
        .tool
        .as_deref()
        .map_or_else(|| "an unnamed tool".to_string(), clean);
    format!(
        "meltemi: permission requested by session {} ({tool}) was denied: there is no terminal to \
         ask in. To grant without a human, write a permission rule in the project's \
         .meltemi/permissions.toml or in your global permissions.toml; rules are read when a \
         session opens.",
        clean(&question.session_id)
    )
}

/// The agent's words, made safe to print. A control character, a bidi
/// override or an invisible format character inside a title, a command or an
/// option name could redraw the very question the human is reading, or make
/// the terminal answer a query as if it were typing, so each one is shown
/// escaped, never obeyed.
#[must_use]
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || is_invisible_format(c) => {
                out.push_str(&format!("\\u{{{:04x}}}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out
}

/// The characters that reorder a line or hide inside it without showing.
pub(crate) fn is_invisible_format(c: char) -> bool {
    matches!(
        c,
        '\u{061c}'
            | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{2069}'
            | '\u{feff}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption {
            option_id: id.into(),
            name: name.into(),
            kind,
        }
    }

    fn mock_options() -> Vec<PermissionOption> {
        vec![
            option("allow", "Allow", PermissionOptionKind::AllowOnce),
            option("reject", "Reject", PermissionOptionKind::RejectOnce),
        ]
    }

    fn push(tool_call: Value, options: Vec<PermissionOption>) -> PermissionRequestParams {
        PermissionRequestParams {
            session_id: "sess-1".into(),
            tool_call,
            options,
            request_id: Some("perm-1".into()),
        }
    }

    fn question(tool_call: Value) -> Question {
        Question::from_push(&push(tool_call, mock_options()), None)
    }

    fn entry(tool: &str, suggested_rule: Option<PermissionRule>) -> PendingPermission {
        PendingPermission {
            request_id: "perm-1".into(),
            session_id: "sess-1".into(),
            tool: tool.into(),
            summary: tool.into(),
            options: mock_options(),
            waiting_seconds: 0,
            expires_in_seconds: None,
            expired: false,
            suggested_rule,
        }
    }

    const ROOT: &str = "/repo";

    fn offered(remember: Remember) -> PermissionRule {
        match remember {
            Remember::Offered(rule) => rule,
            Remember::NotOffered(reason) => panic!("expected a rule, not offered: {reason}"),
        }
    }

    #[test]
    fn the_question_shows_what_the_agent_sent_and_nothing_more() {
        let full = question(json!({
            "toolCallId": "call-1",
            "kind": "execute",
            "title": "Run the tests",
            "rawInput": { "command": "cargo test", "path": "/repo/src/lib.rs" },
            "content": [{ "type": "content", "content": { "type": "text", "text": "why\nit runs" } }]
        }));
        assert_eq!(full.facts.tool.as_deref(), Some("execute"));
        assert_eq!(full.tool_call_id.as_deref(), Some("call-1"));
        let text = full.render(&Remember::NotOffered("x"));
        for line in [
            "permission requested by session sess-1",
            "  tool     execute",
            "  title    Run the tests",
            "  command  cargo test",
            "  path     /repo/src/lib.rs",
            "  content  why",
            "           it runs",
            "  1 Allow  (allow once)",
            "  2 Reject  (reject once)",
            "d deny",
            "Ctrl+C stop the session",
        ] {
            assert!(text.contains(line), "`{line}` in:\n{text}");
        }

        // Lines appear only when the agent sent what they show. Without a
        // kind the title is the tool, and it is not repeated.
        let bare = question(json!({ "toolCallId": "call-2", "title": "Write the proposal" }));
        let text = bare.render(&Remember::NotOffered("x"));
        assert!(text.contains("  tool     Write the proposal"), "{text}");
        for absent in ["title", "command", "path", "content"] {
            assert!(!text.contains(absent), "`{absent}` in:\n{text}");
        }

        // The path also comes from the first location, as the rules read it.
        let located = question(json!({ "kind": "edit", "locations": [{ "path": "/repo/a.rs" }] }));
        assert_eq!(located.facts.path.as_deref(), Some("/repo/a.rs"));

        // Nothing named: said so, never guessed.
        let anonymous = question(json!({}));
        assert_eq!(anonymous.facts.tool, None);
        let text = anonymous.render(&Remember::NotOffered("x"));
        assert!(text.contains("tool     (not named by the agent)"), "{text}");
        assert!(
            text.contains("(the agent did not say what it would do)"),
            "{text}"
        );
    }

    #[test]
    fn the_question_shows_the_command_the_session_streamed() {
        // Scenario: La pregunta muestra el comando que la sesión transmitió
        //
        // An agent that sends the command only in the tool call it streamed
        // before asking (Codex does): the question completes itself with it,
        // while the rule facts stay the push's own.
        let streamed = json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "item-7",
            "kind": "execute",
            "title": "rm -rf target",
            "rawInput": { "command": "rm -rf target" }
        });
        let asked = Question::from_push(
            &push(
                json!({ "toolCallId": "item-7", "kind": "execute" }),
                mock_options(),
            ),
            Some(&streamed),
        );
        let text = asked.render(&Remember::NotOffered("x"));
        assert!(text.contains("  command  rm -rf target"), "{text}");
        assert!(text.contains("  title    rm -rf target"), "{text}");
        assert_eq!(asked.facts.command, None, "a rule reads only the push");

        // What the push says wins over what was streamed.
        let merged = merge_call(
            Some(&streamed),
            &json!({ "title": "rm -rf build", "kind": null }),
        );
        assert_eq!(merged["title"], "rm -rf build");
        assert_eq!(
            merged["kind"], "execute",
            "a null in the update keeps the known field"
        );
    }

    #[test]
    fn content_is_shown_in_full_and_diffs_are_named() {
        let asked = question(json!({
            "kind": "edit",
            "content": [
                { "type": "content", "content": { "type": "text", "text": "{\"command\":\"ls -la\\ncat x\"}" } },
                { "type": "diff", "path": "/repo/a.rs", "oldText": "a", "newText": "a\nb\nc" },
                { "type": "terminal", "terminalId": "t-1" },
                { "type": "content", "content": { "type": "image", "data": "..." } }
            ]
        }));
        assert_eq!(
            asked.content,
            vec![
                "{\"command\":\"ls -la\\ncat x\"}".to_string(),
                "an edit of /repo/a.rs: 3 lines after it".to_string(),
                "the terminal t-1".to_string(),
            ]
        );
    }

    #[test]
    fn the_options_are_the_agents_own_in_its_order() {
        // An agent that offers "allow for this session" gets it shown; one that
        // does not, does not: the approver never invents an option.
        let options = vec![
            option("once", "Allow once", PermissionOptionKind::AllowOnce),
            option(
                "session",
                "Allow for this session",
                PermissionOptionKind::AllowAlways,
            ),
            option("no", "Decline", PermissionOptionKind::RejectOnce),
        ];
        let asked = Question::from_push(&push(json!({ "kind": "edit" }), options), None);
        let text = asked.render(&Remember::NotOffered("x"));
        let first = text.find("1 Allow once").expect("first");
        let second = text
            .find("2 Allow for this session  (allow always)")
            .expect("second");
        let third = text.find("3 Decline  (reject once)").expect("third");
        assert!(first < second && second < third, "{text}");
        assert!(
            !question(json!({ "kind": "edit" }))
                .render(&Remember::NotOffered("x"))
                .contains("session  (")
        );
    }

    #[test]
    fn enter_and_unassigned_keys_decide_nothing() {
        // Scenario: Enter y las teclas sin asignar no deciden
        let asked = question(json!({ "kind": "edit" }));
        assert_eq!(
            answer_for(Key::Char('1'), &asked, false),
            Answer::Choose("allow".into())
        );
        assert_eq!(
            answer_for(Key::Char('2'), &asked, false),
            Answer::Choose("reject".into())
        );
        for key in [
            Key::Enter,
            Key::Char('3'),
            Key::Char('0'),
            Key::Char('x'),
            Key::Char(' '),
            Key::Paste,
            Key::Other,
        ] {
            assert_eq!(answer_for(key, &asked, true), Answer::Nothing, "{key:?}");
        }
    }

    #[test]
    fn an_option_with_which_the_agent_stops_asking_asks_for_confirmation() {
        // Scenario: Una opción con la que el agente deja de preguntar pide confirmación
        let options = vec![
            option("once", "Allow once", PermissionOptionKind::AllowOnce),
            option(
                "session",
                "Allow for this session",
                PermissionOptionKind::AllowAlways,
            ),
            option("never", "Never", PermissionOptionKind::RejectAlways),
        ];
        let asked = Question::from_push(&push(json!({ "kind": "execute" }), options), None);
        assert_eq!(
            answer_for(Key::Char('2'), &asked, false),
            Answer::ChooseConfirmed("session".into())
        );
        // Refusing needs no confirmation, even when the agent remembers it.
        assert_eq!(
            answer_for(Key::Char('3'), &asked, false),
            Answer::Choose("never".into())
        );
        assert_eq!(confirmation_for(Key::Char('y')), Confirmation::Confirm);
        assert_eq!(confirmation_for(Key::Char('Y')), Confirmation::Confirm);
        assert_eq!(confirmation_for(Key::CtrlC), Confirmation::Stop);
        for key in [
            Key::Char('n'),
            Key::Enter,
            Key::Esc,
            Key::Char('2'),
            Key::Paste,
        ] {
            assert_eq!(confirmation_for(key), Confirmation::Back, "{key:?}");
        }
    }

    #[test]
    fn deny_picks_the_agents_rejection_or_cancels_without_one() {
        let asked = question(json!({ "kind": "edit" }));
        for key in [Key::Char('d'), Key::Char('D'), Key::Esc] {
            assert_eq!(
                answer_for(key, &asked, false),
                Answer::Deny(Some("reject".into())),
                "{key:?}"
            );
        }
        let no_reject = Question::from_push(
            &push(
                json!({ "kind": "edit" }),
                vec![option("allow", "Allow", PermissionOptionKind::AllowOnce)],
            ),
            None,
        );
        assert_eq!(
            answer_for(Key::Char('d'), &no_reject, false),
            Answer::Deny(None)
        );
        assert_eq!(answer_for(Key::CtrlC, &asked, false), Answer::Stop);
        assert_eq!(answer_for(Key::Char('r'), &asked, true), Answer::Remember);
        assert_eq!(answer_for(Key::Char('r'), &asked, false), Answer::Nothing);
    }

    #[test]
    fn remembering_writes_the_most_specific_rule() {
        // The exact path, when the push carries one.
        let edit =
            question(json!({ "kind": "edit", "rawInput": { "path": "/repo/notes/plan.md" } }));
        let rule = offered(rule_to_remember(
            &edit,
            Some(&entry("edit", None)),
            Some(ROOT),
        ));
        assert_eq!(
            rule,
            PermissionRule {
                effect: PermissionRuleEffect::Allow,
                tool: Some("edit".into()),
                command_prefix: None,
                path_prefix: Some("/repo/notes/plan.md".into()),
                scope: PermissionRuleScope::Project,
            }
        );
        assert_eq!(
            describe_rule(&rule),
            "allow tool edit, paths starting with /repo/notes/plan.md in this project \
             (.meltemi/permissions.toml)"
        );
        let text = edit.render(&Remember::Offered(rule));
        assert!(text.contains("r allow and remember"), "{text}");
        assert!(!text.contains("no rule to remember"), "{text}");

        // The exact command, preferred to the path.
        let run = question(json!({
            "kind": "execute",
            "rawInput": { "command": "cargo test -p meltemi", "path": "/repo" }
        }));
        let rule = offered(rule_to_remember(
            &run,
            Some(&entry("execute", None)),
            Some(ROOT),
        ));
        assert_eq!(
            rule.command_prefix.as_deref(),
            Some("cargo test -p meltemi")
        );
        assert_eq!(rule.path_prefix, None);

        // The daemon's suggestion, when it is at least that specific.
        let suggested = PermissionRule {
            effect: PermissionRuleEffect::Allow,
            tool: Some("execute".into()),
            command_prefix: Some("cargo test".into()),
            path_prefix: None,
            scope: PermissionRuleScope::Project,
        };
        assert_eq!(
            rule_to_remember(
                &run,
                Some(&entry("execute", Some(suggested.clone()))),
                Some(ROOT)
            ),
            Remember::Offered(suggested)
        );

        // Not offered without the queue entry, without the queue id (an older
        // daemon), for the agent's own question, or without a one-time approval.
        assert!(matches!(
            rule_to_remember(&edit, None, Some(ROOT)),
            Remember::NotOffered(_)
        ));
        let unnamed = Question {
            request_id: None,
            ..edit.clone()
        };
        assert!(matches!(
            rule_to_remember(&unnamed, Some(&entry("edit", None)), Some(ROOT)),
            Remember::NotOffered(_)
        ));
        let asked_by_agent = Question::from_push(
            &push(
                json!({ "kind": "other", "rawInput": { "path": "/repo/a" } }),
                vec![option("a", "Rewrite it", PermissionOptionKind::AllowOnce)],
            ),
            None,
        );
        assert_eq!(
            rule_to_remember(&asked_by_agent, Some(&entry("other", None)), Some(ROOT)),
            Remember::NotOffered("this is the agent's question, not an approval")
        );
        let only_always = Question::from_push(
            &push(
                json!({ "kind": "edit", "rawInput": { "path": "/repo/a" } }),
                vec![
                    option(
                        "s",
                        "Allow for this session",
                        PermissionOptionKind::AllowAlways,
                    ),
                    option("r", "Reject", PermissionOptionKind::RejectOnce),
                ],
            ),
            None,
        );
        assert_eq!(
            rule_to_remember(&only_always, Some(&entry("edit", None)), Some(ROOT)),
            Remember::NotOffered("the agent offers no one-time approval")
        );
    }

    #[test]
    fn without_a_tool_no_rule_is_offered() {
        // Scenario: Sin herramienta no se ofrece regla
        //
        // Neither kind nor title: the queue labels it with its placeholder,
        // and a rule with that label would never govern the request.
        let asked = question(json!({ "toolCallId": "c", "rawInput": { "path": "/repo/a" } }));
        let remember = rule_to_remember(&asked, Some(&entry("permission", None)), Some(ROOT));
        assert_eq!(
            remember,
            Remember::NotOffered("the agent did not name the tool")
        );
        let text = asked.render(&remember);
        assert!(!text.contains("r allow and remember"), "{text}");
        assert!(
            text.contains("no rule to remember: the agent did not name the tool"),
            "{text}"
        );
    }

    #[test]
    fn without_a_command_or_a_path_no_rule_is_offered() {
        // Scenario: Sin comando ni ruta no se ofrece regla
        //
        // What Claude and Codex send today: a kind and a title, no rawInput. A
        // rule on `execute` alone would allow every command. A suggestion that
        // is only the tool is no more specific, and is refused too.
        let asked = question(json!({ "kind": "execute", "title": "Bash: ls" }));
        let tool_only = PermissionRule {
            effect: PermissionRuleEffect::Allow,
            tool: Some("execute".into()),
            command_prefix: None,
            path_prefix: None,
            scope: PermissionRuleScope::Project,
        };
        for suggestion in [None, Some(tool_only)] {
            assert_eq!(
                rule_to_remember(&asked, Some(&entry("execute", suggestion)), Some(ROOT)),
                Remember::NotOffered(
                    "the request names no command or path, and a rule on the tool alone would allow every such request"
                )
            );
        }
    }

    #[test]
    fn no_rule_from_the_terminal_reaches_meltemi() {
        // Scenario: Ninguna regla del terminal alcanza la configuración de Meltemi
        //
        // A path inside `.meltemi/`, in either spelling, and a prefix that
        // covers it — the project root itself — are refused: a rule there
        // would let the agent write its own permissions.
        for path in [
            "/repo/.meltemi/permissions.toml",
            "/repo/.MELTEMI/x",
            "C:\\repo\\.meltemi\\permissions.toml",
            "/repo",
            "/repo/",
            "/",
        ] {
            let asked = question(json!({ "kind": "edit", "rawInput": { "path": path } }));
            assert_eq!(
                rule_to_remember(&asked, Some(&entry("edit", None)), Some(ROOT)),
                Remember::NotOffered("it would reach .meltemi/, where the rules live"),
                "{path}"
            );
        }
        // A suggestion is held to the same line.
        let near = question(json!({ "kind": "edit", "rawInput": { "path": "/repo/src/a.rs" } }));
        let broad = PermissionRule {
            effect: PermissionRuleEffect::Allow,
            tool: Some("edit".into()),
            command_prefix: None,
            path_prefix: Some("/repo".into()),
            scope: PermissionRuleScope::Project,
        };
        assert!(matches!(
            rule_to_remember(&near, Some(&entry("edit", Some(broad))), Some(ROOT)),
            Remember::NotOffered(_)
        ));
        // A sibling that only shares the start of a name is not inside.
        let sibling =
            question(json!({ "kind": "edit", "rawInput": { "path": "/repo/.meltemix/a" } }));
        assert!(matches!(
            rule_to_remember(&sibling, Some(&entry("edit", None)), Some(ROOT)),
            Remember::Offered(_)
        ));
    }

    #[test]
    fn in_a_task_worktree_no_rule_is_offered() {
        // Scenario: En un worktree de tarea no se ofrece regla
        let asked = question(json!({ "kind": "edit", "rawInput": { "path": "/repo/notes/a.md" } }));
        let remember = rule_to_remember(&asked, Some(&entry("edit", None)), None);
        assert!(matches!(remember, Remember::NotOffered(reason) if reason.contains("worktree")));
    }

    #[test]
    fn a_request_stands_only_while_the_queue_lists_it() {
        let pending = entry("edit", None);
        let expired = PendingPermission {
            expired: true,
            ..entry("edit", None)
        };
        assert_eq!(standing(Some(&pending)), Standing::Pending);
        assert_eq!(standing(Some(&expired)), Standing::Expired);
        assert_eq!(standing(None), Standing::Resolved);

        // A snapshot calls for a check when it no longer lists the request or
        // lists it expired — and says nothing when it lists it pending.
        assert!(!calls_for_check(std::slice::from_ref(&pending), "perm-1"));
        assert!(calls_for_check(&[expired], "perm-1"));
        assert!(calls_for_check(&[], "perm-1"));
        assert!(calls_for_check(&[pending], "perm-2"));
    }

    #[test]
    fn streamed_tool_calls_are_kept_by_session_and_id_and_merged() {
        let mut calls = StreamedCalls::default();
        let event = |session: &str, update: Value| {
            json!({ "sessionId": session, "event": {
                "v": 1, "ts": "t", "type": "agent_update", "payload": { "update": update }
            } })
        };
        let first = json!({
            "sessionUpdate": "tool_call", "toolCallId": "c1", "title": "ls", "kind": "execute"
        });
        assert_eq!(
            calls.observe(&event("s1", first)),
            Some(("s1".to_string(), "c1".to_string()))
        );
        let update = json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "c1",
            "rawInput": { "command": "ls -la" }
        });
        calls.observe(&event("s1", update));
        let known = calls.get("s1", Some("c1")).expect("kept");
        assert_eq!(known["title"], "ls");
        assert_eq!(known["rawInput"]["command"], "ls -la");

        // Another session's call, a call without an id, other updates and
        // other events are not this call.
        assert_eq!(calls.get("s2", Some("c1")), None);
        assert_eq!(calls.get("s1", None), None);
        let chunk = json!({ "sessionUpdate": "agent_message_chunk" });
        assert_eq!(calls.observe(&event("s1", chunk)), None);
        let prompt =
            json!({ "sessionId": "s1", "event": { "type": "prompt_sent", "payload": {} } });
        assert_eq!(calls.observe(&prompt), None);

        // Bounded: the oldest calls are let go.
        for n in 0..300 {
            let call = json!({ "sessionUpdate": "tool_call", "toolCallId": format!("n{n}") });
            calls.observe(&event("s1", call));
        }
        assert_eq!(calls.get("s1", Some("c1")), None);
        assert!(calls.get("s1", Some("n299")).is_some());
    }

    #[test]
    fn without_a_terminal_the_notice_names_the_rules() {
        let notice = no_terminal_notice(&question(json!({ "kind": "edit" })));
        assert!(notice.contains("no terminal to ask in"), "{notice}");
        assert!(notice.contains(".meltemi/permissions.toml"), "{notice}");
        assert!(notice.contains("global permissions.toml"), "{notice}");
        assert!(!notice.contains("--mode"), "{notice}");
        assert!(!notice.contains("later release"), "{notice}");
    }

    #[test]
    fn the_agents_words_cannot_redraw_the_question() {
        // Scenario: El texto del agente no controla el terminal
        //
        // A title that clears the line and forges an option, a command that
        // hides behind a bidi override, a query that would make the terminal
        // type an answer, and a zero-width character: all print escaped.
        let asked = Question::from_push(
            &push(
                json!({
                    "kind": "execute",
                    "title": "ok\u{1b}[2K\r  1 Allow  (reject once)",
                    "rawInput": { "command": "rm -rf /\u{202e}txt.sulp" },
                    "content": [{ "type": "content", "content": {
                        "type": "text", "text": "\u{1b}P$qm\u{1b}\\ and\u{200b}more\u{9b}" } }]
                }),
                vec![option(
                    "allow",
                    "Allow\u{7}",
                    PermissionOptionKind::AllowOnce,
                )],
            ),
            None,
        );
        let text = asked.render(&Remember::NotOffered("x"));
        for raw in ['\u{1b}', '\u{202e}', '\u{200b}', '\u{7}', '\u{9b}', '\r'] {
            assert!(!text.contains(raw), "{raw:?} in {text:?}");
        }
        assert!(
            text.contains("ok\\u{001b}[2K\\r  1 Allow  (reject once)"),
            "{text}"
        );
        assert!(text.contains("rm -rf /\\u{202e}txt.sulp"), "{text}");
        assert!(
            text.contains("\\u{001b}P$qm\\u{001b}\\ and\\u{200b}more\\u{009b}"),
            "{text}"
        );
        assert!(text.contains("1 Allow\\u{0007}  (allow once)"), "{text}");
        // Every line of the question is one the approver wrote.
        assert_eq!(text.lines().count(), 8, "{text}");
    }
}
