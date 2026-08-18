// SPDX-License-Identifier: Apache-2.0

//! Minimal configuration (design D13).
//!
//! Precedence, low to high: built-in defaults < user config
//! (`<config_dir>/config.toml`) < project config
//! (`<project_root>/.meltemi/config.toml`) < `MELTEMI_*` environment
//! variables. CLI flags (highest) are applied by the binaries, not here.
//!
//! Fase 0 needed only the agent command; `catalogo-flota` adds the catalog
//! selection (`agent.id`), the registry override (`fleet.registry`) and the
//! user-declared agents (`[[fleet.custom]]`). The shape stays small.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

/// How long an escalated permission request waits for a human decision
/// (espera-humana D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitPolicy {
    /// No deadline: wait as long as at least one client is connected.
    WhileConnected,
    /// Wait at most this many seconds, then deny audited as `timeout`.
    Bounded(u64),
}

/// Environment override for the agent command (a full command line, parsed
/// with shell-words semantics by the ACP layer).
pub const ENV_AGENT_COMMAND: &str = "MELTEMI_AGENT_COMMAND";

/// Environment override for the fleet registry snapshot: a path to a local
/// registry TOML that replaces the embedded one (see `fleet`).
pub const ENV_FLEET_REGISTRY: &str = "MELTEMI_FLEET_REGISTRY";

/// An agent declared by the user outside the registry (`[[fleet.custom]]`).
/// It joins the fleet catalog with source `custom` and participates in
/// detection and selection like any entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomAgent {
    /// User-chosen catalog id, selectable via `agent.id`.
    pub id: String,
    /// Human-readable name shown in the catalog.
    pub name: String,
    /// Launch command (program first); the program is what detection probes.
    pub command: Vec<String>,
}

/// A launch profile (`[[fleet.profile]]`, flota-multiproveedor): a catalog
/// agent run under a selected authentication context. The `env` overlay
/// redirects which account the official binary authenticates as (e.g.
/// `HOME`/`XDG_CONFIG_HOME`) — Meltemi never reads the credential itself (§2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetProfile {
    /// Profile name, selectable per session.
    pub name: String,
    /// The catalog id of the underlying agent to launch.
    pub agent: String,
    /// Environment overlay applied to the binary's subprocess (`${VAR}`
    /// references resolved at launch; never a plaintext secret).
    pub env: Vec<(String, String)>,
    /// The model this profile runs by default, verbatim and opaque.
    ///
    /// "Profile = agent + account + model" is what turns "docs on the cheap
    /// model" into a choice made once instead of a ritual per session. What a
    /// session declares explicitly overrides it; a profile that declares
    /// nothing imposes nothing (modelo-y-esfuerzo-por-sesion design D4).
    pub model: Option<String>,
    /// The effort this profile runs by default, on the same terms.
    pub effort: Option<String>,
}

/// One MCP server the user declared once, to inject into compatible agents
/// (mcp-passthrough D1). Sensitive values live as `$VAR` references, never
/// literals — Meltemi references the user's environment, never stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    /// Human-readable server name (the injection is audited by name).
    pub name: String,
    /// The transport.
    pub transport: McpTransport,
}

/// An MCP server transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpTransport {
    /// A local process speaking MCP over stdio.
    Stdio {
        command: String,
        args: Vec<String>,
        /// Environment entries as `(NAME, "$VAR")` references.
        env: Vec<(String, String)>,
    },
    /// A remote MCP server over HTTP.
    Http { url: String },
}

