// SPDX-License-Identifier: Apache-2.0

//! `mock-claude-wire`: the scripted headless-session wire
//! (adaptadores-propios-acp task 1.3).
//!
//! It stands in for the provider CLI running in its documented headless mode:
//! it emits newline-delimited JSON events — the initial event with its
//! capability array, token-level deltas, a tool call and its result, and the
//! final result — and it accepts the same dialect on its input. It is not a
//! model, it holds no state and it touches no network: it plays a script and
//! checks that what the adapter sends back is the dialect it claims to speak.
//!
//! Scripts, in order of precedence: `--script <path>`, the
//! `MELTEMI_MOCK_CLAUDE_SCRIPT` environment variable, else the embedded
//! default (`--api-key-mode` selects the embedded variant that announces the
//! surface demanding an API key, so the adapter's refusal can be exercised).
//!
//! It also answers `--version` the way the real CLI does — one line, its own
//! version — because that flag is the only place this wire carries a version at
//! all: the session's events announce a feature array, not a number, so the
//! version a session logs is the one the binary answers there.
//!
//! Input is held to the dialect: a line that is not JSON, or a message of a
//! type this wire never receives, exits non-zero with a diagnostic. A fixture
//! that quietly tolerated a malformed turn would let a broken adapter pass.

use std::io::Write;

use mock_provider::{claude_permissions, script};
use serde_json::Value;

/// The signed-in session wire.
const SIGNED_IN: &str = include_str!("../../scripts/claude-signed-in.ndjson");
/// The wire that announces the API-key surface instead.
const API_KEY: &str = include_str!("../../scripts/claude-api-key.ndjson");

/// What this wire answers when asked what it is. Deliberately marked as a mock:
/// a session log that recorded it must read as a fixture, never as a real CLI.
const VERSION: &str = "2.0.0-mock (mock-claude-wire)";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("{VERSION}");
        return;
    }
    let embedded = if args.iter().any(|a| a == "--api-key-mode") {
        API_KEY
    } else {
        SIGNED_IN
    };

    let steps = match script::load(&args, "MELTEMI_MOCK_CLAUDE_SCRIPT", embedded) {
        Ok(steps) => steps,
        Err(error) => fail(&error),
    };

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    let handed = claude_permissions::Handed::from_args(&args);
    if let Err(error) = script::play(&steps, &mut input, &mut output, check_input, |directive| {
        ask(&handed, directive)
    }) {
        fail(&error);
    }
}

/// Runs one permission directive the way the CLI would run that channel, and
/// answers with the events the CLI would emit for the decision.
///
/// Two directives, one per channel, because the provider's documented order
/// means only one of them decides any given call: the hook first and finally,
/// the prompt tool only for what the hook did not decide. A test picks the
/// channel it means to exercise.
fn ask(handed: &claude_permissions::Handed, directive: &Value) -> Result<Vec<String>, String> {
    let which = directive
        .get("mock")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tool = directive
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or("Write");
    let tool_use_id = directive
        .get("toolUseId")
        .and_then(Value::as_str)
        .unwrap_or("toolu_mock_1");
    let input = directive
        .get("input")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let decision = match which {
        "ask-hook" => handed.ask_hook(tool, tool_use_id, &input),
        "ask-prompt-tool" => handed.ask_prompt_tool(tool, tool_use_id, &input),
        // A wire that will not end its turn and will not take the hint: the
        // shape a cancellation has to survive. It ignores the end of its input
        // too, which is the only stop this dialect documents.
        "never-stop" => loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        },
        other => return Err(format!("unknown directive `{other}`: {directive}")),
    };
    Ok(claude_permissions::tool_events(
        tool,
        tool_use_id,
        &input,
        &decision,
    ))
}

/// Holds the adapter to the input dialect: every line must be JSON, and every
/// message that names its type must be one this wire receives.
fn check_input(line: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| format!("input is not JSON ({error}): {line}"))?;
    match value.get("type").and_then(serde_json::Value::as_str) {
        // The user message is the only input shape of this dialect.
        Some("user") | None => Ok(()),
        Some(other) => Err(format!(
            "input message of type `{other}` is not sent on this wire: {line}"
        )),
    }
}

/// Ends with a diagnostic on stderr: a fixture failure must be loud.
fn fail(error: &str) -> ! {
    let _ = writeln!(std::io::stderr(), "mock-claude-wire: {error}");
    std::process::exit(2)
}
