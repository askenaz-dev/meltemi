<!-- SPDX-License-Identifier: Apache-2.0 -->
# Core parity matrix

The living capability → RPC → surface matrix (constitution §4, gui-tauri-paridad
design D3). `tui/tests/parity.rs` is the blocking gate: it fails when a
client-invocable contract method (`proto/meltemi-proto/src/lib.rs`, `methods`)
is missing from the TUI palette registry (`tui/src/shell/palette.rs`), from the
GUI registry (`desktop/ui/src/lib/registry.ts`) or from this document.

Legend — **CLI**: scriptable subcommand (`docs/referencia-cli.md`). **TUI**:
palette entry (interactive wiring may still be announced-as-reserved). **GUI**:
palette registry entry (every method is invocable; some also have a view).
`—` marks a pre-existing scriptable-surface gap: the capability is registered
and reachable in the interactive surfaces, and the CLI verb is tracked as a
future change — never a silent omission.

## Client-invocable methods

| Method | CLI | TUI | GUI |
|---|---|---|---|
| `status` | `status` | `status` | registry |
| `shutdown` | `stop` | `shutdown` | registry |
| `propose` | `propose` | `propose` | registry |
| `fleet/list` | `fleet` | `fleet` | registry + Fleet view |
| `context/project` | `project` | `project` | registry |
| `session/list` | `sessions` | `sessions` | registry + Sessions view |
| `session/log` | — | `sessions` (drill-in) | registry + session drill-in |
| `session/watch` | — | `sessions` (drill-in) | registry + session drill-in (live stream) |
| `session/cancel` | — | `cancel` (`x`) | registry + session drill-in |
| `session/direct` | `direct` | `direct` (drill-in, entrada de instrucción) | registry |
| `session/start` | `session` | `session` | registry |
| `repo/map` | — | `map` | registry |
| `sdd/constitution` | `constitution` | `constitution` | registry |
| `sdd/explore` | `explore` | `explore` | registry |
| `sdd/propose` | — | `propose` | registry |
| `sdd/plan` | `plan` | `plan` | registry |
| `sdd/gate` | — | `gate` | registry |
| `sdd/review` | `review` | `review` | registry |
| `sdd/review-decide` | — | `review` | registry |
| `sdd/verify` | `verify` | `verify` | registry |
| `sdd/verify-mark` | — | `verify` | registry |
| `sdd/archive` | `archive` | `archive` | registry |
| `sdd/implement` | `implement` | `implement` | registry |
| `sdd/validate` | `validate` | `validate` | registry |
| `project/list` | `projects` | `projects` | registry + project switcher |
| `subscription/link` | `link` | `link` (captura verbatim) | registry + ficha de Flota |
| `subscription/unlink` | `unlink` | `unlink` | registry + ficha de Flota |
| `project/register` | `projects register <path>` | `projects register` (ruta tecleada) | registry |
| `project/forget` | `projects forget <path>` | `projects forget` (ruta tecleada) | registry |
| `analytics/usage` | `usage` | `usage` | registry + Usage view |
| `change/list` | `changes` | `changes` | registry + Project view |
| `change/show` | `show` | `show` | registry |
| `change/workspace` | `workspace` | `workspace` | registry |
| `change/land` | `land` | `land` | registry |
| `spec/list` | `specs` | `specs` | registry + Project view |
| `spec/show` | `specs <capability>` | `specs` | registry |
| `permission/pending` | — | `permissions` (`a`, tray) | registry + Permissions view |
| `permission/decide` | — | `permissions` (tray) | registry + Permissions view |
| `worktree/assign` | `assign` | `assign` | registry |
| `worktree/list` | `worktrees` | `worktrees` | registry |
| `worktree/remove` | — | `worktree-remove` | registry |
| `worktree/diff` | `race` | `race` (tablero) | registry + tablero de carrera |
| `worktree/apply-edit` | `apply-edit` | `apply-edit` | registry + editor save |
| `worktree/merge-file` | — | `merge` | registry + tablero (por archivo) |
| `worktree/dispatch` | `dispatch` | `dispatch`, `d` en el tablero | registry + tablero |
| `checkpoint/create` | — | `checkpoints` | registry |
| `checkpoint/list` | `checkpoints` | `checkpoints` | registry |
| `checkpoint/revert` | `revert` | `revert` | registry + tablero (confirma) |
| `checkpoint/record-op` | — | `checkpoints` | registry |
| `commit/task` | `commit` | `commit` | registry + tablero |

A method having a home in the three surfaces is the floor, not the ceiling:
what a method *reports* has to reach them alike too. `fleet/list` is the
worked example — its per-entry detail (the layers, the **provenance** of each
find, the composed state and the remedy) is rendered by the scriptable
subcommand in human mode, by the Fleet view and by the desktop detail, and
travels verbatim under `--json` because it is part of the contract. A field
visible in one surface only is a §4 break even when the method itself is
registered everywhere.

### `detach`, and why one surface declines it

`session/start` and `session/direct` both accept `detach`: the caller declares
it is staying, so the daemon answers as soon as the session exists and keeps the
session alive between turns. Every surface **can** send it — it is a parameter of
a method already registered in all three, so it needs no row of its own.

The desktop surface sends it. The scriptable one does not, and the reason is in
the two rows above rather than in a preference: `session/log` and `session/watch`
are `—` for the CLI. A detached start there would print an identifier and exit,
having shown the user nothing of the turn, and — worse — a client that leaves
takes the session's permissions with it, since zero clients for the grace is the
constitutional deny (§3).

So this is **not** a §4 break: the capability is reachable from every surface,
and the one that declines it declines it for a stated reason, in its own help.
It becomes a break the day the CLI gains a way to read the stream and still does
not offer this (sesion-que-espera design D3).

## Infrastructure

| Method | Role |
|---|---|
| `initialize` | Mandatory version-negotiation handshake; every surface performs it on connect (CLI/TUI `connect_and_init`, GUI bridge). Not a user-invocable capability. |

## Daemon-initiated traffic

Not client-invocable; delivered to every connected surface alike.

| Method | Kind | TUI | GUI |
|---|---|---|---|
| `session/event` | notification | transcript append | `daemon:incoming` → transcript |
| `permission/request` | request (held) | notice; tray decides | `daemon:incoming`; tray decides |
| `permission/timeout` | notification | persistent notice | persistent notice |
| `permission/changed` | notification | tray counter | tray counter |