/// Resolved daemon configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// The agent command line, e.g. `["npx", "-y", "some-acp-agent"]`.
    /// Takes precedence over [`Config::agent_id`] when both are set.
    pub agent_command: Option<Vec<String>>,
    /// A fleet catalog id selecting the agent (`[agent] id`). With neither
    /// this nor a command set, requesting a session yields error 2000
    /// (`agent_command_not_configured`).
    pub agent_id: Option<String>,
    /// Local file replacing the embedded fleet registry snapshot.
    pub fleet_registry: Option<PathBuf>,
    /// User-declared agents joining the fleet catalog, in declaration order
    /// (user config first, project config after; same id overrides).
    pub fleet_custom: Vec<CustomAgent>,
    /// Declared MCP servers to inject into compatible agents; project overrides
    /// global by name (mcp-passthrough D1).
    pub mcp_servers: Vec<McpServerConfig>,
    /// Hygiene diagnostics for MCP declarations with plaintext-secret-looking
    /// values; never carry the value itself.
    pub mcp_diagnostics: Vec<String>,
    /// Launch profiles joining the catalog (`[[fleet.profile]]`); project
    /// overrides global by name (flota-multiproveedor).
    pub fleet_profiles: Vec<FleetProfile>,
    /// Hygiene diagnostics for profiles with plaintext-secret-looking env
    /// values; never carry the value itself.
    pub fleet_diagnostics: Vec<String>,
    /// `[permissions] wait`: escalation policy of interactive flows. `None`
    /// keeps the default (wait while a client is connected).
    pub permission_wait: Option<WaitPolicy>,
    /// `[permissions] implement-wait`: policy of autonomous implement turns.
    /// `None` keeps the default (bounded 30 s, the pre-existing value).
    pub implement_wait: Option<WaitPolicy>,
    /// `[permissions] no-client-grace`: seconds a pending request survives
    /// with no client connected before the constitutional deny (§3). `None`
    /// keeps the default (30).
    pub no_client_grace: Option<u64>,
    /// Diagnostics for invalid `[permissions]` values (default kept).
    pub permission_diagnostics: Vec<String>,
    /// `[sessions] idle-timeout`: seconds a session waits for its next
    /// instruction, holding its agent subprocess, before ending. `None` keeps
    /// the default (900 = fifteen minutes).
    pub idle_timeout: Option<u64>,
    /// `[sessions] max-idle`: how many sessions may wait at once. `None` keeps
    /// the default (3). Reaching it closes the OLDEST wait, never refuses a new
    /// session.
    pub max_idle_sessions: Option<usize>,
    /// Diagnostics for invalid `[sessions]` values (default kept).
    pub session_diagnostics: Vec<String>,
}

impl Config {
    /// The wait policy of interactive flows (propose, SDD cycle, direct).
    pub fn interactive_wait(&self) -> WaitPolicy {
        self.permission_wait.unwrap_or(WaitPolicy::WhileConnected)
    }

    /// The wait policy of autonomous implement turns: bounded by default so
    /// an unattended pipeline never stalls silently.
    pub fn autonomous_wait(&self) -> WaitPolicy {
        self.implement_wait.unwrap_or(WaitPolicy::Bounded(30))
    }

    /// How long a pending request survives with no client connected before
    /// the constitutional deny fires (absorbs a tunnel blip).
    pub fn no_client_grace(&self) -> Duration {
        Duration::from_secs(self.no_client_grace.unwrap_or(30))
    }

    /// How long a session waits for its next instruction before ending.
    ///
    /// The default is deliberately conservative because the resource is real:
    /// a waiting session holds a live agent subprocess, which is memory and a
    /// process of the user's provider. Fifteen minutes is long enough to think
    /// and short enough that a forgotten window does not become a leak
    /// (sesion-que-espera design D4).
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout.unwrap_or(900))
    }

    /// How many sessions may wait at once.
    pub fn max_idle_sessions(&self) -> usize {
        self.max_idle_sessions.unwrap_or(3)
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    agent: RawAgent,
    #[serde(default)]
    fleet: RawFleet,
    #[serde(default)]
    mcp: RawMcp,
    #[serde(default)]
    permissions: RawPermissions,
    #[serde(default)]
    sessions: RawSessions,
}

