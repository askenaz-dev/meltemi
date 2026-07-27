// SPDX-License-Identifier: Apache-2.0

//! The base mapping between an ACP session and the provider subprocess
//! (adaptadores-propios-acp task 1.2).
//!
//! An ACP session and a provider process have different shapes, and the
//! mismatches are where adapters usually rot: two prompts racing on one
//! provider, a cancellation that arrives after the turn already ended, a
//! session that closes while its subprocess keeps running. This state machine
//! is the shared answer, and it is deliberately free of I/O — the dialects own
//! the wire, this owns *when* the subprocess must live, quiesce and die, and
//! the rules are therefore testable without a process in sight.
//!
//! The invariants it enforces:
//!
//! - a session owns **at most one** provider process;
//! - a session runs **at most one turn at a time** — ACP prompts a session
//!   serially, and a provider that received two interleaved turns would answer
//!   neither faithfully;
//! - a cancellation interrupts the turn in flight, and a turn that drains after
//!   one ends **cancelled**, never "completed";
//! - closing a session shuts its provider down, exactly once.

/// Where the session is with respect to the turn it may be running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnState {
    /// No turn in flight.
    Idle,
    /// A turn is in flight on the provider.
    InTurn,
    /// Cancellation was asked for and the turn is draining.
    Cancelling,
    /// The session is closed; no turn will run again.
    Closed,
}

/// Whether this session has a provider process, and whether it still runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderState {
    /// No process launched yet (the dialect launches it when the session needs
    /// it).
    NotStarted,
    /// A process is running under supervision.
    Running,
    /// The process is gone.
    Ended,
}

/// What the adapter must do about a cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelAction {
    /// Interrupt the turn in flight, in the provider's own terms.
    InterruptTurn,
    /// Nothing is in flight: there is nothing to interrupt, and nothing is
    /// remembered either — a stale cancellation must not poison the next turn.
    Nothing,
}

/// How a turn ended, which is what the ACP stop reason is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEnd {
    /// The provider finished the turn.
    Completed,
    /// The turn drained after a cancellation.
    Cancelled,
}

/// What closing the session requires of the adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// A provider process is still running and must be shut down.
    ShutDownProvider,
    /// There is nothing left to end.
    Nothing,
}

/// Why a lifecycle transition was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    /// A prompt arrived while a turn was already in flight.
    TurnAlreadyInFlight,
    /// The session is closed.
    SessionClosed,
    /// A provider process was announced twice for one session.
    ProviderAlreadyRunning,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            LifecycleError::TurnAlreadyInFlight => "a turn is already in flight on this session",
            LifecycleError::SessionClosed => "this session is closed",
            LifecycleError::ProviderAlreadyRunning => {
                "this session already supervises a provider process"
            }
        };
        f.write_str(text)
    }
}

/// One ACP session's lifecycle over its provider subprocess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLifecycle {
    turn: TurnState,
    provider: ProviderState,
}

impl Default for SessionLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionLifecycle {
    /// A session just opened by `session/new`: no provider yet, no turn.
    #[must_use]
    pub fn new() -> Self {
        Self {
            turn: TurnState::Idle,
            provider: ProviderState::NotStarted,
        }
    }

    /// The turn state, as the adapter reports it.
    #[must_use]
    pub fn turn_state(&self) -> TurnState {
        self.turn
    }

    /// Whether a provider process is running under this session.
    #[must_use]
    pub fn provider_running(&self) -> bool {
        self.provider == ProviderState::Running
    }

    /// The dialect launched the provider process for this session.
    ///
    /// # Errors
    ///
    /// Refuses on a closed session, and refuses a second process for one
    /// session: the first would be left unsupervised.
    pub fn provider_started(&mut self) -> Result<(), LifecycleError> {
        if self.turn == TurnState::Closed {
            return Err(LifecycleError::SessionClosed);
        }
        if self.provider == ProviderState::Running {
            return Err(LifecycleError::ProviderAlreadyRunning);
        }
        self.provider = ProviderState::Running;
        Ok(())
    }

