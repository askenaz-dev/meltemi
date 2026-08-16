// SPDX-License-Identifier: Apache-2.0

//! Argument grammar and dispatch planning (design D1, D2).
//!
//! [`plan`] is a pure function of the arguments and whether stdout is a TTY, so
//! the whole grammar and the CLI↔TUI dispatch rule are unit-testable without a
//! terminal or a daemon. It never performs I/O.

/// Subcommands reserved for the SDD authoring cycle. The cycle is now fully
/// operative (comando-implement un-reserved `implement`), so nothing remains
/// reserved; the list is kept for the grammar's forward-compatibility.
use meltemi_proto::AutonomyMode;

use crate::format::Format;

pub const RESERVED: &[&str] = &[];

/// Help text for the scriptable surface.
pub const USAGE: &str = "\
meltemi — spec-driven control plane for coding agents

USAGE:
    meltemi [--json] <subcommand> [args]
    meltemi                       launch the interactive interface

SUBCOMMANDS:
    status              show daemon version, uptime and active sessions
    propose <idea> [project-root] [--agent <id|profile>]
                        scaffold a change proposal and delegate it to an agent
    fleet               list the agent fleet catalog (detection and levels)
    link <agent> <name> link a subscription of a catalog agent: creates the
                        launch profile and prints the login gesture to run
    unlink <name>       unlink a subscription; its auth context is not deleted
    project             regenerate the projected context (AGENTS.md, ...)
    session <instruction> [project-root] [--agent <id|profile>] [--mode <mode>]
                        start a free session on that project: no change, no
                        spec and no gate, and nothing of the government
                        relaxed — the fleet resolves the agent, permissions go
                        through the proxy, the log is append-only, and a
                        restore point is taken before the first turn (or its
                        absence is declared with the remedy that fits).
                        It WAITS for the turn and prints its outcome; the
                        session ends with it. Staying between turns is what the
                        desktop surface does, and it needs the event stream this
                        command has no way to read.
                        `--mode manual|semi|autonomous` says how much the
                        session decides on its own; absent, your permission
                        rules decide exactly as they always did. In every mode
                        an explicit deny of yours prevails and anything
                        irreversible escalates
    sessions            list agent sessions (active and historical)
    explore <topic> [--agent <id|profile>]
                        deliberate with the agent without writing
    plan <change>       refine design and sequence a change's tasks
    constitution        create or edit the project constitution
    review <change>     review a change's spec deltas as a checklist
    worktrees           list the worktrees Meltemi manages for a project
    assign <change> <task> <agents>
                        create an isolated worktree per agent from a common
                        base (comma-separate agents to race them on one task)
    race <change> <task>
                        show each competitor's diff against the common base
    dispatch <change> <task> <agent|profile>
                        run one competitor's turn over its worktree with that
                        agent's own binary (checkpoint -> turn -> commit); the
                        task is never ticked
    apply-edit <file> [<change> <task> <agent>] [confirm]
                        apply a human edit read from stdin through the daemon,
                        onto the project tree or a managed worktree; traceable
                        (human_edit). A running session or in-flight turn on
                        the tree demands the trailing `confirm` (soft lock)
    checkpoints [change]
                        list pre-task checkpoints (ref, moment, irreversibles)
    revert <change> <task> <agent> [confirm]
                        revert a task's worktree to its checkpoint; without the
                        trailing `confirm` it previews the scope (what won't undo)
    commit <change> <task> <agent> <title> [confirm]
                        the atomic per-task commit with traceability trailers;
                        without `confirm` it previews the message and diff
    verify <change>     the per-requirement verification checklist of a change
    archive <change> [confirm]
                        fold a verified change's deltas into the living truth
    projects            list the projects Meltemi has been pointed at
    projects register <path>
                        add a directory to that list: the daemon checks it
                        exists, canonicalizes it and creates nothing inside it
    projects forget <path>
                        drop it from the listing and nothing else — no file, no
                        session and no log is touched, and it comes back the
                        moment the project is used or registered again
    usage [day|week|month|total] [--project <root>|--all] [--since <ts>] [--until <ts>]
                        local usage accounting folded from the session records;
                        tokens only where the agent's official output reports
                        them, never estimated (`--all` spans every project)
    changes             list changes (active and archived) with their state
    show <change>       show a change: its artifacts and per-capability deltas
    specs [capability]  list living-truth capabilities, or show one
    validate [change]   validate a change or the living truth (exit 14 on findings)
    implement <change> <agent> [plan]
                        deploy the agent over the change's tasks.md task by task
                        (checkpoint → turn → commit → tick); `plan` previews
    direct <session> <instruction>
                        steer an existing session: queue the instruction as an
                        active session's next turn, or resume a resumable one
    tunnel [user@host] [--exec]
                        compose the `ssh` command that reverse-forwards this
                        daemon's endpoint to a remote host; `--exec` runs it
    bridge              pump this daemon's local endpoint over stdio: the last
                        metre of remote access (`ssh <pc> meltemi bridge` is a
                        complete channel, named pipes included)
    stop                request an orderly daemon shutdown
    version             print the client version
    help                print this help

GLOBAL FLAGS:
    --json              emit machine-readable JSON on stdout
    --yaml              emit machine-readable YAML on stdout (one of --json/--yaml)
    --no-color          render without colour (also NO_COLOR, TERM=dumb, or a
                        non-terminal stdout)
    -h, --help          print this help
    -V, --version       print the client version";

/// Renders the CLI reference markdown from the grammar ([`USAGE`]) and the
/// exit-code taxonomy ([`crate::exit::EXIT_CODES`]) — a single source, so the
/// committed reference cannot silently drift from the code (documentacion-
/// inicial). The docs freshness test regenerates this and compares.
#[must_use]
pub fn reference() -> String {
    let mut out = String::new();
    out.push_str("<!-- SPDX-License-Identifier: Apache-2.0 -->\n");
    out.push_str("<!-- GENERATED from the CLI grammar by `meltemi::cli::reference()`. Do not edit by hand; regenerate. -->\n\n");
    out.push_str("# CLI reference\n\n");
    out.push_str("The scriptable surface of `meltemi`. This document is generated from the\n");
    out.push_str("subcommand grammar and the exit-code taxonomy in the source.\n\n");
    out.push_str("## Grammar\n\n```\n");
    out.push_str(USAGE);
    out.push_str("\n```\n\n");
    out.push_str("## Exit codes\n\n");
    for (code, description) in crate::exit::EXIT_CODES {
        out.push_str(&format!("- `{code}` — {description}\n"));
    }
    out
}

/// An RPC-backed or local subcommand to run in scriptable mode.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Query daemon status.
    Status,
    /// Scaffold a change proposal and delegate it to an agent.
    Propose {
        idea: String,
        project_root: Option<String>,
        /// The fleet agent to draft it (`--agent`); `None` is the project's
        /// configured agent, exactly as before the flag existed.
        agent: Option<String>,
    },
    /// Start a free session on a project (`session/start`): an instruction and
    /// a root, with no change, no spec and no gate.
    Session {
        instruction: String,
        project_root: Option<String>,
        agent: Option<String>,
        /// How much the session decides on its own (`--mode`); `None` composes
        /// nothing, which is the behaviour that existed before the flag.
        mode: Option<AutonomyMode>,
    },
    /// List the fleet catalog (`fleet/list`).
    Fleet,
    /// Link a subscription of a catalog agent (`subscription/link`).
    Link { agent: String, name: String },
    /// Unlink a subscription (`subscription/unlink`).
    Unlink { name: String },
    /// Regenerate the projected context (`context/project`).
    Project { project_root: Option<String> },
    /// List sessions, active and historical (`session/list`).
    Sessions { project_root: Option<String> },
    /// Deliberate with the agent without writing (`sdd/explore`).
    Explore {
        topic: String,
        agent: Option<String>,
    },
    /// Refine design and sequence a change's tasks (`sdd/plan`).
    Plan { change: String },
    /// Create/edit the project constitution (`sdd/constitution`).
    Constitution { topic: String },
    /// Review a change's spec deltas as a checklist (`sdd/review`).
    Review { change: String },
    /// List the worktrees the daemon manages (`worktree/list`).
    Worktrees { project_root: Option<String> },
    /// List the registered projects (`project/list`).
    Projects,
    /// Add a directory to the project registry (`project/register`).
    ProjectRegister { path: String },
    /// Drop a project from the registry's listing (`project/forget`).
    ProjectForget { path: String },
    /// Local usage accounting over the session records (`analytics/usage`).
    /// `scope` is `all` or a project root; `granularity` is day|week|month|total.
    Usage {
        project_root: Option<String>,
        granularity: Option<String>,
        since: Option<String>,
        until: Option<String>,
    },
    /// Assign a task to one or more agents in isolated worktrees from a common
    /// base (`worktree/assign`); more than one agent races them.
    Assign {
        change: String,
        task: String,
        agents: Vec<String>,
        project_root: Option<String>,
    },
    /// Show each competitor's diff against the common base (`worktree/diff`).
    Race {
        change: String,
        task: String,
        project_root: Option<String>,
    },
    /// Run one competitor's turn over its worktree (`worktree/dispatch`).
    Dispatch {
        change: String,
        task: String,
        agent: String,
        project_root: Option<String>,
    },
    ApplyEdit {
        file: String,
        /// Managed-worktree addressing: (change, task, agent).
        target: Option<(String, String, String)>,
        confirm: bool,
    },
    /// List pre-task checkpoints (`checkpoint/list`).
    Checkpoints { change: Option<String> },
    /// Revert a task's worktree to its checkpoint (`checkpoint/revert`);
    /// `confirm` false previews the honest scope without mutating.
    Revert {
        change: String,
        task: String,
        agent: String,
        confirm: bool,
    },
    /// The atomic per-task commit (`commit/task`); `confirm` false previews the
    /// message and diff without committing.
    Commit {
        change: String,
        task: String,
        agent: String,
        title: String,
        confirm: bool,
    },
    /// The per-requirement verification checklist of a change (`sdd/verify`).
    Verify { change: String },
    /// Fold a verified change's deltas into the living truth (`sdd/archive`);
    /// `confirm` allows folding over a dirty specs tree.
    Archive { change: String, confirm: bool },
    /// Deploy an agent over a change's `tasks.md` (`sdd/implement`); `plan_only`
    /// previews the sequence without acting.
    Implement {
        change: String,
        agent: String,
        plan_only: bool,
    },
    /// List changes with aggregated state (`change/list`).
    Changes,
    /// Show a change: artifacts and per-capability deltas (`change/show`).
    Show { change: String },
    /// List living-truth capabilities (`spec/list`), or show one (`spec/show`).
    Specs { capability: Option<String> },
    /// Validate a change or the living truth without archiving (`sdd/validate`).
    Validate { change: Option<String> },
    /// Steer an existing session (`session/direct`): queue an instruction as an
    /// active session's next turn, or resume a resumable one.
    Direct {
        session: String,
        instruction: String,
        project_root: Option<String>,
    },
    /// Compose (or, with `--exec`, run) the `ssh` reverse-forward that exposes
    /// this daemon's endpoint to a remote host. Local: touches no daemon.
    Tunnel { target: Option<String>, exec: bool },
    /// Pump the local daemon endpoint over stdio (acceso-remoto-en-dos-vias).
    Bridge,
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
    /// The single output-format choice (salida-que-se-lee D1).
    pub format: Format,
    /// The explicit `--no-color`; the other three signals are read from the
    /// environment where the decision is taken, not here (this stays pure).
    pub no_color: bool,
}

