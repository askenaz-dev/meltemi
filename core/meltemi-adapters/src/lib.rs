// SPDX-License-Identifier: Apache-2.0

//! Meltemi's own ACP adapters (adaptadores-propios-acp design D2).
//!
//! Two binaries ship from this crate — `meltemi-claude-acp` and
//! `meltemi-codex-acp` — one per provider dialect. They share this library
//! because more than half of an adapter is dialect-independent: the ACP
//! surface meltemid drives, the supervision of the provider subprocess, and
//! the newline-delimited JSON framing both provider wires use.
//!
//! The invariants below hold for every adapter built here. They are not
//! aspirations: each is kept honest by a test or by the dependency audit.
//!
//! - The adapter speaks ACP over stdio toward meltemid and pilots the
//!   **official CLI of the provider** as a subprocess, with the authentication
//!   that CLI manages on its own (constitution §2). It never embeds the
//!   provider's runtime as a library.
//! - It links **no HTTP client and no TLS stack**: all the network lives
//!   inside the official binary. `deny.toml` bans those crates so the property
//!   is verifiable, not merely claimed.
//! - It never reads, stores or forwards authentication material, and it
//!   listens on no port.
//! - When the layer it needs is missing or announces the wrong surface, it
//!   **refuses with a diagnostic and a remedy** instead of degrading to some
//!   other way in (see [`diagnostic`]).

/// How the daemon names the per-session model to an adapter.
///
/// The adapter is a separate process, so the lever travels the way its other
/// configuration already does: through the environment the daemon composes for
/// it (the same overlay that carries which binary to launch). No new transport,
/// no port, and nothing here is ever a credential (§2).
///
/// The value is the PROVIDER's own string, verbatim. The daemon does not read
/// it and neither does this crate — only the adapter that knows its CLI turns
/// it into a flag or a field (modelo-y-esfuerzo-por-sesion design D1).
pub const SESSION_MODEL_ENV: &str = "MELTEMI_SESSION_MODEL";

/// How the daemon names the per-session effort level, on the same terms.
pub const SESSION_EFFORT_ENV: &str = "MELTEMI_SESSION_EFFORT";

/// The model the daemon named for this session, if any.
#[must_use]
pub fn session_model() -> Option<String> {
    non_empty(std::env::var(SESSION_MODEL_ENV).ok())
}

/// The effort the daemon named for this session, if any.
#[must_use]
pub fn session_effort() -> Option<String> {
    non_empty(std::env::var(SESSION_EFFORT_ENV).ok())
}

/// An empty variable is an absent one: a blank string would reach the provider
/// as though somebody had chosen it.
fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub mod adapter;
pub mod bridge;
pub mod claude;
pub mod codex;
pub mod diagnostic;
pub mod jsonrpc;
pub mod ndjson;
pub mod supervisor;
