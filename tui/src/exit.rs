// SPDX-License-Identifier: Apache-2.0

//! Exit-code taxonomy (design D3).
//!
//! Stable contract: `0/1/2` follow the POSIX/shell convention; domain codes
//! start at `10` to avoid colliding with the shell's reserved range
//! (126–128+n). The set is extensible without renumbering; any change to it is
//! a change to the `cli-contract` capability.

/// A process exit category. The numeric value is the process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// The command completed its purpose.
    Success = 0,
    /// An unexpected internal error.
    Internal = 1,
    /// A usage error: unknown subcommand, invalid flags, missing argument.
    Usage = 2,
    /// The daemon could not be reached or started on demand.
    Unreachable = 10,
    /// The daemon answered with a contract error, or refused the protocol
    /// version.
    Contract = 11,
    /// The operation was refused by policy (denied). Reserved for the
    /// permission proxy (#9); defined here as part of the stable taxonomy.
    Denied = 12,
    /// The operation was cancelled. Reserved for richer flows; defined here as
    /// part of the stable taxonomy.
    Cancelled = 13,
}

impl ExitCode {
    /// The process exit code carried by this category.
    #[must_use]
    pub fn code(self) -> i32 {
        self as i32
    }
}
