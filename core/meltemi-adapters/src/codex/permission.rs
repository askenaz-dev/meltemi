// SPDX-License-Identifier: Apache-2.0

//! Relaying the server's approvals to the permission proxy
//! (adaptadores-propios-acp task 2.4).
//!
//! When the piloted CLI asks to run a command or change files, the adapter has
//! exactly one job: turn that question into `session/request_permission` and
//! hand back whatever the proxy decides. It never decides. It has no rules, no
//! memory of past answers and no opinion — the decision belongs to the proxy
//! meltemid already runs, with the same tray, the same rules and the same log
//! every other agent's requests go through, and the daemon gains no transport
//! whatsoever from this (design D5, mirrored here for the server dialect).
//!
//! Two consequences are written down rather than discovered:
//!
//! - **No decision means no.** A proxy that cannot be reached, an outcome this
//!   adapter does not recognise, an option id that is not one of the three it
//!   offered: all of them are a refusal. The failure mode of a permission relay
//!   must be denial, never a shrug that reads as consent.
//! - **Only what the provider can honour is offered.** The provider's decisions
//!   are accept, accept for this session, decline and cancel. There is no
//!   "reject always", so none is offered: an option that could not be kept
//!   would be a promise made to the human in somebody else's name.

use agent_client_protocol::schema::v1::{
    ContentBlock, PermissionOption, PermissionOptionKind, RequestPermissionRequest, SessionId,
    TextContent, ToolCall, ToolCallContent, ToolCallLocation, ToolCallStatus, ToolCallUpdate,
    ToolCallUpdateFields, ToolKind,
};

use super::wire::{self, ApprovalDecision};

/// Allow this one operation.
pub const ALLOW_ONCE: &str = "allow-once";
/// Allow it, and stop asking for the rest of this session.
pub const ALLOW_SESSION: &str = "allow-session";
/// Refuse it.
pub const REJECT: &str = "reject";

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

/// Whether a request from the server is one this adapter knows how to relay.
#[must_use]
pub fn is_approval(method: &str) -> bool {
    matches!(
        method,
        wire::COMMAND_EXECUTION_APPROVAL | wire::FILE_CHANGE_APPROVAL
    )
}

/// What an approval really grants, when it is more — or other — than the item
/// it is about (codex-al-dia design D7): its title, a line that says it, its
/// kind, and the path it reaches.
struct Grant {
    title: String,
    says: String,
    kind: ToolKind,
    location: Option<String>,
}

/// What an approval grants beyond the item, if anything.
///
/// A network access is not a command: it is shown by host and protocol, under
/// the kind the proxy's rules read as network. Input to a terminal already
/// running is not a new command either. And a file change that also asks for a
/// root to write under for the rest of the session shows that root, as the
/// path the rules match.
fn grant(method: &str, params: &wire::ApprovalParams) -> Option<Grant> {
    if method == wire::COMMAND_EXECUTION_APPROVAL {
        if let Some(network) = &params.network_approval_context {
            return Some(Grant {
                title: format!("Connect to `{}` over {}", network.host, network.protocol),
                says: format!(
                    "A network access, not a new command: allowing it lets the command reach `{}` \
                     over {}.",
                    network.host, network.protocol
                ),
                kind: ToolKind::Fetch,
                location: None,
            });
        }
        if params.kind.as_deref() == Some(wire::COMMAND_KIND_WRITE_STDIN) {
            return Some(Grant {
                title: "Write input to a terminal that is already running".to_string(),
                says: "This sends input to a command already running; it does not start a new one."
                    .to_string(),
                kind: ToolKind::Execute,
                location: None,
            });
        }
    }
    if method == wire::FILE_CHANGE_APPROVAL
        && let Some(root) = params.grant_root.as_ref().filter(|root| !root.is_empty())
    {
        return Some(Grant {
            title: format!("Change files, and write under `{root}` for the rest of the session"),
            says: format!(
                "Allowing it also lets Codex write anywhere under `{root}` for the rest of this \
                 session, not only this change."
            ),
            kind: ToolKind::Edit,
            location: Some(root.clone()),
        });
    }
    None
}

/// The tool an approval is about, for an approval whose item the session never
/// saw announced — a network access has none of its own. Shown before the
/// question, so the question is asked against something the human can see.
#[must_use]
pub fn announcement(method: &str, params: &wire::ApprovalParams) -> ToolCall {
    let (title, kind, locations) = match grant(method, params) {
        Some(grant) => (
            grant.title,
            grant.kind,
            grant
                .location
                .map(ToolCallLocation::new)
                .into_iter()
                .collect(),
        ),
        None if method == wire::COMMAND_EXECUTION_APPROVAL => {
            ("A command".to_string(), ToolKind::Execute, Vec::new())
        }
        None => ("A file change".to_string(), ToolKind::Edit, Vec::new()),
    };
    ToolCall::new(params.item_id.clone(), title)
        .kind(kind)
        .status(ToolCallStatus::Pending)
        .locations(locations)
}

