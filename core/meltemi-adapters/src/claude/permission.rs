// SPDX-License-Identifier: Apache-2.0

//! Relaying the CLI's permission questions to the permission proxy
//! (adaptadores-propios-acp task 3.3, design D5).
//!
//! When the piloted CLI wants to run a tool, the adapter has exactly one job:
//! turn that into `session/request_permission` and hand back whatever the proxy
//! decides. It never decides. It has no rules, no memory of past answers and no
//! opinion — the decision belongs to the proxy meltemid already runs, with the
//! same tray, the same rules and the same log every other agent's requests go
//! through, and the daemon gains no transport whatsoever from this.
//!
//! Three consequences are written down rather than discovered:
//!
//! - **No decision means no.** A proxy that cannot be reached, an outcome this
//!   adapter does not recognise, an option id that is not one of the two it
//!   offered: all of them are a denial. The failure mode of a permission relay
//!   must be denial, never a shrug that reads as consent.
//! - **Only what the provider can honour is offered.** This CLI's permission
//!   channel answers one call at a time: allow this, or refuse this. There is
//!   no "and stop asking" on that wire, so none is offered — an option that
//!   could not be kept would be a promise made to the human in somebody else's
//!   name.
//! - **An allowed call runs exactly as it was asked.** The channel lets the
//!   answer rewrite the tool's arguments. This adapter hands them back
//!   unchanged, always: a bridge that edited what it was relaying would be
//!   deciding something nobody asked it to decide, and the human approved what
//!   they were shown.

use agent_client_protocol::schema::v1::{
    ContentBlock, PermissionOption, PermissionOptionKind, RequestPermissionRequest, SessionId,
    TextContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use serde_json::{Value, json};

use super::mapping;

/// Allow this one call.
pub const ALLOW_ONCE: &str = "allow-once";
/// Refuse it.
pub const REJECT: &str = "reject";

/// The name of the tool the CLI is told to ask through.
pub const TOOL: &str = "approve";
/// The name the permission MCP server is registered under.
pub const SERVER: &str = "meltemi_permissions";

/// What came back from the proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The proxy chose one of the offered options.
    Selected(String),
    /// The turn is being cancelled; the question no longer stands.
    Cancelled,
    /// No decision could be obtained at all.
    Unavailable,
}

/// How the CLI is told to ask: the tool reference its permission flag takes.
#[must_use]
pub fn prompt_tool_reference() -> String {
    format!("mcp__{SERVER}__{TOOL}")
}

/// The MCP server the CLI is told to run so that it has somewhere to ask: this
/// same binary, in shim mode, pointed at the channel back to this adapter.
///
/// The name is Meltemi's own and carries no credential, no rule and no policy —
/// only where to ask. Everything that decides anything lives on the other side
/// of that channel.
#[must_use]
pub fn shim_server(exe: &std::path::Path, channel: &std::path::Path) -> (String, Value) {
    (
        SERVER.to_string(),
        json!({
            "type": "stdio",
            "command": exe.display().to_string(),
            "args": [super::shim::SHIM_ARG, channel.display().to_string()],
        }),
    )
}

/// The permission request the proxy will decide.
///
/// It is addressed to the **same tool call id the call was streamed under**
/// whenever the CLI names one, so a client shows the question against the tool
/// call it already displayed instead of against an anonymous prompt. The
/// arguments travel with it: at the moment of deciding, what the tool would
/// actually do is the thing the human needs.
#[must_use]
pub fn request_for(
    session_id: &SessionId,
    tool: &str,
    tool_use_id: Option<&str>,
    input: &Value,
) -> RequestPermissionRequest {
    let mut fields = ToolCallUpdateFields::new()
        .kind(mapping::kind_of(tool))
        .status(ToolCallStatus::Pending)
        .title(mapping::title_for(tool, input));
    if !input.is_null() {
        fields = fields.content(vec![
            ContentBlock::Text(TextContent::new(input.to_string())).into(),
        ]);
    }
    RequestPermissionRequest::new(
        session_id.clone(),
        // Without an id from the CLI the question still has to be about
        // something: the tool's own name is what the human is looking at.
        ToolCallUpdate::new(tool_use_id.unwrap_or(tool).to_string(), fields),
        vec![
            PermissionOption::new(ALLOW_ONCE, "Allow once", PermissionOptionKind::AllowOnce),
            PermissionOption::new(REJECT, "Reject", PermissionOptionKind::RejectOnce),
        ],
    )
}

