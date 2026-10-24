// SPDX-License-Identifier: Apache-2.0

//! Identity read from the user's own mesh (identidad-propia).
//!
//! Meltemi issues no identity and holds no accounts. What it does is read the
//! official client of the mesh the user already runs — the way the daemon reads
//! `git` — and compose the gesture that links a machine to it. The client's
//! tokens are never read, stored or forwarded (§2).
//!
//! Two things live here:
//!
//! - **Reading status**: one read-only invocation of the user's client, with a
//!   deadline, normalised into the small model of the contract. Nothing outside
//!   that model leaves the parser — not addresses, not keys, and the raw output
//!   is not kept (design D2).
//! - **The linking gesture**, composed from the user's configuration and handed
//!   back as text. The daemon never runs it, because a daemon that executed a
//!   login would be a daemon holding somebody's session (design D3).
//!
//! **Where the shape of the client's JSON comes from.** The task said to capture
//! a real redacted `status --json` first, because a mis-named field would give a
//! green parser in CI and a refusal against the real client. With no mesh client
//! installed, it was verified the stronger way: against the client's own
//! published type definitions at tag `v1.102.5` — `ipn/ipnstate/ipnstate.go`
//! (`Status`, `PeerStatus`, `TailnetStatus`) and `tailcfg/tailcfg.go`
//! (`UserProfile`) — which is what the real CLI serialises. A capture shows only
//! the fields one machine populated in one state; the type enumerates every
//! field with its JSON tag and its presence rule. Four of those rules are the
//! reason this parser looks the way it does, and each has a test:
//!
//! 1. `Self` and `CurrentTailnet` are **null** when the node is not connected.
//! 2. `LastSeen` has no `omitempty`, so an online peer carries the **zero time**
//!    rather than an absent field.
//! 3. The keys of `User` are the id in **decimal as a string**, while
//!    `PeerStatus.UserID` is a **number**.
//! 4. `ShareeNode` marks a node owned by somebody who shared it in; upstream
//!    says those "should be hidden by `tailscale status` ... by default", so they
//!    are not the user's machines and are not listed as such.

use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;

use meltemi_proto::{
    IdentityLinkParams, IdentityLinkResult, IdentityStatusResult, IdentityUnavailable, MeshMachine,
    MeshPresence,
};

use crate::config::{Config, IdentityConfig};

/// How long the client gets to answer.
///
/// A read, not a job: if the mesh client cannot say who you are in this long,
/// something is wrong with it and the honest answer is a refusal that names the
/// timeout. Without a deadline a wedged client would wedge the request, and a
/// surface would show a spinner forever.
pub const BUDGET: Duration = Duration::from_secs(5);

/// The only verb the daemon ever invokes.
///
/// Read-only by construction: `up`, `login` and `logout` are not here, and this
/// is the single place a future change would have to edit to break that promise
/// — which is why the simulated client refuses any other argv.
const READ_ONLY: [&str; 2] = ["status", "--json"];

/// Go's zero `time.Time`, which `LastSeen` carries for a peer that is online.
///
/// Not an absent field: the upstream type has no `omitempty` on it, so a parser
/// that trusted the value would report "last seen: year 1" for the machine that
/// is online right now.
const ZERO_TIME: &str = "0001-01-01";

/// The backend state that means "this machine has a session on the mesh".
///
/// Every other value — `NeedsLogin`, `NoState`, `NeedsMachineAuth`, `Stopped`,
/// `Starting` — means it does not, and they are all the same answer for the
/// user: run the linking gesture.
const RUNNING: &str = "Running";

/// The mesh client's own name, when the configuration did not name a binary.
fn client_binary(identity: &IdentityConfig) -> String {
    identity
        .command
        .clone()
        .unwrap_or_else(|| identity.mesh.clone())
}