/// The permission request the proxy will decide.
///
/// It is addressed to the **same tool call id** the item was streamed under, so
/// a client shows the question against the command or the diff it already
/// displayed instead of against an anonymous prompt. The provider's reason,
/// when it gives one, travels as the content of that update: at the moment of
/// deciding, the reason is the thing the human needs. When the approval grants
/// more or other than the item — a network access, input to a running
/// terminal, a root to write under for the session — the title, the content,
/// the kind and the location say what is really granted, so the human and the
/// rules decide on that.
#[must_use]
pub fn request_for(
    session_id: &SessionId,
    method: &str,
    params: &wire::ApprovalParams,
) -> RequestPermissionRequest {
    let grant = grant(method, params);
    let kind = match &grant {
        Some(grant) => grant.kind,
        None if method == wire::COMMAND_EXECUTION_APPROVAL => ToolKind::Execute,
        None => ToolKind::Edit,
    };
    let mut fields = ToolCallUpdateFields::new()
        .kind(kind)
        .status(ToolCallStatus::Pending);
    let mut content: Vec<ToolCallContent> = Vec::new();
    if let Some(reason) = params.reason.as_ref().filter(|reason| !reason.is_empty()) {
        content.push(ContentBlock::Text(TextContent::new(reason)).into());
    }
    if let Some(grant) = grant {
        content.push(ContentBlock::Text(TextContent::new(grant.says)).into());
        fields = fields.title(grant.title);
        if let Some(root) = grant.location {
            fields = fields.locations(vec![ToolCallLocation::new(root)]);
        }
    }
    if !content.is_empty() {
        fields = fields.content(content);
    }
    RequestPermissionRequest::new(
        session_id.clone(),
        ToolCallUpdate::new(params.item_id.clone(), fields),
        vec![
            PermissionOption::new(ALLOW_ONCE, "Allow once", PermissionOptionKind::AllowOnce),
            PermissionOption::new(
                ALLOW_SESSION,
                "Allow for this session",
                // The provider remembers for the session and no longer; the
                // label says so, because "always" would promise more than the
                // CLI can keep.
                PermissionOptionKind::AllowAlways,
            ),
            PermissionOption::new(REJECT, "Reject", PermissionOptionKind::RejectOnce),
        ],
    )
}

