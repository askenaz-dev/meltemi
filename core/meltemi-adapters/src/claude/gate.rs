// SPDX-License-Identifier: Apache-2.0

//! The hard gate: this same binary, run by the piloted CLI before every tool
//! call (adaptadores-propios-acp task 3.4, design D5).
//!
//! The prompt tool of [`super::shim`] is the CLI's *permission* channel, and it
//! has a gap by design: the CLI consults it only when none of its own static
//! rules already decided, so a permissive mode — or a rule in somebody's
//! settings file — can let a tool run without anyone being asked. That is the
//! gap this module closes. A `PreToolUse` hook runs **before every tool call,
//! in every mode**, including the CLI's most permissive one, and its decision
//! is final.
//!
//! So the hook is where the asking actually happens, and the prompt tool is the
//! belt to its braces: when the hook allows, the CLI's documented order skips
//! the rest and nobody is asked twice; when the hook cannot run at all, the
//! prompt tool still governs. Two channels, one authority — the permission
//! proxy meltemid already runs — and no path on which a tool call reaches the
//! filesystem without a decision.
//!
//! Like the shim, this process decides nothing. It asks over the same private
//! channel and denies when there is no answer.

use std::io::{Read, Write};
use std::path::Path;

use serde_json::{Value, json};

use super::rendezvous;

/// The flag that runs this binary as the hard gate, followed by the address of
/// the channel back to the adapter.
pub const GATE_ARG: &str = "--permission-gate";

/// The hook event this gate answers.
const PRE_TOOL_USE: &str = "PreToolUse";

/// How long the CLI is told to wait for a decision, in seconds.
///
/// The thing being waited on is a human, so the CLI's own default of about a
/// minute is far too short: it would turn "the reviewer stepped away" into a
/// tool call nobody decided. A hook that does time out is not a hole — the
/// prompt tool is still ahead of the call — but it would ask the human twice,
/// which is worth avoiding.
const TIMEOUT_SECONDS: u64 = 60 * 60;

/// The channel address the arguments select, when this binary was launched as
/// the gate.
#[must_use]
pub fn requested(args: &[String]) -> Option<String> {
    args.iter()
        .position(|arg| arg == GATE_ARG)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

/// The settings document that installs this gate in front of every tool call.
///
/// The matcher is everything, deliberately: a gate with a list of tools would
/// be a gate with a list of ways past it. What may run is decided by the proxy,
/// one call at a time, and the CLI's own configuration cannot narrow that.
#[must_use]
pub fn settings(exe: &Path, channel: &Path) -> Value {
    json!({
        "hooks": {
            PRE_TOOL_USE: [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": command(exe, channel),
                    "timeout": TIMEOUT_SECONDS,
                }],
            }],
        },
    })
}

/// The command line the CLI runs, quoted so that a path with spaces in it —
/// which is every default install location on Windows — survives the shell.
fn command(exe: &Path, channel: &Path) -> String {
    format!("\"{}\" {GATE_ARG} \"{}\"", exe.display(), channel.display())
}

/// Answers one hook invocation: read what the CLI is about to run, ask, and
/// print the decision.
///
/// # Errors
///
/// Returns the I/O error when stdio itself fails. Everything else — input that
/// cannot be read, a channel that cannot be reached — is a denial, not an
/// error: a gate that failed noisily and let the call through would be no gate.
pub fn decide(dir: &Path, input: &mut impl Read, output: &mut impl Write) -> std::io::Result<()> {
    let mut said = String::new();
    input.read_to_string(&mut said)?;
    let answer = verdict(dir, &said);
    writeln!(output, "{answer}")?;
    output.flush()
}

/// What the CLI is told about the call it is about to make.
fn verdict(dir: &Path, said: &str) -> Value {
    let Ok(event) = serde_json::from_str::<Value>(said) else {
        return denied("Meltemi could not read what the CLI was about to run, so it was refused");
    };
    let tool = event
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let input = event.get("tool_input").cloned().unwrap_or(Value::Null);
    let tool_use_id = event.get("tool_use_id").and_then(Value::as_str);

    let Some(decision) = rendezvous::ask(dir, tool, tool_use_id, &input) else {
        return denied("Meltemi could not be reached to decide this, so the call was refused");
    };
    // The channel answers in the vocabulary of the CLI's own permission tool,
    // because that is where the answer is composed; here it is translated into
    // the vocabulary of a hook. Same decision, two shapes, one author.
    if decision.get("behavior").and_then(Value::as_str) == Some("allow") {
        return json!({
            "hookSpecificOutput": {
                "hookEventName": PRE_TOOL_USE,
                "permissionDecision": "allow",
                "permissionDecisionReason": "allowed where Meltemi decides permissions",
            },
        });
    }
    denied(
        decision
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("the call was refused where Meltemi decides permissions"),
    )
}

