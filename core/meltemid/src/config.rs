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

use serde::Deserialize;

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
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    agent: RawAgent,
    #[serde(default)]
    fleet: RawFleet,
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
