---
name: no-any-cast
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx}"]
severity: error
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits the TypeScript escape hatches `as any`, `: any`, and `<any>` in committed source; force a real type, `unknown` + narrowing, or a documented `// @ts-expect-error` instead."
tags: [typescript, type-safety, quality, strictness]
owner_team: dx-platform
---

# no-any-cast

## Rule

Do **not** commit TypeScript source matching `**/*.{ts,tsx}` that contains any of the following type-system escape hatches:

- **Type assertions to `any`:** `value as any`, `<any>value`
- **Annotations of `any`:** `function f(x: any) { … }`, `const x: any = …`, `let arr: any[] = …`, `Record<string, any>`
- **Generic param defaults of `any`:** `function f<T = any>(x: T) { … }`

The intent is to keep `any` out of the source surface entirely; the few places where it's truly needed are handled with explicit `// @ts-expect-error` (per the exception section below).

## Why

- **`any` poisons inference.** It silently propagates through call sites: one `any` parameter turns every downstream binding into `any`, and the compiler stops catching real bugs in code that *looks* typed.
- **It hides real types behind a single keyword.** A `Record<string, any>` is almost always either `Record<string, unknown>` (you don't know yet — narrow at use) or a real shape (write the interface). Both are better outcomes.
- **It defeats the editor.** Autocomplete dies on `any`. Refactor-rename misses calls. "Find references" returns false negatives. The cost compounds across the team.
- **It's contagious in tests.** `mock as any` is a smell that almost always means the test is asserting against an outdated shape; fix the mock, not the cast.

## What to use instead

| Bad                                            | Good                                                              |
|-----------------------------------------------|-------------------------------------------------------------------|
| `function handle(req: any) { … }`             | `function handle(req: Request): Response { … }` (write the type) |
| `const data = JSON.parse(s) as any;`          | `const data: unknown = JSON.parse(s); if (isUser(data)) { … }`   |
| `const items: any[] = …`                       | `const items: Item[] = …` or `const items: unknown[] = …`        |
| `mock.method as any`                          | `mock.method as MockedFn` (typed mock helper)                    |
| `Record<string, any>`                          | `Record<string, unknown>` then narrow with a type guard           |
| `(window as any).__DEBUG__`                    | declare in `global.d.ts`: `interface Window { __DEBUG__?: boolean }` |

Examples:

```ts
// ❌ Bad: any swallows the inferred shape
function dispatch(action: any) {
  return action.type;   // no autocomplete; `type` could be anything
}

// ✅ Good: real discriminated union
type Action = { type: 'inc' } | { type: 'set'; value: number };
function dispatch(action: Action) {
  return action.type;   // narrowed; switch is exhaustive
}
```

```ts
// ❌ Bad: JSON.parse leaks `any`
const cfg = JSON.parse(text);
const port = cfg.server.port;   // accepts anything, crashes at runtime

// ✅ Good: unknown + zod (or hand-rolled guard)
import { z } from 'zod';
const cfg = z.object({ server: z.object({ port: z.number() }) }).parse(JSON.parse(text));
const port = cfg.server.port;   // statically typed `number`
```

```ts
// ❌ Bad: test casts to `any` to avoid fixing the mock shape
expect(mock.fetch as any).toHaveBeenCalledWith({ id: 1 });

// ✅ Good: use the test framework's typed mock helper
expect(jest.mocked(mock.fetch)).toHaveBeenCalledWith({ id: 1 });
```

## When this rule does not apply

These cases are out of scope by design:

- **Type definition files for third-party libs** (`*.d.ts`): when ambient-declaring a library that genuinely has unknown shapes (e.g. legacy JS modules), `any` may be appropriate as the public boundary. Prefer `unknown` even here when possible.
- **Generated code** (`*.gen.ts`, `*.pb.ts`, codegen output): the generator may emit `any`; fix the generator config rather than editing outputs.
- **Tests of the rule itself / type-error fixtures**: a file whose purpose is to fail the lint is allowed to contain the bad form (typically `tests/fixtures/has-any.ts`).

When a specific production file legitimately needs `any` (truly, after considering `unknown`), use the in-line escape hatch with a reason:

```ts
// eslint-disable-next-line @typescript-eslint/no-explicit-any -- bridging JS lib with no @types pkg (issue #314)
declare function legacyApi(opts: any): any;
```

Or, when the failure is `tsc` rather than ESLint:

```ts
// @ts-expect-error -- intentional shape mismatch documented in ADR-0011
const result: Foo = legacyHandler();
```

`@ts-expect-error` is preferred over `@ts-ignore` because it errors if the underlying problem disappears, forcing cleanup.

## How this rule is enforced

When `fdh install` materializes this rule into `.claude/rules/no-any-cast.md` (and equivalents for codex/copilot/opencode), the AI coding agent loads it and flags `any` introductions during edits. Enforcement at the agent layer is **advisory**.

The hard gate is the project's TypeScript + lint config:

- **`tsconfig.json`:** `"strict": true` and especially `"noImplicitAny": true`. This catches *implicit* `any` from missing annotations.
- **ESLint:** the `@typescript-eslint/no-explicit-any` rule at severity `"error"`. This catches *explicit* `any` like `: any` and `as any`.
- **Optional, recommended:** `@typescript-eslint/no-unsafe-*` family (`no-unsafe-assignment`, `no-unsafe-call`, `no-unsafe-member-access`, `no-unsafe-return`, `no-unsafe-argument`) to catch downstream usages of `any` even when the cast itself is suppressed somewhere.

This rule complements those gates by surfacing the issue during the agent's edit cycle (faster feedback than waiting for CI's tsc + ESLint pass).
