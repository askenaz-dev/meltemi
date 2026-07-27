// SPDX-License-Identifier: Apache-2.0

//! Playing a scripted JSON-RPC 2.0 conversation (adaptadores-propios-acp task
//! 1.4).
//!
//! A server wire cannot be a list of lines to print: every answer has to carry
//! the id of the request it answers, and the server asks questions of its own
//! (an approval) whose answers it must wait for. So the script is a small
//! transcript of four directives:
//!
//! - `{"mock":"await-request","method":"m"}` — wait until the client asks `m`,
//!   remembering its id;
//! - `{"mock":"respond","result":{…}}` — answer the request last awaited;
//! - `{"mock":"notify","method":"m","params":{…}}` — send a notification;
//! - `{"mock":"request","method":"m","params":{…}}` — ask the client, and wait
//!   for the answer before going on.
//!
//! While the wire waits, anything else the client asks is acknowledged with an
//! empty result instead of being left hanging. A fixture that deadlocked the
//! process under test would turn every adapter bug into the same timeout, and
//! teach nothing; asserting what the adapter *sends* is the adapter's own
//! tests' job.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::script::{Line, lines};

/// One step of a JSON-RPC transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Wait for a request with this method.
    AwaitRequest { method: String },
    /// Answer the request last awaited.
    Respond { result: Value },
    /// Send a notification.
    Notify { method: String, params: Value },
    /// Ask the client and wait for its answer.
    Request { method: String, params: Value },
}

/// Parses a transcript.
///
/// # Errors
///
/// Returns the offending line number and what is wrong with it.
pub fn parse(source: &str) -> Result<Vec<Step>, String> {
    lines(source)?.iter().map(step_of).collect()
}

fn step_of(line: &Line) -> Result<Step, String> {
    let at = line.number;
    let directive = line
        .value
        .get("mock")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("script line {at}: every line of a server wire is a directive"))?;
    let method = || {
        line.value
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("script line {at}: `{directive}` needs a `method`"))
    };
    let params = || line.value.get("params").cloned().unwrap_or(json!({}));
    match directive {
        "await-request" => Ok(Step::AwaitRequest { method: method()? }),
        "respond" => Ok(Step::Respond {
            result: line.value.get("result").cloned().unwrap_or(json!({})),
        }),
        "notify" => Ok(Step::Notify {
            method: method()?,
            params: params(),
        }),
        "request" => Ok(Step::Request {
            method: method()?,
            params: params(),
        }),
        other => Err(format!("script line {at}: unknown directive `{other}`")),
    }
}

/// Plays a transcript over the given streams. Returns when the script ends or
/// the client closes its side.
///
/// # Errors
///
/// Returns the I/O error of the underlying streams, or a script that asks to
/// answer a request nobody made.
pub fn play(
    steps: &[Step],
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<(), String> {
    let mut player = Player {
        input,
        output,
        awaited: None,
        next_id: 1,
    };
    for step in steps {
        if !player.run(step)? {
            // The client closed its side: nothing left to say to it.
            return Ok(());
        }
    }
    Ok(())
}

struct Player<'a, I: BufRead, O: Write> {
    input: &'a mut I,
    output: &'a mut O,
    /// The id of the request last awaited, which `respond` answers.
    awaited: Option<Value>,
    next_id: i64,
}

