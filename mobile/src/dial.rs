// SPDX-License-Identifier: Apache-2.0

//! The companion's dialer (companero-que-conecta D1): what the shared bridge
//! actor calls to open its stream, which here is an SSH channel that ends in
//! `meltemi bridge` on the user's machine.
//!
//! The actor does not know this is a phone. It asks for a stream, serves it,
//! and asks again when it ends; what it learns from a failure is only whether
//! waiting could help. A machine that does not answer may answer in a moment;
//! a machine that refused this app's key, or that is not the machine it was,
//! will not — and the actor stops on those rather than knocking again.

use meltemi_client::bridge::{Dial, Dialer, Refused};
use meltemi_client::transport::Stream;
use russh::keys::PrivateKey;

use crate::ssh::{self, Destination, HostKeys, Refusal, Tunnel};

/// Dials `meltemi bridge` over SSH with the app's own key.
pub struct SshDialer {
    destination: Destination,
    key: PrivateKey,
    hosts: Box<dyn HostKeys>,
    /// The live session behind the stream the actor is serving. The stream
    /// borrows nothing, so this is what keeps it open: replaced on every dial
    /// and dropped with the dialer, which is dropped with the actor.
    tunnel: Option<Tunnel>,
}

impl SshDialer {
    #[must_use]
    pub fn new(destination: Destination, key: PrivateKey, hosts: Box<dyn HostKeys>) -> Self {
        Self {
            destination,
            key,
            hosts,
            tunnel: None,
        }
    }
}

impl Dialer for SshDialer {
    async fn dial(&mut self) -> Result<Stream, Dial> {
        // The previous session goes first: the stream it carried has already
        // ended, or the actor would not be dialling again.
        self.tunnel = None;
        match ssh::open(&self.destination, self.key.clone(), self.hosts.as_mut()).await {
            Ok((stream, tunnel)) => {
                self.tunnel = Some(tunnel);
                Ok(stream)
            }
            Err(refusal) => Err(classify(refusal)),
        }
    }

    fn shown(&self) -> String {
        self.destination.shown()
    }

    fn switch(&mut self, to: String) {
        // Nobody sends `Switch` to the companion today: changing destination
        // restarts the actor from the host. If somebody does, a target that
        // does not parse keeps the current one — dialling something half-read
        // is how a connection goes somewhere else.
        if let Ok(destination) = Destination::parse(&to) {
            self.destination = destination;
        }
    }
}

/// Which refusals the actor waits out, and which end it.
#[must_use]
pub fn classify(refusal: Refusal) -> Dial {
    match refusal {
        Refusal::HostKeyChanged { .. } => Dial::Terminal {
            kind: Refused::HostKeyChanged,
            detail: refusal.to_string(),
        },
        Refusal::Rejected(_) => Dial::Terminal {
            kind: Refused::Rejected,
            detail: refusal.to_string(),
        },
        // `open` never produces this one — it takes a parsed destination — but
        // the type carries it, and a bad destination is not fixed by waiting.
        // The host parses before it dials, so this stays unreachable in
        // practice; if it ever were reached, the honest reading is "refused".
        Refusal::Destination(_) => Dial::Terminal {
            kind: Refused::Rejected,
            detail: refusal.to_string(),
        },
        Refusal::Transport(said) => Dial::Transient(said),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: Un rechazo detiene el actor y lo publica
    #[test]
    fn a_changed_host_key_and_a_rejected_key_are_terminal_and_say_which() {
        let changed = classify(Refusal::HostKeyChanged {
            host: "mac-studio".into(),
            remembered: "SHA256:old".into(),
            offered: "SHA256:new".into(),
        });
        match changed {
            Dial::Terminal { kind, detail } => {
                assert_eq!(kind, Refused::HostKeyChanged);
                assert!(detail.contains("Nothing was sent"), "{detail}");
                assert!(detail.contains("not that machine"), "{detail}");
            }
            Dial::Transient(said) => panic!("a changed host key is never transient: {said}"),
        }

        let rejected = classify(Refusal::Rejected(
            "`ana@mac-studio` did not accept it".into(),
        ));
        match rejected {
            Dial::Terminal { kind, detail } => {
                assert_eq!(kind, Refused::Rejected);
                assert!(detail.contains("authorized_keys"), "{detail}");
            }
            Dial::Transient(said) => panic!("a rejected key is never transient: {said}"),
        }
    }

    // Scenario: Un fallo de transporte reintenta con backoff
    #[test]
    fn a_transport_failure_is_waited_out_with_what_the_transport_said() {
        match classify(Refusal::Transport("could not reach ana@mac-studio".into())) {
            Dial::Transient(said) => assert_eq!(said, "could not reach ana@mac-studio"),
            Dial::Terminal { detail, .. } => {
                panic!("a tunnel that comes and goes is not a refusal: {detail}")
            }
        }
    }
}