/// The answer the provider gets, from the decision the proxy made.
#[must_use]
pub fn decide(decision: &Decision) -> ApprovalDecision {
    match decision {
        Decision::Selected(option) if option == ALLOW_ONCE => ApprovalDecision::Accept,
        Decision::Selected(option) if option == ALLOW_SESSION => ApprovalDecision::AcceptForSession,
        Decision::Cancelled => ApprovalDecision::Cancel,
        // `REJECT`, an option id nobody offered, and no decision at all: the
        // same answer. Anything else would let a gap in the chain read as
        // consent.
        Decision::Selected(_) | Decision::Unavailable => ApprovalDecision::Decline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::PermissionOptionId;

    fn params() -> wire::ApprovalParams {
        wire::ApprovalParams {
            thread_id: "thread-1".into(),
            turn_id: "turn-1".into(),
            item_id: "item-2".into(),
            reason: Some("it wants to write outside the worktree".into()),
            ..wire::ApprovalParams::default()
        }
    }

    /// The text the request shows, all of it.
    fn shown(request: &RequestPermissionRequest) -> String {
        let fields = &request.tool_call.fields;
        let mut text = fields.title.clone().unwrap_or_default();
        for block in fields.content.iter().flatten() {
            if let ToolCallContent::Content(content) = block
                && let ContentBlock::Text(said) = &content.content
            {
                text.push('\n');
                text.push_str(&said.text);
            }
        }
        text
    }

    #[test]
    fn a_network_approval_shows_where_it_connects() {
        // Scenario: Una aprobación de red muestra a qué se conecta
        let network = wire::ApprovalParams {
            item_id: "net-1".into(),
            network_approval_context: Some(wire::NetworkApprovalContext {
                host: "registry.npmjs.org".into(),
                protocol: "https".into(),
            }),
            ..params()
        };
        let request = request_for(
            &SessionId::new("s-1"),
            wire::COMMAND_EXECUTION_APPROVAL,
            &network,
        );
        let text = shown(&request);
        assert!(
            text.contains("registry.npmjs.org") && text.contains("https"),
            "host and protocol: {text}"
        );
        assert_eq!(
            request.tool_call.fields.kind,
            Some(ToolKind::Fetch),
            "under the kind the proxy's rules read as network, not as a command"
        );
        assert!(
            text.contains("outside the worktree"),
            "and the reason still: {text}"
        );

        // Its item was never announced, so it is shown before it is asked.
        let tool = announcement(wire::COMMAND_EXECUTION_APPROVAL, &network);
        assert_eq!(tool.tool_call_id.0.as_ref(), "net-1");
        assert!(tool.title.contains("registry.npmjs.org"));
        assert_eq!(tool.kind, ToolKind::Fetch);
    }

    #[test]
    fn a_write_to_a_running_terminal_is_not_shown_as_a_new_command() {
        // Scenario: Una escritura en una terminal se distingue de un comando
        let stdin = wire::ApprovalParams {
            kind: Some(wire::COMMAND_KIND_WRITE_STDIN.into()),
            ..params()
        };
        let request = request_for(
            &SessionId::new("s-1"),
            wire::COMMAND_EXECUTION_APPROVAL,
            &stdin,
        );
        let text = shown(&request);
        assert!(
            text.contains("already running") && text.contains("does not start a new one"),
            "{text}"
        );
        // A plain command keeps the item's own title.
        let command = wire::ApprovalParams {
            kind: Some("command".into()),
            ..params()
        };
        let request = request_for(
            &SessionId::new("s-1"),
            wire::COMMAND_EXECUTION_APPROVAL,
            &command,
        );
        assert_eq!(request.tool_call.fields.title, None);
        assert_eq!(request.tool_call.fields.kind, Some(ToolKind::Execute));
    }

    #[test]
    fn a_root_to_write_under_for_the_session_is_shown() {
        // Scenario: Una raíz de escritura para toda la sesión se muestra
        let rooted = wire::ApprovalParams {
            grant_root: Some("/home/dev/elsewhere".into()),
            ..params()
        };
        let request = request_for(&SessionId::new("s-1"), wire::FILE_CHANGE_APPROVAL, &rooted);
        let text = shown(&request);
        assert!(
            text.contains("/home/dev/elsewhere") && text.contains("rest of this session"),
            "the root and the scope: {text}"
        );
        let locations = request
            .tool_call
            .fields
            .locations
            .clone()
            .unwrap_or_default();
        assert_eq!(
            locations.first().map(|location| location.path.clone()),
            Some(std::path::PathBuf::from("/home/dev/elsewhere")),
            "as the path the rules match"
        );
    }

    #[test]
    fn the_question_reaches_the_proxy_against_the_item_it_is_about() {
        // Scenario: Aprobación del servidor decidida por el proxy
        let request = request_for(
            &SessionId::new("s-1"),
            wire::FILE_CHANGE_APPROVAL,
            &params(),
        );
        assert_eq!(request.session_id.0.as_ref(), "s-1");
        assert_eq!(
            request.tool_call.tool_call_id.0.as_ref(),
            "item-2",
            "the same id the item was streamed under, so the human sees what it is about"
        );
        assert_eq!(request.tool_call.fields.kind, Some(ToolKind::Edit));
        assert_eq!(
            request.tool_call.fields.status,
            Some(ToolCallStatus::Pending)
        );
        assert!(
            request.tool_call.fields.content.is_some(),
            "and the provider's reason travels with it"
        );

        let offered: Vec<&str> = request
            .options
            .iter()
            .map(|option| option.option_id.0.as_ref())
            .collect();
        assert_eq!(offered, vec![ALLOW_ONCE, ALLOW_SESSION, REJECT]);

        let command = request_for(
            &SessionId::new("s-1"),
            wire::COMMAND_EXECUTION_APPROVAL,
            &params(),
        );
        assert_eq!(command.tool_call.fields.kind, Some(ToolKind::Execute));
    }

    #[test]
    fn no_decision_is_a_refusal_and_never_a_shrug() {
        // Scenario: Aprobación del servidor decidida por el proxy
        //
        // Every way the chain can break ends in the same place. This is the one
        // rule of a permission relay that must never have an exception.
        assert_eq!(
            decide(&Decision::Selected(ALLOW_ONCE.into())),
            ApprovalDecision::Accept
        );
        assert_eq!(
            decide(&Decision::Selected(ALLOW_SESSION.into())),
            ApprovalDecision::AcceptForSession
        );
        assert_eq!(
            decide(&Decision::Selected(REJECT.into())),
            ApprovalDecision::Decline
        );
        assert_eq!(
            decide(&Decision::Selected("something-nobody-offered".into())),
            ApprovalDecision::Decline,
            "an option id this adapter never offered decides nothing"
        );
        assert_eq!(
            decide(&Decision::Unavailable),
            ApprovalDecision::Decline,
            "and neither does a proxy that could not be reached"
        );
        assert_eq!(
            decide(&Decision::Cancelled),
            ApprovalDecision::Cancel,
            "a cancelled turn withdraws the question instead of answering it"
        );
    }

    #[test]
    fn only_the_requests_this_adapter_can_relay_are_treated_as_approvals() {
        assert!(is_approval(wire::COMMAND_EXECUTION_APPROVAL));
        assert!(is_approval(wire::FILE_CHANGE_APPROVAL));
        assert!(!is_approval("thread/somethingElse"));
    }

    #[test]
    fn the_offered_options_are_the_ones_the_provider_can_honour() {
        // The provider has no "reject always", so none is offered: an option
        // that could not be kept would be a promise made in somebody else's
        // name.
        let request = request_for(&SessionId::new("s"), wire::FILE_CHANGE_APPROVAL, &params());
        assert!(
            !request
                .options
                .iter()
                .any(|option| option.kind == PermissionOptionKind::RejectAlways)
        );
        for option in &request.options {
            let id: &PermissionOptionId = &option.option_id;
            assert_ne!(
                decide(&Decision::Selected(id.0.as_ref().to_string())),
                ApprovalDecision::Cancel,
                "every offered option maps to a decision the provider accepts"
            );
        }
    }
}
