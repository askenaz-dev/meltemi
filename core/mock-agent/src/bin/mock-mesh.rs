// SPDX-License-Identifier: Apache-2.0

//! A simulated mesh client, for the end-to-end tests of `identity/status`
//! (identidad-propia design D8).
//!
//! Why a simulated client exists at all: the constitution forbids any test that
//! needs a provider account or a network (§5), and the living spec scenario «La
//! malla del usuario no es una dependencia de Meltemi» forbids pulling a mesh
//! into the workspace. So the tests point `[identity] command` at this binary
//! and read its canonical output.
//!
//! **Where this output comes from, and why it can be trusted.** The task that
//! introduced it was to capture a real redacted `status --json` first, because
//! the shape of that JSON is not verifiable from this repository and a
//! mis-named field would give a green parser in CI and a refusal against the
//! real client. No mesh client was installed on the maintainer's machine, so the
//! shape was verified the stronger way instead: against the client's own
//! published type definitions, pinned at tag `v1.102.5` —
//! `ipn/ipnstate/ipnstate.go` (`Status`, `PeerStatus`, `TailnetStatus`) and
//! `tailcfg/tailcfg.go` (`UserProfile`) — which is what the real CLI serialises.
//! Stronger because a capture shows only the fields that ONE machine populated
//! in ONE state, while the type enumerates every field with its JSON tag and its
//! presence rule. Three of those rules are reproduced here on purpose, because
//! they are the ones a capture from a healthy linked machine would have hidden:
//!
//! 1. `CurrentTailnet` and `Self` are **null** when the machine is not
//!    connected, so the unlinked fixture emits null rather than omitting them.
//! 2. `LastSeen` carries no `omitempty`, so an online peer emits the **zero
//!    time** `0001-01-01T00:00:00Z` — not an absent field. A parser that trusts
//!    it would report "last seen: year 1" for the machine that is online now.
//! 3. The keys of `User` are the user id in **decimal, as a string** (Go demands
//!    text keys), while `PeerStatus.UserID` is a **number**.
//!
//! The fixtures are deliberately written as plain text rather than built from a
//! serialiser, so that a human can diff them against a real client's output when
//! one is at hand.
//!
//! Driven by `MOCK_MESH_SCENARIO`; refuses any argv that is not the one
//! read-only verb, which is what makes «Ningún token entra ni sale del daemon»
//! an assertion rather than a promise.

use std::io::Write;

/// The scenario to play, read from `MOCK_MESH_SCENARIO`.
const SCENARIO: &str = "MOCK_MESH_SCENARIO";

/// The only argv this client accepts. Anything else is a bug in the daemon, and
/// a loud refusal here is how the test notices: a daemon that ever ran `up`,
/// `login` or `logout` would fail the test instead of quietly signing somebody
/// in.
const READ_ONLY: [&str; 2] = ["status", "--json"];

/// Linked, with three machines of which one is offline.
///
/// Three because the scenario «Las máquinas son los pares de la malla» asks for
/// three: this machine plus two peers.
const LINKED: &str = r#"{
  "Version": "1.102.5-mock",
  "TUN": true,
  "BackendState": "Running",
  "AuthURL": "",
  "TailscaleIPs": ["100.64.0.1", "fd7a:115c:a1e0::1"],
  "Self": {
    "ID": "nMOCKSELF",
    "PublicKey": "nodekey:1111111111111111111111111111111111111111111111111111111111111111",
    "HostName": "mi-pc",
    "DNSName": "mi-pc.tailnet-mock.ts.net.",
    "OS": "windows",
    "UserID": 180217,
    "TailscaleIPs": ["100.64.0.1"],
    "Online": true,
    "LastSeen": "0001-01-01T00:00:00Z",
    "ExitNode": false,
    "Active": true
  },
  "Health": [],
  "MagicDNSSuffix": "tailnet-mock.ts.net",
  "CurrentTailnet": {
    "Name": "mock.example",
    "MagicDNSSuffix": "tailnet-mock.ts.net",
    "MagicDNSEnabled": true
  },
  "CertDomains": null,
  "Peer": {
    "nodekey:2222222222222222222222222222222222222222222222222222222222222222": {
      "ID": "nMOCKLAP",
      "PublicKey": "nodekey:2222222222222222222222222222222222222222222222222222222222222222",
      "HostName": "el-portatil",
      "DNSName": "el-portatil.tailnet-mock.ts.net.",
      "OS": "macOS",
      "UserID": 180217,
      "TailscaleIPs": ["100.64.0.2"],
      "Online": true,
      "LastSeen": "0001-01-01T00:00:00Z",
      "ExitNode": false,
      "Active": false
    },
    "nodekey:3333333333333333333333333333333333333333333333333333333333333333": {
      "ID": "nMOCKTWR",
      "PublicKey": "nodekey:3333333333333333333333333333333333333333333333333333333333333333",
      "HostName": "la-torre",
      "DNSName": "la-torre.tailnet-mock.ts.net.",
      "OS": "linux",
      "UserID": 180217,
      "TailscaleIPs": ["100.64.0.3"],
      "Online": false,
      "LastSeen": "2026-09-30T18:21:05Z",
      "ExitNode": false,
      "Active": false
    },
    "nodekey:4444444444444444444444444444444444444444444444444444444444444444": {
      "ID": "nMOCKSHARED",
      "PublicKey": "nodekey:4444444444444444444444444444444444444444444444444444444444444444",
      "HostName": "equipo-de-otra-persona",
      "DNSName": "equipo-de-otra-persona.tailnet-mock.ts.net.",
      "OS": "linux",
      "UserID": 900001,
      "TailscaleIPs": ["100.64.9.9"],
      "Online": true,
      "LastSeen": "0001-01-01T00:00:00Z",
      "ShareeNode": true,
      "ExitNode": false,
      "Active": false
    }
  },
  "User": {
    "180217": {
      "ID": 180217,
      "LoginName": "guillmar@mock.example",
      "DisplayName": "Guillmar Ortiz",
      "ProfilePicURL": ""
    },
    "900001": {
      "ID": 900001,
      "LoginName": "otra-persona@mock.example",
      "DisplayName": "Otra Persona"
    }
  }
}
"#;