impl Plan {
    /// Whether the caller asked for a machine-readable format.
    #[must_use]
    pub fn is_machine(&self) -> bool {
        self.format.is_machine()
    }
}

/// The single format choice from the two machine flags. `Err` is the
/// contradiction: two machine formats at once mean nothing (design D1).
fn resolve_format(json: bool, yaml: bool) -> Result<Format, &'static str> {
    match (json, yaml) {
        (true, true) => Err("`--json` and `--yaml` ask for two machine formats at once; pick one"),
        (true, false) => Ok(Format::Json),
        (false, true) => Ok(Format::Yaml),
        (false, false) => Ok(Format::Human),
    }
}

/// Resolves an invocation from its arguments (without the program name) and
/// whether stdout is connected to a TTY. Pure: no I/O, no environment reads.
#[must_use]
pub fn plan(args: &[String], stdout_is_tty: bool) -> Plan {
    let mut json = false;
    let mut yaml = false;
    let mut no_color = false;
    let mut want_help = false;
    let mut want_version = false;
    let mut exec = false;
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
            "--yaml" => yaml = true,
            "--no-color" => no_color = true,
            "--help" | "-h" => want_help = true,
            "--version" | "-V" => want_version = true,
            // Subcommand-local, but recognized here so the global flag parser
            // does not reject it wherever it appears (only `tunnel` reads it).
            "--exec" => exec = true,
            // Subcommand-local flags that flow through as positionals for their
            // own parser to read. Declared here because the global parser is
            // strict about unknown flags — and a rejected flag would have made
            // the advertised `usage --all` unusable.
            "--all" | "--project" | "--since" | "--until" | "--agent" | "--mode" => {
                positionals.push(arg.as_str());
            }
            flag if flag.starts_with('-') && flag != "-" => {
                // The format so far still decides how this error is rendered:
                // `--json --bogus` must answer with a JSON error object.
                return Plan {
                    action: Action::Usage(format!("unknown flag `{flag}`; run `meltemi help`")),
                    format: resolve_format(json, yaml).unwrap_or(Format::Human),
                    no_color,
                };
            }
            positional => positionals.push(positional),
        }
    }

    // One format, decided once. `--json --yaml` is a state that means nothing,
    // so it is refused here rather than resolved differently at each call site
    // (design D1).
    let format = match resolve_format(json, yaml) {
        Ok(format) => format,
        Err(message) => {
            return Plan {
                action: Action::Usage(message.into()),
                format: Format::Human,
                no_color,
            };
        }
    };

    // Help and version flags win over any subcommand, as is conventional.
    if want_help {
        return Plan {
            action: Action::Help,
            format,
            no_color,
        };
    }
    if want_version {
        return Plan {
            action: Action::Version,
            format,
            no_color,
        };
    }

    // `--agent` is read once, here, wherever it landed on the line.
    let agent = match take_agent(&mut positionals) {
        Ok(agent) => agent,
        Err(message) => {
            return Plan {
                action: Action::Usage(message),
                format,
                no_color,
            };
        }
    };

    // `--mode` is read the same way and in the same place as `--agent`.
    let mode = match take_mode(&mut positionals) {
        Ok(mode) => mode,
        Err(message) => {
            return Plan {
                action: Action::Usage(message),
                format,
                no_color,
            };
        }
    };

    let action = match positionals.split_first() {
        None => {
            // A bare invocation goes interactive only with a TTY and without
            // `--json` (which signals machine use). Otherwise it is a usage
            // error — never a hang waiting on input.
            if stdout_is_tty && !format.is_machine() {
                Action::Interactive
            } else {
                Action::Usage("no subcommand given; run `meltemi help`".into())
            }
        }
        Some((subcommand, rest)) => plan_subcommand(subcommand, rest, exec, agent, mode),
    };

    Plan {
        action,
        format,
        no_color,
    }
}

