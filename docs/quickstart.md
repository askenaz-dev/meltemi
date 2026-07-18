<!-- SPDX-License-Identifier: Apache-2.0 -->
# Quickstart

From zero to your first reviewed proposal, in a pure terminal. The scriptable
steps below run against the built binaries — the docs CI verifies them, so a
divergence between this page and the product fails the build.

## 1. Build

With the pinned toolchain (`rust-toolchain.toml`):

```
cargo build --release
```

This produces the `meltemi` client. It starts the daemon (`meltemid`) on demand
over a local socket — no network port is ever opened.

## 2. Verify the client

```
meltemi version      # prints the client version, no daemon needed
meltemi help         # the full scriptable surface
```

Both are local subcommands: they never touch the daemon.

## 3. Configure an agent

Meltemi runs the official binary of the coding agent you already have, with the
agent's own authentication (fair play: Meltemi never reads or stores agent
credentials). Point the project at your agent in `.meltemi/config.toml`:

```toml
[agent]
command = ["<your-agent-binary>", "--acp"]
```

Check detection:

```
meltemi fleet        # the agent catalog crossed with local detection
```

## 4. Your first proposal

```
meltemi propose "add a dark-mode toggle to the settings page"
```

Meltemi scaffolds the change under `.meltemi/changes/`, then delegates the
proposal to your agent under the permission proxy (deny-by-default without a
connected client). Review the spec deltas as a checklist:

```
meltemi review add-dark-mode-toggle
```

## 5. The rest of the cycle

```
meltemi plan add-dark-mode-toggle          # refine design, sequence tasks
meltemi verify add-dark-mode-toggle        # per-requirement checklist
meltemi implement add-dark-mode-toggle <agent>   # deploy over tasks.md
meltemi archive add-dark-mode-toggle       # fold deltas into the living truth
```

Every task runs in an isolated worktree with a pre-task checkpoint and an atomic
commit that traces back to its requirement.

## Accessibility & remote use

- `--json` on any subcommand emits machine-readable output.
- `NO_COLOR` and an ASCII fallback are honored (see
  [accessibility](accesibilidad.md)).
- For remote use, tunnel the **local socket** over SSH — the daemon never
  listens on the network. See [platform notes](plataformas.md).

## Windows note

Under git-bash / MSYS, environment variables that look like paths (e.g.
`MELTEMI_ENDPOINT`) can be mangled. See the
[platform notes](plataformas.md#windows-git-bash) for the remedy before setting
them.