/// A denial the CLI cannot argue with, carrying the reason into its own
/// transcript.
fn denied(reason: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": PRE_TOOL_USE,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::super::permission;
    use super::*;

    fn answering(dir: &Path, said: &str) -> Value {
        let mut input = std::io::Cursor::new(said.as_bytes().to_vec());
        let mut output = Vec::new();
        decide(dir, &mut input, &mut output).expect("stdio holds");
        serde_json::from_slice(&output).expect("the gate answers JSON")
    }

    fn decision_of(answer: &Value) -> &str {
        answer["hookSpecificOutput"]["permissionDecision"]
            .as_str()
            .unwrap_or_else(|| panic!("a hook answer carries a decision: {answer}"))
    }

    #[test]
    fn a_call_nobody_can_decide_is_denied_and_the_reason_travels_with_it() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The channel points nowhere: no decision is possible. A gate that let
        // the call through here would be no gate at all, and the CLI would be
        // running tools in the one situation where nobody is watching. What is
        // pinned is that the decision is the proxy's or it is a denial —
        // never this process's own.
        let answer = answering(
            Path::new("meltemi-no-such-channel"),
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /"}}"#,
        );
        assert_eq!(decision_of(&answer), "deny");
        assert!(
            answer["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .is_some_and(|said| said.contains("could not be reached")),
            "and it says why: {answer}"
        );
    }

    #[test]
    fn input_the_gate_cannot_read_is_denied_rather_than_waved_through() {
        let answer = answering(Path::new("nowhere"), "not json at all");
        assert_eq!(decision_of(&answer), "deny");
    }

    #[tokio::test]
    async fn what_the_proxy_allowed_is_what_the_gate_allows() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The other half: a decision does come back, and the gate carries it
        // rather than inventing one. The answer is composed in the vocabulary
        // of the CLI's permission tool and translated here — same decision, two
        // shapes.
        let dir = std::env::temp_dir().join(format!("meltemi-gate-test-{}", std::process::id()));
        let rendezvous = rendezvous::Rendezvous::open(dir.clone()).expect("the channel opens");

        let asking = tokio::task::spawn_blocking({
            let dir = dir.clone();
            move || {
                answering(
                    &dir,
                    r#"{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"a.txt"},"tool_use_id":"toolu_5"}"#,
                )
            }
        });

        let ask = rendezvous.next_ask().await;
        assert_eq!(ask.tool, "Read");
        assert_eq!(
            ask.tool_use_id.as_deref(),
            Some("toolu_5"),
            "so the question lands on the call the session already showed"
        );
        rendezvous.answer(
            &ask,
            &permission::payload(
                &permission::Decision::Selected(permission::ALLOW_ONCE.into()),
                &ask.input,
            ),
        );

        let answer = asking.await.expect("the gate finished");
        assert_eq!(decision_of(&answer), "allow");
        rendezvous.close();
    }

    #[test]
    fn the_gate_is_installed_in_front_of_every_tool_call_without_exception() {
        // Scenario: Compuerta dura sobre modo permisivo del CLI
        //
        // A gate with a list of tools would be a gate with a list of ways past
        // it. The matcher is everything, and the quoting survives the paths
        // Windows actually installs into.
        //
        // This is the half of that scenario Meltemi owns: the gate is installed
        // in front of everything, unconditionally, and nothing in the launch
        // reads the mode the CLI announces. The other half — that a CLI in
        // `bypassPermissions` really runs its `PreToolUse` hook and really obeys
        // a denial — is the provider's, documented by them and observable only
        // against their binary; the change's `.verify.jsonl` records exactly
        // that, and `e2e_adaptadores_claude_bypass` pins the whole chain
        // against a wire that announces the permissive mode.
        let settings = settings(
            Path::new(r"C:\Program Files\Meltemi\meltemi-claude-acp.exe"),
            Path::new(r"C:\Users\a b\AppData\Local\Temp\channel"),
        );
        let hook = &settings["hooks"][PRE_TOOL_USE][0];
        assert_eq!(hook["matcher"], "*");
        let command = hook["hooks"][0]["command"]
            .as_str()
            .expect("a command to run");
        assert!(
            command.starts_with('"') && command.contains(GATE_ARG),
            "the binary is quoted and the mode is named: {command}"
        );
        assert!(
            command.contains("\"C:\\Users\\a b\\"),
            "and so is a channel inside a path with a space: {command}"
        );
        assert!(
            hook["hooks"][0]["timeout"].as_u64().unwrap_or_default() >= 600,
            "the CLI waits as long as a human takes: {hook}"
        );
    }

    #[test]
    fn the_gate_is_selected_by_the_argument_that_carries_its_channel() {
        let args = vec![
            "meltemi-claude-acp".to_string(),
            GATE_ARG.to_string(),
            "/tmp/channel".to_string(),
        ];
        assert_eq!(requested(&args), Some("/tmp/channel".to_string()));
        assert_eq!(requested(&args[..1]), None);
    }
}
