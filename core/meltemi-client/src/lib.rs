// SPDX-License-Identifier: Apache-2.0

//! Shared client-side stack for the Meltemi daemon.
//!
//! Extracted from `meltemid` (gui-tauri-paridad design D1) so that every
//! surface — the terminal client, the desktop client and the daemon's own
//! server side — speaks the local socket through one implementation:
//!
//! - [`transport`]: the local endpoint (named pipe on Windows, UDS elsewhere),
//!   client [`transport::connect`] and daemon-side [`transport::Listener`]
//!   over the shared [`transport::Stream`] type.
//! - [`rpc`]: the line-delimited JSON-RPC 2.0 [`rpc::Peer`] with typed
//!   [`rpc::RpcError`] carrying the contract's `ErrorData` remedy.
//! - [`bootstrap`]: connect-or-start — fast-path connect with detached
//!   daemon spawn on demand.
//! - [`paths`]: platform path resolution (endpoint, data/config dirs,
//!   project key).
//!
//! The daemon never opens a network port; neither does this crate.

pub mod bootstrap;
pub mod paths;
pub mod rpc;
pub mod transport;