/// Lifts `--agent <value>` out of the collected positionals, wherever on the
/// line it was written.
///
/// The flag is declared in the global loop like `--all` and friends, because a
/// strict parser that rejected it would make the grammar the help advertises
/// unusable. Reading it here rather than inside each subcommand is what makes
/// `--agent` position-independent — a flag that only works to the right of the
/// instruction is a half-promise — and it keeps one parse instead of three
/// copies of the same scan. What the subcommand gets is the value, verbatim:
/// nothing lowercases an agent name on the way to the daemon.
fn take_agent(positionals: &mut Vec<&str>) -> Result<Option<String>, String> {
    let Some(index) = positionals.iter().position(|token| *token == "--agent") else {
        return Ok(None);
    };
    match positionals.get(index + 1) {
        // A flag where the value belongs means the value is missing: refuse,
        // never consume the next flag as if it were an agent name.
        Some(value) if !value.starts_with('-') => {
            let value = (*value).to_string();
            positionals.drain(index..=index + 1);
            Ok(Some(value))
        }
        _ => Err("`--agent` requires an agent id or launch profile name".into()),
    }
}

/// Reads `--mode`, refusing an unknown name **with the valid ones**.
///
/// A mode that does not exist is never degraded into one that does: the whole
/// point of the flag is to say how much the session decides on its own, and
/// guessing that would be the worst possible guess (modos-de-autonomia design
/// D6).
fn take_mode(positionals: &mut Vec<&str>) -> Result<Option<AutonomyMode>, String> {
    let Some(index) = positionals.iter().position(|token| *token == "--mode") else {
        return Ok(None);
    };
    let valid = AutonomyMode::ALL
        .iter()
        .map(|mode| mode.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    match positionals.get(index + 1) {
        Some(value) if !value.starts_with('-') => match AutonomyMode::parse(value) {
            Some(mode) => {
                positionals.drain(index..=index + 1);
                Ok(Some(mode))
            }
            None => Err(format!("`{value}` is not a mode; the modes are: {valid}")),
        },
        _ => Err(format!("`--mode` requires one of: {valid}")),
    }
}

/// Maps a subcommand and its remaining positionals to an action. `exec` carries
/// the global `--exec` flag (only `tunnel` consumes it); `agent` carries
/// `--agent`, which only the session-starting verbs consume.
fn plan_subcommand(
    subcommand: &str,
    rest: &[&str],
    exec: bool,
    agent: Option<String>,
    mode: Option<AutonomyMode>,
) -> Action {
    // Same reasoning as `--agent`: a subcommand that starts no session has
    // nothing to do with a mode, and swallowing it would be a lie about how the
    // work ran.
    if mode.is_some() && !matches!(subcommand, "session") {
        return Action::Usage(format!("`--mode` applies to `session`, not `{subcommand}`"));
    }
    // `--agent` names who runs a session. A subcommand that starts none has
    // nothing to do with it, and quietly swallowing the flag would be a lie
    // about which agent ran — so it is refused where it means nothing.
    if agent.is_some() && !matches!(subcommand, "session" | "propose" | "explore") {
        return Action::Usage(format!(
            "`--agent` applies to `session`, `propose` and `explore`, not `{subcommand}`"
        ));
    }
    match subcommand {
        "help" => Action::Help,
        "version" => Action::Version,
        "status" if rest.is_empty() => Action::Run(Command::Status),
        "status" => Action::Usage("`status` takes no arguments".into()),
        "fleet" if rest.is_empty() => Action::Run(Command::Fleet),
        "fleet" => Action::Usage("`fleet` takes no arguments".into()),
        // The link name travels verbatim: positionals are never case-folded.
        "link" => match rest {
            [agent, name] => Action::Run(Command::Link {
                agent: (*agent).to_string(),
                name: (*name).to_string(),
            }),
            _ => Action::Usage("`link` requires: meltemi link <agent> <name>".into()),
        },
        "unlink" => match rest {
            [name] => Action::Run(Command::Unlink {
                name: (*name).to_string(),
            }),
            _ => Action::Usage("`unlink` requires: meltemi unlink <name>".into()),
        },
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
                agent,
            }),
            _ => Action::Usage("`explore` takes a single quoted topic".into()),
        },
        // `session`, not `start`: `stop` already means the daemon in this
        // grammar, and a `start` beside it would read as "start the daemon",
        // which is the opposite of what this does (design D9).
        "session" => match rest {
            [] => Action::Usage(
                "`session` requires an instruction: meltemi session \"<instruction>\" \
                 [project-root] [--agent <id|profile>]"
                    .into(),
            ),
            [instruction] => Action::Run(Command::Session {
                instruction: (*instruction).to_string(),
                project_root: None,
                agent,
                mode,
            }),
            [instruction, root] => Action::Run(Command::Session {
                instruction: (*instruction).to_string(),
                project_root: Some((*root).to_string()),
                agent,
                mode,
            }),
            _ => Action::Usage(
                "`session` takes a quoted instruction and at most a project root".into(),
            ),
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
        "worktrees" => match rest {
            [] => Action::Run(Command::Worktrees { project_root: None }),
            [root] => Action::Run(Command::Worktrees {
                project_root: Some((*root).to_string()),
            }),
            _ => Action::Usage("`worktrees` takes at most a project root".into()),
        },
        "assign" => match rest {
            [change, task, agents] | [change, task, agents, _] => {
                let list: Vec<String> = agents
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if list.is_empty() {
                    Action::Usage("`assign` needs at least one agent".into())
                } else {
                    Action::Run(Command::Assign {
                        change: (*change).to_string(),
                        task: (*task).to_string(),
                        agents: list,
                        project_root: rest.get(3).map(|s| (*s).to_string()),
                    })
                }
            }
            _ => Action::Usage(
                "`assign` requires: meltemi assign <change> <task> <agent[,agent...]> [project-root]"
                    .into(),
            ),
        },
        "race" => match rest {
            [change, task] | [change, task, _] => Action::Run(Command::Race {
                change: (*change).to_string(),
                task: (*task).to_string(),
                project_root: rest.get(2).map(|s| (*s).to_string()),
            }),
            _ => Action::Usage(
                "`race` requires: meltemi race <change> <task> [project-root]".into(),
            ),
        },
        "dispatch" => match rest {
            [change, task, agent] | [change, task, agent, _] => Action::Run(Command::Dispatch {
                change: (*change).to_string(),
                task: (*task).to_string(),
                agent: (*agent).to_string(),
                project_root: rest.get(3).map(|s| (*s).to_string()),
            }),
            _ => Action::Usage(
                "`dispatch` requires: meltemi dispatch <change> <task> <agent> [project-root]"
                    .into(),
            ),
        },
        "apply-edit" => match rest {
            [file] => Action::Run(Command::ApplyEdit {
                file: (*file).to_string(),
                target: None,
                confirm: false,
            }),
            [file, kw] if *kw == "confirm" => Action::Run(Command::ApplyEdit {
                file: (*file).to_string(),
                target: None,
                confirm: true,
            }),
            [file, change, task, agent] => Action::Run(Command::ApplyEdit {
                file: (*file).to_string(),
                target: Some((
                    (*change).to_string(),
                    (*task).to_string(),
                    (*agent).to_string(),
                )),
                confirm: false,
            }),
            [file, change, task, agent, kw] if *kw == "confirm" => {
                Action::Run(Command::ApplyEdit {
                    file: (*file).to_string(),
                    target: Some((
                        (*change).to_string(),
                        (*task).to_string(),
                        (*agent).to_string(),
                    )),
                    confirm: true,
                })
            }
            _ => Action::Usage(
                "`apply-edit` requires: meltemi apply-edit <file> [<change> <task> <agent>]                  [confirm] (content is read from stdin)"
                    .into(),
            ),
        },
        "checkpoints" => match rest {
            [] => Action::Run(Command::Checkpoints { change: None }),
            [change] => Action::Run(Command::Checkpoints {
                change: Some((*change).to_string()),
            }),
            _ => Action::Usage("`checkpoints` takes at most a change name".into()),
        },
        "revert" => match rest {
            [change, task, agent] | [change, task, agent, _] => {
                let confirm = rest.get(3).is_some_and(|w| *w == "confirm");
                if rest.len() == 4 && !confirm {
                    Action::Usage(
                        "`revert` accepts only `confirm` as its 4th argument".into(),
                    )
                } else {
                    Action::Run(Command::Revert {
                        change: (*change).to_string(),
                        task: (*task).to_string(),
                        agent: (*agent).to_string(),
                        confirm,
                    })
                }
            }
            _ => Action::Usage(
                "`revert` requires: meltemi revert <change> <task> <agent> [confirm]".into(),
            ),
        },
        "commit" => match rest {
            [change, task, agent, title] | [change, task, agent, title, _] => {
                let confirm = rest.get(4).is_some_and(|w| *w == "confirm");
                if rest.len() == 5 && !confirm {
                    Action::Usage("`commit` accepts only `confirm` as its 5th argument".into())
                } else {
                    Action::Run(Command::Commit {
                        change: (*change).to_string(),
                        task: (*task).to_string(),
                        agent: (*agent).to_string(),
                        title: (*title).to_string(),
                        confirm,
                    })
                }
            }
            _ => Action::Usage(
                "`commit` requires: meltemi commit <change> <task> <agent> \"<title>\" [confirm]"
                    .into(),
            ),
        },
        "verify" => match rest {
            [change] => Action::Run(Command::Verify {
                change: (*change).to_string(),
            }),
            _ => Action::Usage("`verify` requires a change name: meltemi verify <change>".into()),
        },
        "archive" => match rest {
            [change] | [change, _] => {
                let confirm = rest.get(1).is_some_and(|w| *w == "confirm");
                if rest.len() == 2 && !confirm {
                    Action::Usage("`archive` accepts only `confirm` as its 2nd argument".into())
                } else {
                    Action::Run(Command::Archive {
                        change: (*change).to_string(),
                        confirm,
                    })
                }
            }
            _ => Action::Usage(
                "`archive` requires: meltemi archive <change> [confirm]".into(),
            ),
        },
        "implement" => match rest {
            [change, agent] | [change, agent, _] => {
                let plan_only = rest.get(2).is_some_and(|w| *w == "plan");
                if rest.len() == 3 && !plan_only {
                    Action::Usage("`implement` accepts only `plan` as its 3rd argument".into())
                } else {
                    Action::Run(Command::Implement {
                        change: (*change).to_string(),
                        agent: (*agent).to_string(),
                        plan_only,
                    })
                }
            }
            _ => Action::Usage(
                "`implement` requires: meltemi implement <change> <agent> [plan]".into(),
            ),
        },
        // The registry verbs hang off the plural listing verb, which took no
        // positionals: `meltemi project register <path>` would have parsed
        // `register` AS the project root, because `project` already takes one
        // (design D9). The discriminator comes first, like `usage <granularity>`.
        "projects" => match rest {
            [] => Action::Run(Command::Projects),
            ["register", path] => Action::Run(Command::ProjectRegister {
                path: (*path).to_string(),
            }),
            ["forget", path] => Action::Run(Command::ProjectForget {
                path: (*path).to_string(),
            }),
            ["register"] => Action::Usage(
                "`projects register` requires a path: meltemi projects register <path>".into(),
            ),
            ["forget"] => Action::Usage(
                "`projects forget` requires a path: meltemi projects forget <path>".into(),
            ),
            _ => Action::Usage(
                "`projects` lists the registry; `projects register <path>` and \
                 `projects forget <path>` change it"
                    .into(),
            ),
        },
        "usage" => parse_usage(rest),
        "changes" if rest.is_empty() => Action::Run(Command::Changes),
        "changes" => Action::Usage("`changes` takes no arguments".into()),
        "show" => match rest {
            [change] => Action::Run(Command::Show {
                change: (*change).to_string(),
            }),
            _ => Action::Usage("`show` requires a change name: meltemi show <change>".into()),
        },
        "specs" => match rest {
            [] => Action::Run(Command::Specs { capability: None }),
            [capability] => Action::Run(Command::Specs {
                capability: Some((*capability).to_string()),
            }),
            _ => Action::Usage("`specs` takes at most a capability name".into()),
        },
        "validate" => match rest {
            [] => Action::Run(Command::Validate { change: None }),
            [change] => Action::Run(Command::Validate {
                change: Some((*change).to_string()),
            }),
            _ => Action::Usage("`validate` takes at most a change name".into()),
        },
        "direct" => match rest {
            [session, instruction] | [session, instruction, _] => Action::Run(Command::Direct {
                session: (*session).to_string(),
                instruction: (*instruction).to_string(),
                project_root: rest.get(2).map(|s| (*s).to_string()),
            }),
            _ => Action::Usage(
                "`direct` requires: meltemi direct <session> \"<instruction>\" [project-root]"
                    .into(),
            ),
        },
        "tunnel" => match rest {
            [] => Action::Run(Command::Tunnel {
                target: None,
                exec,
            }),
            [target] => Action::Run(Command::Tunnel {
                target: Some((*target).to_string()),
                exec,
            }),
            _ => Action::Usage("`tunnel` takes at most a `user@host` target".into()),
        },
        "bridge" if rest.is_empty() => Action::Run(Command::Bridge),
        "bridge" => Action::Usage("`bridge` takes no arguments".into()),
        "stop" if rest.is_empty() => Action::Run(Command::Stop),
        "stop" => Action::Usage("`stop` takes no arguments".into()),
        "propose" => match rest {
            [] => Action::Usage(
                "`propose` requires an idea: meltemi propose \"<idea>\" [project-root]".into(),
            ),
            [idea] => Action::Run(Command::Propose {
                idea: (*idea).to_string(),
                project_root: None,
                agent,
            }),
            [idea, root] => Action::Run(Command::Propose {
                idea: (*idea).to_string(),
                project_root: Some((*root).to_string()),
                agent,
            }),
            _ => Action::Usage("`propose` takes at most an idea and a project root".into()),
        },
        other if RESERVED.contains(&other) => Action::Run(Command::Reserved(other.to_string())),
        other => Action::Usage(format!("unknown subcommand `{other}`; run `meltemi help`")),
    }
}

