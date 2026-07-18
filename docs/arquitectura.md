<!-- SPDX-License-Identifier: Apache-2.0 -->
# Architecture

Meltemi is a headless daemon with thin, interchangeable clients. All logic lives
in the daemon; any capability it exposes is reachable from every surface (core
parity, constitution §4).

## Components

- **`core/meltemid`** — the daemon. Owns the protocol server, session
  orchestration, the permission proxy, worktree/checkpoint/commit machinery, and
  the SDD verbs.
- **`core/meltemi-spec`** — the spec engine: EARS parsing, delta application
  (`apply_delta`), validation, and context projection.
- **`proto/meltemi-proto`** — the serde types of the daemon↔client contract,
  validated against the JSON Schemas in `proto/schemas/` (the source of truth).
- **`tui/`** — the `meltemi` binary: a scriptable CLI and the interactive TUI.
- **`desktop/`** — the Tauri GUI (phase 2).

## Transport

Clients speak **JSON-RPC 2.0** with line delimitation over a **local socket**:
a Unix domain socket (`0700`) on macOS/Linux, a user-ACL named pipe on Windows.
There is **no network port, ever**. Every connection begins with `initialize`
for contract-version negotiation.

## The agent boundary

The daemon pilots agents over the **Agent Client Protocol (ACP)**, running each
agent's official binary with the agent's own authentication. Meltemi never reads,
stores, or reuses agent credentials (fair play, constitution §2). Agents run in
isolated git worktrees; permission requests are proxied to the client and
resolved by rules or by the human, deny-by-default when no client is connected.

## The spec-driven core

Method artifacts live in `.meltemi/` per repository: the constitution, the
direction (`rumbo`), the living specs, and the changes. A change flows
`propose → plan → review → verify → archive`; `implement` deploys agents over
its `tasks.md`. Archiving folds the change's deltas into the living specs
atomically. The scenarios of a spec are the definition of "done" and the source
of test names.

## Persistence

Session logs are append-only JSONL in the user's data directory (see
[platform notes](plataformas.md)). Metrics are computed locally only — no hidden
telemetry (constitution §9).
