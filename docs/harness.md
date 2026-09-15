<!-- SPDX-License-Identifier: Apache-2.0 -->
# The harness: say it once, adjust it where it matters

The **harness** is what you want every agent to know before it starts working:
your conventions, your stack, the things you would otherwise repeat in ten
repositories. You write it once, and Meltemi carries it to the agents you
actually run.

Phase 1 covers the **Rules** pillar and the effective view. Skills, hooks and
subagents are named as future work and are not here yet.

This document answers four questions: **where the pieces live**, **what a piece
looks like**, **which one wins when two say different things**, and — the one
that matters most — **where each scope is allowed to write**.

## The four scopes

A rule is a directory with a `RULE.md` inside it. The directory's name is the
rule's identity.

```
<config>/meltemi/harness/rules/<name>/RULE.md                 (1) user
<config>/meltemi/harness/per-agent/<id>/rules/<name>/RULE.md   (2) user × agent
<repo>/.meltemi/harness/rules/<name>/RULE.md                   (3) project
<repo>/.meltemi/harness/per-agent/<id>/rules/<name>/RULE.md     (4) project × agent
```

`<config>` is your platform's configuration directory — the same one that
already holds `config.toml`, your permission rules and your subscriptions.

`<id>` is an id from the fleet catalog (`meltemi fleet` lists them:
`claude-code`, `codex-cli`, `opencode`, …). **An id the catalog does not know is
reported, not read**: projecting for an agent Meltemi cannot name is writing
blind, so the directory is listed as unknown and nothing in it is used.

The per-agent axis is a *directory*, not a field inside the file. A directory is
a matter of form, which the daemon may validate; a field would make the daemon
interpret who a rule is for, which it does not do.

## What a piece looks like

