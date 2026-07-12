// SPDX-License-Identifier: Apache-2.0

//! Operational logging for `meltemid` (design D12).
//!
//! The daemon runs detached, with no terminal, so its operational log goes to
//! a rotating file in the user data directory (`<data_dir>/logs/`). Levels are
//! controlled by the `MELTEMI_LOG` environment variable (an
//! [`EnvFilter`](tracing_subscriber::EnvFilter) directive), defaulting to
//! `info`. Returns the appender guard, which must be kept alive for the
//! lifetime of the process so buffered lines are flushed.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt};

/// Environment variable controlling log verbosity.
pub const ENV_LOG: &str = "MELTEMI_LOG";

/// Initializes file-based logging under `<data_dir>/logs/`. Returns the guard
/// that flushes the non-blocking writer on drop.
pub fn init_file_logging(data_dir: &Path) -> std::io::Result<WorkerGuard> {
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let appender = tracing_appender::rolling::daily(&log_dir, "meltemid.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env(ENV_LOG).unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Ok(guard)
}
