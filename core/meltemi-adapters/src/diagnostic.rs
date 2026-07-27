// SPDX-License-Identifier: Apache-2.0

//! Refusals: how an adapter says no without degrading.
//!
//! An adapter that cannot do the governed thing must never do an ungoverned
//! one — no fallback wire, no injected credential, no silent downgrade. It
//! refuses, and the refusal carries what a human needs: the layer it is about,
//! what happened, and what to do. The daemon surfaces it like any agent error.
//!
//! The `data` payload mirrors the daemon's own error shape (`kind`, `detail`,
//! `remedy`) so a refusal reads the same in the CLI, the TUI and the desktop
//! surface. The adapter does not depend on the daemon's contract crate to say
//! so: it is piloted as any third-party ACP agent is, and shares only ACP.

use agent_client_protocol::Error;

/// The JSON-RPC code every adapter refusal carries.
///
/// It sits in the range ACP reserves for protocol-specific errors
/// (-32000..-32099) and is not one of the two the protocol assigns
/// (-32000 auth required, -32002 resource not found). The meaning never
/// travels in the number: it travels in `data.kind`.
pub const REFUSAL_CODE: i32 = -32010;

/// A refusal naming the layer it is about, in the terms the fleet catalog
/// uses, plus the remedy the human can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Machine-readable kind, stable across surfaces.
    pub kind: &'static str,
    /// The layer the refusal is about (the official CLI, the adapter itself).
    pub layer: String,
    /// What happened, in one sentence.
    pub detail: String,
    /// What the human can do about it.
    pub remedy: String,
}

impl Refusal {
    /// A refusal about a layer, with its detail and remedy.
    #[must_use]
    pub fn new(
        kind: &'static str,
        layer: impl Into<String>,
        detail: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            layer: layer.into(),
            detail: detail.into(),
            remedy: remedy.into(),
        }
    }

    /// The single-sentence message, always naming the layer first: the reader
    /// must know *which* piece is missing before knowing why.
    #[must_use]
    pub fn message(&self) -> String {
        format!("{}: {}", self.layer, self.detail)
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.message(), self.remedy)
    }
}

impl From<Refusal> for Error {
    fn from(refusal: Refusal) -> Self {
        let data = serde_json::json!({
            "kind": refusal.kind,
            "layer": refusal.layer,
            "detail": refusal.detail,
            "remedy": refusal.remedy,
        });
        Error::new(REFUSAL_CODE, refusal.message()).data(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_carries_its_layer_and_remedy_into_the_acp_error() {
        // The daemon renders `data`, not our prose: the layer, the detail and
        // the remedy must all survive the crossing, or the human is left with
        // "it failed" — the exact opposite of a diagnosed refusal.
        let refusal = Refusal::new(
            "provider_cli_not_launchable",
            "the official `claude` CLI",
            "the binary could not be launched",
            "Install the provider CLI and sign in.",
        );
        let error: Error = refusal.into();
        assert_eq!(i32::from(error.code), REFUSAL_CODE);
        assert!(error.message.contains("the official `claude` CLI"));
        let data = error.data.expect("a refusal always carries data");
        assert_eq!(data["kind"], "provider_cli_not_launchable");
        assert_eq!(data["remedy"], "Install the provider CLI and sign in.");
    }
}
