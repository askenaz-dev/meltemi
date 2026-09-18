// SPDX-License-Identifier: Apache-2.0

//! Process scopes: ending a launched process ends everything it launched
//! (apagado-entero-y-modos design D1, D2).
//!
//! Meltemi launches providers it does not own — the official CLI of each agent
//! — and on Windows it frequently does not launch the CLI at all. What `npm i
//! -g` installs is a `.cmd` shim: a script that `cmd.exe` runs, which then
//! creates the process that does the work. `TerminateProcess` on the shim ends
//! `cmd.exe` and leaves that process running, holding the worktree it was
//! working in. Measured on Windows 11 (26200) with a shim whose body was `node
//! -e "setTimeout(()=>{},60000)"`: after the shim was terminated, the `node`
//! was still alive.
//!
//! A [`Scope`] is the answer, and on Windows it is a Job Object with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Two properties, and nothing else gives
//! both:
//!
//! 1. **Ending the scope ends the whole tree**, at any depth — the shim, what
//!    it created, and what that created in turn. Nothing here has to know what
//!    is underneath, which matters because the shim's format belongs to
//!    whoever generated it.
//! 2. **Closing the handle ends it too.** The handle lives in the process that
//!    launched; if that process dies without running any cleanup — a crash, a
//!    `TerminateProcess`, the user signing out — the system closes its handles
//!    and the tree goes with them. That is the promise `kill_on_drop` makes and
//!    a destructor cannot keep, kept here by the kernel.
//!
//! Off Windows the scope is inert, and deliberately so: the shims there are
//! scripts with a shebang, the kernel delivers the signal to the process a
//! launcher names, and what an agent starts on its own account — its MCP
//! servers — is the agent's business and its own signal handling. The type
//! exists on every platform so the code that launches carries no `#[cfg]`;
//! only the body is platform-specific.
//!
//! # What this crate deliberately is not
//!
//! It is not a process supervisor. It does not launch, wait, read or write:
//! callers own their child and its streams, and hand this the identity of the
//! process to scope. Keeping it to that is what lets one `unsafe` module serve
//! the daemon and the adapters without either of them linking the other's
//! world.

use std::io;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
use windows as platform;

#[cfg(not(windows))]
mod inert;

#[cfg(not(windows))]
use inert as platform;

/// A scope that owns the lives of the processes adopted into it.
///
/// Create one per provider process, adopt the process into it, and end it when
/// the provider must go. Dropping it releases the scope, which on a platform
/// that binds lives to it also ends whatever is still inside — so a launcher
/// that dies without cleaning up leaves nothing behind.
#[derive(Debug)]
pub struct Scope(platform::Scope);

impl Scope {
    /// Opens a new, empty scope.
    ///
    /// # Errors
    ///
    /// Returns the platform's error when the scope cannot be created. A caller
    /// should treat that as a launch failure rather than launching without a
    /// scope: a provider nobody can fully end is the defect this exists to
    /// prevent, and starting one anyway would trade a loud failure for a quiet
    /// one.
    pub fn new() -> io::Result<Self> {
        platform::Scope::new().map(Self)
    }

    /// Adopts a running process, and thereby everything it goes on to create.
    ///
    /// # Correctness
    ///
    /// The caller MUST still hold the handle to that process — its `Child` —
    /// while this runs. That is not a style preference: an identifier whose
    /// process has already been reaped can be handed to a different process by
    /// the operating system, and adopting the wrong process would put a
    /// stranger under our control. A live `Child` holds the process open on
    /// every platform we support, so the identifier cannot be reused while it
    /// exists.
    ///
    /// Adopting is also **not retroactive**: a child the adopted process
    /// created before this call is outside the scope. The window is the time
    /// between the launch returning and this call, and closing it entirely
    /// would mean launching suspended, which no standard-library launcher
    /// exposes (design D1). It is named, accepted, and watched by the test that
    /// launches a real intermediary.
    ///
    /// # Errors
    ///
    /// Returns the platform's error when the process cannot be adopted.
    pub fn adopt_pid(&self, pid: u32) -> io::Result<()> {
        self.0.adopt_pid(pid)
    }

    /// Ends every process in the scope, at any depth.
    ///
    /// Idempotent: ending a scope whose processes have already gone is not an
    /// error, because "it is already over" is the outcome the caller wanted.
    ///
    /// # Errors
    ///
    /// Returns the platform's error when the scope cannot be ended.
    pub fn end(&self) -> io::Result<()> {
        self.0.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_opens_and_ends_with_nothing_in_it() {
        // Ending an empty scope is not an error anywhere: a caller that failed
        // to launch still ends the scope it opened, and that path must not
        // report a failure of its own on top of the real one.
        let scope = Scope::new().expect("a scope can be opened");
        scope.end().expect("ending an empty scope is not a failure");
    }

    #[test]
    fn ending_twice_is_not_an_error() {
        // The shutdown path can run from more than one place — the grace
        // expiring and the session closing — and neither should have to know
        // whether the other got there first.
        let scope = Scope::new().expect("a scope can be opened");
        scope.end().expect("first");
        scope.end().expect("second");
    }
}
