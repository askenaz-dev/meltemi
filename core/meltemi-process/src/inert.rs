// SPDX-License-Identifier: Apache-2.0

//! The scope everywhere else: a type that exists so the launching code carries
//! no `#[cfg]`, and does nothing because nothing is needed.
//!
//! The reason is not that orphans are impossible here — it is that the two
//! cases this crate answers do not arise the same way. The shims a package
//! manager installs on these platforms are scripts with a shebang, so the
//! launcher's own process *is* the interpreter and a signal to it reaches the
//! work; and what an agent starts on its own account belongs to the agent,
//! which handles its own termination. Binding lives to a launcher here would
//! need a different mechanism per platform — a parent-death signal on Linux,
//! and nothing equivalent on macOS — and no measurement has asked for it
//! (apagado-entero-y-modos, fuera de alcance).

use std::io;

/// The inert scope. Every operation succeeds and none of them does anything.
#[derive(Debug)]
pub struct Scope;

impl Scope {
    pub fn new() -> io::Result<Self> {
        Ok(Self)
    }

    /// Accepts any identifier, including one that names nothing: there is no
    /// scope to put it in, so there is nothing that could fail.
    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn adopt_pid(&self, _pid: u32) -> io::Result<()> {
        Ok(())
    }

    /// Ends nothing. The caller still ends the process it launched by its own
    /// means, which on these platforms reaches the work.
    #[allow(clippy::unused_self)]
    pub fn end(&self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inert_scope_never_refuses_and_never_acts() {
        // Pinned so the inertness is a decision someone reads rather than an
        // omission someone finds: the same calls the Windows scope answers,
        // answered here without a platform to ask.
        let scope = Scope::new().expect("an inert scope opens");
        scope
            .adopt_pid(std::process::id())
            .expect("adopting is accepted");
        scope
            .adopt_pid(0)
            .expect("even an identifier that names nothing");
        scope.end().expect("ending is accepted");
    }
}
