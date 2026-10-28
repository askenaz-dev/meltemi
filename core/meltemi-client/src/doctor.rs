// SPDX-License-Identifier: Apache-2.0

//! `meltemi doctor`: what is installed, what answers and what is missing, in
//! one report the three surfaces share (meltemi-doctor design D1).
//!
//! The report is composed here, in the client, from two kinds of facts: what
//! this machine shows ([`Local::gather`]) and what the daemon answers
//! ([`ask_daemon`]). No RPC is added for it, so the daemon gains nothing and no
//! parity duty is born; every surface offers it because it is useful.
//!
//! Four things this module never does, each with its reason in the design:
//! start a daemon (D2: a diagnosis that starts one changes what it diagnoses),
//! run an agent's binary (D5: fleet detection stays filesystem probing), wait
//! without a bound (D4: "not checked" is a valid answer), and send the report
//! anywhere (D8).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use meltemi_proto::{FleetAgent, FleetInstallState, FleetListResult};
use serde::{Deserialize, Serialize};

/// How long a helper command (`git`, `ssh`) may take before it is "not checked".
pub const COMMAND_BUDGET: Duration = Duration::from_secs(3);

/// How long the daemon gets, in total, to answer the three questions.
pub const DAEMON_BUDGET: Duration = Duration::from_secs(5);

/// How much of the daemon's newest log is read, from its end.
const LOG_TAIL_BYTES: u64 = 64 * 1024;

/// How many error lines of that log the report keeps.
const LOG_LINES: usize = 5;

/// The language a surface words the report in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
}

/// What a row found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// As it should be.
    Ok,
    /// Works, but something deserves a look.
    Warn,
    /// Does not work: the remedy says what to do.
    Fail,
    /// Could not be checked, and the row says why.
    Unchecked,
}

impl Status {
    /// The status's symbol, Unicode or its ASCII twin. Never alone: a surface
    /// shows it next to [`Status::word`], so meaning never rests on the symbol.
    #[must_use]
    pub fn glyph(self, unicode: bool) -> &'static str {
        match (self, unicode) {
            (Self::Ok, true) => "✓",
            (Self::Ok, false) => "+",
            (Self::Warn, true) => "!",
            (Self::Warn, false) => "!",
            (Self::Fail, true) => "✕",
            (Self::Fail, false) => "x",
            (Self::Unchecked, true) => "○",
            (Self::Unchecked, false) => "-",
        }
    }

    /// The status in words.
    #[must_use]
    pub fn word(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Ok, Lang::En) => "ok",
            (Self::Ok, Lang::Es) => "bien",
            (Self::Warn, Lang::En) => "warning",
            (Self::Warn, Lang::Es) => "aviso",
            (Self::Fail, Lang::En) => "failing",
            (Self::Fail, Lang::Es) => "falla",
            (Self::Unchecked, Lang::En) => "not checked",
            (Self::Unchecked, Lang::Es) => "sin comprobar",
        }
    }
}

/// A row's words in one language: what was checked, what was found, and what
/// to do about it when there is something to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    pub title: String,
    pub found: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

/// One row of the report (design D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// Stable: `client`, `daemon`, `versions`, `endpoint`, `data-dir`,
    /// `config-dir`, `user-config`, `agent:<id>`, `agents-absent`, `fleet`,
    /// `git`, `ssh`, `sshd`, `daemon-log`.
    pub id: String,
    pub status: Status,
    /// What the row found, as data (versions, paths, counts).
    pub facts: BTreeMap<String, String>,
    pub en: Words,
    pub es: Words,
}

impl Row {
    /// The row's words in `lang`.
    #[must_use]
    pub fn words(&self, lang: Lang) -> &Words {
        match lang {
            Lang::En => &self.en,
            Lang::Es => &self.es,
        }
    }
}

/// The report: the rows in the order a person reads them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub rows: Vec<Row>,
}

impl Report {
    /// Whether any row fails.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.rows.iter().any(|row| row.status == Status::Fail)
    }

    /// The report as text: one line per row with its symbol and its word, and
    /// the remedy indented under it.
    #[must_use]
    pub fn text(&self, lang: Lang, unicode: bool) -> String {
        let mut out = String::new();
        for row in &self.rows {
            let words = row.words(lang);
            out.push_str(&format!(
                "{} {:<13} {}: {}\n",
                row.status.glyph(unicode),
                row.status.word(lang),
                words.title,
                words.found
            ));
            if let Some(remedy) = &words.remedy {
                let arrow = if unicode { "→" } else { "->" };
                out.push_str(&format!("                {arrow} {remedy}\n"));
            }
        }
        out
    }

    /// The report as JSON: each row's id, status, facts and English words, and
    /// how many rows fail.
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "rows": self.rows.iter().map(|row| serde_json::json!({
                "id": row.id,
                "status": row.status,
                "facts": row.facts,
                "title": row.en.title,
                "found": row.en.found,
                "remedy": row.en.remedy,
            })).collect::<Vec<_>>(),
            "failing": self.rows.iter().filter(|row| row.status == Status::Fail).count(),
        })
    }
}