/// The answer the CLI gets, from the decision the proxy made.
///
/// The shape is the provider's: a payload the permission tool returns, saying
/// allow with the arguments to use, or deny with a reason. The reason is
/// written for the person who will read it in a transcript, and it never
/// pretends the adapter decided anything.
#[must_use]
pub fn payload(decision: &Decision, input: &Value) -> Value {
    match decision {
        Decision::Selected(option) if option == ALLOW_ONCE => json!({
            "behavior": "allow",
            // Unchanged, always: the human approved what they were shown.
            "updatedInput": input,
        }),
        Decision::Selected(option) if option == REJECT => {
            deny("the request was refused where permissions are decided")
        }
        Decision::Cancelled => deny("the turn was cancelled before this was decided"),
        // An option id nobody offered and no decision at all: the same answer.
        // Anything else would let a gap in the chain read as consent.
        Decision::Selected(_) | Decision::Unavailable => {
            deny("no decision could be obtained, so the request was refused")
        }
    }
}

/// A denial with its reason, in the shape the CLI's permission channel takes.
#[must_use]
pub fn deny(reason: &str) -> Value {
    json!({"behavior": "deny", "message": reason})
}

/// What the session shows about a call that was refused.
///
/// A denial the human never sees is a session that quietly did less than it
/// was asked to. It is shown against the call it is about, as a failure with
/// its reason — never hidden, and never dressed up as something that worked.
#[must_use]
pub fn refusal_update(
    tool_use_id: &str,
    reason: &str,
) -> agent_client_protocol::schema::v1::SessionUpdate {
    agent_client_protocol::schema::v1::SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        tool_use_id.to_string(),
        ToolCallUpdateFields::new()
            .status(ToolCallStatus::Failed)
            .content(vec![
                ContentBlock::Text(TextContent::new(reason.to_string())).into(),
            ]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::ToolKind;

    #[test]
    fn the_question_reaches_the_proxy_against_the_call_it_is_about() {
        // Scenario: Permiso decidido por el proxy vigente
        let input = json!({"file_path": "NOTES.md", "content": "hola"});
        let request = request_for(&SessionId::new("s-1"), "Write", Some("toolu_1"), &input);
        assert_eq!(request.session_id.0.as_ref(), "s-1");
        assert_eq!(
            request.tool_call.tool_call_id.0.as_ref(),
            "toolu_1",
            "the same id the call was streamed under, so the human sees what it is about"
        );
        assert_eq!(request.tool_call.fields.kind, Some(ToolKind::Edit));
        assert_eq!(
            request.tool_call.fields.status,
            Some(ToolCallStatus::Pending)
        );
        assert!(
            request.tool_call.fields.content.is_some(),
            "and what the tool would actually do travels with it"
        );

        let anonymous = request_for(&SessionId::new("s-1"), "Bash", None, &json!({}));
        assert_eq!(
            anonymous.tool_call.tool_call_id.0.as_ref(),
            "Bash",
            "a call the CLI did not name is still about something"
        );
    }

    #[test]
    fn no_decision_is_a_denial_and_never_a_shrug() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // Every way the chain can break ends in the same place. This is the one
        // rule of a permission relay that must never have an exception.
        let input = json!({"command": "rm -rf /"});
        let allowed = payload(&Decision::Selected(ALLOW_ONCE.into()), &input);
        assert_eq!(allowed["behavior"], "allow");
        assert_eq!(
            allowed["updatedInput"], input,
            "an allowed call runs exactly as it was approved"
        );

        for decision in [
            Decision::Selected(REJECT.into()),
            Decision::Selected("something-nobody-offered".into()),
            Decision::Cancelled,
            Decision::Unavailable,
        ] {
            let answer = payload(&decision, &input);
            assert_eq!(answer["behavior"], "deny", "for {decision:?}");
            assert!(
                !answer["message"].as_str().unwrap_or_default().is_empty(),
                "and it says why: {answer}"
            );
        }
    }

    #[test]
    fn only_the_options_this_channel_can_honour_are_offered() {
        // The CLI's permission channel answers one call at a time; there is no
        // "and stop asking" on it, so none is offered. An option that could not
        // be kept would be a promise made in somebody else's name.
        let request = request_for(&SessionId::new("s"), "Bash", None, &json!({}));
        let offered: Vec<&str> = request
            .options
            .iter()
            .map(|option| option.option_id.0.as_ref())
            .collect();
        assert_eq!(offered, vec![ALLOW_ONCE, REJECT]);
        assert!(
            !request
                .options
                .iter()
                .any(|option| option.kind == PermissionOptionKind::AllowAlways
                    || option.kind == PermissionOptionKind::RejectAlways),
            "nothing here can be remembered, so nothing here promises to be"
        );
    }

    #[test]
    fn the_cli_is_told_to_ask_through_the_tool_this_adapter_hosts() {
        assert_eq!(prompt_tool_reference(), "mcp__meltemi_permissions__approve");
    }

    #[test]
    fn a_refused_call_is_shown_as_refused_with_its_reason() {
        // Scenario: Interacción no relevable denegada con motivo visible
        let update = refusal_update("toolu_2", "the human said no");
        let shown = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(shown["toolCallId"], "toolu_2");
        assert_eq!(shown["status"], "failed");
        assert!(
            shown.to_string().contains("the human said no"),
            "the reason is in the session, not only in a log: {shown}"
        );
    }
}
