<!-- SPDX-License-Identifier: Apache-2.0 -->
# Meltemi

**The spec-driven control plane for agentic development.** Open source
(Apache-2.0), free, community-built. Meltemi orchestrates the coding agents you
already use — through open standards — under a discipline where no line of code
is written without a reviewed specification.

> **One course, many sails.** A clear spec drives any number of agents, from any
> vendor, without locking you to one.

_Léeme en español: [LEEME.md](LEEME.md)._

## What it is

Meltemi is a headless daemon (`meltemid`) plus thin clients — a terminal UI and
CLI (`meltemi`), with a desktop GUI planned. It speaks open standards: the Agent
Client Protocol to pilot agents, MCP for tools, JSON-RPC over a **local socket
only** (no network port, ever). You bring your own agent, your own key, your own
model.

The workflow is spec-first: a change is proposed, its scenarios reviewed, and
only then implemented — task by task, in isolated git worktrees, with automatic
pre-task checkpoints and an atomic per-task commit that traces every line back to
the requirement that originated it.

## What it is not

Not a general-purpose editor (code editing is utilitarian, in service of the
agent loop), not another coding agent, not a cloud service, not CI/CD, not a
marketplace. No credits, no fees, no lock-in.

## Status

Phase 1, pre-v0.1. The daemon, the spec engine, the permission proxy, worktree
orchestration, checkpoints, per-task commits, and the full SDD cycle
(`propose → plan → review → verify → archive → implement`) are implemented and
tested on Windows, macOS, and Linux — **Windows is first class**. The interactive
TUI shell and the desktop GUI are in progress. See
[`docs/plan-de-cambios.md`](docs/plan-de-cambios.md) for the roadmap.

## Architecture at a glance

```
  meltemi (TUI + CLI) ─┐
                       ├─ JSON-RPC 2.0 over a local socket ─→ meltemid (daemon)
  desktop GUI (phase 2)┘        (UDS 0700 / Windows named pipe)      │
                                                                     ├─ ACP → agent (in a worktree)
                                                                     ├─ spec engine (.meltemi/)
                                                                     └─ permission proxy (deny-by-default)
```

All logic lives in the daemon; clients are thin and interchangeable — any daemon
capability is reachable from every surface (core parity).

## Install

Meltemi is built from source with a pinned Rust toolchain (see
`rust-toolchain.toml`):

```
git clone <this repository>
cd meltemi
cargo build --release
```

The `meltemi` binary is the client; it starts the daemon on demand. Signed,
per-platform release packages arrive with the release change.

## First step

```
meltemi help            # the scriptable surface (see docs/referencia-cli.md)
meltemi propose "add a dark-mode toggle to settings"
```

Then walk the quickstart: [`docs/quickstart.md`](docs/quickstart.md).

## Documentation

- [Quickstart](docs/quickstart.md) — zero to your first reviewed proposal
- [Architecture](docs/arquitectura.md)
- [The SDD method](docs/metodo-sdd.md)
- [CLI reference](docs/referencia-cli.md) (generated)
- [Accessibility](docs/accesibilidad.md)
- [Platform notes](docs/plataformas.md)
- [Contributing](CONTRIBUTING.md) · [Governance](GOVERNANCE.md) · [Security](SECURITY.md)

## License

Apache-2.0, for good (constitution §12). See [CLA.md](CLA.md) for the bounded
contributor agreement.
