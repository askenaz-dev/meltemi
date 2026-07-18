// SPDX-License-Identifier: Apache-2.0

//! Argument grammar and dispatch planning (design D1, D2).
//!
//! [`plan`] is a pure function of the arguments and whether stdout is a TTY, so
//! the whole grammar and the CLI↔TUI dispatch rule are unit-testable without a
//! terminal or a daemon. It never performs I/O.

/// Subcommands reserved for the SDD authoring cycle (#14+): recognized by the
/// grammar so it stays stable, but not implemented in this change.
pub const RESERVED: &[&str] = &["implement", "verify", "archive"];

/// Help text for the scriptable surface.
pub const USAGE: &str = "\
meltemi — spec-driven control plane for coding agents

USAGE:
    meltemi [--json] <subcommand> [args]
    meltemi                       launch the interactive interface

SUBCOMMANDS:
    status              show daemon version, uptime and active sessions
    propose <idea>      scaffold a change proposal and delegate it to an agent
    fleet               list the agent fleet catalog (detection and levels)
    project             regenerate the projected context (AGENTS.md, ...)
    sessions            list agent sessions (active and historical)
    explore <topic>     deliberate with the agent without writing
    plan <change>       refine design and sequence a change's tasks
    constitution        create or edit the project constitution
    review <change>     review a change's spec deltas as a checklist
    stop                request an orderly daemon shutdown
    version             print the client version
    help                print this help

GLOBAL FLAGS:
    --json              emit machine-readable JSON on stdout
    -h, --help          print this help
    -V, --version       print the client version

RESERVED (not yet implemented):
    implement, verify, archive";

/// An RPC-backed or local subcommand to run in scriptable mode.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Query daemon status.
    Status,
    /// Scaffold a change proposal and delegate it to an agent.
    Propose {
        idea: String,
        project_root: Option<String>,
    },
    /// List the fleet catalog (`fleet/list`).
    Fleet,
    /// Regenerate the projected context (`context/project`).
    Project { project_root: Option<String> },
    /// List sessions, active and historical (`session/list`).
    Sessions { project_root: Option<String> },
    /// Deliberate with the agent without writing (`sdd/explore`).
    Explore { topic: String },
    /// Refine design and sequence a change's tasks (`sdd/plan`).
    Plan { change: String },
    /// Create/edit the project constitution (`sdd/constitution`).
    Constitution { topic: String },
    /// Review a change's spec deltas as a checklist (`sdd/review`).
    Review { change: String },
    /// Request an orderly daemon shutdown.
    Stop,
    /// A reserved subcommand recognized by the grammar but not yet implemented.
    Reserved(String),
}

/// The resolved action for an invocation.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Print help and exit successfully.
    Help,
    /// Print the client version and exit successfully.
    Version,
    /// Enter the interactive interface (deferred in this change).
    Interactive,
    /// Run a scriptable subcommand.
    Run(Command),
    /// A usage error with a human message.
    Usage(String),
}

/// A fully resolved invocation plan.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub action: Action,
    pub json: bool,
}

/// Resolves an invocation from its arguments (without the program name) and
/// whether stdout is connected to a TTY. Pure: no I/O, no environment reads.
#[must_use]
pub fn plan(args: &[String], stdout_is_tty: bool) -> Plan {
    let mut json = false;
    let mut want_help = false;
    let mut want_version = false;
    let mut positionals: Vec<&str> = Vec::new();
    let mut end_of_flags = false;

    for arg in args {
        if end_of_flags {
            positionals.push(arg.as_str());
            continue;
        }
        match arg.as_str() {
            "--" => end_of_flags = true,
            "--json" => json = true,
            "--help" | "-h" => want_help = true,
            "--version" | "-V" => want_version = true,
            flag if flag.starts_with('-') && flag != "-" => {
                return Plan {
                    action: Action::Usage(format!("unknown flag `{flag}`; run `meltemi help`")),
                    json,
                };
            }
            positional => positionals.push(positional),
        }
    }

    // Help and version flags win over any subcommand, as is conventional.
    if want_help {
        return Plan {
            action: Action::Help,
            json,
        };
    }
    if want_version {
        return Plan {
            action: Action::Version,
            json,
        };
    }

    let action = match positionals.split_first() {
        None => {
            // A bare invocation goes interactive only with a TTY and without
            // `--json` (which signals machine use). Otherwise it is a usage
            // error — never a hang waiting on input.
            if stdout_is_tty && !json {
                Action::Interactive
            } else {
                Action::Usage("no subcommand given; run `meltemi help`".into())
            }
        }
        Some((subcommand, rest)) => plan_subcommand(subcommand, rest),
    };

    Plan { action, json }
}