/// `usage [<granularity>] [--project <root>|--all] [--since <ts>] [--until <ts>]`.
/// Unknown flags are refused rather than ignored: a mistyped filter that
/// silently widened the query would misreport consumption.
fn parse_usage(rest: &[&str]) -> Action {
    const GRAINS: [&str; 4] = ["day", "week", "month", "total"];
    let mut granularity = None;
    let mut project_root = None;
    let mut all = false;
    let mut since = None;
    let mut until = None;
    let mut index = 0;
    while index < rest.len() {
        let token = rest[index];
        let value_of = || rest.get(index + 1).map(|value| (*value).to_string());
        match token {
            "--all" => all = true,
            "--project" => match value_of() {
                Some(value) => {
                    project_root = Some(value);
                    index += 1;
                }
                None => return Action::Usage("`--project` requires a path".into()),
            },
            "--since" => match value_of() {
                Some(value) => {
                    since = Some(value);
                    index += 1;
                }
                None => return Action::Usage("`--since` requires a timestamp".into()),
            },
            "--until" => match value_of() {
                Some(value) => {
                    until = Some(value);
                    index += 1;
                }
                None => return Action::Usage("`--until` requires a timestamp".into()),
            },
            grain if GRAINS.contains(&grain) => granularity = Some(grain.to_string()),
            other => {
                return Action::Usage(format!(
                    "`usage` does not take `{other}`: meltemi usage [day|week|month|total] \
                     [--project <root>|--all] [--since <ts>] [--until <ts>]"
                ));
            }
        }
        index += 1;
    }
    if all && project_root.is_some() {
        return Action::Usage("`--all` and `--project` are mutually exclusive".into());
    }
    Action::Run(Command::Usage {
        // Default scope is the current project; `--all` widens to every project.
        project_root: if all {
            None
        } else {
            Some(project_root.unwrap_or_default())
        },
        granularity,
        since,
        until,
    })
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
    fn the_usage_verb_accepts_its_own_flags_through_the_global_parser() {
        // The global parser is strict about unknown flags, so a subcommand-local
        // flag must be declared there or the advertised command is unusable.
        let plan = plan_of(&["usage", "month", "--all"], false);
        assert_eq!(
            plan.action,
            Action::Run(Command::Usage {
                project_root: None,
                granularity: Some("month".into()),
                since: None,
                until: None,
            }),
            "`usage month --all` must reach the usage parser"
        );

        let scoped = plan_of(
            &[
                "usage",
                "--project",
                "/repo",
                "--since",
                "2026-07-01T00:00:00Z",
            ],
            false,
        );
        assert_eq!(
            scoped.action,
            Action::Run(Command::Usage {
                project_root: Some("/repo".into()),
                granularity: None,
                since: Some("2026-07-01T00:00:00Z".into()),
                until: None,
            })
        );

        // `--json` still works alongside them, and an unknown flag still fails.
        let machine = plan_of(&["--json", "usage", "total", "--all"], false);
        assert_eq!(machine.format, Format::Json);
        assert!(matches!(
            plan_of(&["usage", "--nope"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn known_subcommand_is_scriptable_even_without_tty() {
        // Scenario: Con subcomando siempre scriptable.
        let p = plan_of(&["status"], false);
        assert_eq!(p.action, Action::Run(Command::Status));
        assert_eq!(p.format, Format::Human);
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
    fn the_sdd_cycle_grammar_has_no_reserved_subcommands() {
        // Scenario: El ciclo completo es operativo
        assert!(RESERVED.is_empty(), "the whole SDD cycle is operative");
    }

    #[test]
    fn implement_is_operative_with_an_optional_plan_preview() {
        // Scenario: implement despliega el plan.
        assert_eq!(
            plan_of(&["implement", "add-thing", "claude"], false).action,
            Action::Run(Command::Implement {
                change: "add-thing".into(),
                agent: "claude".into(),
                plan_only: false,
            })
        );
        assert_eq!(
            plan_of(&["implement", "add-thing", "claude", "plan"], false).action,
            Action::Run(Command::Implement {
                change: "add-thing".into(),
                agent: "claude".into(),
                plan_only: true,
            })
        );
        assert!(matches!(
            plan_of(&["implement", "add-thing"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn navigation_verbs_parse() {
        // Scenario: Subcomando operativo reconocido
        assert_eq!(
            plan_of(&["changes"], false).action,
            Action::Run(Command::Changes)
        );
        assert!(matches!(
            plan_of(&["changes", "x"], false).action,
            Action::Usage(_)
        ));
        assert_eq!(
            plan_of(&["show", "flota-multiproveedor"], false).action,
            Action::Run(Command::Show {
                change: "flota-multiproveedor".into()
            })
        );
        assert!(matches!(plan_of(&["show"], false).action, Action::Usage(_)));
        assert_eq!(
            plan_of(&["specs"], false).action,
            Action::Run(Command::Specs { capability: None })
        );
        assert_eq!(
            plan_of(&["specs", "fleet-catalog"], false).action,
            Action::Run(Command::Specs {
                capability: Some("fleet-catalog".into())
            })
        );
        assert_eq!(
            plan_of(&["validate"], false).action,
            Action::Run(Command::Validate { change: None })
        );
        assert_eq!(
            plan_of(&["validate", "flota-multiproveedor"], false).action,
            Action::Run(Command::Validate {
                change: Some("flota-multiproveedor".into())
            })
        );
    }

    #[test]
    fn direct_parses_session_and_instruction() {
        // Scenario: Instrucción a una sesión activa se despacha como siguiente turno
        assert_eq!(
            plan_of(&["direct", "sess-1", "add a dark theme"], false).action,
            Action::Run(Command::Direct {
                session: "sess-1".into(),
                instruction: "add a dark theme".into(),
                project_root: None,
            })
        );
        assert_eq!(
            plan_of(&["direct", "sess-1", "add a dark theme", "/repo"], false).action,
            Action::Run(Command::Direct {
                session: "sess-1".into(),
                instruction: "add a dark theme".into(),
                project_root: Some("/repo".into()),
            })
        );
        assert!(matches!(
            plan_of(&["direct", "sess-1"], false).action,
            Action::Usage(_)
        ));
    }

    // Scenario: Arrancar desde la CLI sigue mostrando el desenlace
    #[test]
    fn the_scriptable_start_waits_for_the_outcome_and_says_so() {
        // The CLI does not detach, and the reason is measured rather than
        // cautious: `session/watch` and `session/log` are declared gaps in this
        // surface (docs/paridad-nucleo.md), so a detached start here would print
        // an id and exit, having shown the user nothing of the turn. The help
        // says that out loud instead of leaving it to be discovered.
        let plan = plan_of(&["session", "fix the build"], false);
        assert!(
            matches!(plan.action, Action::Run(Command::Session { .. })),
            "the verb still starts a session"
        );
        let session_help = USAGE
            .split("session <instruction>")
            .nth(1)
            .expect("the help documents the verb")
            .split(
                "
    sessions",
            )
            .next()
            .expect("its entry ends where the next verb begins");
        assert!(
            session_help.contains("WAITS for the turn"),
            "the help says the command waits: {session_help}"
        );
        assert!(
            session_help.contains("event stream"),
            "and names what this surface cannot read, which is why it waits: {session_help}"
        );
    }

    // Scenario: Dos formatos de máquina a la vez se rehúsan
    #[test]
    fn the_format_is_one_choice_and_two_machine_flags_are_refused() {
        assert_eq!(plan_of(&["status"], false).format, Format::Human);
        assert_eq!(plan_of(&["--json", "status"], false).format, Format::Json);
        assert_eq!(plan_of(&["--yaml", "status"], false).format, Format::Yaml);

        // Asking for both means nothing, so it is refused rather than resolved
        // by whichever flag came last.
        let both = plan_of(&["--json", "--yaml", "status"], false);
        assert!(matches!(both.action, Action::Usage(_)));
        let Action::Usage(message) = both.action else {
            unreachable!("checked above")
        };
        assert!(
            message.contains("pick one"),
            "the refusal says what to do: {message}"
        );

        // An unknown flag under `--json` still answers as JSON: an error is
        // output too, and a script asked for one shape.
        let bad = plan_of(&["--json", "--bogus", "status"], false);
        assert!(matches!(bad.action, Action::Usage(_)));
        assert_eq!(bad.format, Format::Json);
    }

    #[test]
    fn no_color_is_a_global_flag_anywhere_on_the_line() {
        assert!(plan_of(&["--no-color", "status"], true).no_color);
        assert!(plan_of(&["status", "--no-color"], true).no_color);
        assert!(!plan_of(&["status"], true).no_color);
        // It is not a machine format, so it never changes the choice.
        assert_eq!(
            plan_of(&["--no-color", "status"], true).format,
            Format::Human
        );
    }

    #[test]
    fn a_bare_invocation_with_yaml_is_scriptable_not_interactive() {
        // `--yaml` signals machine use exactly as `--json` does, so a bare
        // invocation with it must not open the interactive shell.
        assert!(matches!(
            plan_of(&["--yaml"], true).action,
            Action::Usage(_)
        ));
    }

    // Scenario: Un modo desconocido se rehúsa con los válidos
    #[test]
    fn an_unknown_mode_is_refused_with_the_valid_ones_and_never_degraded() {
        // Guessing how much a session may decide on its own would be the worst
        // possible guess, so a name that is not a mode refuses — and the
        // refusal carries the names that would have worked.
        let plan = plan_of(&["session", "do it", "--mode", "bypass"], false);
        let Action::Usage(message) = plan.action else {
            panic!("`bypass` is not a mode and must not start a session");
        };
        for valid in ["manual", "semi", "autonomous"] {
            assert!(
                message.contains(valid),
                "the refusal names `{valid}`: {message}"
            );
        }

        // A missing value refuses too, rather than swallowing the next flag.
        assert!(matches!(
            plan_of(&["session", "do it", "--mode"], false).action,
            Action::Usage(_)
        ));
        assert!(matches!(
            plan_of(&["session", "do it", "--mode", "--json"], false).action,
            Action::Usage(_)
        ));

        // The three that exist do start a session, carrying what was asked.
        for (name, expected) in [
            ("manual", AutonomyMode::Manual),
            ("semi", AutonomyMode::Semi),
            ("autonomous", AutonomyMode::Autonomous),
        ] {
            let plan = plan_of(&["session", "do it", "--mode", name], false);
            let Action::Run(Command::Session { mode, .. }) = plan.action else {
                panic!("`{name}` is a mode and starts a session")
            };
            assert_eq!(mode, Some(expected));
        }

        // And a verb that starts no session refuses the flag rather than
        // swallowing it, which would be a lie about how the work ran.
        assert!(matches!(
            plan_of(&["status", "--mode", "manual"], false).action,
            Action::Usage(_)
        ));

        // Absent is absent: no mode, no composition, the behaviour that existed.
        let Action::Run(Command::Session { mode, .. }) =
            plan_of(&["session", "do it"], false).action
        else {
            panic!("a session without a mode still starts")
        };
        assert_eq!(mode, None);
    }

    #[test]
    fn bridge_takes_no_arguments() {
        assert_eq!(
            plan_of(&["bridge"], false).action,
            Action::Run(Command::Bridge)
        );
        // Anything after the verb is a usage error, not a silent ignore: the
        // far side of an ssh exec has nobody to notice a typo.
        assert!(matches!(
            plan_of(&["bridge", "extra"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn tunnel_parses_optional_target_and_exec_flag() {
        // Scenario: Composición del túnel por plataforma
        assert_eq!(
            plan_of(&["tunnel"], false).action,
            Action::Run(Command::Tunnel {
                target: None,
                exec: false,
            })
        );
        assert_eq!(
            plan_of(&["tunnel", "me@server"], false).action,
            Action::Run(Command::Tunnel {
                target: Some("me@server".into()),
                exec: false,
            })
        );
        // Scenario: Ejecución visible solo a pedido
        // `--exec` is opt-in and position-independent (a global flag).
        assert_eq!(
            plan_of(&["tunnel", "me@server", "--exec"], false).action,
            Action::Run(Command::Tunnel {
                target: Some("me@server".into()),
                exec: true,
            })
        );
        assert_eq!(
            plan_of(&["tunnel", "--exec", "me@server"], false).action,
            Action::Run(Command::Tunnel {
                target: Some("me@server".into()),
                exec: true,
            })
        );
    }

    #[test]
    fn verify_and_archive_are_operative() {
        // Scenario: Subcomando operativo reconocido (verify/archive).
        assert_eq!(
            plan_of(&["verify", "add-thing"], false).action,
            Action::Run(Command::Verify {
                change: "add-thing".into()
            })
        );
        assert_eq!(
            plan_of(&["archive", "add-thing"], false).action,
            Action::Run(Command::Archive {
                change: "add-thing".into(),
                confirm: false,
            })
        );
        assert_eq!(
            plan_of(&["archive", "add-thing", "confirm"], false).action,
            Action::Run(Command::Archive {
                change: "add-thing".into(),
                confirm: true,
            })
        );
        assert!(matches!(
            plan_of(&["verify"], false).action,
            Action::Usage(_)
        ));
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
        assert_eq!(p.format, Format::Json);
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
        assert_eq!(plan_of(&["--json", "sessions"], false).format, Format::Json);
    }

    #[test]
    fn link_and_unlink_parse_with_the_name_verbatim() {
        // Scenario: link crea y responde con el gesto de login (grammar half)
        assert_eq!(
            plan_of(&["link", "claude-code", "personal-2"], false).action,
            Action::Run(Command::Link {
                agent: "claude-code".into(),
                name: "personal-2".into(),
            })
        );
        // Positionals are never case-folded: what the daemon refuses, it
        // refuses having SEEN what was typed.
        assert_eq!(
            plan_of(&["link", "claude-code", "MixedCase"], false).action,
            Action::Run(Command::Link {
                agent: "claude-code".into(),
                name: "MixedCase".into(),
            })
        );
        assert_eq!(
            plan_of(&["unlink", "personal-2"], false).action,
            Action::Run(Command::Unlink {
                name: "personal-2".into(),
            })
        );
        assert!(matches!(
            plan_of(&["link", "claude-code"], false).action,
            Action::Usage(_)
        ));
        assert!(matches!(
            plan_of(&["unlink"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn fleet_is_an_operational_subcommand_without_arguments() {
        // Scenario: Subcomando operativo reconocido (fleet).
        let p = plan_of(&["fleet"], false);
        assert_eq!(p.action, Action::Run(Command::Fleet));
        assert_eq!(p.format, Format::Human);
        // The --json variant is captured like any global flag.
        let p = plan_of(&["--json", "fleet"], false);
        assert_eq!(p.action, Action::Run(Command::Fleet));
        assert_eq!(p.format, Format::Json);
        assert!(matches!(
            plan_of(&["fleet", "x"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn worktrees_lists_with_an_optional_root() {
        // Scenario: worktrees lista los gestionados.
        assert_eq!(
            plan_of(&["worktrees"], false).action,
            Action::Run(Command::Worktrees { project_root: None })
        );
        assert_eq!(
            plan_of(&["worktrees", "/repo"], false).action,
            Action::Run(Command::Worktrees {
                project_root: Some("/repo".into())
            })
        );
    }

    #[test]
    fn assign_parses_a_race_from_comma_separated_agents() {
        // Scenario: Carrera etiquetada
        // dos agentes en una tarea.
        assert_eq!(
            plan_of(&["assign", "add-thing", "1.2", "claude,gemini"], false).action,
            Action::Run(Command::Assign {
                change: "add-thing".into(),
                task: "1.2".into(),
                agents: vec!["claude".into(), "gemini".into()],
                project_root: None,
            })
        );
        // A single agent is a plain (non-race) assignment, with a root.
        assert_eq!(
            plan_of(&["assign", "add-thing", "1.2", "claude", "/repo"], false).action,
            Action::Run(Command::Assign {
                change: "add-thing".into(),
                task: "1.2".into(),
                agents: vec!["claude".into()],
                project_root: Some("/repo".into()),
            })
        );
    }

    #[test]
    fn assign_needs_change_task_and_agents() {
        assert!(matches!(
            plan_of(&["assign", "add-thing"], false).action,
            Action::Usage(_)
        ));
        // Empty agent list (bare commas) is a usage error.
        assert!(matches!(
            plan_of(&["assign", "c", "1.2", ","], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn race_shows_competitor_diffs() {
        assert_eq!(
            plan_of(&["race", "add-thing", "1.2"], false).action,
            Action::Run(Command::Race {
                change: "add-thing".into(),
                task: "1.2".into(),
                project_root: None,
            })
        );
        assert!(matches!(
            plan_of(&["race", "add-thing"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn apply_edit_parses_file_target_and_confirm() {
        // Scenario: Guardado de una edición in situ
        assert_eq!(
            plan_of(&["apply-edit", "src/lib.rs"], false).action,
            Action::Run(Command::ApplyEdit {
                file: "src/lib.rs".into(),
                target: None,
                confirm: false,
            })
        );
        assert_eq!(
            plan_of(
                &[
                    "apply-edit",
                    "src/lib.rs",
                    "add-thing",
                    "1.1",
                    "fast",
                    "confirm"
                ],
                false
            )
            .action,
            Action::Run(Command::ApplyEdit {
                file: "src/lib.rs".into(),
                target: Some(("add-thing".into(), "1.1".into(), "fast".into())),
                confirm: true,
            })
        );
    }

    #[test]
    fn dispatch_parses_change_task_agent() {
        // Scenario: Carrera de dos proveedores distintos (per-competitor dispatch).
        assert_eq!(
            plan_of(&["dispatch", "add-thing", "1.1", "work"], false).action,
            Action::Run(Command::Dispatch {
                change: "add-thing".into(),
                task: "1.1".into(),
                agent: "work".into(),
                project_root: None,
            })
        );
        assert!(matches!(
            plan_of(&["dispatch", "add-thing", "1.1"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn checkpoints_lists_with_an_optional_change() {
        // Scenario: Checkpoints consultables.
        assert_eq!(
            plan_of(&["checkpoints"], false).action,
            Action::Run(Command::Checkpoints { change: None })
        );
        assert_eq!(
            plan_of(&["checkpoints", "add-thing"], false).action,
            Action::Run(Command::Checkpoints {
                change: Some("add-thing".into())
            })
        );
    }

    #[test]
    fn revert_requires_confirmation_word_to_execute() {
        // Scenario: Confirmación obligatoria
        // sin `confirm` es una vista previa.
        assert_eq!(
            plan_of(&["revert", "add-thing", "1.1", "claude"], false).action,
            Action::Run(Command::Revert {
                change: "add-thing".into(),
                task: "1.1".into(),
                agent: "claude".into(),
                confirm: false,
            })
        );
        assert_eq!(
            plan_of(&["revert", "add-thing", "1.1", "claude", "confirm"], false).action,
            Action::Run(Command::Revert {
                change: "add-thing".into(),
                task: "1.1".into(),
                agent: "claude".into(),
                confirm: true,
            })
        );
        // A non-`confirm` 4th argument is a usage error, never a silent revert.
        assert!(matches!(
            plan_of(&["revert", "add-thing", "1.1", "claude", "now"], false).action,
            Action::Usage(_)
        ));
        assert!(matches!(
            plan_of(&["revert", "add-thing"], false).action,
            Action::Usage(_)
        ));
    }

    #[test]
    fn commit_previews_without_confirm_and_applies_with_it() {
        // Scenario: Supervisado propone antes de cometer; Autónomo comete.
        assert_eq!(
            plan_of(
                &["commit", "add-thing", "2.1", "claude", "Add the thing"],
                false
            )
            .action,
            Action::Run(Command::Commit {
                change: "add-thing".into(),
                task: "2.1".into(),
                agent: "claude".into(),
                title: "Add the thing".into(),
                confirm: false,
            })
        );
        assert_eq!(
            plan_of(
                &[
                    "commit",
                    "add-thing",
                    "2.1",
                    "claude",
                    "Add the thing",
                    "confirm"
                ],
                false
            )
            .action,
            Action::Run(Command::Commit {
                change: "add-thing".into(),
                task: "2.1".into(),
                agent: "claude".into(),
                title: "Add the thing".into(),
                confirm: true,
            })
        );
        assert!(matches!(
            plan_of(&["commit", "add-thing", "2.1"], false).action,
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
        // Scenario: Sin la flag el comportamiento no cambia
        assert_eq!(
            plan_of(&["propose", "add auth"], false).action,
            Action::Run(Command::Propose {
                idea: "add auth".into(),
                project_root: None,
                agent: None,
            })
        );
        assert_eq!(
            plan_of(&["propose", "add auth", "/repo"], false).action,
            Action::Run(Command::Propose {
                idea: "add auth".into(),
                project_root: Some("/repo".into()),
                agent: None,
            })
        );
        assert_eq!(
            plan_of(&["explore", "how do we cache"], false).action,
            Action::Run(Command::Explore {
                topic: "how do we cache".into(),
                agent: None,
            })
        );
    }

    #[test]
    fn json_flag_is_captured_with_a_subcommand() {
        // Scenario: éxito en JSON (grammar half): the flag reaches the plan.
        let p = plan_of(&["--json", "status"], false);
        assert_eq!(p.action, Action::Run(Command::Status));
        assert_eq!(p.format, Format::Json);
    }

    // Scenario: Los subcomandos locales no tocan el daemon
    #[test]
    fn help_and_version_flags_and_subcommands() {
        // They resolve to actions that are answered in `dispatch` itself:
        // `Action::Help` and `Action::Version` never reach `run::execute`, which
        // is the only thing in this binary that opens a connection. The
        // guarantee is structural, which is why it is asserted here rather than
        // by watching a socket that was never dialed.
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
                agent: None,
            })
        );
    }

    #[test]
    fn the_free_session_verb_takes_an_instruction_and_an_optional_root() {
        // Scenario: El arranque de sesión libre tiene su subcomando
        assert_eq!(
            plan_of(&["session", "look at the failing build"], false).action,
            Action::Run(Command::Session {
                instruction: "look at the failing build".into(),
                project_root: None,
                agent: None,
                mode: None,
            })
        );
        assert_eq!(
            plan_of(&["session", "look at the failing build", "/repo"], false).action,
            Action::Run(Command::Session {
                instruction: "look at the failing build".into(),
                project_root: Some("/repo".into()),
                agent: None,
                mode: None,
            })
        );
        // Without an instruction there is nothing to run: a usage error, never
        // a session started on a default prompt nobody wrote.
        assert!(matches!(
            plan_of(&["session"], false).action,
            Action::Usage(_)
        ));
        // And it does not collide with the daemon verb it sits next to, nor
        // with the listing verb one letter away.
        assert_eq!(plan_of(&["stop"], false).action, Action::Run(Command::Stop));
        assert_eq!(
            plan_of(&["sessions"], false).action,
            Action::Run(Command::Sessions { project_root: None })
        );
    }

    #[test]
    fn the_registry_verbs_read_their_discriminator_and_not_a_path() {
        // Scenario: Alta y baja de proyecto desde la CLI
        assert_eq!(
            plan_of(&["projects", "register", "/repo/Beta"], false).action,
            Action::Run(Command::ProjectRegister {
                path: "/repo/Beta".into()
            })
        );
        assert_eq!(
            plan_of(&["projects", "forget", "/repo/Beta"], false).action,
            Action::Run(Command::ProjectForget {
                path: "/repo/Beta".into()
            })
        );
        // The bare verb still lists, and a discriminator with no path is a
        // usage error rather than a path called `register`.
        assert_eq!(
            plan_of(&["projects"], false).action,
            Action::Run(Command::Projects)
        );
        assert!(matches!(
            plan_of(&["projects", "register"], false).action,
            Action::Usage(_)
        ));
        // The reason these hang off `projects` and not `project`: that verb
        // already takes a root, so `project register` would name a directory.
        assert_eq!(
            plan_of(&["project", "register"], false).action,
            Action::Run(Command::Project {
                project_root: Some("register".into())
            })
        );
    }

    #[test]
    fn the_agent_flag_reaches_the_session_verbs_verbatim() {
        // Scenario: Arranque con agente nombrado desde la CLI
        let expected = |agent: &str| {
            Action::Run(Command::Session {
                instruction: "ship it".into(),
                project_root: None,
                agent: Some(agent.to_string()),
                mode: None,
            })
        };
        assert_eq!(
            plan_of(&["session", "ship it", "--agent", "Work-Sub"], false).action,
            expected("Work-Sub"),
            "the agent name travels with its capitals: a lowercased profile \
             name is a different profile, and on Linux a different binary"
        );
        assert_eq!(
            plan_of(&["propose", "add auth", "--agent", "Work-Sub"], false).action,
            Action::Run(Command::Propose {
                idea: "add auth".into(),
                project_root: None,
                agent: Some("Work-Sub".into()),
            })
        );
        assert_eq!(
            plan_of(&["explore", "caching", "--agent", "Work-Sub"], false).action,
            Action::Run(Command::Explore {
                topic: "caching".into(),
                agent: Some("Work-Sub".into()),
            })
        );
    }

    #[test]
    fn the_agent_flag_is_never_rejected_as_an_unknown_flag() {
        // Scenario: La flag no es rechazada por el parser global
        // The global parser is strict about flags it does not know, so an
        // advertised flag it rejected would be a promise the grammar breaks.
        // It is read wherever it is written — including ahead of the verb.
        for line in [
            &["session", "ship it", "--agent", "work"][..],
            &["session", "--agent", "work", "ship it"][..],
            &["--agent", "work", "session", "ship it"][..],
            &["session", "ship it", "/repo", "--agent", "work"][..],
        ] {
            let action = plan_of(line, false).action;
            assert!(
                matches!(action, Action::Run(Command::Session { .. })),
                "`--agent` must reach its subcommand from {line:?}, got {action:?}"
            );
        }
        // A missing value is refused rather than swallowing the next token.
        assert!(matches!(
            plan_of(&["session", "ship it", "--agent"], false).action,
            Action::Usage(_)
        ));
        // And where it means nothing it is refused, not ignored: a swallowed
        // flag would be a lie about which agent ran.
        assert!(matches!(
            plan_of(&["sessions", "--agent", "work"], false).action,
            Action::Usage(_)
        ));
    }
}
