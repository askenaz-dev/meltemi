// SPDX-License-Identifier: Apache-2.0

//! `meltemi` (alias `mel`): scriptable CLI now, interactive TUI later.
//!
//! Thin wrapper: resolve the invocation ([`meltemi::cli::plan`]), dispatch it,
//! then exit with the resulting code. All logic lives in the library so it is
//! testable off the process boundary.

use std::io::{self, IsTerminal, Write};

use meltemi::cli::plan;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout_is_tty = io::stdout().is_terminal();
    let planned = plan(&args, stdout_is_tty);

    let endpoint = meltemid::paths::endpoint();
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();
    let code = meltemi::dispatch(planned, &endpoint, &mut out, &mut err).await;

    let _ = out.flush();
    let _ = err.flush();
    std::process::exit(code);
}
