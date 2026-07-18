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
    propose <idea>      scaffold a change proposal and delegate it to an agent
    fleet               list the agent fleet catalog (detection and levels)
    project             regenerate the projected context (AGENTS.md, ...)
    sessions            list agent sessions (active and historical)
    explore <topic>     deliberate with the agent without writing
    plan <change>       refine design and sequence a change's tasks
    constitution        create or edit the project constitution
    review <change>     review a change's spec deltas as a checklist
    worktrees           list the worktrees Meltemi manages for a project
    assign <change> <task> <agents>
                        create an isolated worktree per agent from a common
                        base (comma-separate agents to race them on one task)
    race <change> <task>
                        show each competitor's diff against the common base
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
    implement <change> <agent> [plan]
                        deploy the agent over the change's tasks.md task by task
                        (checkpoint → turn → commit → tick); `plan` previews
    stop                request an orderly daemon shutdown
    version             print the client version
    help                print this help

GLOBAL FLAGS:
    --json              emit machine-readable JSON on stdout
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
