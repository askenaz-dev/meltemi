// SPDX-License-Identifier: Apache-2.0

//! Minimal configuration (design D13).
//!
//! Precedence, low to high: built-in defaults < user config
//! (`<config_dir>/config.toml`) < project config
//! (`<project_root>/.meltemi/config.toml`) < `MELTEMI_*` environment
//! variables. CLI flags (highest) are applied by the binaries, not here.
//!
//! Fase 0 only needs the agent command; the shape is intentionally small.

use std::path::Path;

use serde::Deserialize;

/// Environment override for the agent command (a full command line, parsed
/// with shell-words semantics by the ACP layer).
pub const ENV_AGENT_COMMAND: &str = "MELTEMI_AGENT_COMMAND";

/// Resolved daemon configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// The agent command line, e.g. `["npx", "-y", "some-acp-agent"]`.
    /// `None` until the user configures one; requesting a session without it
    /// yields error 2000 (`agent_command_not_configured`).
    pub agent_command: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    agent: RawAgent,
}

#[derive(Debug, Default, Deserialize)]
struct RawAgent {
    /// The agent command, either as a single string (`"npx -y agent"`) or an
    /// explicit argv array (`["npx", "-y", "agent"]`).
    #[serde(default)]
    command: Option<CommandSpec>,
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
    }

    fn apply_env(&mut self) {
        if let Ok(line) = std::env::var(ENV_AGENT_COMMAND) {
            let argv = split_command_line(&line);
            if !argv.is_empty() {
                self.agent_command = Some(argv);
            }
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
