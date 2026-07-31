<!-- SPDX-License-Identifier: Apache-2.0 -->
# The SDD method

Spec-driven development is Meltemi's discipline — and how Meltemi builds itself.
Nothing is implemented without a reviewed change; the scenarios are the
definition of "done".

That rule governs *this repository* (constitution §1). For your own work the
method is an offer, not a gate: a new session is a governed
[free session](sesion-libre.md) by default, and proposing or exploring is one
control away in the same composer. The discipline below is what Meltemi makes
easy to sustain — it is not a toll you pay before starting.

## A change and its artifacts

A change is a kebab-case directory with four artifacts, in order:

1. **proposal** — why, and what changes (capabilities added/modified);
2. **specs** — EARS requirements grouped by capability, each with scenarios;
3. **design** — the decisions and trade-offs, with any new dependency justified;
4. **tasks** — atomic tasks, one commit each.

Trivial changes take the fast forward (all artifacts at once); nothing takes the
null path.

## The cycle (verbs)

- `propose <idea>` — scaffold a change and delegate the proposal to an agent.
- `plan <change>` — refine the design and sequence `tasks.md` by dependencies.
- `review <change>` — review the spec deltas as a checklist.
- `verify <change>` — the per-requirement verification checklist; each scenario
  is linked to a test (by the scenario→test naming convention) or marked manual.
- `implement <change> <agent>` — deploy the agent over `tasks.md`, composing per
  task: checkpoint → turn in the worktree → per-task commit → tick.
- `archive <change>` — fold the change's deltas into the living truth atomically,
  gated by verification, and preserve the change in the dated history.

## EARS requirements

Requirements use EARS phrasing with explicit markers (`WHEN`, `WHILE`, `IF`,
`THEN`, `WHERE`, `AND`). A conformant requirement has at least one scenario, and
each scenario's name is the source of its test's name.

## Traceability

Each task produces an atomic commit carrying `Meltemi-Task: <change>/<task>` and
one `Meltemi-Req: <capability>/<requirement>` per requirement it implements — so
every line traces back to what originated it (constitution §8). Commits never
carry co-authorship trailers.

## Bootstrap (two stages)

Meltemi currently develops itself with a borrowed method tool under
`openspec/changes/`; the constitution and direction already live in `.meltemi/`
(the destination format). When the phase-1 spec engine is operative, the living
specs migrate to `.meltemi/specs/` via a dedicated change.
