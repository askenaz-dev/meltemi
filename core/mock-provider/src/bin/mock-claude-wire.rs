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
//! Input is held to the dialect: a line that is not JSON, or a message of a
//! type this wire never receives, exits non-zero with a diagnostic. A fixture
//! that quietly tolerated a malformed turn would let a broken adapter pass.

use std::io::Write;

use mock_provider::script;

/// The signed-in session wire.
const SIGNED_IN: &str = include_str!("../../scripts/claude-signed-in.ndjson");
/// The wire that announces the API-key surface instead.
const API_KEY: &str = include_str!("../../scripts/claude-api-key.ndjson");

fn main() {
    let args: Vec<String> = std::env::args().collect();
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

    if let Err(error) = script::play(&steps, &mut input, &mut output, check_input) {
        fail(&error);
    }
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