/// What a folder turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirState {
    /// It exists and a file could be written in it.
    Writable,
    /// It does not exist yet: the daemon creates it on first use.
    Absent,
    /// It exists, and nothing can be written in it.
    ReadOnly,
    /// Something that is not a folder is in its place.
    NotADirectory,
}

/// What the user's configuration file turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigState {
    /// There is none, which is a valid configuration.
    Absent,
    /// It reads, and its machines are all usable.
    Valid,
    /// It does not parse: the parser's words.
    Invalid(String),
    /// It parses, and some entries were skipped: why, one each.
    Skipped(Vec<String>),
}

/// What running a helper command found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolState {
    /// Its first line of output, which names its version.
    Found(String),
    /// Not on the `PATH`.
    Missing,
    /// It did not answer within [`COMMAND_BUDGET`], or could not be run.
    Unchecked(String),
}

/// What this machine shows, gathered without asking the daemon anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub client_version: String,
    pub endpoint: String,
    pub data_dir: (PathBuf, DirState),
    pub config_dir: (PathBuf, DirState),
    pub config: ConfigState,
    pub git: ToolState,
    pub ssh: ToolState,
    /// Windows only: whether the OpenSSH server is installed. `None` elsewhere.
    pub sshd: Option<bool>,
    /// The newest daemon log and its last error lines, when there is a log.
    pub daemon_log: Option<(PathBuf, Vec<String>)>,
}

impl Local {
    /// Gathers the facts of this machine, for the endpoint this user resolves
    /// to. Every step is bounded (design D4); it blocks, so an async caller
    /// runs it on a blocking thread.
    #[must_use]
    pub fn gather() -> Self {
        Self::gather_for(&crate::paths::endpoint())
    }

    /// The same, for the endpoint the caller is talking to.
    #[must_use]
    pub fn gather_for(endpoint: &str) -> Self {
        let data = crate::paths::data_dir();
        let config = crate::paths::config_dir();
        Self {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            endpoint: endpoint.to_string(),
            data_dir: (data.clone(), dir_state(&data)),
            config_dir: (config.clone(), dir_state(&config)),
            config: config_state(&config),
            git: tool("git", &["--version"]),
            ssh: tool("ssh", &["-V"]),
            sshd: sshd_installed(),
            daemon_log: daemon_log(&data.join("logs")),
        }
    }
}

/// What the daemon answered, or why it did not. Deserialisable because the
/// desktop asks the daemon over its own connection and hands the answers in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Daemon {
    /// Whether anything answered at the endpoint.
    pub reached: bool,
    /// Why it did not, in the transport's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The version `status` reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// What `fleet/list` reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<FleetListResult>,
}

/// Asks the daemon at `endpoint` for its version and its fleet, **without
/// starting one** (design D2), within [`DAEMON_BUDGET`].
pub async fn ask_daemon(endpoint: &str) -> Daemon {
    match tokio::time::timeout(DAEMON_BUDGET, ask(endpoint)).await {
        Ok(Ok(daemon)) => daemon,
        Ok(Err(detail)) => Daemon {
            reached: false,
            detail: Some(detail),
            ..Daemon::default()
        },
        Err(_) => Daemon {
            reached: false,
            detail: Some(format!("no answer within {}s", DAEMON_BUDGET.as_secs())),
            ..Daemon::default()
        },
    }
}

async fn ask(endpoint: &str) -> Result<Daemon, String> {
    use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

    let stream = crate::transport::connect(endpoint)
        .await
        .map_err(|e| e.to_string())?;
    let (peer, _incoming) = crate::rpc::Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi-doctor".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    let status = peer
        .request(methods::STATUS, &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())?;
    // The fleet is asked separately: a daemon that answers its status and not
    // its fleet is still reached, and the report says what it could not read.
    let fleet = peer
        .request(methods::FLEET_LIST, &serde_json::json!({}))
        .await
        .ok()
        .and_then(|fleet| serde_json::from_value::<FleetListResult>(fleet).ok());
    peer.close();
    Ok(Daemon {
        reached: true,
        detail: None,
        version: status["daemonVersion"].as_str().map(str::to_string),
        fleet,
    })
}

