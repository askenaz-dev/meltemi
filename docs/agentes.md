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
themselves (**level 1**); others are driven through an ACP **adapter**
(**level 2**). That means a level-2 entry has two layers, and both must be
present:

| Layer | What it is | Who authenticates |
|---|---|---|
| `cli` | The provider's own official CLI, as you install it | The CLI, with your own account |
| `adapter` | The ACP bridge Meltemi actually launches | Nothing — it delegates to the CLI |

So "the provider CLI is installed but the Fleet says not detected" is not a
bug: the pilot point is the adapter. The Fleet view names which layer is
missing and what to do about it; **Meltemi never runs an installer for you**
(constitution §3: no silent external effects).

### The adapter layer travels with Meltemi

For the level-2 entries of the catalog, the adapter is **Meltemi's own** and
ships in Meltemi's installers, beside the daemon. There is nothing to install
for it and no third-party package involved: install Meltemi, install the
provider's official CLI, and the entry is `ready`.

Those adapters pilot the provider's official binary as a subprocess and nothing
else. They link no HTTP client and no TLS stack, they listen on no port, and
they never read, store or forward your credentials — the official CLI
authenticates itself, exactly as it does when you run it by hand (constitution
§2).

If the Fleet reports such a layer missing, your Meltemi installation is
incomplete: **reinstall or repair Meltemi**. You will not be given a package
manager command for it, because there is no package to install — and Meltemi
will not invent one.

Detection reports where it found each layer, so a binary you never installed
says where it came from. In the surfaces you will see `bundled with Meltemi`
next to a pilot binary found beside the daemon.

Composed states you will see:

| State | Meaning |
|---|---|
| `ready` | The pilot point is present (and the official CLI too, when the entry declares one) |
| `adapter_missing` | Your official CLI is installed; the ACP adapter is not. For a bundled adapter, reinstall or repair Meltemi |
| `cli_missing` | The adapter is installed; the official CLI is not |
| `not_detected` | Neither layer was found |
| `not_launchable` | Something is installed, but only as a script shim that cannot be launched (see Windows below) |

## Integration levels

| Level | Mechanism | What you get |
|---|---|---|
| 1 | Native ACP over stdio | Full integration: streaming, diffs, sessions, cancellation, permissions through Meltemi's tray |
| 2 | ACP adapter | The same integration, through one intermediate piece — Meltemi's own for the catalog entries, or one you declare yourself |
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
- Meltemi looks for: `agent`

### kiro-cli — Kiro CLI

- Level 1 (native ACP).
- Meltemi looks for: `kiro-cli`

### kilo-code — Kilo Code

- Level 1 (native ACP).
- Meltemi looks for: `kilo`

### opencode — OpenCode

- Level 1 (native ACP), launched as `opencode acp`.
- Meltemi looks for: `opencode`
- Your own model — on your machine, behind one endpoint for many providers, or
  through an aggregator — goes through OpenCode's own configuration: the
  verified recipes and their profiles are in [the local-models guide](local-models.md).

### claude-code — Claude Code