/// Maps a subcommand and its remaining positionals to an action.
fn plan_subcommand(subcommand: &str, rest: &[&str]) -> Action {
    match subcommand {
        "help" => Action::Help,
        "version" => Action::Version,
        "status" if rest.is_empty() => Action::Run(Command::Status),
        "status" => Action::Usage("`status` takes no arguments".into()),
        "fleet" if rest.is_empty() => Action::Run(Command::Fleet),
        "fleet" => Action::Usage("`fleet` takes no arguments".into()),
        "project" => match rest {
            [] => Action::Run(Command::Project { project_root: None }),
            [root] => Action::Run(Command::Project {
                project_root: Some((*root).to_string()),
            }),
            _ => Action::Usage("`project` takes at most a project root".into()),
        },
        "sessions" => match rest {
            [] => Action::Run(Command::Sessions { project_root: None }),
            [root] => Action::Run(Command::Sessions {
                project_root: Some((*root).to_string()),
            }),
            _ => Action::Usage("`sessions` takes at most a project root".into()),
        },
        "explore" => match rest {
            [] => Action::Usage("`explore` requires a topic: meltemi explore \"<topic>\"".into()),
            [topic] => Action::Run(Command::Explore {
                topic: (*topic).to_string(),
            }),
            _ => Action::Usage("`explore` takes a single quoted topic".into()),
        },
        "plan" => match rest {
            [change] => Action::Run(Command::Plan {
                change: (*change).to_string(),
            }),
            _ => Action::Usage("`plan` requires a change name: meltemi plan <change>".into()),
        },
        "constitution" => Action::Run(Command::Constitution {
            topic: rest.first().map(|s| (*s).to_string()).unwrap_or_default(),
        }),
        "review" => match rest {
            [change] => Action::Run(Command::Review {
                change: (*change).to_string(),
            }),
            _ => Action::Usage("`review` requires a change name: meltemi review <change>".into()),
        },
        "stop" if rest.is_empty() => Action::Run(Command::Stop),
        "stop" => Action::Usage("`stop` takes no arguments".into()),
        "propose" => match rest {
            [] => Action::Usage(
                "`propose` requires an idea: meltemi propose \"<idea>\" [project-root]".into(),
            ),
            [idea] => Action::Run(Command::Propose {
                idea: (*idea).to_string(),
                project_root: None,
            }),
            [idea, root] => Action::Run(Command::Propose {
                idea: (*idea).to_string(),
                project_root: Some((*root).to_string()),
            }),
            _ => Action::Usage("`propose` takes at most an idea and a project root".into()),
        },
        other if RESERVED.contains(&other) => Action::Run(Command::Reserved(other.to_string())),
        other => Action::Usage(format!("unknown subcommand `{other}`; run `meltemi help`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn plan_of(items: &[&str], tty: bool) -> Plan {
        plan(&args(items), tty)
    }

    #[test]
    fn known_subcommand_is_scriptable_even_without_tty() {
        // Scenario: Con subcomando siempre scriptable.
        let p = plan_of(&["status"], false);
        assert_eq!(p.action, Action::Run(Command::Status));
        assert!(!p.json);
    }

    #[test]
    fn bare_invocation_with_tty_is_interactive() {
        // Scenario: Invocación desnuda con TTY entra al modo interactivo.
        assert_eq!(plan_of(&[], true).action, Action::Interactive);
    }

    #[test]
    fn bare_invocation_without_tty_is_usage_error() {
        // Scenario: Invocación desnuda sin TTY es error de uso.
        assert!(matches!(plan_of(&[], false).action, Action::Usage(_)));
    }

    #[test]
    fn bare_invocation_with_json_is_never_interactive() {
        // `--json` signals machine use: no interactive fallback.
        assert!(matches!(
            plan_of(&["--json"], true).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn unknown_subcommand_is_usage_error() {
        // Scenario: Subcomando desconocido.
        assert!(matches!(
            plan_of(&["frobnicate"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn reserved_subcommand_is_recognized_not_usage_error() {
        // Scenario: Subcomando reservado no es error de uso.
        for reserved in RESERVED {
            let p = plan_of(&[reserved], false);
            assert_eq!(
                p.action,
                Action::Run(Command::Reserved((*reserved).to_string())),
                "`{reserved}` must be recognized as reserved"
            );
        }
    }

    #[test]
    fn status_rejects_extra_arguments() {
        assert!(matches!(
            plan_of(&["status", "x"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn project_is_operational_with_an_optional_root() {
        // Scenario: project regenera la proyección.
        assert_eq!(
            plan_of(&["project"], false).action,
            Action::Run(Command::Project { project_root: None })
        );
        assert_eq!(
            plan_of(&["project", "/repo"], false).action,
            Action::Run(Command::Project {
                project_root: Some("/repo".into())
            })
        );
        let p = plan_of(&["--json", "project"], false);
        assert!(p.json);
        assert!(matches!(
            plan_of(&["project", "a", "b"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn sessions_is_operational_with_an_optional_root() {
        // Scenario: sessions consulta el histórico.
        assert_eq!(
            plan_of(&["sessions"], false).action,
            Action::Run(Command::Sessions { project_root: None })
        );
        assert_eq!(
            plan_of(&["sessions", "/repo"], false).action,
            Action::Run(Command::Sessions {
                project_root: Some("/repo".into())
            })
        );
        assert!(plan_of(&["--json", "sessions"], false).json);
    }

    #[test]
    fn fleet_is_an_operational_subcommand_without_arguments() {
        // Scenario: Subcomando operativo reconocido (fleet).
        let p = plan_of(&["fleet"], false);
        assert_eq!(p.action, Action::Run(Command::Fleet));
        assert!(!p.json);
        // The --json variant is captured like any global flag.
        let p = plan_of(&["--json", "fleet"], false);
        assert_eq!(p.action, Action::Run(Command::Fleet));
        assert!(p.json);
        assert!(matches!(
            plan_of(&["fleet", "x"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn propose_requires_an_idea() {
        assert!(matches!(
            plan_of(&["propose"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn propose_takes_idea_and_optional_root() {
        assert_eq!(
            plan_of(&["propose", "add auth"], false).action,
            Action::Run(Command::Propose {
                idea: "add auth".into(),
                project_root: None,
            })
        );
        assert_eq!(
            plan_of(&["propose", "add auth", "/repo"], false).action,
            Action::Run(Command::Propose {
                idea: "add auth".into(),
                project_root: Some("/repo".into()),
            })
        );
    }

    #[test]
    fn json_flag_is_captured_with_a_subcommand() {
        // Scenario: éxito en JSON (grammar half): the flag reaches the plan.
        let p = plan_of(&["--json", "status"], false);
        assert_eq!(p.action, Action::Run(Command::Status));
        assert!(p.json);
    }

    #[test]
    fn help_and_version_flags_and_subcommands() {
        for h in [&["help"][..], &["--help"][..], &["-h"][..]] {
            assert_eq!(plan(&args(h), false).action, Action::Help);
        }
        for v in [&["version"][..], &["--version"][..], &["-V"][..]] {
            assert_eq!(plan(&args(v), false).action, Action::Version);
        }
    }

    #[test]
    fn help_flag_wins_over_a_subcommand() {
        assert_eq!(plan_of(&["status", "--help"], false).action, Action::Help);
    }

    #[test]
    fn unknown_flag_is_usage_error() {
        assert!(matches!(
            plan_of(&["--nope"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn double_dash_ends_flag_parsing() {
        // After `--`, flag-looking tokens become positionals.
        assert_eq!(
            plan_of(&["propose", "--", "--weird-idea"], false).action,
            Action::Run(Command::Propose {
                idea: "--weird-idea".into(),
                project_root: None,
            })
        );
    }
}
