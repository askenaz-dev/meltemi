## Why

La autonomía solo es aceptable si deshacer es barato (§4.6, §6.7): antes de cada
tarea debe existir un **checkpoint automático**, y la reversión debe ser granular
y — sobre todo — **honesta sobre su alcance**: qué revierte (worktree) y qué no
(efectos externos: comandos ejecutados, red, estado fuera del árbol). Sin esto,
`/implement` (#20) sería un salto sin cuerda.

## What Changes

- **Checkpoint automático pre-tarea**: instantánea del worktree (mecanismo git:
  commit/stash técnico fuera de la rama del usuario) antes de cada tarea que un
  agente ejecute; identificado y listable.
- **Reversión granular**: volver al checkpoint de una tarea concreta sin arrasar
  el trabajo de otras (por worktree; interacción con tareas dependientes
  declarada).
- **Alcance honesto**: la UX de reversión declara explícitamente qué NO se
  revierte, **enganchado a la clasificación del proxy de permisos** (#9): las
  operaciones aprobadas fuera del árbol (comandos, red) se listan como
  irreversibles junto al checkpoint.
- **Contrato**: listar checkpoints y revertir (aditivos); eventos en el log de
  sesión.

## Capabilities

### New Capabilities
- `checkpoints`: creación automática, listado, reversión y alcance honesto.

### Modified Capabilities
- `worktree-orchestration`: el ciclo de tarea integra checkpoint como paso.
- `permission-rules`: la clasificación alimenta el registro de irreversibles.

## Impact

- `core/meltemid` (mecanismo git + registro), `proto/`, `tui/` (listar/revertir
  con confirmación modal ya existente en el shell).

## Fuera de alcance

- Sandboxing propio (fase 2, §8.3): aquí el aislamiento sigue siendo el heredado.
- Snapshot de estado externo (bases de datos, servicios): jamás se promete.