The format is [FDH](https://github.com/askenaz-dev)'s `RULE.md`, taken whole:

```yaml
---
name: no-any-cast
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx}"]
severity: error
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits the TypeScript escape hatches ..."
tags: [typescript, type-safety, quality, strictness]
owner_team: dx-platform
---

The body of the rule, in Markdown.
```

Meltemi **knows** four keys — `name`, `scope`, `severity`, `description` —
because those are what the view shows and the projection uses. **Every other key
is kept verbatim and shown**: rejecting an unknown key would turn each field FDH
adds into an error here, and dropping it silently would hide what the rule's
author put there on purpose.

Two rules of form, both of which produce a *diagnostic* rather than a guess:

- **`name` must match the directory.** If they differ the rule is listed as
  invalid and is not projected: the directory name is the identity precedence
  resolves by, and a front matter that says otherwise would make the view and
  the resolution talk about different things.
- **Only inline lists are supported** (`key: [a, b, c]`). A block list — `key:`
  followed by lines with dashes — is diagnosed by name, with the expected shape.
  Reading its first line and carrying on would give a silently wrong `scope`: a
  rule applying where nobody asked it to.

`agents_supported` is **kept and shown, and governs nothing**. FDH writes
`codex`; this catalog says `codex-cli`. The vocabularies do not match, and
inventing the equivalence would start applying rules to the wrong agent the day
either side adds a second entry. Who receives a rule is decided by the
`per-agent/<id>/` axis, which uses this catalog's ids and admits no ambiguity.

## Which one wins

Precedence is **1 < 2 < 3 < 4**: the more specific wins, project beats user —
the same direction that already governs config, permissions and profiles.

Resolution is **by rule name, and the winner wins whole**. If `no-any-cast`
exists in scope 1 and in scope 4, scope 4's file applies in its entirety;
fields are not merged. A partial merge would produce a rule nobody wrote, with
one layer's `scope` and another's body, and no one could explain where it came
from.

**What does not apply is shown too, with its reason.** That is the point of the
view: a listing of only what governs answers "what do I have" and leaves "why is
mine not the one applying" unanswered, which is the question people arrive with.

Read it from any surface:

```bash
meltemi harness --agent claude-code
```

- **CLI**: `meltemi harness [--agent <id>]`, with `--json` like every other verb.
- **TUI**: `:harness` or `:harness <id>` in the palette; the reading appears
  under the Fleet catalog. It is asked for, never polled.
- **GUI**: select an agent in the Fleet; the harness is a section of its drawer.

All three are **read only**. Authoring a rule happens in its file, or in FDH.

## Where each scope writes — and where it must not

This is the part worth reading twice.

| content | sources | destinations |
|---|---|---|
| **repository** | constitution + rumbo + active change + **project rules (3, 4)** | the files in `context-targets.toml`, inside the repo (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`) |
| **user** | **only user rules (1, 2)** | each agent's own user-scope instruction file |

**Your global harness never enters a repository file.** Not by convention: the
function that compiles the repository's content does not take the global sources
as an argument, so there is no parameter through which they could arrive. A
guard written as an `if` can be dropped in the next refactor; a guard written as
a signature that does not admit the data cannot. A test projects with a global
harness present and requires the repository to come out untouched.

Project rules enter the managed block with their `scope` in prose next to each
one ("applies to `src/**/*.ts`"), because that block is Markdown for an agent
reading instructions, not a format with fields.

### Writing into an agent's own configuration

The user-scope destinations are files that belong to the agent, not to Meltemi.
Three consequences:

**Only verified destinations exist.** `context-targets.toml` carries a
user-scope entry only when it was verified against the provider's own
documentation, with the page and the date it was read. Today: `~/.claude/CLAUDE.md`,
`~/.codex/AGENTS.md` and `~/.config/opencode/AGENTS.md`. An agent without an
entry is reported as *no verified global destination* — a declared absence, never
a path guessed from a pattern.

Verifying produced two facts no assumption would have given. Codex reads
`AGENTS.override.md` **instead of** `AGENTS.md` when the override exists, so
with one present the destination is reported as ignored by the agent and left
alone — writing it anyway would fill a file nobody reads and look like it
worked. And Codex's directory moves with `CODEX_HOME`.

**The first write is consented to, per agent.** Nothing is written into an
agent's user file until you say so. Consent is passed on `context/project` as
`consentUserScope: ["<id>", …]`, recorded in `harness-consent.toml` in your
configuration directory *before* the projection runs, and is only ever added to.
A destination you have not consented to is reported as awaiting consent and left
untouched. **Withdrawing consent is editing that file**: remove the id, and the
managed block stops being updated — nothing of yours is deleted.

The scriptable surface consents to nothing implicitly: `meltemi project` sends
an empty list, because a flag nobody typed is not a decision. Today consent is
granted through the GUI's generic method form; a dedicated gesture is future
work, not a silent default.

**Only Meltemi's own block is ever touched**, byte for byte, exactly as the
repository projection already works.

## The boundary with FDH

[Forge Development Hub](https://github.com/askenaz-dev) is where rules are
authored and distributed; Meltemi is where they are resolved and projected. The
formats are FDH's, adopted as-is so the work is not done twice.

The two write **disjoint sets of files**, and this is a fact rather than a
promise: FDH's installer writes its own files (`.claude/rules/<name>.md` and a
marker beside them); Meltemi writes inside its managed block in
`CLAUDE.md` / `AGENTS.md`. There is no collision in v1 to resolve.

Aligning the two id vocabularies (D4) is work on FDH's side, after this, against
what is fixed here.

## What is not here yet

- **Skills, hooks and subagents**: named, not implemented. Each is its own change.
- **Authoring from the surfaces**: read only in v1, on purpose.
- **Native scoped destinations** (`.cursor/rules`, Copilot's `applyTo`): out of
  scope; project rules carry their `scope` in prose instead.
- **Whether "the winner wins whole" is enough**: use will say. A finer
  composition is decided with evidence, not in advance.

## See also

- `docs/agentes.md` — the fleet: what to install and how detection works
- `docs/arquitectura.md` — where the daemon and its clients stand
- `docs/referencia-cli.md` — the `harness` verb, generated from the CLI itself