/// The report, from what this machine shows and what the daemon answered.
#[must_use]
pub fn compose(local: &Local, daemon: &Daemon) -> Report {
    let mut rows = Vec::new();

    rows.push(row(
        "client",
        Status::Ok,
        facts(&[("version", &local.client_version)]),
        words("Client", &format!("meltemi {}", local.client_version), None),
        words(
            "Cliente",
            &format!("meltemi {}", local.client_version),
            None,
        ),
    ));

    let kind = endpoint_kind(&local.endpoint);
    if daemon.reached {
        let version = daemon
            .version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        rows.push(row(
            "daemon",
            Status::Ok,
            facts(&[("version", &version), ("endpoint", &local.endpoint)]),
            words(
                "Daemon",
                &format!("meltemid {version} answers at {} ({kind})", local.endpoint),
                None,
            ),
            words(
                "Daemon",
                &format!(
                    "meltemid {version} responde en {} ({})",
                    local.endpoint,
                    kind_es(kind)
                ),
                None,
            ),
        ));
        let matches = daemon.version.as_deref() == Some(local.client_version.as_str());
        rows.push(if matches {
            row(
                "versions",
                Status::Ok,
                facts(&[("client", &local.client_version), ("daemon", &version)]),
                words("Versions", "client and daemon are the same version", None),
                words("Versiones", "cliente y daemon son la misma versión", None),
            )
        } else {
            row(
                "versions",
                Status::Warn,
                facts(&[("client", &local.client_version), ("daemon", &version)]),
                words(
                    "Versions",
                    &format!("client {} and daemon {version} differ", local.client_version),
                    Some("an older daemon is still running: `meltemi stop`, and the next command starts the installed one"),
                ),
                words(
                    "Versiones",
                    &format!("el cliente {} y el daemon {version} no coinciden", local.client_version),
                    Some("sigue corriendo un daemon anterior: `meltemi stop`, y el siguiente comando arranca el instalado"),
                ),
            )
        });
    } else {
        let detail = daemon
            .detail
            .clone()
            .unwrap_or_else(|| "no answer".to_string());
        rows.push(row(
            "daemon",
            Status::Fail,
            facts(&[("endpoint", &local.endpoint), ("detail", &detail)]),
            words(
                "Daemon",
                &format!("nothing answers at {} ({kind}): {detail}", local.endpoint),
                Some("`meltemi status` starts the daemon on this machine; for another machine, open its tunnel first"),
            ),
            words(
                "Daemon",
                &format!("nada responde en {} ({}): {detail}", local.endpoint, kind_es(kind)),
                Some("`meltemi status` arranca el daemon en este equipo; para otra máquina, abre antes su túnel"),
            ),
        ));
    }

    rows.push(dir_row(
        "data-dir",
        "Data folder",
        "Carpeta de datos",
        &local.data_dir,
    ));
    rows.push(dir_row(
        "config-dir",
        "Configuration folder",
        "Carpeta de configuración",
        &local.config_dir,
    ));
    rows.push(config_row(&local.config_dir.0, &local.config));

    match (&daemon.fleet, daemon.reached) {
        (Some(fleet), _) => rows.extend(fleet_rows(fleet)),
        (None, true) => rows.push(row(
            "fleet",
            Status::Unchecked,
            BTreeMap::new(),
            words(
                "Agents",
                "the daemon did not report its fleet",
                Some("`meltemi fleet` shows the error"),
            ),
            words(
                "Agentes",
                "el daemon no informó su flota",
                Some("`meltemi fleet` muestra el error"),
            ),
        )),
        (None, false) => rows.push(row(
            "fleet",
            Status::Unchecked,
            BTreeMap::new(),
            words("Agents", "not checked: the daemon does not answer", None),
            words("Agentes", "sin comprobar: el daemon no responde", None),
        )),
    }

    rows.push(tool_row(
        "git",
        &local.git,
        Status::Fail,
        (
            "Meltemi needs git for worktrees and per-task commits: install it",
            "Meltemi necesita git para los worktrees y los commits por tarea: instálalo",
        ),
    ));
    rows.push(tool_row(
        "ssh",
        &local.ssh,
        Status::Warn,
        (
            "only needed to reach another machine: install the OpenSSH client",
            "solo hace falta para llegar a otra máquina: instala el cliente OpenSSH",
        ),
    ));
    if let Some(present) = local.sshd {
        rows.push(if present {
            row(
                "sshd",
                Status::Ok,
                facts(&[("installed", "true")]),
                words("OpenSSH server", "installed", None),
                words("Servidor OpenSSH", "instalado", None),
            )
        } else {
            row(
                "sshd",
                Status::Warn,
                facts(&[("installed", "false")]),
                words(
                    "OpenSSH server",
                    "not installed",
                    Some("only needed to reach this PC from another machine: see docs/acceso-remoto.md"),
                ),
                words(
                    "Servidor OpenSSH",
                    "no instalado",
                    Some("solo hace falta para llegar a este PC desde otra máquina: ver docs/acceso-remoto.md"),
                ),
            )
        });
    }

    rows.push(match &local.daemon_log {
        None => row(
            "daemon-log",
            Status::Unchecked,
            BTreeMap::new(),
            words(
                "Daemon log",
                "no log yet: the daemon has not run on this machine",
                None,
            ),
            words(
                "Log del daemon",
                "aún no hay log: el daemon no ha corrido en este equipo",
                None,
            ),
        ),
        Some((path, errors)) if errors.is_empty() => row(
            "daemon-log",
            Status::Ok,
            facts(&[("path", &path.display().to_string())]),
            words(
                "Daemon log",
                &format!("no errors at the end of {}", path.display()),
                None,
            ),
            words(
                "Log del daemon",
                &format!("sin errores al final de {}", path.display()),
                None,
            ),
        ),
        Some((path, errors)) => {
            let last = errors.join("\n                  ");
            row(
                "daemon-log",
                Status::Warn,
                facts(&[
                    ("path", &path.display().to_string()),
                    ("errors", &errors.len().to_string()),
                ]),
                words(
                    "Daemon log",
                    &format!(
                        "{} error line(s) at the end of {}:\n                  {last}",
                        errors.len(),
                        path.display()
                    ),
                    None,
                ),
                words(
                    "Log del daemon",
                    &format!(
                        "{} línea(s) de error al final de {}:\n                  {last}",
                        errors.len(),
                        path.display()
                    ),
                    None,
                ),
            )
        }
    });

    Report { rows }
}

