# volver-a-este-turno

> Vía completa (proposal → design → specs → tasks): extiende los checkpoints
> del daemon al grano del turno, con el guardián de reversión que
> `lanzador-conversacional` dejó escrito y su paridad en CLI y TUI. Nace del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha D8). Depende de
> `herramientas-a-la-vista` para que el turno tenga borde visible.

## Why

Orca anuncia «session rewind controls». Meltemi tiene checkpoints por tarea
(`checkpoints`: punto de restauración automático antes de cada tarea,
reversión granular con alcance honesto de lo que no se deshace) y un punto
de restauración al abrir una sesión libre. Lo que no tiene es el grano que
un humano usa al conversar: «vuelve a como estaba antes del turno 3». Hoy
la vuelta es manual con `git`.

El guardián ya decide la frontera: `checkpoint/revert` rehúsa todo árbol que
no sea un worktree gestionado (`lanzador-conversacional`, design D2), porque
un `reset --hard` sobre el árbol del usuario sería armar un verbo contra su
trabajo sin commitear. Volver a un turno vale, por tanto, en sesiones que
corren en worktree; sobre el árbol del usuario se ofrece el comando de
`git` como texto, nunca la acción.

Y una honestidad que la propuesta escribe para que el design no la pierda:
**el árbol vuelve; la memoria del agente no**. ACP no rebobina el contexto
de un agente; la siguiente instrucción tiene que decirle que el árbol
cambió.

## What Changes

- **Un punto de restauración por turno** en sesiones sobre worktree
  gestionado (`refs/meltemi/checkpoints/turn/<sesión>/<n>`), ligero, creado
  al empezar cada turno; listado en `checkpoints` con el turno al que
  pertenece.
- **«Volver a este turno»** en el transcript (GUI y TUI) y como verbo
  (`meltemi revert-turn <sesión> <n> [confirm]`): sin `confirm` previsualiza
  el alcance —archivos que cambian y las acciones irreversibles registradas
  desde entonces, que no se deshacen—; con `confirm` revierte el worktree y
  encola una nota al agente en el siguiente turno («el árbol volvió al
  estado del turno n; tu contexto no»), registrada en el log.
- **Sobre el árbol del usuario**: el botón explica por qué no, y ofrece el
  `git restore --source` equivalente para copiar.
- **Retención**: los puntos por turno se podan al terminar la sesión salvo
  el último, con la política escrita en `checkpoints`.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `checkpoints`: + requisito «Punto de restauración por turno en worktree
  gestionado, con previsualización, alcance honesto y nota al agente».
- `cli-contract`: + `revert-turn`.
- `gui-shell` y `tui-shell`: + la acción en el transcript.

## Impact

- Archivos: `core/meltemid/src/{checkpoints.rs, acp.rs, server.rs}`,
  `proto/schemas/v1/checkpoint.schema.json` (aditivo), `tui/src/`,
  `desktop/ui/src/lib/views/SessionDetail.svelte`, `docs/`.
- Cero dependencias nuevas.
- Riesgo real: el costo de un ref por turno en sesiones largas y la
  interacción con el commit por tarea de `git-per-task`; el design lo mide
  y fija la poda.

## Fuera de alcance

- Rebobinar el contexto del agente: no existe en ACP; se dice, no se finge.
- Revertir sobre el árbol del usuario: el guardián manda.
