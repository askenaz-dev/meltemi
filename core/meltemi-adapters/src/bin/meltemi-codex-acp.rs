// SPDX-License-Identifier: Apache-2.0

//! `meltemi-codex-acp`: Meltemi's own ACP adapter for the provider whose
//! official CLI exposes a **JSON-RPC 2.0 server** with newline delimitation
//! over stdio (adaptadores-propios-acp design D6).
//!
//! It launches the official CLI in that server mode — the same interface the
//! provider's own editor extension uses, published for third parties — and
//! translates its conversation primitives to the ACP session. It explicitly
//! does *not* embed the provider's runtime as a library, the pattern both
//! existing Rust adapters follow: that would make this binary do the network
//! and read the provider's auth store, which §2 forbids however permissive the
//! license is.
//!
//! The binary name is deliberately Meltemi's own, distinct from the
//! third-party adapter's (`codex-acp`): two different bridges must never
//! collide on the PATH (design D2).

use meltemi_adapters::adapter::{AdapterSpec, Dialect, run};

#[tokio::main]
async fn main() -> agent_client_protocol::Result<()> {
    run(AdapterSpec {
        name: "meltemi-codex-acp",
        provider_layer: "the official `codex` CLI",
        provider_bin: "codex",
        dialect: Dialect::JsonRpcServer,
    })
    .await
}