/// The rows of the fleet: each ready agent, each one with a layer missing, and
/// the ones not detected at all in a single row (design D5).
fn fleet_rows(fleet: &FleetListResult) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut absent = Vec::new();
    for agent in &fleet.agents {
        if not_installed(agent) {
            absent.push(agent.id.clone());
        } else {
            rows.push(agent_row(agent));
        }
    }
    if !absent.is_empty() {
        let count = absent.len().to_string();
        rows.push(row(
            "agents-absent",
            Status::Ok,
            facts(&[("count", &count), ("agents", &absent.join(", "))]),
            words(
                "Agents not installed",
                &format!("{count}: {}", absent.join(", ")),
                None,
            ),
            words(
                "Agentes no instalados",
                &format!("{count}: {}", absent.join(", ")),
                None,
            ),
        ));
    }
    if rows.iter().all(|row| !row.id.starts_with("agent:")) {
        rows.insert(
            0,
            row(
                "fleet",
                Status::Warn,
                BTreeMap::new(),
                words(
                    "Agents",
                    "no agent is installed on this machine",
                    Some("install the official CLI of the agent you use: `meltemi fleet` says how"),
                ),
                words(
                    "Agentes",
                    "no hay ningún agente instalado en este equipo",
                    Some("instala el CLI oficial del agente que usas: `meltemi fleet` dice cómo"),
                ),
            ),
        );
    }
    rows
}

/// Whether an agent is simply not installed here. Meltemi's own adapters travel
/// in every installation, so an agent whose only layer found is one of them is
/// an agent the user never installed — not a half installation to repair.
fn not_installed(agent: &FleetAgent) -> bool {
    match agent.install_state {
        Some(FleetInstallState::NotDetected) => true,
        None => !agent.detected,
        Some(FleetInstallState::CliMissing) => agent
            .layers
            .iter()
            .filter(|layer| layer.detected)
            .all(|layer| layer.bundled),
        Some(_) => false,
    }
}

