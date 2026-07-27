// SPDX-License-Identifier: Apache-2.0

//! `mock-codex-wire`: the scripted JSON-RPC server wire
//! (adaptadores-propios-acp task 1.4).
//!
//! It stands where the provider CLI's documented server mode would stand: a
//! JSON-RPC 2.0 conversation with newline delimitation over stdio — handshake
//! with a version, a thread, a turn streamed as items, and one approval the
//! server asks for and waits on. It holds no state, runs no model and touches
//! no network.
//!
//! It also mirrors the CLI's own schema generator: `generate-json-schema`
//! prints the schema of the wire it speaks, which is what lets a conformance
//! test run in CI with no provider binary anywhere. The authoritative
//! per-version dump from the official CLI is a separate fixture (task 2.1);
//! where the two disagree, the official one wins.
//!
//! Invoked the way the adapter invokes the real CLI — a leading `app-server`
//! argument is accepted and ignored, so the fixture can be dropped in as the
//! provider binary without special-casing anything.

use std::io::Write;

use mock_provider::{jsonrpc_wire, script};

/// The scripted conversation.
const TRANSCRIPT: &str = include_str!("../../scripts/codex-app-server.ndjson");
/// The schema of the wire this fixture speaks.
const SCHEMA: &str = include_str!("../../schemas/codex-app-server.schema.json");

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // The CLI this stands in for dumps its schema per version; so does this.
    if args.iter().any(|a| a == "generate-json-schema") {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        if write!(out, "{SCHEMA}").and_then(|()| out.flush()).is_err() {
            fail("the schema could not be written");
        }
        return;
    }

    let source = match script::source(&args, "MELTEMI_MOCK_CODEX_SCRIPT", TRANSCRIPT) {
        Ok(source) => source,
        Err(error) => fail(&error),
    };
    let steps = match jsonrpc_wire::parse(&source) {
        Ok(steps) => steps,
        Err(error) => fail(&error),
    };

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    if let Err(error) = jsonrpc_wire::play(&steps, &mut input, &mut output) {
        fail(&error);
    }
}

/// Ends with a diagnostic on stderr: a fixture failure must be loud.
fn fail(error: &str) -> ! {
    let _ = writeln!(std::io::stderr(), "mock-codex-wire: {error}");
    std::process::exit(2)
}
