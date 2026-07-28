// SPDX-License-Identifier: Apache-2.0

//! The permission shim: this same binary, run as an MCP server by the piloted
//! CLI (adaptadores-propios-acp task 3.3, design D5).
//!
//! The CLI's documented way to ask a third party for permission is to call a
//! tool on one of its MCP servers. So the adapter registers one — itself, in
//! this mode — and points the CLI's permission flag at its single tool. Every
//! call arrives here, is relayed to the adapter over the private channel of
//! [`super::rendezvous`], and is answered with whatever meltemid's permission
//! proxy decided.
//!
//! What this process is **not**: a decision-maker. It holds no rules, reads no
//! configuration, and has no opinion about any tool. If it cannot reach the
//! adapter, if the wait runs out, if anything at all goes wrong, it answers
//! **deny** — a shim that failed open would be a hole under every rule the
//! human wrote.
//!
//! It is deliberately synchronous and single-threaded. One CLI is asking about
//! one tool call at a time and is blocked until it hears back; a runtime here
//! would buy concurrency nobody is waiting for, in the one process of this
//! architecture that most needs to be too simple to be wrong.

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::{Value, json};

use super::{permission, rendezvous};

/// The flag that runs this binary as the permission shim, followed by the
/// address of the channel back to the adapter that launched the CLI.
pub const SHIM_ARG: &str = "--permission-shim";

/// JSON-RPC's own code for a method the receiver does not implement.
const METHOD_NOT_FOUND: i64 = -32601;

/// The MCP revision this shim is written against. Only used when the caller
/// announces none: the CLI's own version is echoed back otherwise, because a
/// shim that argued about protocol revisions with the process that launched it
/// would be failing closed for no reason anyone could act on.
const MCP_VERSION: &str = "2025-06-18";

/// The channel address the arguments select, when this binary was launched as a
/// shim.
#[must_use]
pub fn requested(args: &[String]) -> Option<String> {
    args.iter()
        .position(|arg| arg == SHIM_ARG)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

/// Serves the CLI until it closes this process's input.
///
/// # Errors
///
/// Returns the I/O error when stdio itself fails, which is the one thing this
/// process cannot answer around.
pub fn serve(dir: &Path, input: &mut impl BufRead, output: &mut impl Write) -> std::io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
            // Not JSON at all. There is nothing to answer — a response needs an
            // id — and nothing to be gained by dying: the CLI owns this
            // process's lifetime.
            continue;
        };
        let Some(answer) = handle(dir, &message) else {
            continue;
        };
        writeln!(output, "{answer}")?;
        // The caller is waiting on a line, not on a full buffer.
        output.flush()?;
    }
}

/// What one message from the CLI is answered with, or `None` for a
/// notification, which nothing answers.
fn handle(dir: &Path, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned()?;
    let method = message.get("method").and_then(Value::as_str)?;
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => result(&id, &initialized(&params)),
        "tools/list" => result(&id, &json!({"tools": [tool()]})),
        "tools/call" => result(&id, &call(dir, &params)),
        "ping" => result(&id, &json!({})),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": METHOD_NOT_FOUND, "message": "this server implements only tools/call"},
        }),
    })
}

