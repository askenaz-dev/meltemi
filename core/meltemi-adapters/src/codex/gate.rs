// SPDX-License-Identifier: Apache-2.0

//! The CLI's native hard gate: asked for when the thread opens, and verified
//! against what the CLI made effective (codex-al-dia design D5).
//!
//! `own-adapters` requires the adapter to configure the CLI's native hard gate
//! "even in the CLI's permissive modes", so that no tool call proceeds without
//! passing through the permission proxy. For this provider the gate is three
//! values of `thread/start`: the approval policy, the sandbox and who reviews
//! the approvals. The adapter asks for the strictest of each — ask for every
//! command that is not a trusted read, write nothing without asking, and send
//! every question to the client, which here is the proxy — and it only ever
//! narrows: whatever the proxy then allows, it allows by rule or by a human.
//!
//! What the CLI answers is checked, not trusted. The thread's answer carries
//! the effective values, and the session opens only on exactly the ones asked
//! for. Anything else — a managed configuration that imposes its own, a value a
//! later version adds, a field that is missing — refuses: the check fails
//! closed, so a CLI that changes these values leaves the user without a
//! session, never with a session without a gate.
//!
//! Measured against codex-cli 0.156.1 on 2026-09-24: asked for this gate, the
//! server answered `untrusted`, `user` and `{"type":"readOnly",
//! "networkAccess":false}`; an approved write under the read-only sandbox was
//! applied, and a command that was not a trusted read asked before it ran.

use serde_json::Value;

use super::wire::ThreadStartResult;
use crate::diagnostic::Refusal;

/// The approval policy asked for: ask for every command the CLI does not
/// consider a trusted read.
pub const APPROVAL_POLICY: &str = "untrusted";

/// Who reviews the approvals: the client, which for this adapter is the
/// permission proxy.
pub const APPROVALS_REVIEWER: &str = "user";

/// The sandbox asked for, as `thread/start` takes it.
pub const SANDBOX_MODE: &str = "read-only";

/// The same sandbox, as the thread's answer reports it.
pub const SANDBOX_TYPE: &str = "readOnly";

/// Every approval policy the vendored dump declares; the object form is named
/// by its only key. Pinned by the conformance suite, so a value the provider
/// adds fails the next re-anchoring instead of arriving unannounced.
pub const KNOWN_APPROVAL_POLICIES: &[&str] = &["granular", "never", "on-request", "untrusted"];

/// Every reviewer the vendored dump declares.
pub const KNOWN_REVIEWERS: &[&str] = &["auto_review", "guardian_subagent", "user"];

/// Every sandbox the vendored dump declares, by the `type` the answer carries.
pub const KNOWN_SANDBOXES: &[&str] = &[
    "dangerFullAccess",
    "externalSandbox",
    "readOnly",
    "workspaceWrite",
];

/// The gate the CLI made effective, as its answer reported it, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective {
    /// The effective approval policy.
    pub approval_policy: Value,
    /// Who the effective approvals go to.
    pub approvals_reviewer: Value,
    /// The effective sandbox.
    pub sandbox: Value,
}

/// Checks the gate the thread's answer reports against the one asked for.
///
/// # Errors
///
/// Refuses the session, with the values seen and where to look, unless the
/// approval policy is exactly [`APPROVAL_POLICY`], the reviewer exactly
/// [`APPROVALS_REVIEWER`], and the sandbox is [`SANDBOX_TYPE`] without network
/// access.
pub fn verify(layer: &str, answer: &ThreadStartResult) -> Result<Effective, Refusal> {
    let effective = Effective {
        approval_policy: answer.approval_policy.clone().unwrap_or(Value::Null),
        approvals_reviewer: answer.approvals_reviewer.clone().unwrap_or(Value::Null),
        sandbox: answer.sandbox.clone().unwrap_or(Value::Null),
    };
    let policy = effective.approval_policy.as_str() == Some(APPROVAL_POLICY);
    let reviewer = effective.approvals_reviewer.as_str() == Some(APPROVALS_REVIEWER);
    let sandbox = effective.sandbox.get("type").and_then(Value::as_str) == Some(SANDBOX_TYPE)
        && matches!(
            effective.sandbox.get("networkAccess"),
            None | Some(Value::Bool(false))
        );
    if policy && reviewer && sandbox {
        Ok(effective)
    } else {
        Err(refused(layer, &effective))
    }
}

/// How a value of the gate reads in a diagnosis: as sent, or `absent`.
fn shown(value: &Value) -> String {
    match value {
        Value::Null => "absent".to_string(),
        Value::String(text) => format!("`{text}`"),
        other => format!("`{other}`"),
    }
}

