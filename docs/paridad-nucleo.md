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
| `session/cancel` | — | `cancel` (`x`) | registry + session drill-in |
| `session/direct` | `direct` | `direct` | registry |
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
| `change/list` | `changes` | `changes` | registry + Project view |
| `change/show` | `show` | `show` | registry |
| `spec/list` | `specs` | `specs` | registry + Project view |
| `spec/show` | `specs <capability>` | `specs` | registry |
| `permission/pending` | — | `permissions` (`a`, tray) | registry + Permissions view |
| `permission/decide` | — | `permissions` (tray) | registry + Permissions view |
| `worktree/assign` | `assign` | `assign` | registry |
| `worktree/list` | `worktrees` | `worktrees` | registry |
| `worktree/remove` | — | `worktree-remove` | registry |
| `worktree/diff` | `race` | `race` | registry |
| `worktree/merge-file` | — | `merge` | registry |
| `worktree/dispatch` | `dispatch` | `dispatch` | registry |
| `checkpoint/create` | — | `checkpoints` | registry |
| `checkpoint/list` | `checkpoints` | `checkpoints` | registry |
| `checkpoint/revert` | `revert` | `revert` | registry |
| `checkpoint/record-op` | — | `checkpoints` | registry |
| `commit/task` | `commit` | `commit` | registry |

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
