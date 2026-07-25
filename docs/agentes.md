<!-- SPDX-License-Identifier: Apache-2.0 -->
# Agents: what to install, how detection works, how to configure

Meltemi does not ship agents and never will: it drives the official binaries
you already have, with the authentication each of them manages on its own
(constitution §2, "fair play"). This page tells you, per agent, **what you
install**, **what Meltemi looks for**, and **what to do when the Fleet view
says something is missing**.

The facts below (ids, levels, binaries, install commands) come from the
registry snapshot `core/meltemid/data/fleet-registry.toml` and are verified
against it by a test — if the registry changes and this page does not, the
build fails.

## The two layers of a level-2 agent

Meltemi pilots agents over the Agent Client Protocol (ACP). Some CLIs speak ACP
themselves (**level 1**); others are driven through an official ACP **adapter**
(**level 2**). That means a level-2 entry has two layers, and both must be
present:

| Layer | What it is | Who authenticates |
|---|---|---|
| `cli` | The provider's own official CLI, as you install it | The CLI, with your own account |
| `adapter` | The ACP bridge Meltemi actually launches | Nothing — it delegates to the CLI |

So "the provider CLI is installed but the Fleet says not detected" is not a
bug: the pilot point is the adapter. The Fleet view names which layer is
missing and prints the exact command; **Meltemi never runs an installer for
you** (constitution §3: no silent external effects).

Composed states you will see:

| State | Meaning |
|---|---|
| `ready` | The pilot point is present (and the official CLI too, when the entry declares one) |
| `adapter_missing` | Your official CLI is installed; the ACP adapter is not |
| `cli_missing` | The adapter is installed; the official CLI is not |
| `not_detected` | Neither layer was found |
| `not_launchable` | Something is installed, but only as a script shim that cannot be launched (see Windows below) |

## Integration levels

| Level | Mechanism | What you get |
|---|---|---|
| 1 | Native ACP over stdio | Full integration: streaming, diffs, sessions, cancellation, permissions through Meltemi's tray |
| 2 | Official ACP adapter | The same integration, through one open intermediate piece |
| 3 | Structured headless mode | Programmatic runs inside mandatory guardrails; no rich permission channel |
| 4 | Artifacts only | Context projection: the agent reads your specs from the repository |

A level is *declared* by the registry and *verified* by the conformance suite;
the surfaces show which of the two you are looking at.

## The catalog

### gemini-cli — Gemini CLI

- Level 1 (native ACP). MCP passthrough: yes.
- Meltemi looks for: `gemini`
- Install: see the provider's own documentation.

### copilot-cli — GitHub Copilot CLI

- Level 1 (native ACP).
- Meltemi looks for: `copilot`

### cursor-cli — Cursor CLI

- Level 1 (native ACP).
- Meltemi looks for: `cursor-agent`

### kiro-cli — Kiro CLI

- Level 1 (native ACP).
- Meltemi looks for: `kiro-cli`

### kilo-code — Kilo Code

- Level 1 (native ACP).
- Meltemi looks for: `kilo`

### opencode — OpenCode

- Level 1 (native ACP), launched as `opencode acp`.
- Meltemi looks for: `opencode`

### claude-code — Claude Code

- Level 2 (official ACP adapter). MCP passthrough: yes.
- Layers: official CLI `claude`, adapter `claude-agent-acp`
- Install the CLI: `npm i -g @anthropic-ai/claude-code`
- Install the adapter: `npm i -g @zed-industries/claude-agent-acp`
- **Legal status: grey.** The adapter drives the provider's Agent SDK, and
  consumer subscriptions are not licensed for third-party clients. The safe
  path is piloting the official binary you already signed into. Meltemi shows
  this note in the Fleet view instead of hiding it.

### codex-cli — Codex CLI

- Level 2 (official ACP adapter).
- Layers: official CLI `codex`, adapter `codex-acp`
- Install the CLI: `npm i -g @openai/codex`
- Install the adapter: `cargo install codex-acp`
- Legal status: tolerated — the adapter wraps the CLI's own official
  app-server mode, and the CLI authenticates itself.

### aider — Aider

- Level 3 (structured headless).
- Meltemi looks for: `aider`

### antigravity — Antigravity

- Level 4 (artifacts only): Meltemi projects your context into `AGENTS.md` and
  the agent reads it. Nothing is piloted.
- Meltemi looks for: `agy`

## Where detection looks

For every layer, in order:

1. Each directory of `PATH`, by binary name.
2. The entry's well-known candidate paths (`~/` expands to your home).

Nothing is ever executed to detect it — detection is pure filesystem probing,
and it re-runs on every `fleet/list`, so installing an agent and refreshing the
view is enough.

### Windows

Installers ship `.exe`; npm and nvm shims are `.cmd` (sometimes `.bat`), and
those are the extensions a launch may target. The same shims often also exist
as `.ps1`, which **cannot** be launched directly by the OS: Meltemi reports a
`.ps1`-only find as evidence of an installation with the state
`not_launchable`, instead of handing the launcher a path that will fail.

If your agent shows `not_launchable`, reinstall it so its executable shim is
present (a global npm install normally drops both).

### macOS and Linux

A layer counts as found when the file exists and carries an execute bit.

If an agent you installed shows as not detected:

1. Check that its directory is on `PATH` for the process that started the
   daemon — a shell profile that only exports `PATH` for interactive shells is
   the most common cause. `meltemi status` reports the daemon's endpoint; the
   daemon inherits the environment of whoever launched it.
2. Check the execute bit: `ls -l $(command -v <binary>)`. A file without it is
   reported as not detected, not as broken.
3. If the tool lives outside `PATH` (a version manager, a per-project install),
   point at it explicitly with `agent.command` in the project configuration
   instead of moving files around.
4. A symbolic link is followed, so a link into a version manager's shim
   directory works as well as a real binary.

## Multiple accounts and subscriptions

Launch profiles let you run several accounts of the same provider — two Claude
subscriptions, work and personal — by redirecting the **authentication context**
of the official binary. Meltemi never reads, stores or forwards credentials;
the binary authenticates itself with whatever context it is given.

In `.meltemi/config.toml`:

```toml
[[fleet.profile]]
name = "claude-personal"
agent = "claude-code"
env = { CLAUDE_CONFIG_DIR = "${HOME}/.claude-personal" }

[[fleet.profile]]
name = "claude-work"
agent = "claude-code"
env = { CLAUDE_CONFIG_DIR = "${HOME}/.claude-work" }
```

Values accept `${VAR}` references, resolved from the daemon's environment at
launch time. A value that looks like a plaintext secret is refused: Meltemi's
configuration never persists secret material.

Profiles appear in the Fleet as their own rows (source `profile`, showing the
agent they launch), and you select one per session — so one project can run
`claude-personal` and `claude-work` at the same time, in separate worktrees.

## Declaring an agent Meltemi does not know

```toml
[[fleet.custom]]
id = "my-agent"
name = "My Agent"
command = ["/usr/local/bin/my-agent", "acp"]
```

A custom entry is declared to speak ACP — that is what the daemon pilots.

## When something still does not work

- The Fleet view refreshes detection on every query: install, then reopen it.
- The exact command Meltemi shows is data from the registry; run it yourself.
- An id whose pilot layer is missing is refused at launch with a diagnosis
  naming the layer, never degraded silently to another provider.
- Authentication problems belong to the agent's own binary: Meltemi shows its
  error verbatim rather than interpreting it.
