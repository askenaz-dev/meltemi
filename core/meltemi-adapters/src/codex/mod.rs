// SPDX-License-Identifier: Apache-2.0

//! The JSON-RPC server dialect (adaptadores-propios-acp design D6).
//!
//! One provider ships a documented server mode — JSON-RPC 2.0 with newline
//! delimitation over stdio, the same interface its own editor extension uses —
//! and this module is the translation between that surface and ACP. The
//! official binary is launched as a subprocess and nothing else: the pattern
//! the archived and community Rust adapters take, embedding the provider's
//! runtime as a library, would put the network and the provider's auth store
//! inside this process, which constitution §2 forbids however permissive the
//! licence is.
//!
//! What lives here is the wire itself. What the adapter *does* with it — the
//! ACP session mapping, the permission relay — arrives with the tasks that own
//! those requirements.

pub mod wire;
