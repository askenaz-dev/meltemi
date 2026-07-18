<!-- SPDX-License-Identifier: Apache-2.0 -->
# v0.1 acceptance milestone

The v0.1 milestone has a precise statement (foundational document §10):

> A developer takes a feature from idea to code entirely in the terminal, with
> reviewable specs, using two agents from different vendors in parallel.

This document is the **acceptance script**: the last verification before calling
something v0.1. It has two runs — an automated one (in CI, with simulated agents
and no network) and a manual one (the maintainer, with real agents) — and both
must pass for the milestone to be accepted.

_Resumen en español al final._

## Acceptance criteria

Each step has an observable criterion:

| # | Step | Criterion | Automated coverage |
|---|------|-----------|--------------------|
| C1 | `propose` | a change is scaffolded under `.meltemi/` and delegated to an agent | `e2e_hito::the_milestone_cycle_reaches_implemented_verified_archived` |
| C2 | `review` | the change's deltas are presented as a checklist; a comment reworks and reopens | `e2e_hito` (checklist) · `e2e_review` (rework by comment) |
| C3 | `implement` (parallel) | two distinct-profile agents work in separate worktrees from a common base; commits keep per-task traceability | `e2e_hito::two_distinct_profiles_work_in_parallel_worktrees_with_traceability` |
| C4 | `verify` | every scenario is linked to a test or manually verified | `e2e_hito` (verify complete) |
| C5 | `archive` | the delta folds into the living truth atomically; the change is preserved dated | `e2e_hito` (archived, living updated) |
| C6 | budgets | the TUI binary is under 25 MB and startup is within budget | release pipeline gate (`release.yml`) |

## Automated run (CI)

The automated acceptance run is the e2e suite, executed by
`cargo test --workspace` on the three platforms (the release pipeline runs it as
a hard gate). It drives the full cycle over a fixture through the product's own
methods, with two `mock-agent` profiles (`--profile fast` / `--profile thorough`)
standing in for two vendors — no network, no real agents (constitution).

Regenerating the verdict from the same commit produces the same result: the
suite is deterministic (no wall-clock or randomness in the daemon paths under
test).

## Manual run (maintainer, real agents)

The maintainer performs the equivalent script once with **two real agents from
different vendors**, and records the result in the acceptance report:

1. In a scratch repository, configure two agents (each its own official binary,
   its own auth — fair play).
2. `meltemi propose "<a real feature idea>"` and review the scaffolded proposal.
3. `meltemi review <change>` — comment on at least one requirement to force a
   rework, then approve once addressed.
4. `meltemi implement <change> <agent-A>` and, on a shared task, race
   `<agent-B>` via a second worktree (`meltemi assign <change> <task> A,B`);
   confirm both work in isolation and commit with traceability.
5. `meltemi verify <change>` — every scenario linked or manually verified.
6. `meltemi archive <change>` — the delta folds into `.meltemi/specs/`.

## Acceptance report

The report that accompanies the `v0.1` tag records, per criterion (C1–C6):

- the result (pass/fail) of the automated run (the CI job and commit SHA);
- the result of the manual run (date, the two agents used, per-step outcome);
- the measured budget values against their §12 budgets;
- any deviations.

Template:

```
# Meltemi v0.1 acceptance report
Commit: <sha>            Date: <YYYY-MM-DD>
Automated run (CI): <job url> — C1..C5 pass, C6 budgets: TUI <size> / 25 MB
Manual run: agents <A>, <B>
  C1 propose ....... pass/fail (notes)
  C2 review+rework . pass/fail
  C3 parallel ...... pass/fail
  C4 verify ........ pass/fail
  C5 archive ....... pass/fail
  C6 budgets ....... <values>
Deviations: <none | ...>
Verdict: <accepted | not yet>
```

## Summary (español)

El hito v0.1 es el escenario de aceptación (§10): una feature de idea a código en
terminal, con specs revisables, con dos agentes de proveedores distintos en
paralelo. Dos corridas: la automatizada (suite e2e en CI con dos perfiles de
`mock-agent`, sin red) y la manual del mantenedor con agentes reales; ambas deben
pasar. Los criterios C1–C6 (propose, review+reelaboración, implement paralelo,
verify, archive, presupuestos §12) tienen cobertura observable. El informe
acompaña al tag con veredicto por criterio y es reproducible desde el mismo
commit.