#[derive(Debug, Default, Deserialize)]
struct RawSessions {
    /// Non-negative seconds (0 = do not wait at all: a turn ends its session,
    /// which is the behaviour before sesion-que-espera).
    #[serde(default, rename = "idle-timeout")]
    idle_timeout: Option<i64>,
    /// Non-negative count (0 = do not keep any session waiting).
    #[serde(default, rename = "max-idle")]
    max_idle: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPermissions {
    /// `"while-connected"` or a positive number of seconds.
    #[serde(default)]
    wait: Option<WaitSpec>,
    /// Same shape, for autonomous implement turns.
    #[serde(default, rename = "implement-wait")]
    implement_wait: Option<WaitSpec>,
    /// Non-negative seconds (0 = the constitutional deny fires immediately
    /// when the last client disconnects).
    #[serde(default, rename = "no-client-grace")]
    no_client_grace: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WaitSpec {
    Seconds(i64),
    Mode(String),
}

/// Parses one wait value; an invalid one keeps the default and leaves a
/// diagnostic with the remedy (never a panic).
fn parse_wait(spec: WaitSpec, key: &str, diagnostics: &mut Vec<String>) -> Option<WaitPolicy> {
    match spec {
        WaitSpec::Mode(mode) if mode == "while-connected" => Some(WaitPolicy::WhileConnected),
        WaitSpec::Mode(other) => {
            diagnostics.push(format!(
                "permissions.{key} `{other}` is not recognized; use \"while-connected\" \
                 or a positive number of seconds (default kept)"
            ));
            None
        }
        WaitSpec::Seconds(n) if n >= 1 => Some(WaitPolicy::Bounded(n as u64)),
        WaitSpec::Seconds(n) => {
            diagnostics.push(format!(
                "permissions.{key} `{n}` must be a positive number of seconds (default kept)"
            ));
            None
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawMcp {
    #[serde(default)]
    servers: Vec<RawMcpServer>,
}

#[derive(Debug, Deserialize)]
struct RawMcpServer {
    name: String,
    /// stdio transport.
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    /// http transport.
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawAgent {
    /// The agent command, either as a single string (`"npx -y agent"`) or an
    /// explicit argv array (`["npx", "-y", "agent"]`).
    #[serde(default)]
    command: Option<CommandSpec>,
    /// A fleet catalog id, the declarative alternative to `command`.
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFleet {
    /// Path to a local registry TOML replacing the embedded snapshot.
    #[serde(default)]
    registry: Option<String>,
    /// User-declared agents.
    #[serde(default)]
    custom: Vec<RawCustomAgent>,
    /// Launch profiles (flota-multiproveedor).
    #[serde(default)]
    profile: Vec<RawFleetProfile>,
}

#[derive(Debug, Deserialize)]
struct RawFleetProfile {
    name: String,
    agent: String,
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    /// The provider's own model name. Read as a string and never validated:
    /// what accepts or rejects it is the agent (design D1).
    #[serde(default)]
    model: Option<String>,
    /// The provider's own effort level, on the same terms.
    #[serde(default)]
    effort: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCustomAgent {
    id: String,
    /// Defaults to the id when omitted.
    #[serde(default)]
    name: Option<String>,
    command: CommandSpec,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CommandSpec {
    Line(String),
    Argv(Vec<String>),
}

impl CommandSpec {
    fn into_argv(self) -> Option<Vec<String>> {
        let argv = match self {
            CommandSpec::Line(line) => split_command_line(&line),
            CommandSpec::Argv(argv) => argv,
        };
        if argv.is_empty() { None } else { Some(argv) }
    }
}

/// Splits a command line into arguments honoring double and single quotes.
/// Deliberately small: full shell parsing is not a Fase 0 goal.
fn split_command_line(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_arg = false;
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => {
                    in_arg = true;
                    quote = Some(ch);
                }
                c if c.is_whitespace() => {
                    if in_arg {
                        args.push(std::mem::take(&mut current));
                        in_arg = false;
                    }
                }
                c => {
                    in_arg = true;
                    current.push(c);
                }
            },
        }
    }
    if in_arg {
        args.push(current);
    }
    args
}

impl Config {
    /// Loads and merges configuration from all sources, honoring precedence.
    ///
    /// `config_dir` is the user config directory; `project_root` is the target
    /// repository root (its `.meltemi/config.toml` overrides the user file).
    pub fn load(config_dir: &Path, project_root: Option<&Path>) -> Self {
        let mut config = Config::default();

        // Linked subscriptions load FIRST (vincular-suscripciones D2): they are
        // ordinary profile blocks in the daemon-owned store, and loading them
        // before the user's config means the merge-by-name below lets anything
        // written by hand win over a homonymous link.
        if let Some(raw) = read_config_file(&crate::subscriptions::store_path(config_dir)) {
            config.apply(raw);
        }
        if let Some(raw) = read_config_file(&config_dir.join("config.toml")) {
            config.apply(raw);
        }
        if let Some(root) = project_root
            && let Some(raw) = read_config_file(&root.join(".meltemi").join("config.toml"))
        {
            config.apply(raw);
        }
        config.apply_env();
        config
    }

    fn apply(&mut self, raw: RawConfig) {
        if let Some(spec) = raw.permissions.wait
            && let Some(policy) = parse_wait(spec, "wait", &mut self.permission_diagnostics)
        {
            self.permission_wait = Some(policy);
        }
        if let Some(spec) = raw.permissions.implement_wait
            && let Some(policy) =
                parse_wait(spec, "implement-wait", &mut self.permission_diagnostics)
        {
            self.implement_wait = Some(policy);
        }
        if let Some(grace) = raw.permissions.no_client_grace {
            if grace >= 0 {
                self.no_client_grace = Some(grace as u64);
            } else {
                self.permission_diagnostics.push(format!(
                    "permissions.no-client-grace `{grace}` must be zero or more seconds \
                     (default kept)"
                ));
            }
        }
        if let Some(seconds) = raw.sessions.idle_timeout {
            if seconds >= 0 {
                self.idle_timeout = Some(seconds as u64);
            } else {
                self.session_diagnostics.push(format!(
                    "sessions.idle-timeout `{seconds}` must be zero or more seconds                      (default kept)"
                ));
            }
        }
        if let Some(count) = raw.sessions.max_idle {
            if count >= 0 {
                self.max_idle_sessions = Some(count as usize);
            } else {
                self.session_diagnostics.push(format!(
                    "sessions.max-idle `{count}` must be zero or more (default kept)"
                ));
            }
        }
        if let Some(command) = raw.agent.command.and_then(CommandSpec::into_argv) {
            self.agent_command = Some(command);
        }
        if let Some(id) = raw.agent.id.filter(|id| !id.trim().is_empty()) {
            self.agent_id = Some(id);
        }
        if let Some(registry) = raw.fleet.registry.filter(|p| !p.trim().is_empty()) {
            self.fleet_registry = Some(PathBuf::from(registry));
        }
        for custom in raw.fleet.custom {
            if custom.id.trim().is_empty() {
                tracing::warn!("ignoring fleet.custom entry with an empty id");
                continue;
            }
            let Some(command) = custom.command.into_argv() else {
                tracing::warn!(id = %custom.id, "ignoring fleet.custom entry with an empty command");
                continue;
            };
            let agent = CustomAgent {
                name: custom.name.unwrap_or_else(|| custom.id.clone()),
                id: custom.id,
                command,
            };
            match self.fleet_custom.iter_mut().find(|c| c.id == agent.id) {
                Some(existing) => *existing = agent,
                None => self.fleet_custom.push(agent),
            }
        }
        for raw in raw.mcp.servers {
            // Hygiene lint: flag plaintext-secret-looking values with a remedy,
            // never copying the value into the diagnostic (D1).
            for (key, value) in &raw.env {
                if looks_like_plaintext_secret(value) {
                    self.mcp_diagnostics.push(format!(
                        "mcp server `{}` env `{key}` looks like a plaintext secret; \
                         reference an environment variable instead (e.g. `\"$MY_TOKEN\"`)",
                        raw.name
                    ));
                }
            }
            let transport = if let Some(command) = raw.command {
                McpTransport::Stdio {
                    command,
                    args: raw.args,
                    env: raw.env.into_iter().collect(),
                }
            } else if let Some(url) = raw.url {
                if looks_like_plaintext_secret(&url) {
                    self.mcp_diagnostics.push(format!(
                        "mcp server `{}` url embeds what looks like a secret; \
                         reference an environment variable instead",
                        raw.name
                    ));
                }
                McpTransport::Http { url }
            } else {
                tracing::warn!(name = %raw.name, "mcp server has neither command nor url; ignored");
                continue;
            };
            let server = McpServerConfig {
                name: raw.name,
                transport,
            };
            // Project overrides global by name.
            match self.mcp_servers.iter_mut().find(|s| s.name == server.name) {
                Some(existing) => *existing = server,
                None => self.mcp_servers.push(server),
            }
        }
        for raw in raw.fleet.profile {
            if raw.name.trim().is_empty() || raw.agent.trim().is_empty() {
                tracing::warn!("ignoring fleet.profile with an empty name or agent");
                continue;
            }
            // Hygiene lint: a plaintext-secret-looking env value REFUSES the
            // profile (never launch with a bare secret); the remedy names the
            // `${VAR}` form, never the value itself (§2, reuse of mcp-passthrough).
            let mut refused = false;
            for (key, value) in &raw.env {
                if looks_like_plaintext_secret(value) {
                    self.fleet_diagnostics.push(format!(
                        "fleet profile `{}` env `{key}` looks like a plaintext secret; \
                         reference an environment variable instead (e.g. `\"${{MY_TOKEN}}\"`)",
                        raw.name
                    ));
                    refused = true;
                }
            }
            if refused {
                continue;
            }
            let profile = FleetProfile {
                name: raw.name,
                agent: raw.agent,
                env: raw.env.into_iter().collect(),
                // An empty string is not a choice: it would travel to the agent
                // as though the user had picked something.
                model: raw.model.filter(|m| !m.trim().is_empty()),
                effort: raw.effort.filter(|e| !e.trim().is_empty()),
            };
            match self
                .fleet_profiles
                .iter_mut()
                .find(|p| p.name == profile.name)
            {
                Some(existing) => *existing = profile,
                None => self.fleet_profiles.push(profile),
            }
        }

        // Duplicate-context lens (vincular-suscripciones D6): two profiles of
        // the same agent resolving to the same context value are one
        // subscription under two names — legal, but almost always a silent
        // mistake, so it is said out loud and both keep resolving.
        for i in 0..self.fleet_profiles.len() {
            for j in (i + 1)..self.fleet_profiles.len() {
                let (a, b) = (&self.fleet_profiles[i], &self.fleet_profiles[j]);
                if a.agent != b.agent {
                    continue;
                }
                let shared = a.env.iter().any(|(ak, av)| {
                    !av.trim().is_empty()
                        && b.env.iter().any(|(bk, bv)| {
                            ak == bk && crate::mcp::resolve_ref(av) == crate::mcp::resolve_ref(bv)
                        })
                });
                if shared {
                    let line = format!(
                        "fleet profiles `{}` and `{}` resolve the same context for `{}`:                          they are one subscription under two names",
                        a.name, b.name, a.agent
                    );
                    if !self.fleet_diagnostics.contains(&line) {
                        self.fleet_diagnostics.push(line);
                    }
                }
            }
        }
    }

    fn apply_env(&mut self) {
        if let Ok(line) = std::env::var(ENV_AGENT_COMMAND) {
            let argv = split_command_line(&line);
            if !argv.is_empty() {
                self.agent_command = Some(argv);
            }
        }
        if let Ok(path) = std::env::var(ENV_FLEET_REGISTRY)
            && !path.trim().is_empty()
        {
            self.fleet_registry = Some(PathBuf::from(path));
        }
    }
}

/// Whether a config value looks like a plaintext secret rather than an
/// environment reference: it is not a `$VAR`/`${VAR}` reference and either is a
/// long opaque token or contains an obvious credential marker.
pub fn looks_like_plaintext_secret(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || v.starts_with('$') {
        return false; // an env reference is exactly what we want
    }
    // A long, opaque, high-entropy-looking token. A value carrying a path
    // separator is a PATH, not an opaque credential: a Linux context dir
    // (`/home/u/.local/share/...`) fits the token alphabet end to end, and
    // refusing it would silently kill every linked subscription on the one
    // platform whose default paths fit (vincular-suscripciones D4). A real
    // token without separators still lands exactly where it always did.
    let opaque = v.len() >= 20
        && !v.contains('/')
        && !v.contains('\\')
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '='));
    // A common credential prefix carried literally.
    let prefixed = ["sk-", "ghp_", "xoxb-", "AKIA", "Bearer ", "AIza"]
        .iter()
        .any(|p| v.starts_with(p));
    opaque || prefixed
}

fn read_config_file(path: &Path) -> Option<RawConfig> {
    let contents = std::fs::read_to_string(path).ok()?;
    match toml::from_str(&contents) {
        Ok(raw) => Some(raw),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "ignoring invalid config file");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_splitting_handles_quotes() {
        assert_eq!(split_command_line("npx -y agent"), ["npx", "-y", "agent"]);
        assert_eq!(
            split_command_line(r#"cmd "two words" 'single q'"#),
            ["cmd", "two words", "single q"]
        );
        assert!(split_command_line("   ").is_empty());
    }

    #[test]
    fn project_config_overrides_user_config() {
        let dir = std::env::temp_dir().join(format!("meltemi-cfg-{}", std::process::id()));
        let user_dir = dir.join("user");
        let project_root = dir.join("project");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(project_root.join(".meltemi")).unwrap();
        std::fs::write(
            user_dir.join("config.toml"),
            "[agent]\ncommand = \"user-agent --flag\"\n",
        )
        .unwrap();
        std::fs::write(
            project_root.join(".meltemi").join("config.toml"),
            "[agent]\ncommand = [\"project-agent\", \"run\"]\n",
        )
        .unwrap();

        // SAFETY: single-threaded test; no other thread reads the env here.
        unsafe {
            std::env::remove_var(ENV_AGENT_COMMAND);
        }
        let config = Config::load(&user_dir, Some(&project_root));
        assert_eq!(
            config.agent_command.as_deref(),
            Some(["project-agent".to_string(), "run".to_string()].as_slice())
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fleet_keys_and_agent_id_parse_and_merge() {
        let dir = std::env::temp_dir().join(format!("meltemi-cfg-fleet-{}", std::process::id()));
        let user_dir = dir.join("user");
        let project_root = dir.join("project");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(project_root.join(".meltemi")).unwrap();
        std::fs::write(
            user_dir.join("config.toml"),
            "[agent]\nid = \"user-choice\"\n\n\
             [[fleet.custom]]\nid = \"mine\"\ncommand = \"my-agent --acp\"\n\n\
             [[fleet.custom]]\nid = \"other\"\nname = \"Other\"\ncommand = [\"other-agent\"]\n",
        )
        .unwrap();
        std::fs::write(
            project_root.join(".meltemi").join("config.toml"),
            "[agent]\nid = \"project-choice\"\n\n\
             [fleet]\nregistry = \"reg.toml\"\n\n\
             [[fleet.custom]]\nid = \"mine\"\nname = \"Mine v2\"\ncommand = \"my-agent-v2\"\n",
        )
        .unwrap();

        // SAFETY: single-threaded test; no other thread reads the env here.
        unsafe {
            std::env::remove_var(ENV_AGENT_COMMAND);
            std::env::remove_var(ENV_FLEET_REGISTRY);
        }
        let config = Config::load(&user_dir, Some(&project_root));
        // Project id overrides the user id; no command is set.
        assert_eq!(config.agent_command, None);
        assert_eq!(config.agent_id.as_deref(), Some("project-choice"));
        assert_eq!(
            config.fleet_registry.as_deref(),
            Some(Path::new("reg.toml"))
        );
        // Custom agents merge by id: the project redefines `mine`; `other`
        // survives from the user config untouched.
        assert_eq!(config.fleet_custom.len(), 2);
        let mine = config.fleet_custom.iter().find(|c| c.id == "mine").unwrap();
        assert_eq!(mine.name, "Mine v2");
        assert_eq!(mine.command, vec!["my-agent-v2".to_string()]);
        let other = config
            .fleet_custom
            .iter()
            .find(|c| c.id == "other")
            .unwrap();
        assert_eq!(other.name, "Other");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_linked_subscription_loads_first_so_hand_written_config_wins() {
        // Scenario: Lo escrito a mano gana y no se desvincula por superficie
        // (the merge half: the store loads before config.toml, and the
        // merge-by-name rule makes the later, hand-written profile win).
        let dir = std::env::temp_dir().join(format!("meltemi-cfg-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            crate::subscriptions::store_path(&dir),
            "[[fleet.profile]]
name = \"work\"
agent = \"provider-a\"
env = { PROVIDER_CONTEXT_DIR = 'C:/ctx/linked' }
",
        )
        .unwrap();
        std::fs::write(
            dir.join("config.toml"),
            "[[fleet.profile]]
name = \"work\"
agent = \"provider-b\"
env = { PROVIDER_CONTEXT_DIR = 'C:/ctx/manual' }
",
        )
        .unwrap();
        let config = Config::load(&dir, None);
        let work: Vec<_> = config
            .fleet_profiles
            .iter()
            .filter(|p| p.name == "work")
            .collect();
        assert_eq!(work.len(), 1, "merged by name, not appended");
        assert_eq!(work[0].agent, "provider-b", "the hand-written profile wins");
        assert_eq!(work[0].env[0].1, "C:/ctx/manual");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_context_path_is_not_a_plaintext_secret() {
        // Scenario: La ruta de contexto no es un secreto
        // The three platforms' default context paths all survive the lens; a
        // real token without separators still trips it.
        for path in [
            "/home/u/.local/share/meltemi/subscriptions/personal",
            r"C:\Users\u\AppData\Roaming\meltemi\data\subscriptions\personal",
            "/Users/u/Library/Application Support/meltemi/subscriptions/personal",
        ] {
            assert!(
                !looks_like_plaintext_secret(path),
                "a path is not an opaque credential: {path}"
            );
        }
        assert!(
            looks_like_plaintext_secret("sk-abcdefghijklmnopqrstuvwxyz12"),
            "a prefixed token still trips"
        );
        assert!(
            looks_like_plaintext_secret("abcdefghijklmnopqrstuvwx1234"),
            "an opaque separator-less token still trips"
        );

        // And a linked-shaped profile RESOLVES instead of being refused.
        let mut config = Config::default();
        config.apply(
            toml::from_str(
                "[[fleet.profile]]\nname = \"personal\"\nagent = \"provider-a\"\nenv = { PROVIDER_CONTEXT_DIR = '/home/u/.local/share/meltemi/subscriptions/personal' }\n",
            )
            .unwrap(),
        );
        assert_eq!(
            config.fleet_profiles.len(),
            1,
            "{:?}",
            config.fleet_diagnostics
        );
        assert!(config.fleet_diagnostics.is_empty());
    }

    #[test]
    fn two_profiles_on_one_context_are_called_one_subscription() {
        // Scenario: Mismo contexto dos veces se advierte
        let mut config = Config::default();
        config.apply(
            toml::from_str(
                "[[fleet.profile]]\nname = \"work\"\nagent = \"provider-a\"\nenv = { CTX = '/home/u/ctx/shared' }\n\n[[fleet.profile]]\nname = \"spare\"\nagent = \"provider-a\"\nenv = { CTX = '/home/u/ctx/shared' }\n\n[[fleet.profile]]\nname = \"other\"\nagent = \"provider-b\"\nenv = { CTX = '/home/u/ctx/shared' }\n",
            )
            .unwrap(),
        );
        // The duplicate is diagnosed by name, and BOTH keep resolving.
        assert!(
            config.fleet_diagnostics.iter().any(|d| d.contains("`work`")
                && d.contains("`spare`")
                && d.contains("one subscription")),
            "{:?}",
            config.fleet_diagnostics
        );
        assert_eq!(config.fleet_profiles.len(), 3, "nothing is refused");
        // A different agent on the same path is NOT the same subscription.
        assert!(
            !config
                .fleet_diagnostics
                .iter()
                .any(|d| d.contains("`other`")),
            "{:?}",
            config.fleet_diagnostics
        );
    }

    #[test]
    fn fleet_profiles_merge_and_refuse_plaintext_secrets() {
        // Scenario: Secreto en claro rehusado por higiene
        // flota-multiproveedor: a clean profile is kept; a profile whose env
        // looks like a plaintext secret is REFUSED with a ${VAR} remedy (never
        // the value).
        let dir = std::env::temp_dir().join(format!("meltemi-prof-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            "[[fleet.profile]]\nname = \"work\"\nagent = \"gemini-cli\"\nenv = { HOME = \"${WORK_HOME}\" }\n\n\
             [[fleet.profile]]\nname = \"leaky\"\nagent = \"gemini-cli\"\nenv = { TOKEN = \"sk-abcdefghijklmnopqrstuvwxyz\" }\n",
        )
        .unwrap();
        let config = Config::load(&dir, None);
        // The clean profile survives; the secret-looking one is refused.
        assert_eq!(
            config.fleet_profiles.len(),
            1,
            "only the clean profile is kept"
        );
        assert_eq!(config.fleet_profiles[0].name, "work");
        assert!(
            config
                .fleet_diagnostics
                .iter()
                .any(|d| d.contains("leaky") && d.contains("${")),
            "the remedy names the ${{VAR}} form: {:?}",
            config.fleet_diagnostics
        );
        assert!(
            !config
                .fleet_diagnostics
                .iter()
                .any(|d| d.contains("sk-abcdefghij")),
            "the value never appears in a diagnostic"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn custom_name_defaults_to_its_id() {
        let dir = std::env::temp_dir().join(format!("meltemi-cfg-name-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            "[[fleet.custom]]\nid = \"bare\"\ncommand = \"bare-agent\"\n",
        )
        .unwrap();
        // SAFETY: single-threaded test.
        unsafe {
            std::env::remove_var(ENV_AGENT_COMMAND);
            std::env::remove_var(ENV_FLEET_REGISTRY);
        }
        let config = Config::load(&dir, None);
        assert_eq!(config.fleet_custom[0].name, "bare");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mcp_servers_merge_by_name_and_lint_secrets() {
        // Scenarios: Servidor declarado una vez; Nombre repetido por ámbito;
        // Secreto en claro detectado (mcp-passthrough).
        let dir = std::env::temp_dir().join(format!("meltemi-mcp-{}", std::process::id()));
        let user_dir = dir.join("user");
        let project_root = dir.join("project");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(project_root.join(".meltemi")).unwrap();
        std::fs::write(
            user_dir.join("config.toml"),
            "[[mcp.servers]]\nname = \"fs\"\ncommand = \"user-fs\"\n\n\
             [[mcp.servers]]\nname = \"leaky\"\ncommand = \"x\"\n[mcp.servers.env]\nTOKEN = \"sk-abcdefghijklmnopqrstuvwxyz\"\n",
        )
        .unwrap();
        std::fs::write(
            project_root.join(".meltemi").join("config.toml"),
            "[[mcp.servers]]\nname = \"fs\"\ncommand = \"project-fs\"\n[mcp.servers.env]\nKEY = \"$MY_KEY\"\n",
        )
        .unwrap();
        // SAFETY: single-threaded test.
        unsafe {
            std::env::remove_var(ENV_AGENT_COMMAND);
        }
        let config = Config::load(&user_dir, Some(&project_root));

        // Project overrides the user's `fs` by name; `leaky` survives.
        let fs = config.mcp_servers.iter().find(|s| s.name == "fs").unwrap();
        match &fs.transport {
            McpTransport::Stdio { command, env, .. } => {
                assert_eq!(command, "project-fs", "project wins by name");
                assert_eq!(
                    env[0],
                    ("KEY".into(), "$MY_KEY".into()),
                    "env is a reference"
                );
            }
            _ => panic!("stdio"),
        }
        // The plaintext token was flagged, and its value never entered the
        // diagnostic text.
        assert!(
            config
                .mcp_diagnostics
                .iter()
                .any(|d| d.contains("leaky") && d.contains("plaintext")),
            "the plaintext secret is flagged: {:?}",
            config.mcp_diagnostics
        );
        assert!(
            !config
                .mcp_diagnostics
                .iter()
                .any(|d| d.contains("sk-abcdefghijklmnop")),
            "the secret value must never appear in a diagnostic"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn secret_detector_positives_and_negatives() {
        assert!(looks_like_plaintext_secret("sk-1234567890abcdef1234"));
        assert!(looks_like_plaintext_secret("ghp_aaaaaaaaaaaaaaaaaaaa"));
        assert!(looks_like_plaintext_secret("AKIAIOSFODNN7EXAMPLE"));
        assert!(looks_like_plaintext_secret(
            "a-very-long-opaque-token-value-1234"
        ));
        // References and short/obvious non-secrets are not flagged.
        assert!(!looks_like_plaintext_secret("$MY_TOKEN"));
        assert!(!looks_like_plaintext_secret("${MY_TOKEN}"));
        assert!(!looks_like_plaintext_secret("./local/path"));
        assert!(!looks_like_plaintext_secret("short"));
    }

    #[test]
    fn defaults_to_no_command() {
        let empty = std::env::temp_dir().join(format!("meltemi-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();
        // SAFETY: single-threaded test.
        unsafe {
            std::env::remove_var(ENV_AGENT_COMMAND);
        }
        let config = Config::load(&empty, None);
        assert_eq!(config.agent_command, None);
        std::fs::remove_dir_all(&empty).ok();
    }
}

#[cfg(test)]
mod espera_humana_tests {
    use super::*;

    fn parse(toml: &str) -> Config {
        let dir = std::env::temp_dir().join(format!(
            "meltemi-cfg-wait-{}-{}",
            std::process::id(),
            toml.len()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.toml"), toml).unwrap();
        let config = Config::load(&dir, None);
        std::fs::remove_dir_all(&dir).ok();
        config
    }

    #[test]
    fn defaults_wait_for_the_human_and_bound_implement() {
        let config = parse("");
        assert_eq!(config.interactive_wait(), WaitPolicy::WhileConnected);
        assert_eq!(config.autonomous_wait(), WaitPolicy::Bounded(30));
        assert_eq!(config.no_client_grace(), Duration::from_secs(30));
        assert!(config.permission_diagnostics.is_empty());
    }

    #[test]
    fn bounded_and_mode_values_parse() {
        let config = parse(
            "[permissions]\nwait = 300\nimplement-wait = \"while-connected\"\nno-client-grace = 0\n",
        );
        assert_eq!(config.interactive_wait(), WaitPolicy::Bounded(300));
        assert_eq!(config.autonomous_wait(), WaitPolicy::WhileConnected);
        assert_eq!(config.no_client_grace(), Duration::ZERO);
        assert!(config.permission_diagnostics.is_empty());
    }

    #[test]
    fn invalid_values_keep_defaults_with_a_diagnostic() {
        let config =
            parse("[permissions]\nwait = \"forever\"\nimplement-wait = 0\nno-client-grace = -5\n");
        assert_eq!(config.interactive_wait(), WaitPolicy::WhileConnected);
        assert_eq!(config.autonomous_wait(), WaitPolicy::Bounded(30));
        assert_eq!(config.no_client_grace(), Duration::from_secs(30));
        assert_eq!(
            config.permission_diagnostics.len(),
            3,
            "{:?}",
            config.permission_diagnostics
        );
    }
}
