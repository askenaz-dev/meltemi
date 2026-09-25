// SPDX-License-Identifier: Apache-2.0

//! Meltemi terminal client (`cli-contrato`): the scriptable CLI surface.
//!
//! This crate builds the single `meltemi` binary (alias `mel`, shipped at
//! packaging time). In this change it is a scriptable CLI; the interactive TUI
//! (#6) replaces the deferred [`cli::Action::Interactive`] branch without
//! touching the dispatch rule. The library exposes the grammar, the output
//! discipline and the command↔RPC mapping so all of it is testable off the
//! process boundary.

pub mod approver;
pub mod cli;
pub mod exit;
pub mod format;
pub mod output;
pub mod run;
pub mod shell;
pub mod tunnel;

use std::io::Write;

use cli::{Action, Command, Plan};
use exit::ExitCode;
use output::{CliError, render_error, render_outcome, render_value};

/// What the CLI writes to stderr when a response declares an empty turn
/// (turno-vacio-es-error design D4): what happened, and a remedy that stays
/// conditional, because an agent may have had nothing to do.
pub const EMPTY_TURN_WARNING: &str = "warning: the agent ended its turn without saying or doing \
     anything — if that was not expected, try the agent on its own and check its provider and \
     credentials";

/// The exit code of a command that completed, from what it answered.
///
/// The rule follows the response's fields and never a list of verbs: any
/// answer that declares an empty turn exits `15`, whichever verb got it, and a
/// validation with findings exits `14`.
#[must_use]
pub fn exit_for(validates: bool, answer: &serde_json::Value) -> ExitCode {
    if answer.get("emptyTurn") == Some(&serde_json::Value::Bool(true)) {
        return ExitCode::EmptyTurn;
    }
    if validates && answer.get("clean") == Some(&serde_json::Value::Bool(false)) {
        return ExitCode::Validation;
    }
    ExitCode::Success
}

/// Concludes a command that completed: picks its exit code and, in human mode,
/// says on stderr what an empty turn means. Machine output carries the verdict
/// in the answer itself, so it gets no prose.
pub fn conclude(
    validates: bool,
    answer: &serde_json::Value,
    machine: bool,
    err: &mut impl Write,
) -> ExitCode {
    let exit = exit_for(validates, answer);
    if exit == ExitCode::EmptyTurn && !machine {
        let _ = writeln!(err, "{EMPTY_TURN_WARNING}");
    }
    exit
}

/// Runs a resolved [`Plan`] against the daemon at `endpoint`, writing to the
/// given stdout/stderr sinks, and returns the process exit code. Permission
/// requests are refused: nothing in the library waits for a key on its own.
pub async fn dispatch(
    plan: Plan,
    endpoint: &str,
    stdout_is_tty: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    dispatch_with(
        plan,
        endpoint,
        stdout_is_tty,
        out,
        err,
        approver::Approver::deny(),
    )
    .await
}