fn agent_row(agent: &FleetAgent) -> Row {
    let id = format!("agent:{}", agent.id);
    let verified = agent
        .verified_version
        .as_deref()
        .map(|version| format!(", verified with {version}"))
        .unwrap_or_default();
    let verified_es = agent
        .verified_version
        .as_deref()
        .map(|version| format!(", verificado con {version}"))
        .unwrap_or_default();
    let remedy = match (&agent.remedy, &agent.remedy_command) {
        // The fleet's sentence often names its command already.
        (Some(remedy), Some(command)) if remedy.contains(command.as_str()) => Some(remedy.clone()),
        (Some(remedy), Some(command)) => Some(format!("{remedy} `{command}`")),
        (Some(remedy), None) => Some(remedy.clone()),
        (None, Some(command)) => Some(format!("`{command}`")),
        (None, None) => None,
    };
    let (status, en, es) = match agent.install_state {
        Some(FleetInstallState::Ready) | None => (
            Status::Ok,
            format!("ready{verified}"),
            format!("listo{verified_es}"),
        ),
        Some(FleetInstallState::AdapterMissing) => (
            Status::Fail,
            "the official CLI is installed; the ACP adapter is missing".to_string(),
            "el CLI oficial está instalado; falta el adaptador ACP".to_string(),
        ),
        Some(FleetInstallState::CliMissing) => (
            Status::Fail,
            "the adapter is installed; the official CLI is missing".to_string(),
            "el adaptador está instalado; falta el CLI oficial".to_string(),
        ),
        Some(FleetInstallState::NotLaunchable) => (
            Status::Fail,
            "installed, but only as a script that cannot be launched".to_string(),
            "instalado, pero solo como un script que no se puede lanzar".to_string(),
        ),
        Some(FleetInstallState::NotDetected) => (
            Status::Ok,
            "not installed".to_string(),
            "no instalado".to_string(),
        ),
    };
    let mut found = facts(&[("state", &format!("{:?}", agent.install_state))]);
    if let Some(version) = &agent.verified_version {
        found.insert("verifiedVersion".to_string(), version.clone());
    }
    Row {
        id,
        status,
        facts: found,
        en: Words {
            title: agent.display_name.clone(),
            found: en,
            remedy: remedy.clone(),
        },
        es: Words {
            title: agent.display_name.clone(),
            found: es,
            remedy,
        },
    }
}

fn dir_row(id: &str, title_en: &str, title_es: &str, (path, state): &(PathBuf, DirState)) -> Row {
    let shown = path.display().to_string();
    let (status, en, es, remedy) = match state {
        DirState::Writable => (
            Status::Ok,
            format!("{shown}, writable"),
            format!("{shown}, se puede escribir"),
            None,
        ),
        DirState::Absent => (
            Status::Ok,
            format!("{shown}, not created yet"),
            format!("{shown}, aún no creada"),
            None,
        ),
        DirState::ReadOnly => (
            Status::Fail,
            format!("{shown}, nothing can be written in it"),
            format!("{shown}, no se puede escribir en ella"),
            Some((
                "give your user write permission on it",
                "da permiso de escritura a tu usuario",
            )),
        ),
        DirState::NotADirectory => (
            Status::Fail,
            format!("{shown} is not a folder"),
            format!("{shown} no es una carpeta"),
            Some((
                "move that file away: Meltemi creates the folder",
                "aparta ese archivo: Meltemi crea la carpeta",
            )),
        ),
    };
    row(
        id,
        status,
        facts(&[("path", &shown)]),
        words(title_en, &en, remedy.map(|(en, _)| en)),
        words(title_es, &es, remedy.map(|(_, es)| es)),
    )
}

fn config_row(config_dir: &Path, state: &ConfigState) -> Row {
    let file = config_dir.join("config.toml").display().to_string();
    match state {
        ConfigState::Absent => row(
            "user-config",
            Status::Ok,
            facts(&[("path", &file)]),
            words("User configuration", "none, which is valid", None),
            words("Configuración de usuario", "no hay, y es válido", None),
        ),
        ConfigState::Valid => row(
            "user-config",
            Status::Ok,
            facts(&[("path", &file)]),
            words("User configuration", &format!("{file} reads"), None),
            words(
                "Configuración de usuario",
                &format!("{file} se lee bien"),
                None,
            ),
        ),
        ConfigState::Invalid(why) => row(
            "user-config",
            Status::Fail,
            facts(&[("path", &file), ("error", why)]),
            words(
                "User configuration",
                &format!("{file} does not parse: {why}"),
                Some("fix the line it names"),
            ),
            words(
                "Configuración de usuario",
                &format!("{file} no se puede leer: {why}"),
                Some("corrige la línea que nombra"),
            ),
        ),
        ConfigState::Skipped(reasons) => row(
            "user-config",
            Status::Warn,
            facts(&[("path", &file), ("skipped", &reasons.len().to_string())]),
            words(
                "User configuration",
                &format!("{file} reads; skipped: {}", reasons.join("; ")),
                None,
            ),
            words(
                "Configuración de usuario",
                &format!("{file} se lee; se omitió: {}", reasons.join("; ")),
                None,
            ),
        ),
    }
}

fn tool_row(name: &str, state: &ToolState, missing: Status, remedy: (&str, &str)) -> Row {
    match state {
        ToolState::Found(version) => row(
            name,
            Status::Ok,
            facts(&[("version", version)]),
            words(name, version, None),
            words(name, version, None),
        ),
        ToolState::Missing => row(
            name,
            missing,
            BTreeMap::new(),
            words(name, "not found on the PATH", Some(remedy.0)),
            words(name, "no está en el PATH", Some(remedy.1)),
        ),
        ToolState::Unchecked(why) => row(
            name,
            Status::Unchecked,
            facts(&[("detail", why)]),
            words(name, &format!("not checked: {why}"), None),
            words(name, &format!("sin comprobar: {why}"), None),
        ),
    }
}

