<!-- SPDX-License-Identifier: Apache-2.0 -->
# Contributing to Meltemi

Thank you for your interest in Meltemi. Contribution here is **spec-driven**:
functionality enters as a reviewed change, not as a surprise pull request. This
keeps "a clear course drives many sails" true in the project's own development.

_Resumen en español al final._

## Spec-driven contribution

Every feature — anything beyond a trivial fix — enters as a **change proposal**
with its artifacts, in this order:

1. **proposal** — why, and what changes (capabilities added/modified);
2. **specs** — the EARS requirements and their scenarios (the scenarios are the
   definition of "done");
3. **design** — the decisions and trade-offs, with any new dependency justified;
4. **tasks** — the atomic tasks, one commit each.

Open a [change-proposal issue](.github/ISSUE_TEMPLATE/change-proposal.md) first
so the direction is agreed before code is written. A pull request that adds
functionality **must** link an approved change; the PR template asks for it.

## Fast track

Genuinely trivial contributions do not need the full set of artifacts:

- typo and wording fixes in docs or comments;
- formatting-only changes;
- obviously-correct one-line corrections.

Declare the fast track in your PR. Anything that changes behavior is a feature
and takes the full path — when in doubt, open a proposal.

## Quality checklist

Before a change is merged, all of the following must hold (the PR template
repeats them):

- the change is linked and its scenarios are covered by tests or a documented
  verification;
- `cargo clippy -- -D warnings` is clean;
- `cargo fmt --check` is clean;
- the test suite passes on **all three platforms** (Windows, macOS, Linux) —
  Windows is first class, not a later port;
- every source file carries its SPDX header (`docs/politica-spdx.md`);
- dependencies are minimal, pinned, and justified in the change's design.

## Commit convention

- One atomic commit per task; the message references the change and task
  (e.g. `(add-thing 1.2)`), imperative English title.
- Code, identifiers, and commit messages are in **English**; method artifacts
  (proposals, specs, designs, tasks) are in **neutral Spanish**.
- **No co-authorship trailers.** Authorship is the contributor's own git
  identity. `Co-authored-by:` lines are not accepted; the tooling strips them
  and CI rejects them.

## Languages

Community-facing documents (this file, `GOVERNANCE.md`, `CODE_OF_CONDUCT.md`,
`SECURITY.md`) are published in English with a short Spanish summary. The method
artifacts stay in neutral Spanish. End-user text is internationalized (Spanish
and English first).

## Contributor agreement

By contributing you agree to the terms in [CLA.md](CLA.md): your contribution is
licensed under Apache-2.0 with a patent grant, and you assign no copyright.

## Summary (español)

Contribuir es spec-driven: toda funcionalidad entra como propuesta de cambio con
sus artefactos (proposal → specs → design → tasks); abre primero un issue de
propuesta. Las correcciones triviales tienen vía corta declarada. Antes del
merge: clippy/fmt/tests verdes en las tres plataformas, cabecera SPDX, commits
atómicos con referencia y **sin trailers de co-autoría**. Artefactos del método
en español; documentos comunitarios y commits en inglés.
