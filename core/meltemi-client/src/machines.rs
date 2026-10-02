// SPDX-License-Identifier: Apache-2.0

//! The machines this user has, by name (maquinas-remotas).
//!
//! A machine is a name and an endpoint, and an endpoint is either this
//! platform's own default or a command that speaks the protocol over its stdio
//! — which is the whole of remote access already built by `via-ssh-del-cliente`.
//! Nothing here reaches a network, starts a daemon or holds a key.
//!
//! **User scope, and no nuance about it.** The list is read from the user's
//! configuration directory and from nowhere else, by the same path and with the
//! same shape as [`crate::paths::configured_endpoint`]: an endpoint can name a
//! command to run, so a repository able to set one would be a repository that
//! runs a command on your machine by the act of opening it. Cloning is an
//! ordinary thing to do and must not be a security decision.
//!
//! **No key material, by construction.** Not a key path, not a passphrase, not
//! a port, not a user. Those live in the user's own `~/.ssh/config` and in their
//! key agent, where `ssh` already reads them; a field here for a key path would
//! be the first step towards reading one, and §2 says Meltemi does not touch
//! anybody's credentials. The parser refuses such a field loudly rather than
//! ignoring it, because somebody who wrote it there is about to wonder why it
//! did nothing.

use std::path::Path;
use std::time::{Duration, Instant};

/// The endpoint value that means "this platform's own default".
///
/// Sugar with a reason: a named pipe with the user's SID in it is not something
/// to retype by hand, and the first person to get it subtly wrong would get a
/// client that quietly talks to nothing.
pub const LOCAL: &str = "local";

/// Fields that would be key material, and are refused with their remedy.
///
/// Refused rather than ignored: the failure of ignoring them is a user who
/// believes Meltemi is using their key and cannot see why it is not.
const REFUSED: &[(&str, &str)] = &[
    ("identity-file", "your ~/.ssh/config, with `IdentityFile`"),
    ("identityfile", "your ~/.ssh/config, with `IdentityFile`"),
    (
        "password",
        "your key agent; Meltemi never handles passwords",
    ),
    (
        "passphrase",
        "your key agent; Meltemi never handles passphrases",
    ),
    ("port", "your ~/.ssh/config, with `Port`"),
    ("user", "your ~/.ssh/config, with `User`"),
    ("key", "your ~/.ssh/config and your key agent"),
];

/// One machine the user named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// What the user calls it.
    pub name: String,
    /// What the user wrote: `local`, a socket path, or an `exec:` command.
    pub configured: String,
    /// What to connect to: `configured`, with `local` resolved.
    pub endpoint: String,
}

impl Machine {
    /// Whether this machine is reached by running a command.
    #[must_use]
    pub fn is_remote(&self) -> bool {
        crate::exec::is_exec(&self.endpoint)
    }

    /// The short form of the command, for chrome that names the way in.
    #[must_use]
    pub fn via(&self) -> Option<String> {
        crate::exec::shown(&self.endpoint)
    }
}

/// The machines the user configured, with whatever could not be read.
///
/// Both halves travel together on purpose: a list that silently dropped a bad
/// entry would leave somebody staring at a machine that is in their file and not
/// on their screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Machines {
    pub machines: Vec<Machine>,
    /// What was refused, and what to do about it. English, like every remedy.
    pub diagnostics: Vec<String>,
}

impl Machines {
    /// The machine whose endpoint is `endpoint`, if the user named one.
    #[must_use]
    pub fn named(&self, endpoint: &str) -> Option<&Machine> {
        self.machines
            .iter()
            .find(|machine| machine.endpoint == endpoint)
    }

    /// The machine called `name`, if there is one.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&Machine> {
        let name = name.trim();
        self.machines
            .iter()
            .find(|machine| machine.name.eq_ignore_ascii_case(name))
    }

    /// The names, for a refusal that says what the user could have meant.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.machines
            .iter()
            .map(|machine| machine.name.as_str())
            .collect()
    }
}