impl<I: BufRead, O: Write> Player<'_, I, O> {
    /// Runs one step. `false` means the client's side ended.
    fn run(&mut self, step: &Step) -> Result<bool, String> {
        match step {
            Step::AwaitRequest { method } => self.await_request(method),
            Step::Respond { result } => {
                let id = self
                    .awaited
                    .clone()
                    .ok_or_else(|| "the script responds to a request nobody made".to_string())?;
                self.send(&json!({"jsonrpc": "2.0", "id": id, "result": result}))?;
                Ok(true)
            }
            Step::Notify { method, params } => {
                self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}))?;
                Ok(true)
            }
            Step::Request { method, params } => {
                let id = self.next_id;
                self.next_id += 1;
                self.send(
                    &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
                )?;
                self.await_response(id)
            }
        }
    }

    /// Waits for the client's request with this method, acknowledging anything
    /// else it asks meanwhile.
    fn await_request(&mut self, method: &str) -> Result<bool, String> {
        loop {
            let Some(message) = self.receive()? else {
                return Ok(false);
            };
            let Some(id) = message.get("id").cloned() else {
                // A notification: nothing to answer, nothing to wait for.
                continue;
            };
            if message.get("method").and_then(Value::as_str) == Some(method) {
                self.awaited = Some(id);
                return Ok(true);
            }
            self.acknowledge(id)?;
        }
    }

    /// Waits for the answer to the request the wire asked.
    fn await_response(&mut self, id: i64) -> Result<bool, String> {
        loop {
            let Some(message) = self.receive()? else {
                return Ok(false);
            };
            let answers_us = message.get("id").and_then(Value::as_i64) == Some(id)
                && (message.get("result").is_some() || message.get("error").is_some());
            if answers_us {
                return Ok(true);
            }
            if let Some(other) = message.get("id").cloned()
                && message.get("method").is_some()
            {
                self.acknowledge(other)?;
            }
        }
    }

    /// Answers a request the script did not plan for, so the client is never
    /// left hanging.
    fn acknowledge(&mut self, id: Value) -> Result<(), String> {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "result": {}}))
    }

    fn send(&mut self, message: &Value) -> Result<(), String> {
        writeln!(self.output, "{message}").map_err(|e| format!("cannot write: {e}"))?;
        // The client is waiting on this line, not on a full buffer.
        self.output
            .flush()
            .map_err(|e| format!("cannot flush: {e}"))
    }

    /// The next message from the client, or `None` when it closed its side.
    fn receive(&mut self) -> Result<Option<Value>, String> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .input
                .read_line(&mut line)
                .map_err(|e| format!("cannot read: {e}"))?;
            if read == 0 {
                return Ok(None);
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            return serde_json::from_str(trimmed).map(Some).map_err(|error| {
                format!("client sent a line that is not JSON ({error}): {trimmed}")
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transcript_answers_with_the_id_it_was_asked_with() {
        // The whole reason a server wire is not a list of lines: the answer
        // belongs to the request, and only the request knows its id.
        let steps = parse(
            "{\"mock\":\"await-request\",\"method\":\"initialize\"}\n\
             {\"mock\":\"respond\",\"result\":{\"userAgent\":\"codex_cli_rs/0.77.0\"}}\n\
             {\"mock\":\"notify\",\"method\":\"turn/started\",\"params\":{\"threadId\":\"t1\"}}\n",
        )
        .unwrap();

        let mut input = std::io::Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":41,\"method\":\"initialize\",\"params\":{}}\n",
        );
        let mut output = Vec::new();
        play(&steps, &mut input, &mut output).unwrap();

        let sent: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(
            sent[0]["id"], 41,
            "the answer carries the id it was asked with"
        );
        assert_eq!(sent[0]["result"]["userAgent"], "codex_cli_rs/0.77.0");
        assert_eq!(sent[1]["method"], "turn/started");
        assert!(sent[1].get("id").is_none(), "a notification has no id");
    }

    #[test]
    fn the_wire_asks_and_waits_and_never_leaves_the_client_hanging() {
        // The approval: the server asks, the client answers, and whatever else
        // the client asks meanwhile is acknowledged rather than ignored — a
        // fixture that deadlocks turns every bug into the same timeout.
        let steps = parse(
            "{\"mock\":\"request\",\"method\":\"item/fileChange/requestApproval\",\"params\":{\"itemId\":\"i1\"}}\n\
             {\"mock\":\"notify\",\"method\":\"turn/completed\",\"params\":{\"threadId\":\"t1\"}}\n",
        )
        .unwrap();

        let mut input = std::io::Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"turn/interrupt\",\"params\":{}}\n\
             {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"decision\":\"accept\"}}\n",
        );
        let mut output = Vec::new();
        play(&steps, &mut input, &mut output).unwrap();

        let sent: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(sent[0]["method"], "item/fileChange/requestApproval");
        assert_eq!(sent[1]["id"], 7, "the unplanned request was acknowledged");
        assert_eq!(
            sent[2]["method"], "turn/completed",
            "and the transcript went on once the answer arrived"
        );
    }

    #[test]
    fn a_transcript_that_answers_nobody_is_rejected() {
        let steps = parse("{\"mock\":\"respond\",\"result\":{}}\n").unwrap();
        let mut input = std::io::Cursor::new("");
        let mut output = Vec::new();
        let error = play(&steps, &mut input, &mut output).expect_err("nothing was awaited");
        assert!(error.contains("nobody made"), "{error}");
    }
}
