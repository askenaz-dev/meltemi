<!-- SPDX-License-Identifier: Apache-2.0 -->
# Free sessions and permissions

[Español](es/sesion-libre.md)

Start a free session when you want an agent to work on your project without creating a change or
specification first. Choose the project and agent, send an instruction, and follow the work. The
session uses the same permission policy and append-only log as the other agent flows.

## Start from the desktop or terminal

The desktop home view is a composer. Select the project and agent or launch profile, keep the
workflow on **Free**, write the instruction and send. The conversation shows the session, agent
updates and permission requests. Another instruction goes to the same session while it remains
available; sending an instruction does not cancel the running turn.

```bash
meltemi session "tidy up the error handling in the parser"
meltemi session "same, but on another checkout" /path/to/project
meltemi session "..." --agent codex-cli
```

The CLI waits for that turn and exits with its outcome. It prints the session id, restore point
or reason one was unavailable, and any denied-permission warning. Desktop sessions can wait for
further turns; the one-shot CLI command does not keep that conversation open after its turn.

In the TUI use the `session` command from the palette. `direct` sends another instruction from
the selected session's detail. You can also use:

```bash
meltemi direct <session> "add a test for the error case"
meltemi watch <session>
meltemi log <session>
```

A direction can be queued behind a running turn or resume an ended session when that agent
supports resumption. If it cannot, Meltemi reports why. `watch` follows the log; Ctrl+C stops
following without cancelling the session. Use `meltemi cancel <session>` to cancel it.

## Where the agent works

A free session runs on your **project root**, including your working changes. It does not
create an isolated worktree. Use [the SDD task workflow](metodo-sdd.md) when you want tasks run
in managed worktrees. Review the agent's changes before committing or discarding work.

The session is listed with your other sessions. The log records prompts, reported tool calls,
permission decisions and turn results. The selected agent's own permission behavior still
matters: Meltemi can decide only the requests that reach its proxy. Read [agents](agentes.md).

## The restore point

Before the first turn, a Git repository with history receives one snapshot. Meltemi prints its
reference, under `refs/meltemi/checkpoints/free/`, and lists it with `meltemi checkpoints` under
the `free` group. Taking the snapshot does not move your branch or change your Git index.

A free-session checkpoint is **not offered as a whole-tree revert**. The ordinary checkpoint
revert is restricted to worktrees Meltemi manages, so it refuses your project root. To recover a
particular file, inspect the snapshot and use Git's own path-based restore deliberately:

```
git restore --source refs/meltemi/checkpoints/free/<session>-<agent> -- <path>
```

This replaces the selected path. Preserve any later changes you want before using it.

The session can still start when no snapshot is available:

| Cause | What you see | Next step |
|---|---|---|
| The folder is not a Git repository | `no restore point: this root is not a git repository` | Initialize Git if you want version history |
| The repository has no commits | `no restore point: this repository has no history yet` | Make a first commit when ready |

## Choose the agent, model and effort

Use `--agent <id|profile>` or the agent chip in the desktop composer. Without a selection,
Meltemi uses the project's configured agent; if none is configured, it reports available
candidates and asks the surface to offer a choice.

```bash
meltemi session "..." --agent opencode --model anthropic/claude-haiku-4-5
meltemi session "..." --agent codex-cli --model <model> --effort high
```

Model and effort names belong to the provider. Meltemi passes them through the agent's declared
launch control or the session option it announces. Unsupported values are refused before the
first prompt rather than silently replaced. See [agent controls](agentes.md#model-and-effort-per-session).

The CLI's `--mode manual|semi|autonomous` selects the session's autonomy policy. It is separate
from choosing Free, Propose or Explore. Explicit deny rules continue to win, and irreversible
actions require approval; choosing a mode is not a blanket grant of tool access.

## Answer a permission request

With stdin and stderr attached to a terminal, the CLI asks on stderr and keeps stdout available
for `--json`. It shows the tool, title and input the agent reported, plus that agent's options:

```
permission requested by session 3f2c9a
  tool     execute
  title    Bash: cargo test
  content  {"command":"cargo test","description":"Run the tests"}
  1 Allow once  (allow once)
  2 Reject  (reject once)
  d deny   Ctrl+C stop the session
```

- A digit chooses that option. Enter chooses nothing. An option that stops the agent asking for
  similar requests requires another `y` confirmation.
- `d` or Esc denies using the agent's rejection, or cancels when no rejection is offered.
- `r`, when offered, approves once and remembers a rule for the exact tool and command or path.
  `y` confirms writing it. It applies from the next session started without `--mode`.
- Ctrl+C cancels the session; the command exits `13`.

Remembering is not offered for a tool without a usable command or path, for `.meltemi/` changes,
or from task worktree flows such as `implement` and `dispatch`. A command allow rule does not
allow another command chained, piped, redirected or substituted after it; those still ask.

Keys typed before a readable question, or pasted together, do not answer it. Expired or already
decided requests are withdrawn. In a script or pipe, unresolved requests are denied rather than
waiting for input. Configure explicit permission rules in the project's
`.meltemi/permissions.toml` or your user configuration when unattended work needs them.

## Continue with a change

The desktop composer also offers **Propose** and **Explore**:

| Workflow | CLI command | Result |
|---|---|---|
| Free | `meltemi session` | Work on the project root with the session controls above |
| Propose | `meltemi propose` | Create a change proposal and ask the agent to draft it |
| Explore | `meltemi explore` | Explore the topic without the method writing into the tree |

The workflow choice does not approve a proposal or alter your project's constitution. Use
[the SDD method](metodo-sdd.md) for reviewed tasks and [the CLI reference](referencia-cli.md)
for all command arguments and exit codes.
