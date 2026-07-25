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

Meltemi is one product with two surfaces on three operating systems:

- **A desktop application** (Windows, macOS, Linux) — the visual control plane:
  sessions with live transcripts, the permission tray, the fleet of agents, the
  spec editor, line-by-line diff review, and a utilitarian code editor.
- **A terminal application** (`meltemi`) — the same power in a TUI plus a fully
  scriptable CLI, so it works over SSH on a headless box.

Both are thin clients over a headless daemon (`meltemid`) that holds all the
logic. The rule is absolute: **any daemon capability is reachable from every
surface** — that is core parity, and CI fails the build if a capability lands in
only one place.

It speaks open standards: the Agent Client Protocol to pilot agents, MCP for
tools, LSP for code intelligence, JSON-RPC over a **local socket only** (no
network port, ever). You bring your own agent, your own key, your own model.

The workflow is spec-first: a change is proposed, its scenarios reviewed, and
only then implemented — task by task, in isolated git worktrees, with automatic
pre-task checkpoints and an atomic per-task commit that traces every line back
to the requirement that originated it.

## What it is not

Not a general-purpose editor (code editing is utilitarian, in service of the
agent loop), not another coding agent, not a cloud service, not CI/CD, not a
marketplace. No accounts, no credits, no fees, no lock-in, no telemetry.

---

## Install

### Option A — the release installers (recommended)

Download the artifacts for your platform from the latest release, verify them,
and install:

| Platform | Desktop app | Terminal client |
|---|---|---|
| Windows 10 1809+ / 11 | `Meltemi_<version>_x64_en-US.msi` | `meltemi-Windows.zip` |
| macOS 13+ | `Meltemi_<version>_x64.dmg` | `meltemi-macOS.tar.gz` |
| Linux (glibc) | `meltemi_<version>_amd64.AppImage` or `.deb` | `meltemi-Linux.tar.gz` |

Every release publishes `SHA256SUMS` with a detached signature. Verify before
installing:

```bash
sha256sum --check SHA256SUMS
```

(`shasum -a 256` on macOS; `Get-FileHash` on Windows.)

The desktop installer stays under 15 MB because it uses your operating system's
own web engine — nothing is embedded; on Windows it bootstraps the system
runtime when missing.

**Windows**

```powershell
Get-FileHash .\Meltemi_0.1.0_x64_en-US.msi -Algorithm SHA256
msiexec /i .\Meltemi_0.1.0_x64_en-US.msi
```

Then the terminal client, with its one-line installer:

```powershell
iwr -useb https://meltemi.dev/install.ps1 | iex
```

**macOS and Linux**

```bash
curl -fsSL https://meltemi.dev/install.sh | sh
```

That places `meltemi`, `meltemid` and the short `mel` alias on your `PATH`. For
the desktop app, open the `.dmg`, or on Linux install the `.deb` (or run the
`.AppImage`):

```bash
sudo dpkg -i meltemi_0.1.0_amd64.deb
```

Both installer scripts are short, auditable, publish their own hash, verify what
they download and refuse to proceed on a mismatch — there is no blind
`curl | sh`. Manual instructions live in each script's header and in
[`docs/release.md`](docs/release.md).

### Option B — from source

You need the pinned Rust toolchain (`rust-toolchain.toml` selects it
automatically) and, for the desktop app, Node 24+.

```bash
git clone <this repository>
cd meltemi
cargo build --release -p meltemid -p meltemi
```

For the desktop app, build the frontend first — it is embedded at compile time:

```bash
npm ci --prefix desktop/ui
npm run build --prefix desktop/ui
cargo build --release -p meltemi-desktop
```

On Linux the desktop app needs the system web engine's development packages:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Binaries land in `target/release/`: `meltemi` (terminal), `meltemid` (daemon),
`meltemi-desktop` (desktop app). To build the installers for your platform:

```bash
cd desktop && ui/node_modules/.bin/tauri build
```

### Verify your installation

```bash
meltemi status
```

