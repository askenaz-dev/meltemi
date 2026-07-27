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

pub mod adapter;
pub mod bridge;
pub mod claude;
pub mod codex;
pub mod diagnostic;
pub mod jsonrpc;
pub mod ndjson;
pub mod supervisor;
