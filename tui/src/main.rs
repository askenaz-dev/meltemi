// SPDX-License-Identifier: Apache-2.0

//! `meltemi` (alias `mel`): scriptable CLI now, interactive TUI later.
//!
//! Thin wrapper: resolve the invocation ([`meltemi::cli::plan`]), dispatch it,
//! then exit with the resulting code. All logic lives in the library so it is
//! testable off the process boundary.

use std::io::{self, IsTerminal, Write};

use meltemi::approver::Approver;
use meltemi::approver::keys::TerminalKeys;
use meltemi::cli::plan;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout_is_tty = io::stdout().is_terminal();
    let planned = plan(&args, stdout_is_tty);

    let endpoint = meltemi_client::paths::endpoint();
    // Permission requests are asked only where a terminal can answer them:
    // stdin and stderr are terminals (aprobador-en-el-cli design D1).
    // Everywhere else they are refused, saying how to grant without a human.
    let approver = if TerminalKeys::available() {
        Approver::terminal()
    } else {
        Approver::deny()
    };
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();
    let code = meltemi::dispatch_with(
        planned,
        &endpoint,
        stdout_is_tty,
        &mut out,
        &mut err,
        approver.clone(),
    )
    .await;

    let _ = out.flush();
    let _ = err.flush();
    // `exit` runs no destructors: the terminal leaves raw mode here if a read
    // was somehow left in it (design D12).
    approver.restore();
    std::process::exit(code);
}
