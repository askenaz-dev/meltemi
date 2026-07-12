// SPDX-License-Identifier: Apache-2.0

//! `meltemid` entry point: initializes file logging and runs the daemon.

fn main() -> anyhow::Result<()> {
    // Keep the appender guard alive for the whole process so buffered log
    // lines are flushed on exit.
    let _guard = meltemid::logging::init_file_logging(&meltemid::paths::data_dir())?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(meltemid::run())
}