/// The client is installed and running, and this machine has no session.
///
/// `Self`, `CurrentTailnet` and `Peer` are null, which is what the types say
/// happens when the node is not connected — and the whole reason this fixture
/// exists separately instead of being the linked one with a flag flipped.
const UNLINKED: &str = r#"{
  "Version": "1.102.5-mock",
  "TUN": true,
  "BackendState": "NeedsLogin",
  "AuthURL": "",
  "TailscaleIPs": null,
  "Self": null,
  "Health": ["not logged in"],
  "MagicDNSSuffix": "",
  "CurrentTailnet": null,
  "CertDomains": null,
  "Peer": null,
  "User": null
}
"#;

/// Linked, and the only machine on the mesh.
///
/// A mesh of one is a mesh, and the contract says an empty peer list is not an
/// error — so there is a fixture that proves it rather than a comment claiming
/// it.
const ALONE: &str = r#"{
  "Version": "1.102.5-mock",
  "TUN": true,
  "BackendState": "Running",
  "AuthURL": "",
  "TailscaleIPs": ["100.64.0.1"],
  "Self": {
    "ID": "nMOCKSELF",
    "PublicKey": "nodekey:1111111111111111111111111111111111111111111111111111111111111111",
    "HostName": "mi-pc",
    "DNSName": "mi-pc.tailnet-mock.ts.net.",
    "OS": "windows",
    "UserID": 180217,
    "Online": true,
    "LastSeen": "0001-01-01T00:00:00Z",
    "Active": true
  },
  "Health": [],
  "CurrentTailnet": {
    "Name": "mock.example",
    "MagicDNSSuffix": "tailnet-mock.ts.net",
    "MagicDNSEnabled": true
  },
  "Peer": {},
  "User": {
    "180217": {
      "ID": 180217,
      "LoginName": "guillmar@mock.example",
      "DisplayName": "Guillmar Ortiz"
    }
  }
}
"#;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != READ_ONLY {
        eprintln!(
            "mock-mesh refuses `{}`: this simulated client answers only `{}`, because the daemon \
             must never invoke anything that changes mesh state",
            args.join(" "),
            READ_ONLY.join(" ")
        );
        std::process::exit(64);
    }

    let scenario = std::env::var(SCENARIO).unwrap_or_else(|_| "linked".to_string());
    let body = match scenario.as_str() {
        "linked" => LINKED,
        "unlinked" => UNLINKED,
        "alone" => ALONE,
        // A client that is there and fails. The daemon must survive it and say
        // something true about it.
        "fail" => {
            eprintln!("mock-mesh: failed to connect to local tailscaled");
            std::process::exit(1);
        }
        // A client that answers something that is not the JSON we know. The
        // honest answer is a refusal, never a half-parsed identity.
        "garbage" => "this is not the json you are looking for\n",
        // A client that hangs. The read has a deadline, and this is what proves
        // the deadline bites rather than being a constant nobody tests.
        "slow" => {
            std::thread::sleep(std::time::Duration::from_secs(120));
            std::process::exit(0);
        }
        other => {
            eprintln!("mock-mesh: unknown {SCENARIO} `{other}`");
            std::process::exit(64);
        }
    };

    let mut out = std::io::stdout();
    let _ = out.write_all(body.as_bytes());
    let _ = out.flush();
}