That starts the daemon on demand and reports its version. Then:

```bash
meltemi fleet
```

lists the agents detected on this machine.

---

## Connect your agents

Meltemi ships no agents: it drives the official binaries you already have, with
the authentication each of them manages on its own. Open the **Fleet** view (or
run `meltemi fleet`) and you will see, per agent, whether it is detected and —
when something is missing — **which layer is missing and the exact command that
installs it**.

Some CLIs speak the Agent Client Protocol natively; others are driven through an
official ACP adapter, which means two layers must be present. That is why an
agent can be installed and still not be pilotable: the Fleet view names the gap
instead of just saying "not detected".

**Read [the agents guide](docs/agentes.md)** for the per-agent detail: what to
install, how detection works on each operating system, how to declare an agent
Meltemi does not know, and how to run **several accounts or subscriptions of the
same provider at the same time** — launch profiles redirect the authentication
context of the official binary, and Meltemi never reads, stores or forwards
credentials.

---

## First steps

```bash
cd your-project
meltemi
```

That opens the interactive terminal UI. Or start from the CLI:

```bash
meltemi propose "add a dark-mode toggle"
```

In the desktop application, **New session** in the top bar picks an agent and a
mode (explore, propose, dispatch a task, direct a running session). The command
palette (`Ctrl+K`) reaches every daemon capability, with typed forms generated
from the protocol's own schemas.

Then walk the quickstart: [`docs/quickstart.md`](docs/quickstart.md).

## Architecture at a glance

```
  meltemi (TUI + CLI) ─┐
                       ├─ JSON-RPC 2.0 over a local socket ─→ meltemid (daemon)
  meltemi-desktop ─────┘        (UDS 0700 / Windows named pipe)      │
                                                                     ├─ ACP → agent (in a worktree)
                                                                     ├─ spec engine (.meltemi/)
                                                                     ├─ LSP → your own language servers
                                                                     └─ permission proxy (deny-by-default)
```

Remote work: the daemon never opens a network port. `meltemi tunnel` composes
the `ssh` command that reverse-forwards its local socket, so a laptop can drive
a fleet running on a workstation.

## Privacy and posture

- **No accounts, no server.** Nothing to sign up for; nothing phones home.
- **No telemetry.** Every metric is computed and kept on your machine
  (constitution §9). Any future telemetry would be opt-in and publicly specified
  before existing.
- **No credentials.** Meltemi never reads, stores or reuses an agent's
  credentials, and never impersonates another client (constitution §2).
- **Deny by default.** With no client connected every permission request is
  denied; agents work in isolated worktrees with pre-task checkpoints.

## Status

Phase 2. The daemon, the spec engine, the permission proxy, worktree
orchestration, checkpoints, per-task commits and the full SDD cycle
(`propose → plan → review → verify → archive → implement`) are implemented and
tested on Windows, macOS and Linux — **Windows is first class**. The desktop
application and the terminal surface are in core parity, verified by CI. See
[`docs/plan-de-cambios.md`](docs/plan-de-cambios.md) for the roadmap.

## Documentation

- [Quickstart](docs/quickstart.md) — zero to your first reviewed proposal
- [Agents guide](docs/agentes.md) — install, detect and configure your agents
- [Architecture](docs/arquitectura.md)
- [The SDD method](docs/metodo-sdd.md)
- [CLI reference](docs/referencia-cli.md) (generated)
- [Core parity matrix](docs/paridad-nucleo.md) — capability → RPC → surface
- [Design system](docs/ux/design-system.md)
- [Accessibility](docs/accesibilidad.md) · [Platform notes](docs/plataformas.md)
- [Releases and verification](docs/release.md) · [Versioning](docs/versionado.md)
- [Contributing](CONTRIBUTING.md) · [Governance](GOVERNANCE.md) · [Security](SECURITY.md)

## License

Apache-2.0, for good (constitution §12). See [CLA.md](CLA.md) for the bounded
contributor agreement.
