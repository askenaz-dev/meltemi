<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- GENERATED from the CLI grammar by `meltemi::cli::reference()`. Do not edit by hand; regenerate. -->

# CLI reference

The scriptable surface of `meltemi`. This document is generated from the
subcommand grammar and the exit-code taxonomy in the source.

## Grammar

```
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
    workspace <change> [--branch <branch>|--unique]
                        the change's workshop: its own branch (the bare change
                        name) and a managed worktree from the default branch
                        tip; asking again re-encounters, never fails.
                        `--branch` mounts a chosen branch (naming it is
                        consent), `--unique` mints a suffixed workshop that
                        never collides — one or the other, not both
    land <change> [--branch <branch>] [confirm]
                        land the workshop branch on the default branch with a
                        --no-ff merge; without the trailing `confirm` it
                        previews the commits and files that would land. A
                        conflicted merge is aborted — the default branch stays
                        intact and the conflict is yours to resolve in git
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
    -V, --version       print the client version
```

## Exit codes

- `0` — success — the command completed its purpose
- `1` — internal — an unexpected internal error
- `2` — usage — unknown subcommand, invalid flags, or a missing argument
- `10` — unreachable — the daemon could not be reached or started
- `11` — contract — the daemon answered with a contract/protocol error
- `12` — denied — the operation was refused by policy (permission proxy)
- `13` — cancelled — the operation was cancelled
- `14` — validation — a validation completed with findings (not an error)