/// The refusal: what was asked, what came back, and where the override lives.
fn refused(layer: &str, effective: &Effective) -> Refusal {
    Refusal::new(
        "provider_gate_refused",
        layer,
        format!(
            "Meltemi asked for approval policy `{APPROVAL_POLICY}`, the client as reviewer \
             (`{APPROVALS_REVIEWER}`) and a read-only sandbox without network, and the thread came \
             back with approval policy {}, reviewer {} and sandbox {}; under that, tools could run \
             without the permission proxy deciding",
            shown(&effective.approval_policy),
            shown(&effective.approvals_reviewer),
            shown(&effective.sandbox)
        ),
        "Something in the Codex configuration overrides what Meltemi asks for: look for \
         `approval_policy`, `approvals_reviewer` and `sandbox_mode` in its config.toml, in a \
         profile, or in a managed requirements file, and remove the override for sessions under \
         Meltemi. A Codex session opens only under this gate; the permission proxy, its rules and \
         the autonomy modes are where Meltemi relaxes it.",
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const LAYER: &str = "the official `codex` CLI";

    fn answering(policy: Value, reviewer: Value, sandbox: Value) -> ThreadStartResult {
        let mut answer = json!({"thread": {"id": "thread-1"}});
        for (key, value) in [
            ("approvalPolicy", policy),
            ("approvalsReviewer", reviewer),
            ("sandbox", sandbox),
        ] {
            if !value.is_null() {
                answer[key] = value;
            }
        }
        serde_json::from_value(answer).expect("the answer parses")
    }

    fn gated() -> ThreadStartResult {
        answering(
            json!("untrusted"),
            json!("user"),
            json!({"type": "readOnly", "networkAccess": false}),
        )
    }

    #[test]
    fn the_gate_asked_for_opens_the_session() {
        // Scenario: La compuerta efectiva consta en el log
        //
        // The values measured against 0.156.1 when asked for, kept verbatim so
        // the log can say under which gate the CLI ran.
        let effective = verify(LAYER, &gated()).expect("exactly what was asked");
        assert_eq!(effective.approval_policy, json!("untrusted"));
        assert_eq!(effective.approvals_reviewer, json!("user"));
        assert_eq!(
            effective.sandbox,
            json!({"type": "readOnly", "networkAccess": false})
        );
        // A read-only sandbox that does not mention the network has none.
        assert!(
            verify(
                LAYER,
                &answering(
                    json!("untrusted"),
                    json!("user"),
                    json!({"type": "readOnly"})
                )
            )
            .is_ok()
        );
    }

    #[test]
    fn an_effective_value_other_than_the_one_asked_for_refuses() {
        // Scenario: Un valor efectivo distinto del pedido rehúsa la sesión
        let read_only = json!({"type": "readOnly", "networkAccess": false});
        for (policy, reviewer, sandbox) in [
            (json!("on-request"), json!("user"), read_only.clone()),
            (json!("never"), json!("user"), read_only.clone()),
            (
                json!({"granular": {"mcp_elicitations": false, "rules": false, "sandbox_approval": false}}),
                json!("user"),
                read_only.clone(),
            ),
            (json!("untrusted"), json!("auto_review"), read_only.clone()),
            (
                json!("untrusted"),
                json!("guardian_subagent"),
                read_only.clone(),
            ),
            (
                json!("untrusted"),
                json!("user"),
                json!({"type": "workspaceWrite", "networkAccess": false}),
            ),
            (
                json!("untrusted"),
                json!("user"),
                json!({"type": "externalSandbox"}),
            ),
            (
                json!("untrusted"),
                json!("user"),
                json!({"type": "readOnly", "networkAccess": true}),
            ),
        ] {
            let refusal = verify(
                LAYER,
                &answering(policy.clone(), reviewer.clone(), sandbox.clone()),
            )
            .expect_err("anything but the gate asked for refuses");
            assert_eq!(refusal.kind, "provider_gate_refused");
            assert!(
                refusal.remedy.contains("approval_policy")
                    && refusal.remedy.contains("sandbox_mode"),
                "the remedy says where the override lives: {}",
                refusal.remedy
            );
        }
    }

    #[test]
    fn the_most_permissive_configuration_does_not_open_a_session() {
        // Scenario: Compuerta dura sobre modo permisivo del CLI
        //
        // A CLI configured to never ask and to run without a sandbox would run
        // every tool past the proxy. Asked for the gate and answering that, it
        // does not get a session at all.
        let refusal = verify(
            LAYER,
            &answering(
                json!("never"),
                json!("user"),
                json!({"type": "dangerFullAccess"}),
            ),
        )
        .expect_err("the most permissive mode never opens");
        assert!(
            refusal.detail.contains("`never`") && refusal.detail.contains("dangerFullAccess"),
            "the diagnosis names what came back: {}",
            refusal.detail
        );
    }

    #[test]
    fn an_unknown_or_missing_value_refuses() {
        // Scenario: Un valor desconocido o ausente rehúsa la sesión
        let read_only = json!({"type": "readOnly"});
        for (policy, reviewer, sandbox) in [
            (Value::Null, json!("user"), read_only.clone()),
            (json!("untrusted"), Value::Null, read_only.clone()),
            (json!("untrusted"), json!("user"), Value::Null),
            (json!("sometimes"), json!("user"), read_only.clone()),
            (json!("untrusted"), json!("committee"), read_only.clone()),
            (
                json!("untrusted"),
                json!("user"),
                json!({"type": "readOnlyish"}),
            ),
            (
                json!("untrusted"),
                json!("user"),
                json!({"type": "readOnly", "networkAccess": "restricted"}),
            ),
        ] {
            let refusal = verify(LAYER, &answering(policy, reviewer, sandbox))
                .expect_err("the check fails closed");
            assert_eq!(refusal.kind, "provider_gate_refused");
        }
        assert!(
            verify(LAYER, &answering(Value::Null, json!("user"), read_only))
                .unwrap_err()
                .detail
                .contains("approval policy absent"),
            "a missing value is named as missing"
        );
    }
}