    /// A prompt arrived: the turn begins.
    ///
    /// # Errors
    ///
    /// Refuses a concurrent turn and a turn on a closed session.
    pub fn turn_started(&mut self) -> Result<(), LifecycleError> {
        match self.turn {
            TurnState::Idle => {
                self.turn = TurnState::InTurn;
                Ok(())
            }
            TurnState::InTurn | TurnState::Cancelling => Err(LifecycleError::TurnAlreadyInFlight),
            TurnState::Closed => Err(LifecycleError::SessionClosed),
        }
    }

    /// `session/cancel` arrived.
    pub fn cancel_requested(&mut self) -> CancelAction {
        match self.turn {
            TurnState::InTurn => {
                self.turn = TurnState::Cancelling;
                CancelAction::InterruptTurn
            }
            // Already cancelling, idle or closed: nothing to interrupt. Asking
            // the provider twice would be noise, not safety.
            _ => CancelAction::Nothing,
        }
    }

    /// The provider reported the turn over. The answer is what the ACP stop
    /// reason must say.
    pub fn turn_ended(&mut self) -> TurnEnd {
        let end = match self.turn {
            TurnState::Cancelling => TurnEnd::Cancelled,
            _ => TurnEnd::Completed,
        };
        if self.turn != TurnState::Closed {
            self.turn = TurnState::Idle;
        }
        end
    }

    /// The session closes — because the daemon closed it, or because the ACP
    /// connection ended. Idempotent: the provider is shut down once.
    pub fn close(&mut self) -> CloseAction {
        self.turn = TurnState::Closed;
        if self.provider == ProviderState::Running {
            self.provider = ProviderState::Ended;
            CloseAction::ShutDownProvider
        } else {
            CloseAction::Nothing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_turn_at_a_time_on_one_provider() {
        // Two prompts racing on one subprocess would interleave two turns on a
        // wire that has no way to tell them apart.
        let mut session = SessionLifecycle::new();
        session.provider_started().unwrap();
        assert!(session.provider_running());
        assert_eq!(
            session.provider_started(),
            Err(LifecycleError::ProviderAlreadyRunning)
        );

        session.turn_started().unwrap();
        assert_eq!(
            session.turn_started(),
            Err(LifecycleError::TurnAlreadyInFlight)
        );
        assert_eq!(session.turn_ended(), TurnEnd::Completed);
        session
            .turn_started()
            .expect("the next turn runs once the first is over");
    }

    #[test]
    fn a_cancelled_turn_ends_cancelled_and_a_stale_cancellation_changes_nothing() {
        // The stop reason must tell the truth: a turn that drained after a
        // cancellation did not complete. And a cancellation with nothing in
        // flight must not be remembered — it would kill the next honest turn.
        let mut session = SessionLifecycle::new();
        session.provider_started().unwrap();

        assert_eq!(session.cancel_requested(), CancelAction::Nothing);
        session.turn_started().unwrap();
        assert_eq!(session.cancel_requested(), CancelAction::InterruptTurn);
        assert_eq!(
            session.cancel_requested(),
            CancelAction::Nothing,
            "the provider is asked to interrupt once, not once per notification"
        );
        assert_eq!(session.turn_ended(), TurnEnd::Cancelled);

        session.turn_started().unwrap();
        assert_eq!(
            session.turn_ended(),
            TurnEnd::Completed,
            "the next turn is not tainted by the cancelled one"
        );
    }

    #[test]
    fn closing_a_session_ends_its_provider_exactly_once() {
        // The subprocess must not outlive the session that asked for it, and
        // must not be shut down twice (the second attempt would kill whatever
        // took its process id).
        let mut session = SessionLifecycle::new();
        session.provider_started().unwrap();
        assert_eq!(session.close(), CloseAction::ShutDownProvider);
        assert_eq!(session.close(), CloseAction::Nothing);
        assert_eq!(session.turn_state(), TurnState::Closed);
        assert_eq!(session.turn_started(), Err(LifecycleError::SessionClosed));
        assert_eq!(
            session.provider_started(),
            Err(LifecycleError::SessionClosed)
        );

        // A session that never launched a provider closes with nothing to end.
        let mut never_started = SessionLifecycle::new();
        assert_eq!(never_started.close(), CloseAction::Nothing);
    }
}
