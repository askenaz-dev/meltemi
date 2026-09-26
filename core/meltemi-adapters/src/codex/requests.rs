// SPDX-License-Identifier: Apache-2.0

//! What the adapter answers to each request the server may make of it
//! (codex-al-dia design D7).
//!
//! The dumped `ServerRequest.json` declares ten. The two approvals are relayed
//! to the permission proxy; every other one is refused with the protocol's
//! "method not found", and every refusal is shown in the session by what it is,
//! never with its parameters. The conformance suite walks the vendored dump and
//! fails on any method this table does not answer — and on any answer for a
//! method the dump no longer declares — so a request the provider adds cannot
//! arrive unannounced.
//!
//! What the server does with the refusal, read in the tag the dump came from:
//!
//! | Request | Effect of `-32601` |
//! | --- | --- |
//! | `item/permissions/requestApproval` | an empty grant for the turn |
//! | `mcpServer/elicitation/request` | declined |
//! | `item/tool/requestUserInput` | empty answers; the turn goes on |
//! | `item/tool/call` | `success: false`; only sent for dynamic tools, which this adapter never declares |
//! | `account/chatgptAuthTokens/refresh` | the backend call fails; only exists under external authentication |
//! | `attestation/generate` | never sent: it needs `capabilities.requestAttestation` |
//! | `applyPatchApproval`, `execCommandApproval` | never sent: they belong to v1 turns, which can no longer be started |
//!
//! Two of them ask for what Meltemi never touches: the account's credentials
//! and an attestation of the client's identity (constitution §2). They are
//! refused like the rest — without the proxy, and without anything of theirs
//! kept — and [`NEVER_TO_THE_PROXY`] names them so a test can pin it.

use super::wire;

/// What the adapter does with one kind of request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Put to the permission proxy.
    Relay,
    /// Refused, for this reason, in the words the session shows.
    Refuse(&'static str),
}

/// Every request the dumped schema allows the server, and its answer.
pub const POLICY: &[(&str, Answer)] = &[
    (wire::COMMAND_EXECUTION_APPROVAL, Answer::Relay),
    (wire::FILE_CHANGE_APPROVAL, Answer::Relay),
    (
        wire::TOOL_REQUEST_USER_INPUT,
        Answer::Refuse("a question from a tool, which this adapter does not relay yet"),
    ),
    (
        wire::MCP_SERVER_ELICITATION,
        Answer::Refuse("an MCP server asking for input, which this adapter does not relay yet"),
    ),
    (
        wire::PERMISSIONS_APPROVAL,
        Answer::Refuse("more permissions for the turn, which this adapter does not grant"),
    ),
    (
        wire::DYNAMIC_TOOL_CALL,
        Answer::Refuse("a dynamic tool, which this adapter never declared"),
    ),
    (
        wire::AUTH_TOKENS_REFRESH,
        Answer::Refuse("account credentials, which Meltemi never handles"),
    ),
    (
        wire::ATTESTATION_GENERATE,
        Answer::Refuse("an attestation of the client's identity, which Meltemi never gives"),
    ),
    (
        wire::APPLY_PATCH_APPROVAL_V1,
        Answer::Refuse("an approval of the retired v1 turns, which this adapter never starts"),
    ),
    (
        wire::EXEC_COMMAND_APPROVAL_V1,
        Answer::Refuse("an approval of the retired v1 turns, which this adapter never starts"),
    ),
];

/// The requests that ask for credentials or for the client's identity: never
/// put to the proxy, and nothing of them kept (constitution §2).
pub const NEVER_TO_THE_PROXY: &[&str] = &[wire::AUTH_TOKENS_REFRESH, wire::ATTESTATION_GENERATE];

/// The answer for a request a later version sends and this table does not
/// know: refused like the others, and in sight.
const UNKNOWN: Answer = Answer::Refuse("a request this adapter does not know");

/// The answer to a request, by its method.
#[must_use]
pub fn answer_for(method: &str) -> Answer {
    POLICY
        .iter()
        .find(|(known, _)| *known == method)
        .map_or(UNKNOWN, |(_, answer)| *answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::permission;

    #[test]
    fn exactly_the_approvals_are_relayed() {
        for (method, answer) in POLICY {
            assert_eq!(
                *answer == Answer::Relay,
                permission::is_approval(method),
                "`{method}`: what the relay treats as an approval is what the table relays"
            );
        }
    }

    #[test]
    fn credentials_and_identity_are_refused() {
        // Scenario: Credenciales y atestación jamás llegan al proxy
        for method in NEVER_TO_THE_PROXY {
            assert!(
                matches!(answer_for(method), Answer::Refuse(_)),
                "`{method}` never reaches the proxy"
            );
        }
    }

    #[test]
    fn a_request_nobody_declared_is_refused_too() {
        // Scenario: Una petición no relevada se rehúsa a la vista
        assert_eq!(answer_for("item/somethingNew/request"), UNKNOWN);
    }
}