- Level 2 (Meltemi's own ACP adapter). MCP passthrough: yes.
- Layers: official CLI `claude`, adapter `meltemi-claude-acp`
- Install the CLI: `npm i -g @anthropic-ai/claude-code`
- The adapter: nothing to install — `meltemi-claude-acp` travels in Meltemi's
  installers, beside the daemon. If it is missing, reinstall or repair your
  Meltemi installation.
- **Legal status: grey.** Meltemi's adapter pilots the official CLI with the
  session you already signed into — the safe path, never the provider's agent
  SDK. It stays grey because the provider has published no position on
  third-party orchestration of that CLI; if one appears, the note will cite it.
  Meltemi shows this note in the Fleet view instead of hiding it.
- Per-session levers by launch flag: `model`. The adapter has no verified place
  for an effort, so asking for one is refused before the session opens (see
  [model and effort per session](#model-and-effort-per-session)).

### codex-cli — Codex CLI

- Level 2 (Meltemi's own ACP adapter).
- Layers: official CLI `codex`, adapter `meltemi-codex-acp`
- Install the CLI: `npm i -g @openai/codex`
- The adapter: nothing to install — `meltemi-codex-acp` travels in Meltemi's
  installers, beside the daemon. If it is missing, reinstall or repair your
  Meltemi installation.
- Legal status: tolerated — the adapter drives the CLI's own documented
  app-server mode, published by the provider for third-party clients, and the
  CLI authenticates itself.
- Minimum CLI version: **0.156.1**, the version the adapter was verified
  against. An older CLI is refused with the remedy to update it to 0.156.1 or
  newer (`npm i -g @openai/codex`). A newer one below 1.0.0 opens, and the
  session says before its first turn that it runs on a version newer than the
  verified one; the Fleet shows the version the verified level was measured
  against. From 1.0.0 on, the session is refused until Meltemi is updated.
- The gate Meltemi asks for: on every thread, approval policy `untrusted`, a
  `read-only` sandbox without network, and every approval sent to Meltemi's
  permission proxy. If Codex answers with anything else — because its own
  `config.toml`, a profile or a managed requirements file overrides it — the
  session is refused and the remedy names those settings. A Codex whose remote
  control is on is refused too: another client could answer its approvals.
- What that gate does not cover, and the session says so as it opens: commands
  Codex trusts as read-only, which run inside its read-only sandbox; the
  provider's own web search; and calls to MCP servers from Codex's own
  configuration that Codex does not submit for approval. Each of those servers
  is named in the session when Codex starts it.
- The name Meltemi uses: the adapter introduces itself to the CLI as
  `meltemi-codex-acp`, and the CLI reports that name to the provider's backend
  as the `originator` of its requests. Meltemi never presents itself as one of
  the provider's own clients. The CLI labels the threads it opens with a
  `source` of its own choosing; Meltemi does not set it.
- Per-session levers by launch flag: `model`, `effort` (see
  [model and effort per session](#model-and-effort-per-session)).

### aider — Aider

- Level 3 (structured headless).
- Meltemi looks for: `aider`

### antigravity — Antigravity

- Level 4 (artifacts only): Meltemi projects your context into `AGENTS.md` and
  the agent reads it. Nothing is piloted.
- Meltemi looks for: `agy`

## Measured against the real CLIs

What this guide says about an agent is what its registry entry declares. What
an agent **did**, against its real CLI, is measured by the integrated run
(`scripts/smoke-agents.ps1`, or `scripts/smoke-agents.sh` on macOS and Linux):
the same seven verbs for every ready agent, isolated from your daemon, and a
dated report in `docs/qa/`. The last one:

| Agent | Version | Run | Result |
| --- | --- | --- | --- |
| `claude-code` | 2.1.280 | 2026-09-26 | all seven verbs complete; verified level 2 by the conformance run; a path-scoped permission rule cannot match its requests (they carry no path) |
| `opencode` | 1.18.32 | 2026-09-26 | all seven verbs complete; **it edits without asking for permission**, so the proxy does not govern it in its default configuration |
| `codex-cli` | 0.156.1 | 2026-09-26 | six of seven: it cannot be resumed with `direct`; a path-scoped rule cannot match its requests either |

The report: [`qa/2026-09-26-corrida-integrada.md`](qa/2026-09-26-corrida-integrada.md).
Agents of the catalog that are not installed on the machine that ran it are
listed there as not measured, never as passing.

**The rule**: the run is repeated whenever the version of a pinned CLI or of
one of Meltemi's own adapters changes, and its date is part of what this table
says. A result older than the version you have is not a result about your
version.

## Model and effort per session

A session can ask for a model and an effort: `--model` and `--effort` on
`meltemi session`, the model and effort chips in the desktop composer, or the
`model` and `effort` of a launch profile. Meltemi keeps no catalog of models
(constitution §5): the words are the provider's, and each one reaches the agent
by the way that agent actually has.

1. **A launch flag its entry declares.** The registry's `levers` field lists what
   an entry's adapter reads from the environment Meltemi composes for it:
   `meltemi-codex-acp` takes `model` and `effort`, `meltemi-claude-acp` takes
   `model`. The value travels untouched and is not validated here — the agent
   accepts or rejects it. A lever the entry does not declare is refused before
   anything is created (error 2006), naming the agent and the lever.
2. **The option the agent announces.** An entry that declares no levers — every
   native ACP agent, and any literal `agent.command`, whatever its binary is
   called — takes them through the options the agent announces when the session
   opens: the one option of category `model` for the model, the one of category
   `thought_level` for the effort. Meltemi sets the model first, then the effort
   against what the model left on offer, all **before the first prompt**, and
   sends the prompt only if the agent's last answer holds exactly what was asked
   and its mode did not move.
3. **Otherwise it is refused, and no prompt is sent.** No option of the
   category, or more than one; a value the agent did not announce (a toggle takes
   only `true` or `false`, the agent's own words); an answer that keeps another
   value or moves the mode; no answer within `[sessions] lever-wait` seconds (30
   by default). The session ends as `lever_refused`, the verb answers error 2007
   with the lever, what was asked, the reason and what the agent announced or
   left — twenty values at most, and how many more — and every surface says it
   in words. A prompt carries your repository, so it goes only to the model you
   chose.

Cancelling while a lever is being set ends the session without a prompt. The
history lists what governed, and a refused lever is never listed as the model a
session ran with. `propose`, `explore`, `plan`, `constitution`, `dispatch` and
`implement` apply a profile's levers the same way, before they create anything.

The category is the key, deliberately: ACP presents it as interface data, but
nothing else tells "the model" apart without guessing by id or by value. An
agent that announces its selector without a category, or under two, is refused
with the options it did announce; set the one you mean by its id in a live
session with `meltemi set-option <session> <option-id> <value>`.

How long the agent may take to answer each lever is `lever-wait`, in seconds,
under `[sessions]` in the project's `.meltemi/config.toml` or your user config;
one second is the least it accepts.

## Where detection looks

For every layer, in order:

1. Each directory of `PATH`, by binary name.
2. The entry's well-known candidate paths (`~/` expands to your home).
3. For a layer that ships with Meltemi, the directory of the running daemon.

The order matters, and it is the one you would want: a copy you installed
yourself always outranks the copy that came in the box. If you build an adapter
from this repository and put it on your `PATH`, that is the one Meltemi runs,
and the Fleet tells you so rather than leaving you to guess between two files
with the same name.

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

#### What a shim means for ending an agent

A `.cmd` shim is not the agent. Windows runs it with `cmd.exe`, and the script
then starts the process that actually speaks the protocol — usually `node`. So
the thing Meltemi launches and the thing doing the work are two different
processes, one under the other:

```text
meltemid ──launch──> cmd.exe (agent.cmd) ──launch──> node (the agent) ──ACP──> meltemid
```

Ending a process on Windows ends that process and nothing below it. Ending the
shim alone therefore used to leave the agent running: holding its worktree,
holding its files, and invisible to anything that thought the session was over.
Measured on Windows 11 with a shim whose body was `node -e
"setTimeout(()=>{},60000)"` — after the shim was terminated, the `node` was
still alive.

This no longer happens, and not because Meltemi hunts for children. Every agent
is launched inside a **process scope** (a Job Object), which owns the whole tree
underneath it. Two consequences, and the second is the one that matters:

- Ending the agent ends the shim and everything it started, at any depth.
  Meltemi does not need to know what the shim runs, which is good, because the
  shim's contents belong to whoever generated it.
- **If the daemon dies without running any shutdown** — a crash, a kill from
  Task Manager, signing out — the system closes its handles, and the agents go
  with them. A destructor cannot promise that. The kernel can.

Nothing here reads, signals or inspects another vendor's process: the scope is
opened before the launch and the agent is placed in it, which is the whole
mechanism.

If a scope cannot be opened, Meltemi **refuses the launch** and says so, rather
than starting an agent it would not be able to fully end. That is deliberate: a
loud failure at the start is worth more than a quiet orphan an hour later.

On macOS and Linux there is no equivalent step, and none is needed: the shims
there are scripts with a shebang, so the process Meltemi launches *is* the
interpreter doing the work, and ending it ends the work.

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

### The linked way (recommended)

Where the registry declares an entry's **authentication-context variable**,
linking a subscription is one gesture from any surface:

```
meltemi link claude-code personal
```

— or the "Link a subscription" form on the agent's card in the Fleet, or the
`link` verb in the interactive shell. Linking writes a launch profile whose
env pins that variable to a fresh, empty directory under Meltemi's data dir,
and answers with the **login gesture** to run once — the provider's official
binary authenticates itself there; Meltemi never runs the login, never reads
the directory, and never deletes it (unlinking leaves it behind, named).

The variables are registry data, verified against the real CLIs:

| Entry | Variable | Login gesture | Verified |
|---|---|---|---|
| `claude-code` | `CLAUDE_CONFIG_DIR` | `claude` (and run `/login` inside) | 2026-08-08: an empty context answers logged-out while the default stays logged in |
| `codex-cli` | `CODEX_HOME` | `codex login` | 2026-08-08, same probe |

Linked profiles live in `subscriptions.toml` beside your `config.toml`,
machine-managed and loaded first — so a profile you write by hand under the
same name **wins**, and `unlink` refuses to touch hand-written ones. Two
profiles of one agent resolving the same context are diagnosed out loud: that
is one subscription under two names.

### The manual way

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

## Using a third-party ACP adapter instead

The catalog ships Meltemi's own adapters and no longer points at anyone else's.
That is a change of recommendation, not a prohibition: third-party ACP adapters
exist, they are open source, and Meltemi pilots one exactly like any other
entry the moment you declare it. Nothing in the daemon treats it differently
for not being ours.

Install the adapter however its publisher documents, then declare it and select
it:

```toml
[[fleet.custom]]
id = "vendor-acp-community"
name = "Vendor ACP (third-party adapter)"
command = ["vendor-acp", "--acp"]

[agent]
id = "vendor-acp-community"
```

A one-off is shorter: `agent.command = ["vendor-acp", "--acp"]` in the project
configuration pins the literal command and skips the catalog entirely.

What you should weigh before doing it, said plainly rather than sold either
way:

- **The status and the note are the entry's, not the adapter's.** The legal
  status Meltemi shows for `claude-code` and `codex-cli` describes *the path
  Meltemi ships*. A third-party adapter is a different path with its own terms,
  and Meltemi has no note to show you about it — read the adapter's own
  license and the provider's terms yourself.
- **Some adapters drive a provider's agent SDK rather than its official
  binary.** Where a provider's terms restrict which clients may use a consumer
  subscription, that distinction is the whole question. Meltemi's own adapters
  pilot the official binary with the session you already signed into, which is
  why the catalog takes that route.
- **The layer stops travelling with Meltemi.** Its versions, its updates and
  its supply chain become yours to follow.

Meltemi does not claim any provider blesses its own route either: the Fleet
shows each entry's status and note as the registry declares them, and if a
provider ever publishes a position, the note will be updated **with its
source** rather than with an assumption.

## When something still does not work

- The Fleet view refreshes detection on every query: install, then reopen it.
- The exact command Meltemi shows is data from the registry; run it yourself.
- An id whose pilot layer is missing is refused at launch with a diagnosis
  naming the layer, never degraded silently to another provider.
- Authentication problems belong to the agent's own binary: Meltemi shows its
  error verbatim rather than interpreting it.