/// Reads `[[client.machines]]` from the user's configuration directory.
///
/// Takes the configuration directory and no project root, so there is no path by
/// which a project could reach it.
#[must_use]
pub fn read(config_dir: &Path) -> Machines {
    let Ok(raw) = std::fs::read_to_string(config_dir.join("config.toml")) else {
        return Machines::default();
    };
    parse(&raw)
}

/// Parses the machines out of a configuration document.
///
/// Separate from the read so every rule below is tested without a filesystem.
#[must_use]
pub fn parse(raw: &str) -> Machines {
    let mut out = Machines::default();
    // A `Table`, not a `Value`: parsing a whole document as `Value` compiles and
    // then quietly answers nothing, which cost a failing test once already.
    let Ok(parsed) = raw.parse::<toml::Table>() else {
        out.diagnostics
            .push("config.toml could not be parsed, so no machines were read".into());
        return out;
    };
    let Some(list) = parsed
        .get("client")
        .and_then(|client| client.get("machines"))
    else {
        return out;
    };
    let Some(entries) = list.as_array() else {
        out.diagnostics.push(
            "[client] machines must be a list of tables: write `[[client.machines]]` for each one"
                .into(),
        );
        return out;
    };

    for (index, entry) in entries.iter().enumerate() {
        let Some(table) = entry.as_table() else {
            out.diagnostics.push(format!(
                "machine #{} is not a table and was skipped",
                index + 1
            ));
            continue;
        };
        // Key material is refused before anything else is read, so a file that
        // carries it never half-works.
        let mut refused = false;
        for (field, lives_in) in REFUSED {
            if table.keys().any(|key| key.eq_ignore_ascii_case(field)) {
                let name = table
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("(unnamed)");
                out.diagnostics.push(format!(
                    "machine `{name}` was skipped: `{field}` is key material and Meltemi never \
                     handles it — that belongs in {lives_in}"
                ));
                refused = true;
                break;
            }
        }
        if refused {
            continue;
        }

        let name = table
            .get("name")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let Some(name) = name else {
            out.diagnostics.push(format!(
                "machine #{} has no `name` and was skipped",
                index + 1
            ));
            continue;
        };
        let configured = table
            .get("endpoint")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|endpoint| !endpoint.is_empty());
        let Some(configured) = configured else {
            out.diagnostics.push(format!(
                "machine `{name}` has no `endpoint` and was skipped: write `local`, a socket path, \
                 or `exec:<command>`"
            ));
            continue;
        };
        if out
            .machines
            .iter()
            .any(|machine| machine.name.eq_ignore_ascii_case(name))
        {
            out.diagnostics.push(format!(
                "machine `{name}` is named twice; the first one was kept"
            ));
            continue;
        }
        let endpoint = if configured.eq_ignore_ascii_case(LOCAL) {
            crate::paths::default_endpoint()
        } else {
            configured.to_string()
        };
        out.machines.push(Machine {
            name: name.to_string(),
            configured: configured.to_string(),
            endpoint,
        });
    }
    out
}

/// How long a connection test gets before it is called a failure.
///
/// A test the user asked for and is waiting on: long enough for `ssh` to a
/// sleepy host, short enough that an unreachable one answers rather than hangs.
pub const PROBE_BUDGET: Duration = Duration::from_secs(20);

/// What a connection test found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The daemon's version, when it answered.
    pub version: Option<String>,
    /// How long the whole thing took, in milliseconds.
    pub millis: u64,
    /// Why it did not answer, when it did not.
    pub detail: Option<String>,
}

impl Probe {
    /// Whether the daemon on the other side answered.
    #[must_use]
    pub fn answered(&self) -> bool {
        self.version.is_some()
    }
}

