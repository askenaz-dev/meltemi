<!-- SPDX-License-Identifier: Apache-2.0 -->
# Shared instructions with the harness

[Español](es/harness.md)

Use harness rules for conventions you want agents to read before working: language style,
project structure or review expectations. Write a `RULE.md` file, inspect which version applies,
then project the context into the agent's instruction files. Rules are instructions for the
agent; they do not grant tool permissions or replace the permission proxy.

## The four scopes

`<config-dir>` is Meltemi's configuration folder, shown by `meltemi doctor` and listed in
[platforms](plataformas.md#data-paths). `<repo>` is the project root:

```
<config-dir>/harness/rules/<name>/RULE.md                 (1) user
<config-dir>/harness/per-agent/<id>/rules/<name>/RULE.md   (2) user × agent
<repo>/.meltemi/harness/rules/<name>/RULE.md               (3) project
<repo>/.meltemi/harness/per-agent/<id>/rules/<name>/RULE.md (4) project × agent
```

Use ids printed by `meltemi fleet`, such as `claude-code` or `codex-cli`. An unknown per-agent
directory is reported and is not projected. The per-agent directory determines who receives the
rule; a metadata field such as `agents_supported` does not select an agent.

## Write a rule

Create a directory named `no-any-cast`, then save this as its `RULE.md`:

```yaml
---
name: no-any-cast
scope: ["**/*.{ts,tsx}"]
severity: error
description: "Avoid TypeScript escape hatches"
---

Keep types explicit. Explain any unavoidable unsafe cast in the review.
```

The `name` must match the directory name. Use inline lists (`scope: [a, b]`); block lists are
not supported. An invalid or unreadable rule is diagnosed and excluded from projection.

The view understands `name`, `scope`, `severity` and `description`. Other metadata is preserved
and displayed. A `scope` describes the rule in the projected prose; it does not enforce file
access or filter an agent's own operations.

## Which rule applies

Precedence is **1 < 2 < 3 < 4**. The rule with the same name in the more specific scope wins in
full; fields from different versions are not merged. The effective view also shows overridden
and invalid rules with their reasons.

```bash
meltemi harness --agent claude-code
```

- CLI: `meltemi harness [--agent <id>]`, optionally with `--json`.
- TUI: `:harness` or `:harness <id>` in the command palette.
- Desktop: select an agent in Fleet and open its harness section.

These views are read only. Edit a rule in its file, then query the view again.

## Project and user destinations

Project rules join the constitution, direction and active change in the managed block of the
repository's instruction files, such as `AGENTS.md` and `CLAUDE.md`.

```bash
meltemi project
```

Only project rules enter repository files. User rules are projected separately to the supported
agent's own global instruction file after you consent. Meltemi modifies only its managed block;
content outside that block remains yours.

Supported user destinations are `~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md` and
`~/.config/opencode/AGENTS.md`. Codex's base folder follows `CODEX_HOME`; if
`AGENTS.override.md` exists, the ordinary `AGENTS.md` destination is reported as ignored and left
untouched. An agent without a configured global destination is reported rather than assigned a
guessed path.

## Consent for global instructions

To enable a user destination, use the desktop command palette's `context/project` form and put
the relevant catalog ids in `consentUserScope`. The consent is stored in
`harness-consent.toml` in Meltemi's configuration folder. `meltemi project` grants no new consent
implicitly; it uses the already saved consent.

To withdraw it, remove the agent id from that consent file. Future projections stop updating that
agent's global managed block; withdrawing consent does not delete existing file content.

## Current limits

Harness management currently covers rules. Meltemi does not install skills, hooks or subagents
through this view. It does not create native scoped rule formats for other tools. Configure those
in the tool that owns them, and use [agents](agentes.md) to understand how Meltemi launches it.
