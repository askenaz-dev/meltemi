# usage-desde-acp

> Vía rápida (fast-forward): deltas solo ADDED sobre `local-analytics` y las
> superficies que la muestran; campos aditivos en `analytics/usage`. Nace de
> la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha LM-7).

## Why

La sesión de OpenCode del 2026-09-24 dejó en su log apend-only un
`agent_update` con `sessionUpdate: "usage_update"`, `used: 12267`,
`size: 200000` y un costo en USD. Un minuto después, `meltemi usage --all`
dijo de esa misma sesión: `unreportedSessions: 1`, motivo
`protocol_carries_no_usage`. Las dos cosas están en el mismo directorio de
datos y se contradicen.

`local-analytics` promete «tokens solo donde la salida oficial del agente los
reporta, jamás estimados» — y un `usage_update` que el agente envía por ACP
**es** su salida oficial. `core/meltemid/src/analytics.rs` no conoce ese
`sessionUpdate` (ningún `usage` en el archivo), así que lo pliega como
«no reportado». El motivo era cierto el día que se escribió y dejó de serlo
cuando los agentes empezaron a enviarlo. Orca muestra medidores de uso por
proveedor en la pantalla principal y en el teléfono; Meltemi los tiene en el
disco y dice que no los tiene.

## What Changes

- **El plegado lee `usage_update`**: por sesión, el último valor de contexto
  usado y de tamaño de contexto, y la suma de costo por moneda cuando el
  agente la envía. La celda de `analytics/usage` gana campos aditivos
  (`contextUsed`, `contextSize`, `cost[{amount, currency}]`) y la cobertura
  pasa a `measured` para esas sesiones, con un motivo nuevo y estable para las
  que solo reportan contexto sin costo. Nada se estima: si el agente no envía,
  la frontera sigue diciendo «no reportado» con su motivo.
- **Tres superficies, el mismo número**: la vista Uso de la GUI, `usage` en la
  TUI y `meltemi usage --json` muestran contexto y costo junto a la actividad
  que ya muestran, con la declaración de honestidad que ya llevan
  (`disclosure`) ampliada con «lo que el protocolo transporta».
- **La forma del evento se verifica contra el crate ACP pineado**: si
  `usage_update` es parte del contrato de la versión pineada, se lee tipado;
  si llega como extensión, se lee de forma laxa y el design lo deja escrito.
  En ambos casos el valor se registra como llegó (ya ocurre: el log guarda el
  `update` verbatim).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `local-analytics`: + requisito «Uso que el protocolo transporta» (contexto y
  costo por sesión desde `usage_update`; motivo de cobertura para contexto sin
  costo; nada estimado).
- `gui-shell`, `tui-shell`, `cli-contract`: + los campos nuevos en las tres
  superficies.

## Impact

- Archivos: `core/meltemid/src/analytics.rs`,
  `proto/schemas/v1/analytics.schema.json` (aditivo),
  `desktop/ui/src/lib/views/Usage.svelte`, `tui/src/shell/`, `tui/src/run.rs`,
  `docs/paridad-nucleo.md`, la página de uso en docs.
- Cero dependencias nuevas.
- Riesgo: agentes que envían `used` acumulado frente a agentes que envían el
  delta del turno; el design lo fija con el agente simulado emitiendo ambas
  formas y con la corrida real de OpenCode.

## Fuera de alcance

- Tokens de los adaptadores propios (Claude Code, Codex), que no llegan por
  ACP: `extension-acp-usage`.
- Límites de cuota, saldos o facturación: constitución §2 y `local-analytics`
  (`no-quota-balance-or-billing`).
