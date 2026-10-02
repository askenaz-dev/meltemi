// SPDX-License-Identifier: Apache-2.0

//! The Meltemi mobile companion: the Agent Boss's remote post.
//!
//! Four verbs and no more — monitor, approve, review, direct — over the user's
//! own SSH, into the same local socket every other client reaches. The spec
//! `mobile-companion` governs the boundary and this crate does not widen it:
//! there is no authoring of code or specs here, and no path to any of it.
//!
//! **What is different from the desktop, and it is one thing.** A phone has no
//! system `ssh` to launch, so the `exec:` endpoint — which is the whole of
//! remote access everywhere else — has nothing to run. This crate speaks SSH
//! itself and hands the client stack a stream that, on the far side, is
//! `meltemi bridge`. Everything above that stream is the code the desktop
//! already uses: the client's JSON-RPC peer takes any
//! `AsyncRead + AsyncWrite + Unpin + Send`, so not a line of `meltemi-client`
//! changes to serve a phone (design D2).
//!
//! The SSH client is confined here. On the desktop the `ssh` is the user's own,
//! and that does not change.

pub mod keys;
pub mod ssh;
