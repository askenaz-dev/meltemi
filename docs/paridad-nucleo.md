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
| `harness/effective` | `harness` | `harness [agente]` (bajo Flota) | registry + drill-in de Flota |
| `session/list` | `sessions` | `sessions` | registry + Sessions view |
| `session/log` | `log` | `sessions` (drill-in) | registry + session drill-in |
| `session/watch` | `watch` | `sessions` (drill-in) | registry + session drill-in (live stream) |
| `session/cancel` | `cancel` | `cancel` (`x`) | registry + session drill-in |
| `session/direct` | `direct` | `direct` (drill-in, entrada de instrucción) | registry |
| `session/set-config-option` | `set-option` | `set-option` (reservado) | registry + selector en la ficha de sesión |
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
| `permission/pending` | `permissions`; the terminal approver (every verb: a question is asked only while it still stands) | `permissions` (`a`, tray) | registry + Permissions view |
| `permission/decide` | `approve`, `deny`; the terminal approver (`r` + `y` in `session` and `propose`) | `permissions` (tray) | registry + Permissions view |
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
registered everywhere. The verified level follows the same rule, and with it
the agent version it was measured against (`verifiedVersion`, codex-al-dia
D11): «verified L2 with 0.156.1» in the scriptable subcommand, «verif L2
0.156.1» in the Fleet view, «verificado con 0.156.1» in the desktop pill and
detail.

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

### The empty-turn verdict, and where each surface reads it

A turn the agent closes as finished having said and done nothing, and that
nobody stopped, is declared empty (turno-vacio-es-error). The verdict has two
carriers, and every surface reads the one it already lives off:

| Carrier | CLI | TUI | GUI |
|---|---|---|---|
| `turn_empty` in the session log (every turn, every runner) | — (the CLI does not read the stream; see `detach` above) | a line in words, live and in the historical detail | a state of the turn in the conversation (`∅`, a word, the remedy) and a glyph in the operator log |
| `emptyTurn` in an answer that reports a turn's status | any answer carrying it exits `15` and warns on stderr, whichever verb got it | — (the shell starts no sessions of its own yet) | — (the desktop starts sessions detached, so its answers carry no status) |

Some answers carry no verdict by declaration, and the log is the source there:
a detached `session/start`, a queued or relayed `session/direct`,
`sdd/review-decide` (it discards the answer of the gate it triggers) and
`sdd/implement` (a task whose turn was empty is already reported `pending`).

The activity counts the verdicts too: `emptyTurns` in `analytics/usage` is shown
by `meltemi usage` when it is not zero and by the Usage view as one more column.
The TUI's `usage` home stays reserved, as it is for the rest of the activity.

This is **not** a §4 break: the capability is the verdict, and it reaches all
three surfaces; each reads it from the carrier it already consumes.

### What an agent reports over ACP, and where each surface shows it

An agent may report over ACP the context it holds and its cumulative cost
(`usage_update`). The accounting reads it as the agent's figures, apart from the
tokens — one conversation at a time, once, summed per currency, never converted
(usage-desde-acp):

| Contract | CLI | TUI | GUI |
|---|---|---|---|
| `agentReport` in a cell (sessions, fullest context, cost per currency) | a line under the cell, labelled as reported by the agent | — (the `usage` home stays reserved) | «Context (max)» and «Cost (agent-reported)» columns |
| `agentReport` in the totals (sessions, cost per currency; no context) | a totals line | — | the cost in the totals row |
| reason `protocol_carries_context_not_consumption` | in the coverage line | — | in the absence marker's reason |
| disclosure key `agent-reported-context-and-cost` | printed with the numbers | — | in the disclosure beside the table |

The TUI's reserved entry keeps the method's home in the palette, as it does for
the rest of the accounting; this is **not** a §4 break for the same reason the
activity is not.

### A lever's route, what governed, and a refusal at open

A model or an effort reaches the agent as the launch flag its entry declares or
through the option the agent announces, confirmed before the first prompt
(modelo-por-opcion-anunciada). What that adds to the contract, and where each
surface reads it:

| Contract | CLI | TUI | GUI |
|---|---|---|---|
| `levers_applied` event (what the agent confirmed) | — (the result carries it) | transcript line, live and historical | conversation line; the model chip reads it while the session runs |
| `lever_refused` event (lever, requested, reason, announced or left) | — (the error carries it) | transcript line in words, ES/EN | its own conversation item in words, ES/EN |
| `session_ended` reason `lever_refused` | — | transcript | translated ending |
| `session/start` result `model`, `effort` (what governed) | printed with the outcome | — (the session list carries it) | — (the session list carries it) |
| error 2007 `data.leverRefused` | printed bounded and escaped, exit `11` | a refused `dispatch` from the board says it | — (the conversation says it) |
| `worktree/dispatch` params `model`, `effort` | not sent (no flags) | not sent | not offered |

The same record travels in the log and in the error, so every surface can say it
in its own words, and each bounds the lists and escapes the agent's text itself.

### `requestId`, and why only the CLI reads it from the push

The live push carries the id its request holds in the pending queue
(aprobador-en-el-cli D4). The CLI reads it because it answers the push itself:
with the id it confirms the request still stands before asking, remembers a rule
by `permission/decide`, and withdraws the question once the queue lets the
request go. The TUI and the GUI decide from the queue — `permission/pending` and
`permission/changed` — where every entry already carries that id, so they have no
need to read it from the push.

This is **not** a §4 break: deciding and remembering by id is reachable from all
three surfaces, and the field only lets the one surface that answers pushes do
it from the push.

## Infrastructure

| Method | Role |
|---|---|
| `initialize` | Mandatory version-negotiation handshake; every surface performs it on connect (CLI/TUI `connect_and_init`, GUI bridge). Not a user-invocable capability. |

## Daemon-initiated traffic

Not client-invocable. `permission/changed` reaches every connected client;
`permission/request` and `permission/timeout` go to the connection that started
the session; `session/event` goes to that connection and to those watching the
session.

| Method | Kind | CLI | TUI | GUI |
|---|---|---|---|---|
| `session/event` | notification | the approver keeps tool calls, to show what a question is about | transcript append | `daemon:incoming` → transcript |
| `permission/request` | request (held) | asked on the terminal; without one, refused with the agent's rejection | notice; tray decides | `daemon:incoming`; tray decides |
| `permission/timeout` | notification | withdraws the open question, checked against the queue | persistent notice | persistent notice |
| `permission/changed` | notification | withdraws the open question once the queue no longer lists it | tray counter | tray counter |
