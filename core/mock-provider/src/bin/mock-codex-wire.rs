// SPDX-License-Identifier: Apache-2.0

//! `mock-codex-wire`: the scripted JSON-RPC server wire
//! (adaptadores-propios-acp task 1.4, re-anchored in task 2.1).
//!
//! It stands where the provider CLI's documented server mode would stand: a
//! JSON-RPC 2.0 conversation with newline delimitation over stdio — handshake
//! with a user agent, a thread, a turn streamed as items and chunks, and one
//! approval the server asks for and waits on. It holds no state, runs no model
//! and touches no network.
//!
//! It also mirrors the CLI's schema generator, `generate-json-schema --out
//! <dir>`, and what it writes is the **official dump vendored verbatim** under
//! `schemas/codex-app-server/` — not a hand-written imitation of it. That is
//! what lets the adapter's conformance test run in CI with no provider binary
//! anywhere while still validating against the real contract.
//!
//! Invoked the way the adapter invokes the real CLI — a leading `app-server`
//! argument is accepted and ignored, so the fixture can be dropped in as the
//! provider binary without special-casing anything.

use std::io::Write;
use std::path::Path;

use mock_provider::{jsonrpc_wire, script};

/// The scripted conversation.
const TRANSCRIPT: &str = include_str!("../../scripts/codex-app-server.ndjson");

/// The vendored official dump, file by file, exactly as the CLI writes it.
macro_rules! vendored_schemas {
    ($($file:literal),* $(,)?) => {
        &[$(($file, include_str!(concat!("../../schemas/codex-app-server/", $file)))),*]
    };
}

/// The schema files this fixture reproduces: the subset of the official dump
/// that the adapter's wire touches (see `schemas/codex-app-server/PROVENANCE.md`).
const SCHEMAS: &[(&str, &str)] = vendored_schemas![
    "AgentMessageDeltaNotification.json",
    "CommandExecutionRequestApprovalParams.json",
    "CommandExecutionRequestApprovalResponse.json",
    "ErrorNotification.json",
    "FileChangeRequestApprovalParams.json",
    "FileChangeRequestApprovalResponse.json",
    "InitializeParams.json",
    "InitializeResponse.json",
    "ItemCompletedNotification.json",
    "ItemStartedNotification.json",
    "JSONRPCMessage.json",
    "ReasoningSummaryTextDeltaNotification.json",
    "ReasoningTextDeltaNotification.json",
    "ThreadStartParams.json",
    "ThreadStartResponse.json",
    "TurnCompletedNotification.json",
    "TurnInterruptParams.json",
    "TurnInterruptResponse.json",
    "TurnStartParams.json",
    "TurnStartResponse.json",
    "TurnStartedNotification.json",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // The CLI this stands in for dumps its schema per version into a directory;
    // so does this, with the same required `--out`.
    if args.iter().any(|a| a == "generate-json-schema") {
        let Some(out) = flag_value(&args, "--out") else {
            fail("generate-json-schema needs `--out <dir>`, as the official CLI does");
        };
        dump_schemas(Path::new(&out));
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

/// The value that follows a flag, if it was given.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

/// Writes the vendored dump into `out`, verbatim.
fn dump_schemas(out: &Path) {
    if let Err(error) = std::fs::create_dir_all(out) {
        fail(&format!("cannot create {}: {error}", out.display()));
    }
    for (name, contents) in SCHEMAS {
        if let Err(error) = std::fs::write(out.join(name), contents) {
            fail(&format!("cannot write {name}: {error}"));
        }
    }
}

/// Ends with a diagnostic on stderr: a fixture failure must be loud.
fn fail(error: &str) -> ! {
    let _ = writeln!(std::io::stderr(), "mock-codex-wire: {error}");
    std::process::exit(2)
}