/// Opens `endpoint`, says hello, notes the version and the time, and closes.
///
/// Never starts anything. `bootstrap::may_start` already refuses to start a
/// daemon for an `exec:` endpoint unconditionally, and this does not go through
/// it at all: testing a machine that is switched off must FAIL, which is the
/// true answer, and not turn into a surprise remote start.
///
/// # Errors
///
/// Never: a machine that does not answer is a result, not an error of the
/// client, and the reason travels in `detail`.
pub async fn probe(endpoint: &str) -> Probe {
    let started = Instant::now();
    let outcome = tokio::time::timeout(PROBE_BUDGET, say_hello(endpoint)).await;
    let millis = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match outcome {
        Ok(Ok(version)) => Probe {
            version: Some(version),
            millis,
            detail: None,
        },
        Ok(Err(detail)) => Probe {
            version: None,
            millis,
            detail: Some(detail),
        },
        Err(_) => Probe {
            version: None,
            millis,
            detail: Some(format!("no answer within {}s", PROBE_BUDGET.as_secs())),
        },
    }
}

/// Connects, initializes, reads the daemon's version and drops the connection.
async fn say_hello(endpoint: &str) -> Result<String, String> {
    use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

    let stream = crate::transport::connect(endpoint)
        .await
        .map_err(|e| e.to_string())?;
    let (peer, _incoming) = crate::rpc::Peer::start(stream);
    let answer = peer
        .request(
            methods::INITIALIZE,
            &InitializeParams {
                protocol_version: PROTOCOL_VERSION,
                client: PeerInfo {
                    name: "meltemi-probe".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                },
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    // The connection is dropped here, which is the point: a test leaves nothing
    // open and does not become a second client.
    Ok(answer["daemonVersion"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: La lista no lleva material de claves
    #[test]
    fn a_machine_carrying_key_material_is_refused_and_told_where_that_lives() {
        // The failure this prevents is not a leak — Meltemi would not read the
        // file — but a user who believes their key is in use and cannot see why
        // nothing happens. Refused loudly, with the place it belongs.
        for (field, value) in [
            ("identity-file", "\"~/.ssh/id_ed25519\""),
            ("passphrase", "\"hunter2\""),
            ("port", "2222"),
            ("user", "\"guillermo\""),
        ] {
            let read = parse(&format!(
                "[[client.machines]]\nname = \"la-torre\"\nendpoint = \"exec:ssh t meltemi \
                 bridge\"\n{field} = {value}\n"
            ));
            assert!(
                read.machines.is_empty(),
                "`{field}` must not produce a machine: {read:?}"
            );
            let said = read.diagnostics.join(" ");
            assert!(said.contains(field), "{field}: {said}");
            assert!(
                said.contains("ssh") || said.contains("key agent"),
                "{field}: the remedy names where it lives: {said}"
            );
        }
    }

    // Scenario: Las máquinas se leen del usuario y nunca de un repositorio
    #[test]
    fn the_list_is_read_from_a_directory_and_never_from_a_project() {
        // The guarantee is structural: `read` takes a configuration directory and
        // nothing else, so there is no argument through which a project root
        // could arrive. This pins the shape, which is what would have to change
        // for the rule to break.
        let dir = std::env::temp_dir().join(format!("meltemi-machines-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(
            dir.join("config.toml"),
            "[[client.machines]]\nname = \"esta\"\nendpoint = \"local\"\n",
        )
        .expect("the file");
        let listed = read(&dir);
        assert_eq!(listed.names(), vec!["esta"]);
        assert!(listed.diagnostics.is_empty(), "{listed:?}");

        // A directory with no file at all is not an error: most people have no
        // machines, and that is the ordinary case rather than a broken one.
        let empty = std::env::temp_dir().join(format!("meltemi-nomach-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).expect("a directory");
        assert_eq!(read(&empty), Machines::default());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn local_resolves_to_this_platforms_own_endpoint_and_anything_else_is_verbatim() {
        let read = parse(
            "[[client.machines]]\nname = \"esta\"\nendpoint = \"local\"\n\n\
             [[client.machines]]\nname = \"la-torre\"\nendpoint = \"exec:ssh la-torre meltemi \
             bridge\"\n",
        );
        assert_eq!(read.names(), vec!["esta", "la-torre"]);
        // `local` is sugar for the platform default, which nobody should retype.
        assert_eq!(read.machines[0].endpoint, crate::paths::default_endpoint());
        assert_eq!(read.machines[0].configured, "local");
        assert!(!read.machines[0].is_remote());
        // Anything else travels exactly as written.
        assert_eq!(
            read.machines[1].endpoint,
            "exec:ssh la-torre meltemi bridge"
        );
        assert!(read.machines[1].is_remote());
        assert_eq!(read.machines[1].via().as_deref(), Some("ssh la-torre"));
    }

    #[test]
    fn every_unusable_entry_is_skipped_with_a_reason_and_the_rest_survive() {
        // The house discipline: a bad entry must not cost the good ones, and it
        // must say what to do.
        let read = parse(
            "[[client.machines]]\nname = \"buena\"\nendpoint = \"local\"\n\n\
             [[client.machines]]\nendpoint = \"local\"\n\n\
             [[client.machines]]\nname = \"sin-endpoint\"\n\n\
             [[client.machines]]\nname = \"BUENA\"\nendpoint = \"local\"\n",
        );
        assert_eq!(read.names(), vec!["buena"], "{read:?}");
        let said = read.diagnostics.join(" | ");
        assert!(said.contains("has no `name`"), "{said}");
        assert!(said.contains("has no `endpoint`"), "{said}");
        assert!(
            said.contains("named twice"),
            "a repeated name keeps the first and says so: {said}"
        );
    }

    #[test]
    fn a_file_that_says_nothing_about_machines_is_not_a_complaint() {
        assert_eq!(parse("[agent]\nname = \"x\"\n"), Machines::default());
        assert_eq!(parse(""), Machines::default());
        // But a file that cannot be parsed at all is worth saying.
        let broken = parse("[[client.machines]\nname =");
        assert!(broken.machines.is_empty());
        assert!(!broken.diagnostics.is_empty(), "{broken:?}");
    }

    #[test]
    fn a_machine_is_found_by_name_without_minding_case_and_by_endpoint() {
        let read = parse(
            "[[client.machines]]\nname = \"La-Torre\"\nendpoint = \"exec:ssh t meltemi bridge\"\n",
        );
        assert!(read.by_name("la-torre").is_some());
        assert!(read.by_name("  LA-TORRE  ").is_some());
        assert!(read.by_name("otra").is_none());
        assert_eq!(
            read.named("exec:ssh t meltemi bridge")
                .map(|m| m.name.as_str()),
            Some("La-Torre")
        );
    }

    // Scenario: Probar una conexión no arranca nada
    #[tokio::test]
    async fn probing_a_machine_that_is_not_there_fails_rather_than_starting_one() {
        // An endpoint nothing is listening on. The answer must be "it did not
        // answer", with the reason — never a daemon started on the user's behalf.
        #[cfg(windows)]
        let dead = format!(r"\\.\pipe\meltemi-no-such-daemon-{}", std::process::id());
        #[cfg(unix)]
        let dead = std::env::temp_dir()
            .join(format!(
                "meltemi-no-such-daemon-{}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned();

        let probe = probe(&dead).await;
        assert!(!probe.answered(), "{probe:?}");
        assert!(probe.version.is_none());
        assert!(probe.detail.is_some(), "the reason travels: {probe:?}");
        // And it answered rather than hanging on the budget.
        assert!(probe.millis < PROBE_BUDGET.as_millis() as u64, "{probe:?}");
    }

    #[tokio::test]
    async fn probing_a_command_that_says_nothing_runs_out_of_its_budget_and_says_so() {
        // The budget is a constant somebody could delete; this is what makes it a
        // behaviour. A command that speaks nothing never completes `initialize`.
        let sleeper = if cfg!(windows) {
            "exec:cmd /c pause"
        } else {
            "exec:sh -c 'sleep 120'"
        };
        // Shortened by running the probe under a tighter outer limit: what is
        // under test is that it ENDS, not that it waits the whole budget.
        let probe = tokio::time::timeout(Duration::from_secs(30), probe(sleeper)).await;
        let probe = probe.expect("the probe ends on its own");
        assert!(!probe.answered(), "{probe:?}");
        assert!(probe.detail.is_some(), "{probe:?}");
    }
}
