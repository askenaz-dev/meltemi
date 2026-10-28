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
            [--model <name>] [--effort <level>]
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
                        irreversible escalates.
                        `--model` and `--effort` are the PROVIDER's own words,
                        never checked against a catalog of ours: each travels
                        as the launch flag the agent's entry declares, else by
                        the option the agent announces, set and confirmed
                        before the first prompt. A lever the agent does not
                        take is refused by name, and no prompt is sent; the
                        output says the model and effort that governed
    sessions [root] [--follow]
                        list agent sessions (active and historical); with
                        --follow, write each change of state as it happens,
                        one record per line, until Ctrl+C or the daemon goes
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
                        them, never estimated; the context and the cost an
                        agent reports over ACP are shown as its own figures,
                        summed per currency and never converted (`--all`
                        spans every project)
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
    set-option <session> <option-id> <value>
                        set one of the options the AGENT announced for a live
                        session, without relaunching it; the option ids and
                        their values are the agent's, never a list of ours
    watch <session>     follow a live session: each record of it as it happens,
                        one JSON object per line, until the session ends —
                        records only `log` has if this reader falls behind.
                        Ctrl+C stops following; the session keeps running
    log <session>       the session's append-only record as it is: one JSON
                        object per line, from its first record to now
    cancel <session>    cancel a live session, as the interactive `x` does: the
                        agent closes its turn and the session ends; says
                        whether it was still running when answered
    permissions         the permission requests waiting for a decision: id,
                        session, tool, title, how long each has waited, when it
                        expires, and the options its agent offered
    approve <request> [--option <option-id>]
                        decide ONE waiting request with an option its agent
                        offered: its one-time approval when it offered exactly
                        one; `--option` names another of its approvals, and
                        naming it is consent. A request no longer waiting is
                        not decided (exit 12): the first decision wins
    deny <request> [--option <option-id>]
                        decide ONE waiting request with the agent's rejection,
                        or cancel it when the agent offered none; `--option`
                        names one of its rejections. Nothing decides in bulk
    harness [--agent <id>]
                        the harness that applies: every piece with the layer it
                        comes from, plus what is covered or unreadable and why
    machines            the machines you configured, with the one in use marked.
                        Reading only: each run of this CLI resolves its own
                        endpoint, so there is nobody here to switch
    identity            who you are on your OWN mesh, which machines it knows
                        and whether each is online. Meltemi reads your mesh
                        client; it has no accounts and issues no identity
    identity-link [--hostname <name>] [--exec]
                        print the gesture that links THIS machine to your mesh;
                        `--exec` runs your own client, which opens your browser
                        so you sign in with your own provider
    tunnel [user@host] [--exec]
                        compose the `ssh` command that reverse-forwards this
                        daemon's endpoint to a remote host; `--exec` runs it
    bridge              pump this daemon's local endpoint over stdio: the last
                        metre of remote access (`ssh <pc> meltemi bridge` is a
                        complete channel, named pipes included)
    stop                request an orderly daemon shutdown
    doctor              what is installed, what answers and what is missing,
                        each with what to do; never starts a daemon and never
                        runs an agent. Exits 14 when a row fails. Nothing is
                        sent anywhere: copy what you choose
    version             print the client version
    help                print this help

PERMISSIONS:
    When stdin and stderr are a terminal, each permission an agent asks for
    is asked on stderr — never on stdout — with what the agent sent (tool,
    title, command, path, content) and the agent's own options by number.
    `d` or Esc denies with the agent's rejection; `r` allows once and
    remembers the most specific rule (offered only when one can be written,
    confirmed with `y`, in force from the next session opened without
    `--mode`); Ctrl+C cancels the session and the command exits 13. A key
    counts only once the question could be read: keys typed before it, and
    keys arriving together, answer nothing. A question that expires or is
    decided elsewhere is withdrawn with its reason.
    Without a terminal every request is denied with the agent's rejection;
    to grant without a human, write a rule in .meltemi/permissions.toml or in
    your global permissions.toml. No flag grants permissions.
    A request that waits in the daemon's queue — one opened by the desktop
    surface or the interactive shell — can be decided from any other shell,
    a script or `ssh <pc>` with no terminal: `meltemi permissions` lists
    them and `meltemi approve|deny <request>` decides one of them.

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
- `12` — denied — refused by the permission proxy's policy: a decision reached a request no longer waiting (the first decision wins)
- `13` — cancelled — the operation was cancelled
- `14` — validation — a validation completed with findings (not an error)
- `15` — empty turn — the agent's turn ended without saying or doing anything