// ---------------------------------------------------------------------------
// The client's output, as the client's own types define it.
//
// Every field is optional and unknown fields are ignored, which is deliberate:
// the daemon reads a tool it does not version, and a new field in a client
// update must not turn into a refusal.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct RawStatus {
    #[serde(default, rename = "BackendState")]
    backend_state: Option<String>,
    /// `Self` in the client's JSON, which is a keyword here.
    #[serde(default, rename = "Self")]
    this: Option<RawPeer>,
    #[serde(default, rename = "CurrentTailnet")]
    tailnet: Option<RawTailnet>,
    #[serde(default, rename = "Peer")]
    peers: Option<BTreeMap<String, RawPeer>>,
    #[serde(default, rename = "User")]
    users: Option<BTreeMap<String, RawUser>>,
}

#[derive(Debug, serde::Deserialize)]
struct RawPeer {
    #[serde(default, rename = "HostName")]
    host_name: Option<String>,
    #[serde(default, rename = "OS")]
    os: Option<String>,
    #[serde(default, rename = "UserID")]
    user_id: Option<i64>,
    #[serde(default, rename = "Online")]
    online: bool,
    #[serde(default, rename = "LastSeen")]
    last_seen: Option<String>,
    #[serde(default, rename = "ShareeNode")]
    sharee_node: bool,
}

#[derive(Debug, serde::Deserialize)]
struct RawTailnet {
    #[serde(default, rename = "Name")]
    name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct RawUser {
    #[serde(default, rename = "LoginName")]
    login_name: Option<String>,
}

/// An answer that carries no identity, with the reason and its remedy.
///
/// `available: false` is an **answer**, not an error of the daemon: a user who
/// never wanted identity is not in a broken state (design D5).
fn unavailable(reason: IdentityUnavailable, remedy: impl Into<String>) -> IdentityStatusResult {
    IdentityStatusResult {
        available: false,
        reason: Some(reason),
        remedy: Some(remedy.into()),
        user: None,
        provider: None,
        this: None,
        machines: Vec::new(),
    }
}

/// Answers who the user is on their mesh, and which of their machines are there.
///
/// Never fails: the three ways identity cannot be read are answers with a reason
/// and a remedy, and no other capability of the daemon is affected by any of
/// them.
pub async fn status(config: &Config) -> IdentityStatusResult {
    let Some(identity) = config.identity.as_ref() else {
        return unavailable(
            IdentityUnavailable::NotConfigured,
            "identity is off: add an [identity] table with `mesh` to your user config.toml",
        );
    };
    let binary = client_binary(identity);
    match read(&binary).await {
        Ok(json) => match interpret(&json, identity) {
            Ok(result) => result,
            Err((reason, remedy)) => unavailable(reason, remedy),
        },
        Err((reason, remedy)) => unavailable(reason, remedy),
    }
}

/// Runs the user's client once, read-only, with a deadline.
///
/// The process is the user's, the network is their mesh daemon's, and not a byte
/// leaves Meltemi's own code — the same arrangement as `git` and the agent
/// binaries, which is why `deny.toml` stays untouched by this change.
///
/// # Errors
///
/// Every failure here is reported as `ClientMissing`, including a client that is
/// present and fails. The reason enum answers "what should I do?", and the
/// remedy for all of these is the same one: look at your mesh client. The exact
/// diagnosis travels in the remedy rather than being flattened away, so nobody
/// has to guess which of them happened.
async fn read(binary: &str) -> Result<String, (IdentityUnavailable, String)> {
    // A scope so that ending the read ends whatever the client launched, which
    // on Windows is the difference between ending a shim and ending the tool
    // underneath it (apagado-entero-y-modos D1).
    let scope = meltemi_process::Scope::new().ok();
    let mut command = tokio::process::Command::new(binary);
    command
        .args(READ_ONLY)
        // No input: a client that decided to prompt must fail its deadline, not
        // borrow the daemon's stdin and wait forever.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = command.spawn().map_err(|e| {
        (
            IdentityUnavailable::ClientMissing,
            format!(
                "could not run `{binary} {}`: {e} — install your mesh client or set [identity] \
                 command to its path in your user config.toml",
                READ_ONLY.join(" ")
            ),
        )
    })?;
    if let (Some(scope), Some(pid)) = (scope.as_ref(), child.id()) {
        let _ = scope.adopt_pid(pid);
    }
    match tokio::time::timeout(BUDGET, child.wait_with_output()).await {
        Ok(Ok(output)) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Ok(output)) => Err((
            IdentityUnavailable::ClientMissing,
            format!(
                "`{binary} {}` failed: {} — check that your mesh client is working",
                READ_ONLY.join(" "),
                first_line(&String::from_utf8_lossy(&output.stderr))
            ),
        )),
        Ok(Err(e)) => Err((
            IdentityUnavailable::ClientMissing,
            format!("could not read the answer of `{binary}`: {e}"),
        )),
        Err(_) => {
            // The dropped future kills the child; the scope ends anything it
            // started.
            if let Some(scope) = scope.as_ref() {
                let _ = scope.end();
            }
            Err((
                IdentityUnavailable::ClientMissing,
                format!(
                    "`{binary} {}` did not answer within {}s — check that your mesh client is \
                     running",
                    READ_ONLY.join(" "),
                    BUDGET.as_secs()
                ),
            ))
        }
    }
}

