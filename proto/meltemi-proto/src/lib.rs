// SPDX-License-Identifier: Apache-2.0

//! Serde types for the Meltemi daemon<->client protocol.
//!
//! The JSON Schemas under `proto/schemas/` are the language-neutral source
//! of truth for this contract; the types in this crate must serialize in
//! conformance with them (validated by the conformance test suite).
