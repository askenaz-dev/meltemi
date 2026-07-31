<!-- SPDX-License-Identifier: Apache-2.0 -->
# Free sessions: work without a spec, never without government

A **free session** is plain work on a repository: you write an instruction, an
agent runs it, and there is no change, no task, no specification and no gate in
the way. It is what a new session is by default in Meltemi.

Nothing of the government is relaxed to get there. That sentence is the whole
point of this page, so it is worth spelling out what "governed" buys you, where
the work happens, what the restore point is — and, just as important, what it is
not.

If you are looking for the reviewed-change flow instead, read
[the SDD method](metodo-sdd.md); the two live in the same composer and switching
is one control, not one migration.

## What a free session is not

- **Not an unsupervised session.** A human is at the composer. Every permission
  request the rules do not resolve escalates to you.
- **Not a session outside the record.** It writes the same append-only JSONL log
  as every other session, and shows up in `meltemi sessions`, in the analytics
  and in the session index like any other.
- **Not a session in a worktree.** It runs on the project root. See
  [Where it runs](#where-it-runs).
- **Not a way to skip the method.** Proposing and exploring are one chip away in
  the same composer. The method stopped being a toll; it did not stop being the
  point.

## What governs it

Every piece below is the same code path any other session takes — there is no
"free session mode" in the daemon that turns things off.

| Piece | What it means for a free session |
|---|---|
| Agent resolution | Resolved from the fleet in the usual order (launch profile → catalog id → the project's configured agent). A name that resolves to an undetected binary **refuses**; it never silently falls back to another vendor. The effective binary and the source of the resolution are written to the session log. |
| Permission proxy | Deny-by-default. The same rules file, the same queue, the same escalation to the human. With no client connected, requests are denied — constitution §3, and a free session is not an exception to it. |
| Session log | Append-only JSONL: the prompt, every agent update, every permission decision with who took it, the close of each turn. |
| Project registry | The project is registered on start, with the root you launched on. |
| Event stream | The full event set reaches the connection that started the session and any connection watching it, so a surface can render the conversation live. |
| Edit lock | Editing a tree that has a live session asks for confirmation (`human_edit`), exactly as elsewhere. |
| Direction | The session is directable while it lives: the next instruction runs as the next turn of the same agent session. |

## Where it runs

**On the project root you chose** — not in an isolated worktree. This is
deliberate and it is the one real trade of the free session, so here is the
reasoning rather than the conclusion alone.

Every human-attended path in Meltemi already runs on the root: `propose`, the
`sdd/*` verbs, and resuming a session by direction. Worktrees are where an agent
runs **unattended on an assigned task** — that is the case constitution §3 names,
and it is the case `worktree/dispatch` and `sdd/implement` serve. A free session
is the opposite situation: you are watching, you see each permission card, and
you decide.

Making it use a worktree anyway would have cost more than it bought: there is no
worktree API that does not demand a `(change, task, agent)` triple, so a free
session would have to invent synthetic values that then leak into
`worktree/list`, `worktree/diff` and the competitor model — entries competing
against nobody. It would also demand a git repository with at least one commit,
so free sessions would refuse to start exactly where `propose` works today.

What protects you on the root, concretely: the permission proxy, the append-only
log, the soft edit lock, your own git — and the restore point below.

## The restore point

When you start a free session on a repository that has history, the daemon takes
**one snapshot before the first turn** and tells you its git ref:

```
$ meltemi session "tidy up the error handling in the parser"
session 9e1c… [completed]
claude --acp
restore point: refs/meltemi/checkpoints/free/9e1c…-claude-code
```

What it is:

- A real git ref under `refs/meltemi/checkpoints/free/`, listable with
  `meltemi checkpoints` (it appears under the pseudo-change `free`, which is how
  it stays apart from the checkpoints of real changes).
- Taken through a scratch index, so it **moves no branch of yours and does not
  touch your index**.

What it is **not** — and this is the part to read twice:

- **It is not an offered revert.** Reverting a checkpoint means `git reset
  --hard` plus `git clean -fd` over the recorded tree. For a free session that
  tree is *yours*, with your uncommitted work and your untracked files in it. So
  `checkpoint/revert` **refuses** any checkpoint whose tree is not a worktree
  Meltemi manages, and no surface offers the control. The refusal carries the
  remedy:

  ```
  git restore --source refs/meltemi/checkpoints/free/<session>-<agent> -- <path>
  ```

  You take back what you want, with the tool that owns your tree. A guided
  revert of a human tree is a different feature with its own confirmation
  design; it is not this one.

### When there is no restore point

The session **still starts** — refusing to work because a folder has no history
would be a worse trade than the one being made. The result says why, and the two
causes get different remedies because they do not substitute for each other:

| Cause | What you see | Remedy |
|---|---|---|
| The root is not a git repository | `no restore point: this root is not a git repository` | `git init` in that directory |
| It is a repository with no commits yet | `no restore point: this repository has no history yet` | make the first commit |

Telling somebody to `git init` a repository they already have would be worse than
saying nothing, which is why the daemon distinguishes the two instead of printing
one string.

## Starting one

### Desktop

The arrival view **is** the composer. Type the instruction, and the chips inside
it carry the context: project, agent or profile, and mode. The mode chip starts
on **free**; the method it will dispatch is written next to the send button
before anything runs. Sending navigates into the conversation — the session's
identifier reaches the surface before the agent's first token, so you walk into
the turn instead of watching a dialog close.

Inside the conversation the composer stays: the next instruction goes to that
same session. It tells you the truth about what happened to it — *queued* with
its position while a turn is running, *resume* when the session ended and can be
resumed, and the daemon's diagnostic with its remedy when the session does not
accept direction. Cancelling is a separate control; sending never interrupts.

### Terminal

```
meltemi session "tidy up the error handling in the parser"
meltemi session "same, but on another checkout" /path/to/project
meltemi session "..." --agent codex-cli
```

The call blocks until the turn ends, like every other start verb, and prints the
session id, how the turn ended, the restore point (or its absence with the
remedy), and a warning if any permission was denied — a denied permission usually
means the work is incomplete, and silence about it would be a lie.

In the TUI, the palette has the same verb, and `direct` steers the open session
from its drill-in with the same honest states as the desktop.

### Choosing the agent

Everywhere: `--agent <id|profile>` on the command line, the agent chip in the
composer, the palette form in the TUI. Profiles are offered by the name of the
subscription, next to the agent underneath them. Omitting it means the project's
configured agent — exactly as before this existed.

If nothing is configured and nothing is named, the refusal is not a sentence you
have to parse: the error carries the **detected candidates** of your fleet with
their install state and their remedy, so the surface can offer you a choice.

## Switching to the method, from the same composer

The mode chip has three positions, and they dispatch three contract verbs:

| Mode | Verb | What it does |
|---|---|---|
| **Free** (default) | `session/start` | The session this page describes. |
| Propose | `propose` | Scaffolds a change proposal and delegates its drafting. |
| Explore | `sdd/explore` | Deliberates without writing anything to the tree. |

Same instruction, same project, same agent — one chip decides which verb takes
it. From the terminal the same three are `meltemi session`, `meltemi propose` and
`meltemi explore`, all of them accepting `--agent`.

The order matters more than it looks: you can start free, see what the work
actually is, and *then* propose it as a change with the understanding you just
bought. That is the shape Meltemi is arguing for — the reviewed specification as
the standard it makes easy to sustain, not the toll that keeps you from starting.

## Related

- [The SDD method](metodo-sdd.md) — the reviewed-change cycle
- [Agents guide](agentes.md) — install, detect and configure the agents a session
  can resolve
- [CLI reference](referencia-cli.md) — generated grammar, including `session`
- [Architecture](arquitectura.md) — the daemon, the proxy and the surfaces