fn row(id: &str, status: Status, facts: BTreeMap<String, String>, en: Words, es: Words) -> Row {
    Row {
        id: id.to_string(),
        status,
        facts,
        en,
        es,
    }
}

fn words(title: &str, found: &str, remedy: Option<&str>) -> Words {
    Words {
        title: title.to_string(),
        found: found.to_string(),
        remedy: remedy.map(str::to_string),
    }
}

fn facts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

/// What kind of endpoint a string names.
fn endpoint_kind(endpoint: &str) -> &'static str {
    if crate::exec::is_exec(endpoint) {
        "a command"
    } else if endpoint.starts_with(r"\\.\pipe\") {
        "a named pipe"
    } else {
        "a local socket"
    }
}

fn kind_es(kind: &str) -> &'static str {
    match kind {
        "a command" => "un comando",
        "a named pipe" => "un named pipe",
        _ => "un socket local",
    }
}

/// What a folder is: probed by writing and removing a file, which is the only
/// honest answer to "can the daemon write here".
fn dir_state(path: &Path) -> DirState {
    match std::fs::metadata(path) {
        Err(_) => DirState::Absent,
        Ok(meta) if !meta.is_dir() => DirState::NotADirectory,
        Ok(_) => {
            let probe = path.join(format!(".meltemi-doctor-{}", std::process::id()));
            match std::fs::write(&probe, b"") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&probe);
                    DirState::Writable
                }
                Err(_) => DirState::ReadOnly,
            }
        }
    }
}

/// What the user's `config.toml` is.
fn config_state(config_dir: &Path) -> ConfigState {
    let Ok(raw) = std::fs::read_to_string(config_dir.join("config.toml")) else {
        return ConfigState::Absent;
    };
    if let Err(error) = raw.parse::<toml::Table>() {
        return ConfigState::Invalid(error.message().to_string());
    }
    let read = crate::machines::read(config_dir);
    if read.diagnostics.is_empty() {
        ConfigState::Valid
    } else {
        ConfigState::Skipped(read.diagnostics)
    }
}

/// Runs a helper command and keeps the first line it prints, within
/// [`COMMAND_BUDGET`]. A command that does not finish in time is killed.
fn tool(program: &str, args: &[&str]) -> ToolState {
    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: a diagnosis from the desktop must not flash a
        // console.
        command.creation_flags(0x0800_0000);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return ToolState::Missing,
        Err(error) => return ToolState::Unchecked(error.to_string()),
    };
    let deadline = Instant::now() + COMMAND_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return ToolState::Unchecked(format!(
                    "no answer within {}s",
                    COMMAND_BUDGET.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => return ToolState::Unchecked(error.to_string()),
        }
    }
    let Ok(output) = child.wait_with_output() else {
        return ToolState::Unchecked("its output could not be read".to_string());
    };
    // `ssh -V` writes to stderr, `git --version` to stdout.
    let said = [output.stdout, output.stderr]
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .find_map(|text| {
            text.lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
        });
    match said {
        Some(line) => ToolState::Found(line),
        None => ToolState::Unchecked("it printed nothing".to_string()),
    }
}

/// Windows: whether the OpenSSH server is installed where Windows installs it.
fn sshd_installed() -> Option<bool> {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        Some(
            PathBuf::from(root)
                .join("System32")
                .join("OpenSSH")
                .join("sshd.exe")
                .is_file(),
        )
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The newest daemon log in `logs` and the error lines at its end.
fn daemon_log(logs: &Path) -> Option<(PathBuf, Vec<String>)> {
    let newest = std::fs::read_dir(logs)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("meltemid.log")
        })
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)?;
    let errors = tail(&newest, LOG_TAIL_BYTES)
        .lines()
        .filter(|line| line.contains(" ERROR "))
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let keep = errors.len().saturating_sub(LOG_LINES);
    Some((newest, errors[keep..].to_vec()))
}

