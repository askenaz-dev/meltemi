// SPDX-License-Identifier: Apache-2.0

//! Meltemi terminal client (`cli-contrato`): the scriptable CLI surface.
//!
//! This crate builds the single `meltemi` binary (alias `mel`, shipped at
//! packaging time). In this change it is a scriptable CLI; the interactive TUI
//! (#6) replaces the deferred [`cli::Action::Interactive`] branch without
//! touching the dispatch rule. The library exposes the grammar, the output
//! discipline and the command↔RPC mapping so all of it is testable off the
//! process boundary.

pub mod cli;
pub mod exit;
pub mod output;
pub mod run;

use std::io::Write;

use cli::{Action, Command, Plan};
use exit::ExitCode;
use output::{CliError, render_error, render_outcome};

/// Runs a resolved [`Plan`] against the daemon at `endpoint`, writing to the
/// given stdout/stderr sinks, and returns the process exit code.
pub async fn dispatch(
    plan: Plan,
    endpoint: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let json = plan.json;
    match plan.action {
        Action::Help => {
            let _ = writeln!(out, "{}", cli::USAGE);
            ExitCode::Success.code()
        }
        Action::Version => {
            if json {
                let object =
                    serde_json::json!({ "name": "meltemi", "version": env!("CARGO_PKG_VERSION") });
                let _ = writeln!(out, "{object}");
            } else {
                let _ = writeln!(out, "meltemi {}", env!("CARGO_PKG_VERSION"));
            }
            ExitCode::Success.code()
        }
        Action::Interactive => {
            // Deferred in this change: the dispatch rule is fixed, the TUI (#6)
            // fills the body. stdout stays clean; the notice goes to stderr.
            let _ = writeln!(
                err,
                "meltemi: the interactive interface arrives in a later release; \
                 run `meltemi help` for the scriptable commands"
            );
            ExitCode::Success.code()
        }
        Action::Usage(message) => {
            let error = CliError::usage(message);
            let _ = render_error(&error, json, out, err);
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
        Action::Run(command) => match run::execute(command, endpoint).await {
            Ok(outcome) => {
                let _ = render_outcome(&outcome, json, out);
                ExitCode::Success.code()
            }
            Err(error) => {
                let _ = render_error(&error, json, out, err);
                error.exit.code()
            }
        },
    }
}
