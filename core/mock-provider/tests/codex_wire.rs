// SPDX-License-Identifier: Apache-2.0

//! The scripted server wire behaves as the adapter will need it to
//! (adaptadores-propios-acp task 1.4).
//!
//! The test drives the fixture the way the adapter will: a real process, real
//! pipes, one JSON-RPC conversation from handshake to closed turn — including
//! the approval the server asks for and waits on. A broken fixture fails here,
//! not in the middle of an adapter's test where it would read as an adapter
//! bug.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// The wire under test, launched the way the adapter launches the real CLI.
struct Wire {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Wire {
    fn start(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mock-codex-wire"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("the scripted wire is built next to this test");
        let input = child.stdin.take().expect("piped stdin");
        let output = BufReader::new(child.stdout.take().expect("piped stdout"));
        Self {
            child,
            input,
            output,
        }
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.input, "{message}").expect("write to the wire");
        self.input.flush().expect("flush");
    }

    /// The next message that satisfies the predicate, collecting what came
    /// before it.
    fn read_until(&mut self, matches: impl Fn(&Value) -> bool) -> (Value, Vec<Value>) {
        let mut before = Vec::new();
        loop {
            let mut line = String::new();
            let read = self
                .output
                .read_line(&mut line)
                .expect("read from the wire");
            assert_ne!(read, 0, "the wire closed before the awaited message");
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let message: Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("not JSON ({e}): {trimmed}"));
            if matches(&message) {
                return (message, before);
            }
            before.push(message);
        }
    }

    /// Closes the client side and asserts the wire ends cleanly.
    fn finish(mut self) {
        drop(self.input);
        let status = self.child.wait().expect("the wire ends on its own");
        assert_eq!(status.code(), Some(0), "the wire exits cleanly");
    }
}

fn is_response_to(id: i64) -> impl Fn(&Value) -> bool {
    move |m: &Value| m.get("id").and_then(Value::as_i64) == Some(id) && m.get("result").is_some()
}

#[test]
fn the_wire_plays_a_whole_conversation_from_handshake_to_closed_turn() {
    let mut wire = Wire::start(&["app-server"]);

    // 1. Handshake: the version the adapter checks for skew.
    wire.send(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"clientInfo": {"name": "meltemi-codex-acp", "version": "0.1.0"}}
    }));
    let (initialized, _) = wire.read_until(is_response_to(1));
    assert!(
        initialized["result"]["appServerVersion"].is_string(),
        "the handshake announces a version: {initialized}"
    );

    // 2. A thread: what the ACP session maps onto.
    wire.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "newThread", "params": {"cwd": "/mock/project"}
    }));
    let (thread, _) = wire.read_until(is_response_to(2));
    let thread_id = thread["result"]["threadId"]
        .as_str()
        .expect("the thread has an id")
        .to_string();

    // 3. A turn, streamed as items, up to the approval the server asks for.
    wire.send(&json!({
        "jsonrpc": "2.0", "id": 3, "method": "sendUserTurn",
        "params": {"threadId": thread_id, "items": [{"type": "text", "text": "do it"}]}
    }));
    let (approval, streamed) =
        wire.read_until(|m| m.get("method").and_then(Value::as_str) == Some("applyPatchApproval"));

    let events: Vec<&str> = streamed
        .iter()
        .filter_map(|m| m["params"]["event"]["type"].as_str())
        .collect();
    assert!(
        events.contains(&"turn.started") && events.contains(&"item.completed"),
        "the turn streams its items before asking anything: {events:?}"
    );
    assert!(
        approval["id"].is_i64(),
        "the approval is a request, not a notification: it must be answered"
    );
    assert!(approval["params"]["fileChanges"].is_object());

    // 4. The decision comes from the client — here standing in for the proxy.
    wire.send(&json!({
        "jsonrpc": "2.0", "id": approval["id"].clone(), "result": {"decision": "approved"}
    }));

    // 5. Only then does the turn close and the turn request get its answer.
    let (turn, after) = wire.read_until(is_response_to(3));
    let closing: Vec<&str> = after
        .iter()
        .filter_map(|m| m["params"]["event"]["type"].as_str())
        .collect();
    assert!(
        closing.contains(&"turn.completed"),
        "the turn completes after the decision: {closing:?}"
    );
    assert_eq!(turn["result"]["status"], "completed");

    wire.finish();
}

#[test]
fn the_wire_dumps_the_schema_of_what_it_speaks() {
    // The provider CLI dumps its schema per version and this fixture mirrors
    // it, so the conformance test can run in CI with no provider binary
    // anywhere.
    let dumped = Command::new(env!("CARGO_BIN_EXE_mock-codex-wire"))
        .args(["app-server", "generate-json-schema"])
        .output()
        .expect("the scripted wire is built next to this test");
    assert_eq!(dumped.status.code(), Some(0));

    let schema: Value = serde_json::from_slice(&dumped.stdout).expect("the dump is JSON");
    assert!(
        schema["$defs"]["ApplyPatchApprovalParams"].is_object(),
        "the dump describes the approval the wire asks for"
    );
    assert!(
        schema["$defs"]["InitializeResult"]["properties"]["appServerVersion"].is_object(),
        "and the version field a skew check reads"
    );
}
