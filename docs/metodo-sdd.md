<!-- SPDX-License-Identifier: Apache-2.0 -->
# The SDD method

[Español](es/metodo-sdd.md)

Use a change when you want the requirements, design, implementation tasks and verification to
stay with your repository. The living specifications are under `.meltemi/specs/`; a change's
artifacts live under `.meltemi/changes/<name>/` until you archive it.

You can also start a [free session](sesion-libre.md) and choose Propose or Explore in the same
composer when you need the method. This page describes those existing controls; it does not
change your project's constitution or approve a proposal for you.

## A change and its artifacts

A change has a kebab-case name and four artifacts:

1. **proposal** — why, and what changes (capabilities added/modified);
2. **specs** — EARS requirements grouped by capability, each with scenarios;
3. **design** — the decisions and trade-offs, with any new dependency justified;
4. **tasks** — atomic tasks, one commit each.

Review the proposal and its scenarios before starting implementation. For a small change, you
can prepare the artifacts together while keeping the same review and verification.

## The cycle (verbs)

- `propose <idea>` — scaffold a change and delegate the proposal to an agent.
- `plan <change>` — refine the design and sequence `tasks.md` by dependencies.
- `review <change>` — review the spec deltas as a checklist.
- `verify <change>` — the per-requirement verification checklist; each scenario
  is linked to a test (by the scenario→test naming convention) or marked manual.
- `implement <change> <agent>` — run the agent over `tasks.md`, composing per
  task: checkpoint → turn in the worktree → per-task commit → tick.
- `archive <change> [confirm]` — preview the specification fold; add `confirm` to apply it
  after verification and preserve the change in dated history.

## EARS requirements

Requirements use EARS phrasing with explicit markers (`WHEN`, `WHILE`, `IF`,
`THEN`, `WHERE`, `AND`). A conformant requirement has at least one scenario, and
each scenario's name is the source of its test's name.

## Traceability

Implementation tasks produce commits carrying `Meltemi-Task: <change>/<task>` and
`Meltemi-Req: <capability>/<requirement>` references. Inspect the task result and diff, then use
the verification checklist to record which scenarios passed. A checklist is evidence to review,
not a substitute for running the relevant tests.

## Archive a verified change

`meltemi archive <change>` previews the operation. Add the final `confirm` argument to apply
the specification deltas to the living specifications and preserve the change in dated history. Verification gates can refuse an incomplete change;
read the reported requirement or scenario and complete its evidence before retrying.

For command syntax and exit codes, see the [CLI reference](referencia-cli.md).
