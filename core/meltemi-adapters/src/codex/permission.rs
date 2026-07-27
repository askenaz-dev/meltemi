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
    TextContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
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

/// The permission request the proxy will decide.
///
/// It is addressed to the **same tool call id** the item was streamed under, so
/// a client shows the question against the command or the diff it already
/// displayed instead of against an anonymous prompt. The provider's reason,
/// when it gives one, travels as the content of that update: at the moment of
/// deciding, the reason is the thing the human needs.
#[must_use]
pub fn request_for(
    session_id: &SessionId,
    method: &str,
    params: &wire::ApprovalParams,
) -> RequestPermissionRequest {
    let kind = if method == wire::COMMAND_EXECUTION_APPROVAL {
        ToolKind::Execute
    } else {
        ToolKind::Edit
    };
    let mut fields = ToolCallUpdateFields::new()
        .kind(kind)
        .status(ToolCallStatus::Pending);
    if let Some(reason) = params.reason.as_ref().filter(|reason| !reason.is_empty()) {
        fields = fields.content(vec![ContentBlock::Text(TextContent::new(reason)).into()]);
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
        }
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
