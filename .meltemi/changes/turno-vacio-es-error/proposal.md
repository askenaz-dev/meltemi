# turno-vacio-es-error

> Vía rápida (fast-forward): deltas solo ADDED sobre capacidades existentes,
> un campo aditivo en el contrato y su render en las tres superficies. Nace de
> la prueba integrada del 2026-09-24
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
el turno **llevó algo**. En la GUI el resultado es la frase «sin salida»
(`desktop/ui/src/lib/views/SessionDetail.svelte:900`, `conv.noOutput`) sin
explicación; en la TUI, un turno cerrado; en un script, código 0. Un turno
que termina sin una palabra ni una acción es, en la práctica, un fallo del
proveedor que el agente no supo reportar — y un plano de control que lo
llama éxito enseña a desconfiar de todos sus éxitos.

## What Changes

- **El daemon cuenta lo que el turno llevó**: al cerrar cada turno,
  `TurnCompleted` gana un campo aditivo `carried` con los conteos de mensajes,
  pensamientos, llamadas a herramienta y peticiones de permiso, y un booleano
  `empty` cuando todos son cero. El motivo de parada del agente se conserva
  intacto: Meltemi no interpreta al agente, lo mide.
- **Las respuestas de los verbos que arrancan sesión lo dicen**:
  `session/start`, `sdd/propose`, `sdd/explore` y `worktree/dispatch` devuelven
  `emptyTurn: true` (aditivo) junto al estado. El CLI imprime una advertencia
  con remedio («el agente terminó sin decir ni hacer nada: prueba el agente
  por su cuenta y revisa su proveedor y sus credenciales») y **no** sale con
  0: el design fija el código de salida (candidato: uno nuevo en la tabla de
  `cli-contract`, porque `warning` con 0 es lo que un script no lee).
- **GUI y TUI lo pintan como estado, no como ausencia**: la burbuja del turno
  dice «turno vacío» con el remedio, con glifo y palabra (nunca solo color),
  en vez de «sin salida». La barra lateral no cambia: la sesión sigue
  «detenida».
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
  (conteos aditivos y `empty`; el motivo del agente se conserva).
- `cli-contract`: + advertencia con remedio y código de salida propio en los
  verbos que arrancan sesión.
- `gui-shell` y `tui-shell`: + el estado «turno vacío» en el transcript, con
  glifo y palabra.
- `local-analytics`: + `emptyTurns` en la actividad.

## Impact

- Archivos: `core/meltemid/src/{session_finalize.rs, acp.rs, analytics.rs}`,
  `proto/schemas/v1/{session-event, session-start, propose, worktree,
  analytics}.schema.json` (campos aditivos), `tui/src/run.rs`,
  `tui/src/shell/`, `desktop/ui/src/lib/{conversation.ts, views/SessionDetail.svelte}`,
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
