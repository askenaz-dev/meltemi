// SPDX-License-Identifier: Apache-2.0

//! Wire scripts: what a scripted provider emits, and when it waits.
//!
//! A script is a text file of lines:
//!
//! - a line starting with `#` is a comment (the fixtures explain their own
//!   wire, which a plain `.json` file could not do);
//! - `{"mock":"await-input"}` waits for one line of input before continuing;
//! - every other line must be valid JSON and is emitted verbatim.
//!
//! Emitted verbatim matters: a fixture that re-serialized its lines would
//! normalize key order and drop the exact bytes a real provider sent, and the
//! point of freezing a wire is to freeze it.

use std::io::{BufRead, Write};

/// One step of a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Emit this line verbatim.
    Emit(String),
    /// Wait for one line of input before going on.
    AwaitInput,
}

/// Parses a script, rejecting a line that is neither a comment nor JSON: a
/// fixture must fail at startup, never halfway through a turn.
///
/// # Errors
///
/// Returns the offending line number and its content.
pub fn parse(source: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|error| format!("script line {}: not JSON ({error}): {trimmed}", index + 1))?;
        match value.get("mock").and_then(serde_json::Value::as_str) {
            Some("await-input") => steps.push(Step::AwaitInput),
            Some(other) => {
                return Err(format!(
                    "script line {}: unknown directive `{other}`",
                    index + 1
                ));
            }
            None => steps.push(Step::Emit(trimmed.to_string())),
        }
    }
    Ok(steps)
}

/// Loads the script to play: `--script <path>` wins, then the environment
/// variable, else the binary's embedded default.
///
/// # Errors
///
/// Returns a message naming the unreadable path or the offending line.
pub fn load(args: &[String], env_var: &str, embedded: &str) -> Result<Vec<Step>, String> {
    let from_args = args
        .iter()
        .position(|a| a == "--script")
        .and_then(|at| args.get(at + 1))
        .cloned();
    let path = from_args.or_else(|| std::env::var(env_var).ok());
    match path {
        Some(path) => {
            let source = std::fs::read_to_string(&path)
                .map_err(|error| format!("script `{path}` cannot be read: {error}"))?;
            parse(&source)
        }
        None => parse(embedded),
    }
}

/// Plays a script over the given input and output.
///
/// The first `await-input` is the turn boundary: once the script is exhausted
/// the player loops back to it, so a preamble (a handshake, an init event) is
/// emitted once and every further input replays a turn. A script with no
/// `await-input` is emitted once and ends.
///
/// `on_input` sees every input line: a fixture uses it to assert the adapter
/// speaks the dialect it claims to.
///
/// # Errors
///
/// Returns the I/O error of the underlying streams, or whatever `on_input`
/// rejected.
pub fn play(
    steps: &[Step],
    input: &mut impl BufRead,
    output: &mut impl Write,
    mut on_input: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let turn_start = steps
        .iter()
        .position(|step| *step == Step::AwaitInput)
        .unwrap_or(steps.len());
    let mut at = 0;
    loop {
        let Some(step) = steps.get(at) else {
            if turn_start >= steps.len() {
                return Ok(());
            }
            at = turn_start;
            continue;
        };
        match step {
            Step::Emit(line) => {
                writeln!(output, "{line}").map_err(|e| format!("cannot write: {e}"))?;
                // The adapter is waiting on this line, not on a full buffer.
                output.flush().map_err(|e| format!("cannot flush: {e}"))?;
            }
            Step::AwaitInput => {
                let mut line = String::new();
                loop {
                    line.clear();
                    let read = input
                        .read_line(&mut line)
                        .map_err(|e| format!("cannot read: {e}"))?;
                    if read == 0 {
                        // End of input: the adapter is done with us.
                        return Ok(());
                    }
                    if !line.trim().is_empty() {
                        break;
                    }
                }
                on_input(line.trim())?;
            }
        }
        at += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_emits_its_preamble_once_and_replays_a_turn_per_input() {
        // The shape every wire fixture needs: a handshake before any input,
        // then one scripted turn for each message the adapter sends.
        let steps = parse(
            "# a comment\n\
             {\"type\":\"system\"}\n\
             {\"mock\":\"await-input\"}\n\
             {\"type\":\"result\"}\n",
        )
        .unwrap();
        assert_eq!(steps.len(), 3);

        let mut input = std::io::Cursor::new("{\"type\":\"user\"}\n{\"type\":\"user\"}\n");
        let mut output = Vec::new();
        let mut seen = Vec::new();
        play(&steps, &mut input, &mut output, |line| {
            seen.push(line.to_string());
            Ok(())
        })
        .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"type\":\"system\"}\n{\"type\":\"result\"}\n{\"type\":\"result\"}\n",
            "the preamble is emitted once; the turn replays per input"
        );
        assert_eq!(seen.len(), 2, "every input line reached the assertion hook");
    }

    #[test]
    fn a_malformed_script_fails_at_startup_naming_the_line() {
        // Failing halfway through a turn would look like a provider quirk. A
        // broken fixture must look like a broken fixture.
        let error = parse("{\"ok\":1}\nnot json\n").expect_err("the second line is not JSON");
        assert!(error.contains("line 2"), "{error}");

        let error = parse("{\"mock\":\"teleport\"}\n").expect_err("unknown directive");
        assert!(error.contains("teleport"), "{error}");
    }
}