/// The handshake answer: the caller's own protocol revision, and the one
/// capability this server has.
fn initialized(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(MCP_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {
            "name": permission::SERVER,
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

/// The single tool this server offers.
fn tool() -> Value {
    json!({
        "name": permission::TOOL,
        "description": "Asks Meltemi's permission proxy whether a tool call may proceed. \
                        The answer is the proxy's, never this server's.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "tool_name": {"type": "string", "description": "The tool awaiting a decision."},
                "input": {"type": "object", "description": "The arguments it would run with."},
                "tool_use_id": {"type": "string", "description": "The id of the call, when there is one."},
            },
            "required": ["tool_name", "input"],
        },
    })
}

/// Relays one permission question and answers with what came back.
fn call(dir: &Path, params: &Value) -> Value {
    if params.get("name").and_then(Value::as_str) != Some(permission::TOOL) {
        return payload(&permission::deny(
            "this server offers no such tool, so the call was refused",
        ));
    }
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    let tool = arguments
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let input = arguments.get("input").cloned().unwrap_or(Value::Null);
    let tool_use_id = arguments.get("tool_use_id").and_then(Value::as_str);

    // The whole of this process's judgement: ask, and deny when there is no
    // answer. It does not know what the tool does and must not learn.
    let decided = rendezvous::ask(dir, tool, tool_use_id, &input).unwrap_or_else(|| {
        permission::deny("Meltemi could not be reached to decide this, so the request was refused")
    });
    payload(&decided)
}

/// The tool result the CLI reads its decision out of: the payload as text,
/// which is the shape this channel takes.
fn payload(decision: &Value) -> Value {
    json!({
        "content": [{"type": "text", "text": decision.to_string()}],
    })
}

fn result(id: &Value, result: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives the shim over in-memory stdio and hands back what it said.
    fn serving(dir: &Path, said: &str) -> Vec<Value> {
        let mut input = std::io::Cursor::new(said.as_bytes().to_vec());
        let mut output = Vec::new();
        serve(dir, &mut input, &mut output).expect("stdio holds");
        String::from_utf8(output)
            .expect("the shim speaks UTF-8")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("and JSON"))
            .collect()
    }

    /// What the CLI reads out of a tool result.
    fn decision_in(answer: &Value) -> Value {
        serde_json::from_str(
            answer["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("a tool result carries text: {answer}")),
        )
        .expect("and that text is the decision")
    }

    #[test]
    fn the_shim_announces_one_tool_and_the_caller_s_own_protocol_revision() {
        let said = serving(
            Path::new("nowhere"),
            concat!(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01"}}"#,
                "\n",
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                "\n",
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
                "\n",
            ),
        );
        assert_eq!(said.len(), 2, "a notification is not answered: {said:?}");
        assert_eq!(
            said[0]["result"]["protocolVersion"], "2099-01-01",
            "arguing about revisions with the process that launched us would fail closed for nothing"
        );
        assert_eq!(said[1]["result"]["tools"][0]["name"], permission::TOOL);
    }

    #[test]
    fn a_call_the_shim_cannot_relay_is_denied_and_never_allowed() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The channel points at a directory that does not exist: nobody can be
        // asked. A shim that failed open here would be a hole under every rule
        // the human wrote.
        let said = serving(
            Path::new("meltemi-no-such-channel"),
            concat!(
                r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"approve","#,
                r#""arguments":{"tool_name":"Bash","input":{"command":"rm -rf /"}}}}"#,
                "\n",
            ),
        );
        let decision = decision_in(&said[0]);
        assert_eq!(decision["behavior"], "deny");
        assert!(
            decision["message"]
                .as_str()
                .is_some_and(|said| said.contains("could not be reached")),
            "and it says why: {decision}"
        );
    }

    #[test]
    fn a_tool_this_server_does_not_offer_is_denied_rather_than_ignored() {
        let said = serving(
            Path::new("nowhere"),
            concat!(
                r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"something_else","arguments":{}}}"#,
                "\n",
            ),
        );
        assert_eq!(decision_in(&said[0])["behavior"], "deny");
    }

    #[test]
    fn a_method_this_server_does_not_implement_gets_the_protocols_own_answer() {
        let said = serving(
            Path::new("nowhere"),
            "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"resources/list\"}\n",
        );
        assert_eq!(said[0]["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn the_shim_is_selected_by_the_argument_that_carries_its_channel() {
        let args = vec![
            "meltemi-claude-acp".to_string(),
            SHIM_ARG.to_string(),
            "/tmp/channel".to_string(),
        ];
        assert_eq!(requested(&args), Some("/tmp/channel".to_string()));
        assert_eq!(requested(&args[..1]), None);
        assert_eq!(
            requested(&args[..2]),
            None,
            "the flag without an address is not an instruction"
        );
    }

    #[tokio::test]
    async fn a_whole_relay_answers_the_cli_with_what_the_adapter_decided() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The shim's half of the round trip, over a real channel: the shim runs
        // where it always does, on a thread of its own with nothing async about
        // it, and what the CLI reads is exactly what the other side decided.
        let dir = std::env::temp_dir().join(format!("meltemi-shim-test-{}", std::process::id()));
        let rendezvous = rendezvous::Rendezvous::open(dir.clone()).expect("the channel opens");

        let shim = tokio::task::spawn_blocking({
            let dir = dir.clone();
            move || {
                serving(
                    &dir,
                    concat!(
                        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"approve","#,
                        r#""arguments":{"tool_name":"Write","tool_use_id":"toolu_1","input":{"file_path":"NOTES.md"}}}}"#,
                        "\n",
                    ),
                )
            }
        });

        let ask = rendezvous.next_ask().await;
        assert_eq!(ask.tool, "Write");
        assert_eq!(ask.tool_use_id.as_deref(), Some("toolu_1"));
        rendezvous.answer(
            &ask,
            &json!({"behavior": "allow", "updatedInput": ask.input}),
        );

        let said = shim.await.expect("the shim finished");
        let decision = decision_in(&said[0]);
        assert_eq!(decision["behavior"], "allow");
        assert_eq!(
            decision["updatedInput"]["file_path"], "NOTES.md",
            "an allowed call runs exactly as it was approved"
        );
        rendezvous.close();
    }
}