/// [`dispatch`], answering the permission requests a command's sessions raise
/// with `approver` (aprobador-en-el-cli). The binary passes the terminal's
/// when there is one to ask in.
pub async fn dispatch_with(
    plan: Plan,
    endpoint: &str,
    stdout_is_tty: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    approver: approver::Approver,
) -> i32 {
    let format = plan.format;
    // One decision, taken once: the format, the explicit flag, the environment
    // and whether stdout is a terminal (design D4).
    let painting = crate::format::paints(format, plan.no_color, stdout_is_tty);
    match plan.action {
        Action::Help => {
            let _ = writeln!(out, "{}", cli::USAGE);
            ExitCode::Success.code()
        }
        Action::Version => {
            if format.is_machine() {
                let object =
                    serde_json::json!({ "name": "meltemi", "version": env!("CARGO_PKG_VERSION") });
                let _ = render_value(&object, format, out);
            } else {
                let _ = writeln!(out, "meltemi {}", env!("CARGO_PKG_VERSION"));
            }
            ExitCode::Success.code()
        }
        Action::Interactive => {
            // Launch the shell: it draws immediately and connects to the daemon
            // asynchronously, reconnecting with backoff.
            match shell::run(endpoint).await {
                Ok(()) => ExitCode::Success.code(),
                Err(error) => {
                    let _ = writeln!(err, "meltemi: {error}");
                    ExitCode::Internal.code()
                }
            }
        }
        Action::Usage(message) => {
            let error = CliError::usage(message);
            let _ = render_error(&error, format, out, err);
            error.exit.code()
        }
        Action::Run(Command::Reserved(name)) => {
            // Recognized by the grammar, not yet implemented: distinct from an
            // unknown subcommand (which is a usage error, exit 2).
            let _ = writeln!(
                err,
                "meltemi: `{name}` is a reserved subcommand, not yet implemented"
            );
            ExitCode::Success.code()
        }
        Action::Run(Command::Bridge) => {
            // stdout is a byte channel here, not a rendering, so nothing human
            // may be written to it — and the bytes go through the very guard
            // this process locked (see `run::bridge`).
            match run::bridge(endpoint, out).await {
                Ok(()) => ExitCode::Success.code(),
                Err(error) => {
                    let _ = writeln!(err, "meltemi: {}", error.message);
                    error.exit.code()
                }
            }
        }
        Action::Run(command) => {
            // A result is not always a success: `validate` findings exit `14`
            // (its `clean` field is the stable signal), and any answer that
            // declares an empty turn exits `15` (see [`conclude`]).
            let is_validate = matches!(command, Command::Validate { .. });
            let result = run::execute_with(command, endpoint, painting, approver.clone()).await;
            // No read is left in raw mode, whatever the command came to.
            approver.settle().await;
            let code = match result {
                Ok(outcome) => {
                    let _ = render_outcome(&outcome, format, out);
                    conclude(is_validate, &outcome.json, format.is_machine(), err).code()
                }
                Err(error) => {
                    let _ = render_error(&error, format, out, err);
                    error.exit.code()
                }
            };
            // The human stopped it at a question: whatever the daemon answered
            // is shown, and the command says it was cancelled (design D11).
            if approver.stopped() {
                ExitCode::Cancelled.code()
            } else {
                code
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_turn_exits_fifteen_and_a_worked_one_zero() {
        // Scenario: Turno vacío distinguible
        let empty = serde_json::json!({ "status": "completed", "emptyTurn": true });
        let worked = serde_json::json!({ "status": "completed" });
        assert_eq!(exit_for(false, &empty), ExitCode::EmptyTurn);
        assert_eq!(exit_for(false, &empty).code(), 15);
        assert_eq!(exit_for(false, &worked), ExitCode::Success);
        // Validation keeps its own code.
        assert_eq!(
            exit_for(true, &serde_json::json!({ "clean": false })),
            ExitCode::Validation
        );
    }

    #[test]
    fn an_authoring_verb_follows_the_same_rule() {
        // Scenario: Un verbo de autoría sigue la misma regla
        //
        // What `meltemi plan` gets back from `sdd/plan` when its turn was
        // empty: the rule reads the field, whichever verb carried it.
        let planned = serde_json::json!({
            "changeName": "add-thing",
            "phase": "invalid",
            "artifact": "design",
            "diagnostics": ["design.md is missing or empty"],
            "emptyTurn": true,
        });
        let mut err = Vec::new();
        assert_eq!(
            conclude(false, &planned, false, &mut err),
            ExitCode::EmptyTurn
        );
        let said = String::from_utf8(err).unwrap();
        assert!(said.contains("without saying or doing anything"), "{said}");
        assert!(
            said.contains("provider") && said.contains("credentials"),
            "the remedy: {said}"
        );

        // In machine mode the verdict is the answer's own field: no prose.
        let mut err = Vec::new();
        assert_eq!(
            conclude(false, &planned, true, &mut err),
            ExitCode::EmptyTurn
        );
        assert!(err.is_empty());
    }
}
