// SPDX-License-Identifier: Apache-2.0

//! `meltemi-claude-acp`: Meltemi's own ACP adapter for the provider whose
//! official CLI exposes a **headless session of JSON events** over stdio
//! (adaptadores-propios-acp design D4).
//!
//! It pilots the official binary with the session the user already signed into
//! — never the provider's agent SDK, never a mode that demands an API key.
//! All the network and all the authentication live inside that official
//! binary, which is exactly where constitution §2 requires them.
//!
//! The binary name is deliberately Meltemi's own, distinct from the
//! third-party adapters' (`claude-agent-acp`): two different bridges must never
//! collide on the PATH, and detection must never be ambiguous about which one
//! it found (design D2).
//!
//! **One binary, three roles.** Run by the daemon, it is the adapter. Run by
//! the piloted CLI as one of its MCP servers, it is the permission shim of
//! design D5. Run by that same CLI as a hook, before every tool call, it is the
//! hard gate. The two smaller roles decide nothing: they carry questions to the
//! adapter that launched the CLI and deny when nobody answers. Shipping one
//! binary is the point — a shim can never be a different version from the
//! adapter it answers to, and the installers carry one file, not three.

use std::path::Path;

use meltemi_adapters::adapter::run;
use meltemi_adapters::claude::{ClaudeDialect, gate, shim};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();

    // Neither of the small roles needs a runtime: each is one question, asked
    // by a CLI that is blocked until it hears back.
    if let Some(channel) = shim::requested(&args) {
        shim::serve(Path::new(&channel), &mut stdin.lock(), &mut stdout.lock())?;
        return Ok(());
    }
    if let Some(channel) = gate::requested(&args) {
        gate::decide(Path::new(&channel), &mut stdin.lock(), &mut stdout.lock())?;
        return Ok(());
    }
    adapter()?;
    Ok(())
}

#[tokio::main]
async fn adapter() -> agent_client_protocol::Result<()> {
    run(ClaudeDialect::new()).await
}
