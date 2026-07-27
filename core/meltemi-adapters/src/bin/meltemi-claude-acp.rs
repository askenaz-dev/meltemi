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

use meltemi_adapters::adapter::{AdapterSpec, Dialect, run};

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    run(AdapterSpec {
        name: "meltemi-claude-acp",
        provider_layer: "the official `claude` CLI",
        provider_bin: "claude",
        dialect: Dialect::HeadlessSession,
    })
    .await
}
