// SPDX-License-Identifier: Apache-2.0

//! Output discipline (design D4).
//!
//! stdout carries only the command's useful output — human text, or under
//! `--json` exactly one JSON object; stderr carries diagnostics. Rendering is
//! parameterized over the output/error sinks so the discipline is testable
//! against in-memory buffers.

use std::io::Write;

use serde_json::{Value, json};

use crate::exit::ExitCode;

/// A successful command result: a human rendering and a machine value.
#[derive(Debug)]
pub struct Outcome {
    pub human: String,
    pub json: Value,
}

/// A failed command: an exit category, a stable machine `kind` and a human
/// message.
#[derive(Debug)]
pub struct CliError {
    pub exit: ExitCode,
    pub kind: &'static str,
    pub message: String,
}

impl CliError {
    fn new(exit: ExitCode, kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit,
            kind,
            message: message.into(),
        }
    }

    /// The daemon could not be reached or started (exit 10).
    pub fn unreachable(detail: impl std::fmt::Display) -> Self {
        Self::new(
            ExitCode::Unreachable,
            "daemon_unreachable",
            format!("daemon unreachable: {detail}"),
        )
    }

    /// The daemon answered with a contract error (exit 11).
    pub fn contract(detail: impl std::fmt::Display) -> Self {
        Self::new(
            ExitCode::Contract,
            "contract_error",
            format!("contract error: {detail}"),
        )
    }

    /// An unexpected internal error (exit 1).
    pub fn internal(detail: impl std::fmt::Display) -> Self {
        Self::new(
            ExitCode::Internal,
            "internal_error",
            format!("internal error: {detail}"),
        )
    }

    /// A usage error (exit 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(ExitCode::Usage, "usage", message)
    }
}

/// Writes a successful outcome: under `json`, exactly one JSON object on
/// stdout; otherwise the human rendering on stdout.
pub fn render_outcome(outcome: &Outcome, json: bool, out: &mut impl Write) -> std::io::Result<()> {
    if json {
        writeln!(
            out,
            "{}",
            serde_json::to_string(&outcome.json).expect("JSON value serializes")
        )
    } else {
        writeln!(out, "{}", outcome.human)
    }
}

/// Writes an error: under `json`, exactly one JSON error object on stdout
/// (carrying the exit code); otherwise a human message on stderr, leaving
/// stdout untouched.
pub fn render_error(
    error: &CliError,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> std::io::Result<()> {
    if json {
        let object = json!({
            "error": {
                "code": error.exit.code(),
                "kind": error.kind,
                "message": error.message,
            }
        });
        writeln!(
            out,
            "{}",
            serde_json::to_string(&object).expect("JSON value serializes")
        )
    } else {
        writeln!(err, "error: {}", error.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings() -> (Vec<u8>, Vec<u8>) {
        (Vec::new(), Vec::new())
    }

    #[test]
    fn human_outcome_goes_to_stdout() {
        let (mut out, _err) = strings();
        let outcome = Outcome {
            human: "daemon ok".into(),
            json: json!({"ok": true}),
        };
        render_outcome(&outcome, false, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "daemon ok\n");
    }

    #[test]
    fn json_outcome_is_one_object_on_stdout() {
        let (mut out, _err) = strings();
        let outcome = Outcome {
            human: "ignored".into(),
            json: json!({"daemonVersion": "0.1.0"}),
        };
        render_outcome(&outcome, true, &mut out).unwrap();
        let printed = String::from_utf8(out).unwrap();
        let parsed: Value = serde_json::from_str(printed.trim()).expect("one JSON object");
        assert_eq!(parsed["daemonVersion"], "0.1.0");
    }

    #[test]
    fn human_error_goes_to_stderr_only() {
        // Scenario: Error en modo humano va a stderr.
        let (mut out, mut err) = strings();
        let error = CliError::unreachable("no daemon");
        render_error(&error, false, &mut out, &mut err).unwrap();
        assert!(out.is_empty(), "stdout must stay clean on a human error");
        assert!(
            String::from_utf8(err)
                .unwrap()
                .contains("daemon unreachable")
        );
    }

    #[test]
    fn json_error_is_one_object_on_stdout_with_code() {
        // Scenario: Error en JSON emite un objeto de error.
        let (mut out, mut err) = strings();
        let error = CliError::contract("[3000] change_already_exists");
        render_error(&error, true, &mut out, &mut err).unwrap();
        assert!(err.is_empty(), "stderr must stay free of JSON");
        let printed = String::from_utf8(out).unwrap();
        let parsed: Value = serde_json::from_str(printed.trim()).expect("one JSON object");
        assert_eq!(parsed["error"]["code"], ExitCode::Contract.code());
        assert_eq!(parsed["error"]["kind"], "contract_error");
    }
}
