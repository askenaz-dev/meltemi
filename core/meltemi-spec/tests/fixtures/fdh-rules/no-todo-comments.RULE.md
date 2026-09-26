---
name: no-todo-comments
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx,js,jsx,go,py,rs,java,kt,swift,rb}"]
severity: warning
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits naked TODO/FIXME/XXX/HACK/BUG comments in committed source; require a tracker reference (issue, ADR, or spec) so unfinished work is auditable instead of buried."
tags: [code-quality, debt-tracking, comments, process]
owner_team: dx-platform
---

# no-todo-comments

## Rule

Do **not** commit source files containing comments of the form:

```
TODO       FIXME       XXX       HACK       BUG       REVIEW       OPTIMIZE
```

— unless each occurrence is **immediately followed by an explicit tracker reference** that future readers can audit. Accepted reference shapes:

- An issue id: `TODO(#123): describe what's left`
- A spec or ADR id: `TODO(ADR-0007): waiting on auth model`
- An OpenSpec change id: `TODO(component-versioning): finish bundle hashing`
- An author handle WITH a target: `TODO(@gortizr, expires 2026-09): retire fallback after migration`

The `scope` glob covers the eight languages forge code is written in: `.ts`, `.tsx`, `.js`, `.jsx`, `.go`, `.py`, `.rs`, `.java`, `.kt`, `.swift`, `.rb`.

## Why

- **Naked TODOs are write-once, read-never.** A `// TODO: fix this later` with no context decays into permanent noise within weeks. Future readers (or the original author six months later) cannot distinguish "trivial cleanup" from "this is unsafe in production".
- **They hide real risk.** `// FIXME: race condition` deserves an issue, a deadline, and probably a `// nolint` or feature flag, not a comment that fades into background.
- **They short-circuit accountability.** Tickets get triaged, prioritized, and assigned; comments don't. Forcing the tracker reference moves the work into the system that's designed to handle it.
- **They corrode signal.** Once a codebase has 200 TODOs, nobody reads any of them. Anchoring each to a tracker keeps the signal honest: a TODO without an id is a smell, not status.

## What to use instead

```ts
// ❌ Bad: untracked, indefinite, will rot
// TODO: refactor this when we have time
function legacyPricing() { /* ... */ }

// ❌ Bad: vague urgency, no owner, no deadline
// FIXME: this is racy
await fetchAndCache(id);

// ✅ Good: trackable, scopeable, can be resolved
// TODO(#482): retire legacyPricing once 2026-Q3 pricing API ships
function legacyPricing() { /* ... */ }

// ✅ Good: explicit risk + tracker
// FIXME(#491): race when two requests arrive within 100ms — locked here pending design.
await fetchAndCache(id);

// ✅ Good: dated waiver with owner accountable for cleanup
// HACK(@gortizr, expires 2026-09): polyfill until Node 22 lands in CI
require("./node18-polyfill");
```

## When this rule does not apply

- **Generated code** (`*.gen.{ts,go,py}`, `*_pb.go`, `*.pb.ts`, vendored `node_modules/`, `dist/`, `build/`): the rule's enforcement targets committed *human* source; build outputs are out of scope.
- **External vendor snippets** copied verbatim (license headers, embedded SDK fragments): leave their comments intact and document the source in a sibling README.
- **Top-level CHANGELOG/ADR/spec markdown**: tracker references are the file itself, not embedded TODO comments.
- **Test fixtures** asserting on the lint itself (e.g. `tests/fixtures/should-fail/has-todo.ts`): exempt — the fixture must contain the bad form to be testable.

## How this rule is enforced

When `fdh install` materializes this rule into the consumer's `.claude/rules/no-todo-comments.md` (and equivalents for codex/copilot/opencode), the AI coding agent loads it into context and flags offending edits during sessions. Enforcement at the agent layer is **advisory** — the agent won't refuse a write but will surface the issue.

Hard gates for full enforcement live in the project's lint pipeline:

- **TypeScript/JavaScript:** ESLint's `no-warning-comments` rule (configured with `terms: ["todo", "fixme", "xxx", "hack", "bug", "review", "optimize"]`, `location: "anywhere"`) — combine with an in-house plugin that whitelists the `TODO(<ref>):` shape.
- **Go:** `go vet` does not catch this; use `golangci-lint`'s `godox` linter with `keywords: [TODO, FIXME, XXX, HACK, BUG]`.
- **Python:** `ruff` or `pylint`'s `W0511` (`fixme`) — disable globally then re-enable with a custom allowlist matching the tracker-reference shape.

This rule complements those gates by surfacing the issue during the agent's edit cycle rather than waiting until PR-time CI fails.
