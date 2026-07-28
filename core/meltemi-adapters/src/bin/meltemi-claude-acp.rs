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
//! **One binary, two roles.** Run by the daemon, it is the adapter. Run by the
//! piloted CLI as one of its MCP servers — which is how that CLI asks a third
//! party for permission — it is the shim of design D5, and it does nothing but
//! carry questions to the adapter that launched the CLI. Shipping one binary is
//! the point: the shim can never be a different version from the adapter it
//! answers to, and the installers carry one file, not two.

use std::path::Path;

use meltemi_adapters::adapter::run;
use meltemi_adapters::claude::{ClaudeDialect, shim};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(channel) = shim::requested(&args) {
        // The permission shim needs no runtime: one CLI asks about one tool
        // call at a time and is blocked until it hears back.
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        shim::serve(Path::new(&channel), &mut stdin.lock(), &mut stdout.lock())?;
        return Ok(());
    }
    adapter()?;
    Ok(())
}

#[tokio::main]
async fn adapter() -> agent_client_protocol::Result<()> {
    run(ClaudeDialect::new()).await
}
