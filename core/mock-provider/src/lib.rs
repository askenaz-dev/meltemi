// SPDX-License-Identifier: Apache-2.0

//! Scripted provider wires (adaptadores-propios-acp design D10).
//!
//! CI never runs a real agent and never touches the network, so what gets
//! simulated is **the provider's side of the wire**, not the adapter: the
//! adapters under test are the real binaries, and what they pilot is one of the
//! scripted processes this crate ships. That is the same bargain `mock-agent`
//! struck for the ACP side, one layer down.
//!
//! A wire fixture is a script: a file of newline-delimited JSON lines the
//! process emits in order, with one directive — wait for input — to mark the
//! turn boundaries. Each binary embeds its default script and accepts another
//! by argument or environment variable, so a test can freeze the exact wire it
//! needs without a new binary and without new code.
//!
//! What these fixtures are **not**: a specification of the providers' wires.
//! They freeze the contract as observed for a supported version, which is the
//! honest thing a fixture can do; the run against the real CLIs stays manual
//! and opt-in (task 5.2), and it is that run — not this crate — that says the
//! observed contract still holds.

pub mod script;