/// The first non-empty line of a tool's complaint, for a one-line remedy.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output")
        .to_string()
}

/// Normalises the client's output into the model of the contract.
///
/// Pure, so every rule below is tested without launching anything.
///
/// # Errors
///
/// `NotLinked` when the client says this machine has no session; `ClientMissing`
/// when the answer cannot be read at all, because a half-parsed identity would be
/// worse than an admitted refusal.
fn interpret(
    json: &str,
    identity: &IdentityConfig,
) -> Result<IdentityStatusResult, (IdentityUnavailable, String)> {
    let raw: RawStatus = serde_json::from_str(json).map_err(|e| {
        (
            IdentityUnavailable::ClientMissing,
            format!(
                "could not read the answer of `{} {}`: {e} — is [identity] command really your \
                 mesh client?",
                client_binary(identity),
                READ_ONLY.join(" ")
            ),
        )
    })?;

    let state = raw.backend_state.as_deref().unwrap_or("");
    // Not linked, and said as one case however the client phrased it: `Self` is
    // null exactly when the node is not connected, so either signal is enough.
    if state != RUNNING || raw.this.is_none() {
        let said = if state.is_empty() { "no state" } else { state };
        return Err((
            IdentityUnavailable::NotLinked,
            format!(
                "your mesh client is installed and this machine has no session on the mesh (it \
                 says `{said}`): run the linking gesture to sign in"
            ),
        ));
    }
    let this = raw.this.expect("checked above");
    let Some(name) = non_empty(this.host_name.as_deref()) else {
        return Err((
            IdentityUnavailable::ClientMissing,
            "your mesh client answered without naming this machine, so there is nothing to \
             report: check that [identity] command really points at your mesh client"
                .to_string(),
        ));
    };

    // The login name lives in a map keyed by the id in DECIMAL AS A STRING,
    // while the peer carries it as a number. Looking the number up without
    // converting it finds nothing, and the user would come back empty against the
    // real client.
    let user = this
        .user_id
        .and_then(|id| {
            raw.users
                .as_ref()
                .and_then(|users| users.get(&id.to_string()))
        })
        .and_then(|user| non_empty(user.login_name.as_deref()));

    // The server the user configured, else what the mesh calls itself, else the
    // mesh's own name. One slot that answers "whose mesh is this?".
    let provider = identity
        .login_server
        .clone()
        .or_else(|| {
            raw.tailnet
                .as_ref()
                .and_then(|net| non_empty(net.name.as_deref()))
        })
        .or_else(|| Some(identity.mesh.clone()));

    let mut machines = vec![machine_of(&this, &name, true)];
    for peer in raw.peers.iter().flat_map(BTreeMap::values) {
        // Owned by somebody who shared it in: upstream hides these from its own
        // `status` by default, and they are not the user's machines.
        if peer.sharee_node {
            continue;
        }
        if let Some(peer_name) = non_empty(peer.host_name.as_deref()) {
            machines.push(machine_of(peer, &peer_name, false));
        }
    }
    // Deterministic, and in the order a person would want: this machine first,
    // then by name. The client's own map is keyed by node key, which is neither.
    machines.sort_by(|a, b| {
        b.this_machine
            .cmp(&a.this_machine)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(IdentityStatusResult {
        available: true,
        reason: None,
        remedy: None,
        user,
        provider,
        this: Some(name),
        machines,
    })
}

/// One machine of the model, with nothing from the client's output that is not
/// in the model: no addresses, no keys, no raw fields kept.
fn machine_of(peer: &RawPeer, name: &str, this_machine: bool) -> MeshMachine {
    MeshMachine {
        name: name.to_string(),
        presence: if peer.online {
            MeshPresence::Online
        } else {
            MeshPresence::Offline
        },
        system: non_empty(peer.os.as_deref()),
        last_seen: last_seen_of(peer),
        this_machine,
    }
}

/// When the mesh last saw a machine, and only when that means something.
///
/// The client's comment says the field is meaningful "only present if offline",
/// but the type carries no `omitempty`, so an online peer really does ship the
/// zero time. Reporting it would be reporting a wrong fact with a straight face.
fn last_seen_of(peer: &RawPeer) -> Option<String> {
    if peer.online {
        return None;
    }
    non_empty(peer.last_seen.as_deref()).filter(|seen| !seen.starts_with(ZERO_TIME))
}

/// A trimmed value, or nothing when the client sent an empty one.
fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Composes the gesture that links THIS machine to the user's mesh.
///
/// Pure composition: nothing is launched, nothing is read from the client, and
/// no secret is put in. A pre-authorisation key would be a secret, so none is
/// composed — the user signs in through their own browser, which is the whole
/// point of handing back a gesture instead of running one.
///
/// # Errors
///
/// Refuses when identity is not configured, which is the same refusal
/// `identity/status` gives and for the same reason: there is nothing to link to.
pub fn link(
    config: &Config,
    params: &IdentityLinkParams,
    this_machine: &str,
) -> Result<IdentityLinkResult, (IdentityUnavailable, String)> {
    let Some(identity) = config.identity.as_ref() else {
        return Err((
            IdentityUnavailable::NotConfigured,
            "identity is off: add an [identity] table with `mesh` to your user config.toml"
                .to_string(),
        ));
    };

    let binary = client_binary(identity);
    let hostname = params
        .hostname
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(this_machine);

    let mut parts = vec![binary, "up".to_string()];
    if let Some(server) = &identity.login_server {
        parts.push("--login-server".to_string());
        parts.push(server.clone());
    }
    parts.push("--hostname".to_string());
    parts.push(hostname.to_string());

    // The same words in both shells, quoted the way each one wants them. Not one
    // string shown twice: a user pasting a POSIX line into PowerShell is exactly
    // how a hostname with a space becomes two arguments.
    let posix = parts
        .iter()
        .map(|part| quote_posix(part))
        .collect::<Vec<_>>()
        .join(" ");
    let powershell = parts
        .iter()
        .map(|part| quote_powershell(part))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(IdentityLinkResult {
        // The plain reading, for a person looking at it rather than pasting it.
        command: parts.join(" "),
        // The same words as a vector, so a surface that runs the gesture runs
        // these exact arguments instead of re-splitting a string.
        args: parts.clone(),
        posix,
        powershell,
        // Signing in opens a browser, and a surface says so before the user is
        // surprised by one. An enumerated fact, not prose: the contract's strings
        // are English and the localisation lives in the surfaces (§11).
        opens_browser: true,
        login_server: identity.login_server.clone(),
    })
}

/// Quotes one argument for a POSIX shell, and only when it needs it.
fn quote_posix(part: &str) -> String {
    if !part.is_empty() && part.chars().all(safe_bare) {
        return part.to_string();
    }
    format!("'{}'", part.replace('\'', r"'\''"))
}

/// Quotes one argument for PowerShell, and only when it needs it.
fn quote_powershell(part: &str) -> String {
    if !part.is_empty() && part.chars().all(safe_bare) {
        return part.to_string();
    }
    format!("'{}'", part.replace('\'', "''"))
}

/// Whether a character can stand unquoted in either shell.
///
/// Deliberately narrow. The cost of quoting something that did not need it is a
/// pair of quotes; the cost of not quoting something that did is a command that
/// does something else.
fn safe_bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured(login_server: Option<&str>, command: Option<&str>) -> Config {
        Config {
            identity: Some(IdentityConfig {
                mesh: "tailscale".into(),
                login_server: login_server.map(str::to_string),
                command: command.map(str::to_string),
            }),
            ..Config::default()
        }
    }

    fn identity_of(config: &Config) -> &IdentityConfig {
        config.identity.as_ref().expect("configured")
    }

    /// Linked, three machines of the user and one shared in by somebody else.
    ///
    /// Written out rather than borrowed from the simulated client, because what
    /// this fixture pins is the four presence rules of the client's own types —
    /// if the simulated client drifted, these tests must still hold the line.
    const LINKED: &str = r#"{
      "BackendState": "Running",
      "Self": {
        "HostName": "mi-pc", "OS": "windows", "UserID": 180217,
        "Online": true, "LastSeen": "0001-01-01T00:00:00Z",
        "TailscaleIPs": ["100.64.0.1"],
        "PublicKey": "nodekey:1111"
      },
      "CurrentTailnet": { "Name": "mock.example", "MagicDNSEnabled": true },
      "Peer": {
        "nodekey:2222": {
          "HostName": "la-torre", "OS": "linux", "UserID": 180217,
          "Online": false, "LastSeen": "2026-09-30T18:21:05Z",
          "TailscaleIPs": ["100.64.0.3"]
        },
        "nodekey:3333": {
          "HostName": "el-portatil", "OS": "macOS", "UserID": 180217,
          "Online": true, "LastSeen": "0001-01-01T00:00:00Z",
          "TailscaleIPs": ["100.64.0.2"]
        },
        "nodekey:4444": {
          "HostName": "equipo-de-otra-persona", "OS": "linux", "UserID": 900001,
          "Online": true, "ShareeNode": true
        }
      },
      "User": {
        "180217": { "ID": 180217, "LoginName": "guillmar@mock.example" },
        "900001": { "ID": 900001, "LoginName": "otra-persona@mock.example" }
      }
    }"#;

    // Scenario: Quién soy, según la malla
    #[test]
    fn the_mesh_answers_who_i_am_and_what_it_calls_this_machine() {
        let config = configured(Some("https://headscale.askenaz.dev"), None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");

        assert!(answer.available);
        assert!(answer.reason.is_none(), "{answer:?}");
        assert_eq!(
            answer.user.as_deref(),
            Some("guillmar@mock.example"),
            "the login name is looked up through the id, which is a NUMBER on the peer and a \
             DECIMAL STRING as the map's key"
        );
        assert_eq!(answer.this.as_deref(), Some("mi-pc"));
        // The configured login server wins as the answer to "whose mesh is this".
        assert_eq!(
            answer.provider.as_deref(),
            Some("https://headscale.askenaz.dev")
        );
    }

    #[test]
    fn without_a_login_server_the_provider_is_what_the_mesh_calls_itself() {
        let config = configured(None, None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");
        assert_eq!(answer.provider.as_deref(), Some("mock.example"));
    }

    // Scenario: Las máquinas son los pares de la malla
    #[test]
    fn three_machines_with_their_presence_and_this_one_marked() {
        let config = configured(None, None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");

        let names: Vec<&str> = answer.machines.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["mi-pc", "el-portatil", "la-torre"],
            "this machine first, then by name — the client's map is keyed by node key, which is \
             neither deterministic nor useful"
        );
        assert_eq!(
            answer.machines.iter().filter(|m| m.this_machine).count(),
            1,
            "exactly one machine is this one"
        );
        assert!(answer.machines[0].this_machine);

        let offline = &answer.machines[2];
        assert_eq!(offline.presence, MeshPresence::Offline);
        assert_eq!(offline.system.as_deref(), Some("linux"));
        assert_eq!(
            offline.last_seen.as_deref(),
            Some("2026-09-30T18:21:05Z"),
            "the one that is away is the one with a last-seen"
        );
        assert_eq!(answer.machines[1].presence, MeshPresence::Online);
        assert_eq!(answer.machines[1].system.as_deref(), Some("macOS"));
    }

    #[test]
    fn an_online_machine_has_no_last_seen_because_the_zero_time_is_not_a_date() {
        // Trap 2: `LastSeen` carries no `omitempty`, so an online peer really
        // does ship `0001-01-01T00:00:00Z`. Reporting it would tell the user
        // their machine was last seen in the year 1.
        let config = configured(None, None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");
        for machine in answer
            .machines
            .iter()
            .filter(|m| m.presence == MeshPresence::Online)
        {
            assert!(
                machine.last_seen.is_none(),
                "{} is online and must not carry a last-seen: {:?}",
                machine.name,
                machine.last_seen
            );
        }
    }

    #[test]
    fn a_machine_somebody_else_shared_in_is_not_one_of_mine() {
        // Trap 4: upstream says a `ShareeNode` "should be hidden by `tailscale
        // status` ... by default". Listing it would put a colleague's machine in
        // the user's own list.
        let config = configured(None, None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");
        assert!(
            !answer
                .machines
                .iter()
                .any(|m| m.name == "equipo-de-otra-persona"),
            "{:?}",
            answer.machines
        );
    }

    // Scenario: Ningún token entra ni sale del daemon
    #[test]
    fn nothing_outside_the_model_survives_the_parser() {
        // The client's output CONTAINS public keys and addresses — the peer map is
        // indexed by key. The rule is not "do not read them" but that nothing
        // outside the model leaves, so this asserts by absence over the whole
        // serialised answer.
        let config = configured(None, None);
        let answer = interpret(LINKED, identity_of(&config)).expect("linked");
        let whole = serde_json::to_string(&answer).expect("serializes");
        for leaked in ["nodekey", "100.64.0", "TailscaleIPs", "PublicKey", "180217"] {
            assert!(
                !whole.contains(leaked),
                "`{leaked}` must not survive the parser: {whole}"
            );
        }
    }

    // Scenario: Cliente presente, equipo sin enlazar
    #[test]
    fn a_client_that_is_there_and_not_signed_in_says_so_and_not_missing() {
        // Trap 1: `Self` and `CurrentTailnet` are null when the node is not
        // connected, which is exactly this case — so a parser that assumed them
        // would panic on the scenario it most needs to answer.
        let config = configured(None, None);
        let unlinked = r#"{
          "BackendState": "NeedsLogin",
          "Self": null,
          "CurrentTailnet": null,
          "Peer": null,
          "User": null
        }"#;
        let (reason, remedy) = interpret(unlinked, identity_of(&config)).expect_err("not linked");
        assert_eq!(
            reason,
            IdentityUnavailable::NotLinked,
            "distinct from a client that is not there at all"
        );
        assert!(remedy.contains("no session"), "{remedy}");
        assert!(
            remedy.contains("linking gesture"),
            "the remedy offers the gesture: {remedy}"
        );
        assert!(
            remedy.contains("NeedsLogin"),
            "it says what the client said"
        );
    }

    #[test]
    fn every_state_that_is_not_running_is_the_same_answer() {
        let config = configured(None, None);
        for state in ["NoState", "NeedsMachineAuth", "Stopped", "Starting"] {
            let json = format!(r#"{{"BackendState": "{state}", "Self": {{"HostName": "mi-pc"}}}}"#);
            let (reason, _) = interpret(&json, identity_of(&config)).expect_err("{state}");
            assert_eq!(reason, IdentityUnavailable::NotLinked, "{state}");
        }
    }

    #[test]
    fn a_running_client_that_names_no_self_is_refused_and_does_not_panic() {
        // Found by neutering the `Self` guard and watching nothing go red: the
        // state check alone covers the ordinary unlinked case, so without this
        // test the guard looked redundant. It is not. The two signals can
        // disagree — a client coming up can say `Running` before it has a
        // profile — and the honest answer is a refusal. Unwrapping instead would
        // panic inside a request handler, which is strictly worse than any
        // refusal.
        let config = configured(None, None);
        let disagreeing = r#"{"BackendState": "Running", "Self": null}"#;
        let (reason, remedy) = interpret(disagreeing, identity_of(&config)).expect_err("no self");
        assert_eq!(reason, IdentityUnavailable::NotLinked);
        assert!(remedy.contains("no session"), "{remedy}");
    }

    #[test]
    fn a_mesh_of_one_machine_is_a_mesh() {
        // An empty peer list is not an error, and the contract says so.
        let config = configured(None, None);
        let alone = r#"{
          "BackendState": "Running",
          "Self": { "HostName": "mi-pc", "OS": "windows", "UserID": 7, "Online": true },
          "CurrentTailnet": { "Name": "mock.example" },
          "Peer": {},
          "User": { "7": { "LoginName": "solo@mock.example" } }
        }"#;
        let answer = interpret(alone, identity_of(&config)).expect("linked");
        assert!(answer.available);
        assert_eq!(answer.machines.len(), 1);
        assert!(answer.machines[0].this_machine);
    }

    #[test]
    fn an_answer_that_cannot_be_read_is_refused_and_never_half_parsed() {
        let config = configured(None, Some("not-really-tailscale"));
        let (reason, remedy) = interpret(
            "this is not the json you are looking for",
            identity_of(&config),
        )
        .expect_err("garbage");
        assert_eq!(reason, IdentityUnavailable::ClientMissing);
        assert!(
            remedy.contains("not-really-tailscale status --json"),
            "the remedy names what was run: {remedy}"
        );
        assert!(remedy.contains("[identity] command"), "{remedy}");
    }

    #[test]
    fn a_field_the_client_adds_tomorrow_is_ignored_not_refused() {
        // The daemon reads a tool it does not version. A client update that adds
        // a field must not turn into a refusal.
        let config = configured(None, None);
        let future = r#"{
          "BackendState": "Running",
          "SomethingNewIn2027": { "nested": [1, 2, 3] },
          "Self": { "HostName": "mi-pc", "Online": true, "UserID": 7, "WhatIsThis": true },
          "CurrentTailnet": { "Name": "mock.example" },
          "Peer": {},
          "User": { "7": { "LoginName": "solo@mock.example", "Groups": ["group:eng"] } }
        }"#;
        let answer = interpret(future, identity_of(&config)).expect("tolerant");
        assert_eq!(answer.user.as_deref(), Some("solo@mock.example"));
    }

    #[test]
    fn a_client_that_does_not_name_this_machine_is_not_a_linked_identity() {
        let config = configured(None, None);
        let nameless = r#"{"BackendState": "Running", "Self": {"HostName": "   "}}"#;
        let (reason, remedy) = interpret(nameless, identity_of(&config)).expect_err("nameless");
        assert_eq!(reason, IdentityUnavailable::ClientMissing);
        assert!(remedy.contains("naming this machine"), "{remedy}");
    }

    // Scenario: Sin cliente de malla, rehúso con remedio
    #[tokio::test]
    async fn without_an_identity_table_the_answer_is_an_answer_with_a_remedy() {
        let answer = status(&Config::default()).await;
        assert!(!answer.available);
        assert_eq!(answer.reason, Some(IdentityUnavailable::NotConfigured));
        let remedy = answer.remedy.expect("a remedy");
        assert!(remedy.contains("[identity]"), "{remedy}");
        assert!(remedy.contains("user config.toml"), "{remedy}");
        assert!(answer.machines.is_empty());
        assert!(answer.user.is_none());
    }

    #[tokio::test]
    async fn a_binary_that_is_not_there_says_install_it_and_names_the_read() {
        let config = configured(None, Some("meltemi-no-such-mesh-client"));
        let answer = status(&config).await;
        assert!(!answer.available);
        assert_eq!(answer.reason, Some(IdentityUnavailable::ClientMissing));
        let remedy = answer.remedy.expect("a remedy");
        assert!(
            remedy.contains("meltemi-no-such-mesh-client status --json"),
            "the remedy shows the read-only verb that was attempted: {remedy}"
        );
        assert!(remedy.contains("install"), "{remedy}");
    }

    #[test]
    fn the_only_verb_is_read_only() {
        // The promise of §2 as an assertion rather than a comment: this is the one
        // place a future change would edit to make the daemon run a login.
        assert_eq!(READ_ONLY, ["status", "--json"]);
        for forbidden in ["up", "login", "logout", "set", "down"] {
            assert!(
                !READ_ONLY.contains(&forbidden),
                "`{forbidden}` must never be invoked by the daemon"
            );
        }
    }

    // Scenario: El gesto de vínculo se imprime, no se ejecuta
    #[test]
    fn the_gesture_names_the_mesh_the_server_and_this_machine() {
        let config = configured(Some("https://headscale.askenaz.dev"), None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");

        assert_eq!(
            gesture.command,
            "tailscale up --login-server https://headscale.askenaz.dev --hostname mi-pc"
        );
        assert_eq!(gesture.posix, gesture.command, "nothing needed quoting");
        assert_eq!(gesture.powershell, gesture.command);
        assert!(
            gesture.opens_browser,
            "a surface warns before a browser appears"
        );
        assert_eq!(
            gesture.login_server.as_deref(),
            Some("https://headscale.askenaz.dev")
        );
    }

    #[test]
    fn without_a_login_server_the_gesture_is_the_meshs_own_default() {
        // Somebody on the mesh's hosted service names no server, and the gesture
        // must not invent one.
        let config = configured(None, None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");
        assert_eq!(gesture.command, "tailscale up --hostname mi-pc");
        assert!(gesture.login_server.is_none());
    }

    #[test]
    fn a_named_hostname_wins_over_this_machines_name() {
        let config = configured(None, None);
        let asked = IdentityLinkParams {
            hostname: Some("el-portatil".into()),
        };
        let gesture = link(&config, &asked, "mi-pc").expect("configured");
        assert!(
            gesture.command.ends_with("--hostname el-portatil"),
            "{gesture:?}"
        );

        // And a blank one falls back rather than asking the mesh for an empty
        // name.
        let blank = IdentityLinkParams {
            hostname: Some("   ".into()),
        };
        let gesture = link(&config, &blank, "mi-pc").expect("configured");
        assert!(gesture.command.ends_with("--hostname mi-pc"), "{gesture:?}");
    }

    #[test]
    fn a_name_with_a_space_is_quoted_for_each_shell_and_not_shared() {
        // The failure this prevents: a POSIX line pasted into PowerShell where a
        // hostname with a space becomes two arguments.
        let config = configured(None, Some("C:/Program Files/Tailscale/tailscale.exe"));
        let asked = IdentityLinkParams {
            hostname: Some("el portatil".into()),
        };
        let gesture = link(&config, &asked, "mi-pc").expect("configured");

        assert!(
            gesture
                .posix
                .contains("'C:/Program Files/Tailscale/tailscale.exe'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.posix.ends_with("'el portatil'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.powershell.ends_with("'el portatil'"),
            "{}",
            gesture.powershell
        );
        // A quote inside a name is escaped the way each shell wants, which is not
        // the same way.
        let tricky = IdentityLinkParams {
            hostname: Some("it's-mine".into()),
        };
        let gesture = link(&config, &tricky, "mi-pc").expect("configured");
        assert!(
            gesture.posix.contains(r"'it'\''s-mine'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.powershell.contains("'it''s-mine'"),
            "{}",
            gesture.powershell
        );
    }

    // Scenario: El gesto no lleva secretos
    #[test]
    fn the_gesture_carries_no_secret_of_any_kind() {
        // A pre-authorisation key would be a secret, so none is composed: the
        // user signs in through their own browser, which is the whole reason a
        // gesture is handed back instead of being run.
        let config = configured(Some("https://headscale.askenaz.dev"), None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");
        let whole = format!(
            "{} {} {}",
            gesture.command, gesture.posix, gesture.powershell
        );
        for secret in ["--authkey", "--auth-key", "tskey-", "token", "password"] {
            assert!(
                !whole.to_lowercase().contains(secret),
                "`{secret}` must never be composed into the gesture: {whole}"
            );
        }
    }

    #[test]
    fn without_identity_configured_there_is_nothing_to_link_to() {
        let config = Config::default();
        let (reason, remedy) =
            link(&config, &IdentityLinkParams::default(), "mi-pc").expect_err("off");
        assert_eq!(reason, IdentityUnavailable::NotConfigured);
        assert!(remedy.contains("[identity]"), "{remedy}");
        assert!(remedy.contains("user config.toml"), "{remedy}");
    }
}