/// The last `bytes` of a file, from the first whole line in them.
fn tail(path: &Path, bytes: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = length.saturating_sub(bytes);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut raw = Vec::new();
    if file.read_to_end(&mut raw).is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    if start == 0 {
        return text;
    }
    // The cut may fall mid-line; that half line is not evidence of anything.
    text.split_once('\n')
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::{FleetAgentSource, FleetLayer, FleetLayerKind};

    fn local() -> Local {
        Local {
            client_version: "0.2.0".to_string(),
            endpoint: "/run/user/1000/meltemi/meltemid.sock".to_string(),
            data_dir: (
                PathBuf::from("/home/u/.local/share/meltemi"),
                DirState::Writable,
            ),
            config_dir: (PathBuf::from("/home/u/.config/meltemi"), DirState::Absent),
            config: ConfigState::Absent,
            git: ToolState::Found("git version 2.45.0".to_string()),
            ssh: ToolState::Missing,
            sshd: None,
            daemon_log: None,
        }
    }

    fn agent(id: &str, state: FleetInstallState, detected: bool) -> FleetAgent {
        FleetAgent {
            id: id.to_string(),
            display_name: id.to_string(),
            source: FleetAgentSource::Registry,
            integration_level: 2,
            verified_level: None,
            verified_at: None,
            verified_version: None,
            mcp_support: false,
            detected,
            binary_path: None,
            configured: false,
            underlying_agent: None,
            layers: vec![FleetLayer {
                kind: FleetLayerKind::Cli,
                bin: id.to_string(),
                detected,
                binary_path: None,
                evidence_only: false,
                install: None,
                bundled: false,
                source: None,
            }],
            install_state: Some(state),
            remedy: (state == FleetInstallState::AdapterMissing)
                .then(|| "The ACP adapter is missing:".to_string()),
            remedy_command: (state == FleetInstallState::AdapterMissing)
                .then(|| "npm i -g some-adapter".to_string()),
            legal_status: None,
            legal_note: None,
            auth_context_var: None,
            model: None,
            effort: None,
        }
    }

    #[test]
    fn an_agent_with_only_meltemis_own_adapter_is_not_installed_not_broken() {
        let mut codex = agent("codex-like", FleetInstallState::CliMissing, true);
        codex.layers = vec![
            FleetLayer {
                kind: FleetLayerKind::Cli,
                bin: "codex".to_string(),
                detected: false,
                binary_path: None,
                evidence_only: false,
                install: Some("npm i -g @openai/codex".to_string()),
                bundled: false,
                source: None,
            },
            FleetLayer {
                kind: FleetLayerKind::Adapter,
                bin: "meltemi-codex-acp".to_string(),
                detected: true,
                binary_path: Some("/opt/meltemi/meltemi-codex-acp".to_string()),
                evidence_only: false,
                install: None,
                bundled: true,
                source: None,
            },
        ];
        codex.remedy = Some("install it: npm i -g @openai/codex".to_string());
        codex.remedy_command = Some("npm i -g @openai/codex".to_string());
        let mut third_party = codex.clone();
        third_party.id = "third-party".to_string();
        third_party.layers[1].bundled = false;
        let daemon = Daemon {
            reached: true,
            version: Some("0.2.0".to_string()),
            fleet: Some(FleetListResult {
                registry_version: "1".to_string(),
                agents: vec![codex, third_party],
            }),
            ..Daemon::default()
        };
        let report = compose(&local(), &daemon);
        assert!(
            report.rows.iter().all(|row| row.id != "agent:codex-like"),
            "the bundled adapter alone is no installation: {report:#?}"
        );
        assert_eq!(
            by_id(&report, "agents-absent").facts["agents"],
            "codex-like"
        );
        let broken = by_id(&report, "agent:third-party");
        assert_eq!(
            broken.status,
            Status::Fail,
            "an adapter the user installed is half an install"
        );
        assert_eq!(
            broken.en.remedy.as_deref(),
            Some("install it: npm i -g @openai/codex"),
            "a command the sentence already names is not repeated"
        );
    }

    fn by_id<'a>(report: &'a Report, id: &str) -> &'a Row {
        report
            .rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("no row `{id}` in {report:#?}"))
    }

    // Scenario: Cada problema dice su remedio
    #[test]
    fn a_half_installed_agent_says_its_state_and_its_remedy() {
        let daemon = Daemon {
            reached: true,
            detail: None,
            version: Some("0.2.0".to_string()),
            fleet: Some(FleetListResult {
                registry_version: "1".to_string(),
                agents: vec![
                    agent("ready-one", FleetInstallState::Ready, true),
                    agent("half-one", FleetInstallState::AdapterMissing, true),
                    agent("absent-one", FleetInstallState::NotDetected, false),
                    agent("absent-two", FleetInstallState::NotDetected, false),
                ],
            }),
        };
        let report = compose(&local(), &daemon);
        let half = by_id(&report, "agent:half-one");
        assert_eq!(half.status, Status::Fail);
        assert_eq!(
            half.en.remedy.as_deref(),
            Some("The ACP adapter is missing: `npm i -g some-adapter`"),
            "the remedy the fleet declares, with its command"
        );
        assert_eq!(by_id(&report, "agent:ready-one").status, Status::Ok);
        let absent = by_id(&report, "agents-absent");
        assert_eq!(absent.facts["count"], "2", "the absent ones in one row");
        assert!(report.has_failures());

        // Every failing row says what to do.
        for row in report.rows.iter().filter(|row| row.status == Status::Fail) {
            assert!(
                row.en.remedy.is_some() && row.es.remedy.is_some(),
                "{row:#?}"
            );
        }
    }

    #[test]
    fn an_unreachable_daemon_is_a_failing_row_and_the_rest_is_still_there() {
        let daemon = Daemon {
            reached: false,
            detail: Some("connection refused".to_string()),
            ..Daemon::default()
        };
        let report = compose(&local(), &daemon);
        let row = by_id(&report, "daemon");
        assert_eq!(row.status, Status::Fail);
        assert!(row.en.remedy.as_deref().unwrap().contains("meltemi status"));
        assert_eq!(by_id(&report, "fleet").status, Status::Unchecked);
        for id in [
            "client",
            "data-dir",
            "config-dir",
            "user-config",
            "git",
            "ssh",
        ] {
            by_id(&report, id);
        }
        assert_eq!(
            by_id(&report, "ssh").status,
            Status::Warn,
            "ssh is only for remote use"
        );
    }

    #[test]
    fn versions_that_differ_are_a_warning_with_the_way_out() {
        let daemon = Daemon {
            reached: true,
            version: Some("0.1.0".to_string()),
            fleet: Some(FleetListResult {
                registry_version: "1".to_string(),
                agents: vec![],
            }),
            ..Daemon::default()
        };
        let report = compose(&local(), &daemon);
        let versions = by_id(&report, "versions");
        assert_eq!(versions.status, Status::Warn);
        assert!(
            versions
                .en
                .remedy
                .as_deref()
                .unwrap()
                .contains("meltemi stop")
        );
        assert_eq!(
            by_id(&report, "fleet").status,
            Status::Warn,
            "no agent installed"
        );
    }

    #[test]
    fn every_status_is_a_symbol_with_a_twin_and_a_word() {
        for status in [Status::Ok, Status::Warn, Status::Fail, Status::Unchecked] {
            assert!(status.glyph(false).is_ascii());
            assert!(!status.word(Lang::En).is_empty() && !status.word(Lang::Es).is_empty());
        }
        let report = compose(&local(), &Daemon::default());
        let text = report.text(Lang::Es, false);
        assert!(text.contains("x falla") && text.contains("-> "), "{text}");
        for symbol in ["✓", "✕", "○", "→"] {
            assert!(!text.contains(symbol), "ASCII mode uses the twins: {text}");
        }
        let json = report.json();
        assert_eq!(json["rows"][0]["id"], "client");
        assert!(json["failing"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn a_folder_is_probed_by_writing_in_it() {
        let dir = std::env::temp_dir().join(format!("meltemi-doctor-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(dir_state(&dir), DirState::Absent);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(dir_state(&dir), DirState::Writable);
        let file = dir.join("a-file");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(dir_state(&file), DirState::NotADirectory);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "the probe leaves nothing behind"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_log_gives_its_last_error_lines_and_a_missing_one_says_so() {
        let dir = std::env::temp_dir().join(format!("meltemi-doctor-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(daemon_log(&dir), None);
        std::fs::create_dir_all(&dir).unwrap();
        let mut text = String::new();
        for at in 0..8 {
            text.push_str(&format!(
                "2026-10-28T10:00:0{at}Z  INFO meltemid: fine {at}\n"
            ));
            text.push_str(&format!(
                "2026-10-28T10:00:0{at}Z ERROR meltemid: broke {at}\n"
            ));
        }
        std::fs::write(dir.join("meltemid.log.2026-10-28"), text).unwrap();
        let (_, errors) = daemon_log(&dir).expect("a log");
        assert_eq!(errors.len(), LOG_LINES);
        assert!(errors.last().unwrap().ends_with("broke 7"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_command_that_is_not_there_is_missing_not_an_error() {
        assert_eq!(
            tool("meltemi-doctor-no-such-program", &["--version"]),
            ToolState::Missing
        );
    }

    #[test]
    fn a_configuration_that_does_not_parse_fails_and_names_why() {
        let dir = std::env::temp_dir().join(format!("meltemi-doctor-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(config_state(&dir), ConfigState::Absent);
        std::fs::write(dir.join("config.toml"), "this is = = not toml").unwrap();
        assert!(matches!(config_state(&dir), ConfigState::Invalid(_)));
        std::fs::write(dir.join("config.toml"), "[client]\nendpoint = \"x\"\n").unwrap();
        assert_eq!(config_state(&dir), ConfigState::Valid);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
