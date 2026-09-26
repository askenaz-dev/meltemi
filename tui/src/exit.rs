// SPDX-License-Identifier: Apache-2.0

//! Exit-code taxonomy (design D3).
//!
//! Stable contract: `0/1/2` follow the POSIX/shell convention; domain codes
//! start at `10` to avoid colliding with the shell's reserved range
//! (126–128+n). The set is extensible without renumbering; any change to it is
//! a change to the `cli-contract` capability.

/// A process exit category. The numeric value is the process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitCode {
    /// The command completed its purpose.
    #[default]
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
    /// The operation was refused by policy: the permission proxy's own rule
    /// refused it — a decision that reached a request no longer waiting, where
    /// the first decision wins (verbos-del-agent-boss design D4).
    Denied = 12,
    /// The operation was cancelled. Reserved for richer flows; defined here as
    /// part of the stable taxonomy.
    Cancelled = 13,
    /// A validation completed with findings (not an error of the channel, a
    /// result — `sdd/validate`). Lets CI distinguish clean from findings.
    Validation = 14,
    /// The agent's turn ended having said and done nothing: a result, not an
    /// error of the channel, that a script must be able to tell from success
    /// (turno-vacio-es-error design D4).
    EmptyTurn = 15,
}

impl ExitCode {
    /// The process exit code carried by this category.
    #[must_use]
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// The stable exit-code taxonomy as data, so the CLI reference is generated
/// from a single source (documentacion-inicial). Any change here regenerates
/// the reference; a stale reference fails the freshness check.
pub const EXIT_CODES: &[(i32, &str)] = &[
    (0, "success — the command completed its purpose"),
    (1, "internal — an unexpected internal error"),
    (
        2,
        "usage — unknown subcommand, invalid flags, or a missing argument",
    ),
    (
        10,
        "unreachable — the daemon could not be reached or started",
    ),
    (
        11,
        "contract — the daemon answered with a contract/protocol error",
    ),
    (
        12,
        "denied — refused by the permission proxy's policy: a decision reached a request \
         no longer waiting (the first decision wins)",
    ),
    (13, "cancelled — the operation was cancelled"),
    (
        14,
        "validation — a validation completed with findings (not an error)",
    ),
    (
        15,
        "empty turn — the agent's turn ended without saying or doing anything",
    ),
];
