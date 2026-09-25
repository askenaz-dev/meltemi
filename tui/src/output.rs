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
use crate::format::Format;

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

    /// The daemon answered with a contract error, said in the CLI's own words
    /// when it is a lever refused as its session opened: the lever, what was
    /// asked, why, and what the agent announced or left — bounded, and with the
    /// agent's text escaped before it reaches the terminal
    /// (modelo-por-opcion-anunciada D4). Anything else is
    /// [`CliError::contract`] unchanged, and so is the exit code either way.
    pub fn from_rpc(error: meltemi_client::rpc::RpcError) -> Self {
        let data = error.data.as_ref();
        let Some(refusal) = data
            .and_then(|data| data.get("leverRefused"))
            .and_then(|value| {
                serde_json::from_value::<meltemi_proto::LeverRefusal>(value.clone()).ok()
            })
        else {
            return Self::contract(error);
        };
        let lang = crate::shell::messages::Lang::En;
        let mut message = format!(
            "contract error: [{}] {}",
            error.code,
            crate::lever::refusal_line(&refusal, lang)
        );
        if let Some(line) = crate::lever::announced_line(&refusal, lang) {
            message.push_str(&format!("\n  {line}"));
        }
        if let Some(remedy) = data
            .and_then(|data| data.get("remedy"))
            .and_then(Value::as_str)
        {
            message.push_str(&format!("\n  remedy: {}", crate::approver::clean(remedy)));
        }
        Self::new(ExitCode::Contract, "contract_error", message)
    }

    /// Refused by the permission proxy's policy (exit 12): a decision that
    /// reached a request no longer waiting, where the first decision wins
    /// (verbos-del-agent-boss design D4).
    pub fn denied(kind: &'static str, message: impl Into<String>) -> Self {
        Self::new(ExitCode::Denied, kind, message)
    }

    /// Something the daemon's own answer says does not exist (exit 11): the
    /// same fact the daemon reports as a contract error when a verb names it
    /// directly — an unknown session is `session_not_found` for `direct` too
    /// (verbos-del-agent-boss design D4).
    pub fn not_found(kind: &'static str, message: impl Into<String>) -> Self {
        Self::new(ExitCode::Contract, kind, message)
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
pub fn render_outcome(
    outcome: &Outcome,
    format: Format,
    out: &mut impl Write,
) -> std::io::Result<()> {
    if format.is_machine() {
        render_value(&outcome.json, format, out)
    } else {
        writeln!(out, "{}", outcome.human)
    }
}

/// Writes one machine-readable document. The single place that knows how each
/// machine format serializes, so a new one is one arm and not a search.
pub fn render_value(value: &Value, format: Format, out: &mut impl Write) -> std::io::Result<()> {
    match format {
        Format::Json => writeln!(
            out,
            "{}",
            serde_json::to_string(value).expect("JSON value serializes")
        ),
        Format::Yaml => write!(out, "{}", crate::format::to_yaml(value)),
        // A machine document was asked for; the human path never arrives here.
        Format::Human => Ok(()),
    }
}

/// Writes an error: under `json`, exactly one JSON error object on stdout
/// (carrying the exit code); otherwise a human message on stderr, leaving
/// stdout untouched.
pub fn render_error(
    error: &CliError,
    format: Format,
    out: &mut impl Write,
    err: &mut impl Write,
) -> std::io::Result<()> {
    if format.is_machine() {
        let object = json!({
            "error": {
                "code": error.exit.code(),
                "kind": error.kind,
                "message": error.message,
            }
        });
        render_value(&object, format, out)
    } else {
        writeln!(err, "error: {}", error.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: El error en YAML también es un documento
    #[test]
    fn an_error_in_yaml_is_one_document_on_stdout_and_nothing_on_stderr() {
        let error = CliError::unreachable("the daemon is not listening");
        let (mut out, mut err) = strings();
        render_error(&error, Format::Yaml, &mut out, &mut err).unwrap();

        let stdout = String::from_utf8(out).expect("utf8");
        let stderr = String::from_utf8(err).expect("utf8");

        // The document carries the exit code from the taxonomy, so a script
        // reads the same thing it would read from `--json`.
        assert!(
            stdout.contains("\"code\": 10"),
            "the exit code travels: {stdout}"
        );
        assert!(stdout.contains("\"kind\": \"daemon_unreachable\""));
        assert!(stdout.contains("\"message\":"));
        // One document: no human prose mixed in, and stderr left alone.
        assert!(
            !stdout.contains("error:"),
            "no human text on stdout: {stdout}"
        );
        assert!(stderr.is_empty(), "stderr stays free of it: {stderr:?}");
        // And never painted, whatever the terminal.
        assert!(
            !stdout.contains('\u{1b}'),
            "a machine document is not decorated"
        );
    }

    fn strings() -> (Vec<u8>, Vec<u8>) {
        (Vec::new(), Vec::new())
    }

    // Scenario: El rehúso se imprime acotado y escapado
    #[test]
    fn a_refused_lever_prints_bounded_and_escaped_and_json_stays_one_object() {
        // More values than a terminal should carry, one of them with a control
        // sequence that would clear the screen, and what the agent left with a
        // bell in it: the agent's text, which is never obeyed.
        let mut values: Vec<String> = (0..25).map(|n| format!("model-{n}")).collect();
        values[3] = "evil\u{1b}[2J".into();
        let error = meltemi_client::rpc::RpcError {
            code: 2007,
            message: "the agent did not take the lever".into(),
            data: Some(json!({
                "kind": "lever_refused",
                "detail": "raw detail \u{1b}[31m",
                "remedy": "Ask for one of the values the agent announced for its model.",
                "leverRefused": {
                    "lever": "model",
                    "requested": "gpt-x",
                    "reason": "not_confirmed",
                    "values": values,
                    "more": 3,
                    "left": "fast\u{7}",
                },
            })),
        };
        let cli = CliError::from_rpc(error);
        assert_eq!(cli.exit, ExitCode::Contract, "still a contract error");
        let message = &cli.message;
        assert!(message.contains("the model `gpt-x`"), "{message}");
        assert!(message.contains("kept `fast\\u{0007}`"), "{message}");
        assert!(
            message.contains("model-19"),
            "the twentieth is shown: {message}"
        );
        assert!(
            !message.contains("model-20"),
            "the twenty-first is not: {message}"
        );
        assert!(
            message.contains("and 8 more"),
            "the rest are counted: {message}"
        );
        assert!(message.contains("evil\\u{001b}[2J"), "escaped: {message}");
        assert!(
            !message.contains('\u{1b}') && !message.contains('\u{7}'),
            "no control character reaches the terminal: {message:?}"
        );
        assert!(
            !message.contains("raw detail"),
            "the daemon's prose is not printed in place of the escaped words"
        );
        assert!(message.contains("remedy: Ask for one of the values"));

        // Under --json, stdout still carries exactly one object.
        let (mut out, mut err) = strings();
        render_error(&cli, Format::Json, &mut out, &mut err).unwrap();
        let printed = String::from_utf8(out).unwrap();
        assert_eq!(printed.trim().lines().count(), 1, "{printed}");
        let parsed: Value = serde_json::from_str(printed.trim()).expect("one JSON object");
        assert_eq!(parsed["error"]["code"], 11);
        assert!(err.is_empty());

        // Any other daemon error reads exactly as before.
        let other = meltemi_client::rpc::RpcError {
            code: 2004,
            message: "session not found".into(),
            data: None,
        };
        assert_eq!(
            CliError::from_rpc(other.clone()).message,
            CliError::contract(other).message
        );
    }

    #[test]
    fn human_outcome_goes_to_stdout() {
        let (mut out, _err) = strings();
        let outcome = Outcome {
            human: "daemon ok".into(),
            json: json!({"ok": true}),
        };
        render_outcome(&outcome, Format::Human, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "daemon ok\n");
    }

    #[test]
    fn json_outcome_is_one_object_on_stdout() {
        // Scenario: Éxito en JSON emite un objeto
        let (mut out, _err) = strings();
        let outcome = Outcome {
            human: "ignored".into(),
            json: json!({"daemonVersion": "0.1.0"}),
        };
        render_outcome(&outcome, Format::Json, &mut out).unwrap();
        let printed = String::from_utf8(out).unwrap();
        let parsed: Value = serde_json::from_str(printed.trim()).expect("one JSON object");
        assert_eq!(parsed["daemonVersion"], "0.1.0");
    }

    #[test]
    fn human_error_goes_to_stderr_only() {
        // Scenario: Error en modo humano va a stderr.
        let (mut out, mut err) = strings();
        let error = CliError::unreachable("no daemon");
        render_error(&error, Format::Human, &mut out, &mut err).unwrap();
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
        render_error(&error, Format::Json, &mut out, &mut err).unwrap();
        assert!(err.is_empty(), "stderr must stay free of JSON");
        let printed = String::from_utf8(out).unwrap();
        let parsed: Value = serde_json::from_str(printed.trim()).expect("one JSON object");
        assert_eq!(parsed["error"]["code"], ExitCode::Contract.code());
        assert_eq!(parsed["error"]["kind"], "contract_error");
    }
}
