---
name: no-console-log
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx,js,jsx}"]
severity: warning
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits console.log/info/warn/error/debug in committed TS/JS source; use the project's logger instead."
tags: [typescript, javascript, logging, quality, observability]
owner_team: dx-platform
---

# no-console-log

## Rule

Do **not** commit calls to `console.log()`, `console.debug()`, `console.info()`, `console.warn()`, or `console.error()` in TypeScript or JavaScript source files matched by the glob `**/*.{ts,tsx,js,jsx}`.

## Why

- `console.*` leaks debug output to production browsers and server logs where it is unstructured, unfilterable, and indexed inconsistently.
- forge services emit structured logs via a project-configured logger (correlation IDs, log levels, PII scrubbing, sink routing). Mixing `console.*` fragments observability and breaks log aggregation parsing.
- It is the most common source of accidental PII / token leaks in browser builds — credentials passed to `console.log("debug", req)` end up in browser devtools and any error reporting service the user has enabled.

## What to use instead

Every forge service has a configured logger. The exact import depends on the stack:

- **Backend Node/TypeScript services:** `import { logger } from '@/lib/logger'` (project-specific path; check the README).
- **Frontend React/Next:** `import { logger } from '@/lib/log'` or the analytics/telemetry helper documented in the project README.
- **Generic tooling scripts:** if the script is small and standalone, prefer `process.stderr.write(...)` for diagnostics rather than `console`.

Examples:

```ts
// ❌ Bad: leaks to browser console + breaks log parsing + risks PII exposure
console.log("user clicked checkout", { userId, sessionToken });

// ✅ Good: structured, level-controlled, correlation-aware, PII-scrubbed by logger
logger.info("user clicked checkout", { userId, event: "checkout.clicked" });
```

## When this rule does not apply

Out of scope by design — the `scope` glob does not need to exclude these because the project's lint config typically handles them, but agents should still recognize the exemption:

- **Test files** (`*.test.{ts,tsx,js,jsx}`, `*.spec.*`): `console.log` for debugging is fine; CI typically scrubs or ignores them.
- **Build scripts** (`scripts/*.{ts,js}`, `*.config.{ts,js}`, `vite.config.ts`, etc.): infrastructure code may use `console` for build-time diagnostics.
- **Standalone CLI tools** (binaries the team distributes via npm or similar): `stdout` / `stderr` via `console.log` and `console.error` is the right output channel.

If a specific production file legitimately needs `console.*` (rare, e.g. catastrophic-failure logger of last resort), add an in-line comment with a reason:

```ts
// eslint-disable-next-line no-console -- last-resort log when telemetry pipeline is unreachable
console.error("telemetry sink unreachable; falling back to console:", err);
```

## How this rule is enforced

When `fdh install` materializes this rule into the consumer's `.claude/rules/no-console-log.md` (and equivalents for other agents), the AI coding agent loads it into context and uses it to flag offending edits during sessions. Enforcement at the agent layer is **advisory**.

The hard gate is the project's lint pipeline — typically ESLint's `no-console` rule configured at `severity: "error"`. This rule complements that gate by surfacing the issue earlier (during the agent's edit) rather than at PR time.
