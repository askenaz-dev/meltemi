# turno-vacio-es-error

> Vía completa. Se redactó como vía rápida; el design la saca de ella porque
> el código de salida nuevo modifica la taxonomía viva de `cli-contract`
> (design D4), y un MODIFIED no es elegible para la vía rápida. Nace de la
> prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha AI-11).

## Why

El 2026-09-24, una sesión libre con OpenCode 1.14.33 terminó así por el CLI:
`{"status":"completed","deniedPermissions":0,...}`. Éxito, según Meltemi. El
log apend-only de esa sesión tenía diez eventos y **ni un solo
`agent_message_chunk`, ni un `tool_call`, ni un pensamiento**: solo
`available_commands_update` y un `usage_update` con cero tokens. El agente no
había dicho ni hecho nada — sus tres credenciales estaban rotas (404, 401, y
un tier gratuito que exige otra versión) y su ACP cerró el turno con
`end_turn` igual. Corregido el agente, la misma sesión tardó 20 s, leyó dos
archivos y contestó.

Meltemi no mintió: transcribió fielmente lo que el agente dijo. Pero el
protocolo solo entrega el motivo de parada del agente, y
`core/meltemid/src/session_finalize.rs:65-73` lo convierte tal cual en
`TurnCompleted { stop_reason }` + `SessionEnded("completed")`. Nadie mira si
el turno **llevó algo**. En la GUI el resultado son líneas de sistema neutras y
`■ turn_completed {"stopReason":"completed"}`: el pliegue no abre turno del
agente sin una parte, así que ni siquiera se pinta «sin salida» (design,
contexto); en la TUI, `[id] turn_completed`; en un script, código 0. Un turno
que termina sin una palabra ni una acción es, en la práctica, un fallo del
proveedor que el agente no supo reportar — y un plano de control que lo
llama éxito enseña a desconfiar de todos sus éxitos.

## What Changes

- **El daemon mide cada turno y declara el vacío**: cuenta lo que el agente
  envía durante el turno y, si un turno `completed` no llevó ni una palabra ni
  una acción, escribe en el log un evento propio `turn_empty` — por turno, no
  un campo de `TurnCompleted`, que se escribe una vez por sesión y que dos
  runners no escriben (design D1, D2). El motivo de parada del agente se
  conserva intacto: Meltemi no interpreta al agente, lo mide.
- **Las respuestas que informan un turno lo dicen**: `session/start` sin
  `detach`, `propose`, `session/direct` (reanudación), `worktree/dispatch` y
  `sdd/explore`, `sdd/propose`, `sdd/plan`, `sdd/constitution` y `sdd/gate`
  devuelven `emptyTurn: true` (aditivo). Cualquier verbo del CLI cuya respuesta
  lo traiga imprime en stderr una advertencia con remedio y termina con un
  código nuevo, `15`, porque un `0` con aviso es lo que un script no lee
  (design D4). Un turno que el humano detuvo nunca se declara vacío (design D2).
- **GUI y TUI lo pintan como estado, no como ausencia**: la conversación dice
  «turno vacío» con el remedio, con glifo y palabra (nunca solo color); la TUI,
  con palabras en su transcript en vivo y en el detalle histórico. La barra
  lateral no cambia, por decisión: el veredicto vive en la conversación
  (design D5).
- **La analítica local lo cuenta**: `analytics/usage` gana `emptyTurns` en la
  actividad de cada celda, aditivo, para que un agente que falla en silencio
  se vea en el resumen de la semana.
- **Los adaptadores propios no cambian**: ya propagan los rechazos del
  proveedor con sus palabras (Codex lo hizo el mismo día). Esta change cubre
  al agente que no los propaga.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «Un turno que no llevó nada se declara vacío»
  (evento `turn_empty` por turno; el motivo del agente se conserva).
- `acp-session` además: + «Las respuestas de sesión dicen si el turno fue
  vacío».
- `propose-flow`, `worktree-orchestration` y `sdd-authoring`: + un requisito
  cada una para `emptyTurn` en la respuesta que poseen.
- `cli-contract`: MODIFIED «Taxonomía de códigos de salida» (+ `15`) y ADDED
  «Un turno vacío se advierte con remedio».
- `gui-shell` y `tui-shell`: + el estado «turno vacío» en la conversación y
  en el transcript, con palabra (y glifo en la GUI).
- `local-analytics`: + «Los turnos vacíos se cuentan» (`emptyTurns`).

## Impact

- Archivos: `core/meltemid/src/{acp.rs, free_session.rs, propose.rs,
  sdd_flow.rs, server.rs, analytics.rs}`, `core/mock-agent/src/main.rs`,
  `proto/schemas/v1/{session-event, session-start, propose, session-direct,
  sdd, worktree, analytics}.schema.json` (campos aditivos), `tui/src/{exit.rs,
  lib.rs, run.rs, shell/}`, `desktop/ui/src/lib/{conversation.ts,
  views/SessionDetail.svelte, views/Usage.svelte, messages.ts}`,
  `docs/referencia-cli.md`, `docs/paridad-nucleo.md`.
- Cero dependencias nuevas. El contrato solo gana campos opcionales: un
  cliente previo sigue leyendo la respuesta.
- Riesgo: un agente que legítimamente no responde a un prompt vacío o a una
  instrucción que solo cambia opciones. El design lo enumera; `empty` describe
  el turno, y el remedio habla en condicional.

## Fuera de alcance

- Leer el `stderr` del agente para diagnosticar **por qué** no dijo nada: es
  otra change, con su análisis de qué se registra y qué no.
- Reintentar el turno automáticamente: nunca sin el humano.
