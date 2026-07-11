// SPDX-License-Identifier: Apache-2.0

//! Meltemi headless daemon library.
//!
//! `meltemid` exposes its internals as a library so that development tooling
//! (`meltemi-devclient`) and the e2e suite can reuse the local transport and
//! the client-side bootstrap helpers. The binary in `main.rs` is a thin
//! wrapper over [`run`].

pub mod paths;
pub mod transport;
