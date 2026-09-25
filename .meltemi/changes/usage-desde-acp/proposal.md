# usage-desde-acp

> Vía completa: un requisito ADDED sobre `local-analytics` y dos MODIFIED —la
> frontera entre medido y no reportado, que enumera los motivos, y el evento de
> uso de `session-history`, que daba por hecho que ACP no transporta uso—; campos
> aditivos en `analytics/usage`. Nace de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha LM-7). Se redactó como
> vía rápida; la revisión adversarial del plan (2026-09-25) mostró los MODIFIED y
> la unidad de conteo correcta.

## Why

La sesión de OpenCode del 2026-09-24 dejó en su log apend-only un
`agent_update` con `sessionUpdate: "usage_update"`, `used: 12267`,
`size: 200000` y un costo en USD. Un minuto después, `meltemi usage --all`
dijo de esa misma sesión: `unreportedSessions: 1`, motivo
`protocol_carries_no_usage`. Las dos cosas están en el mismo directorio de
datos y se contradicen.

`local-analytics` promete «tokens solo donde la salida oficial del agente los
reporta, jamás estimados», y un `usage_update` que el agente envía por ACP es
parte del contrato del protocolo pineado: el contexto que ocupa, el tamaño de su
ventana y el costo acumulado de la sesión. `core/meltemid/src/analytics.rs` no
lo conoce, así que lo pliega como «no reportado». El motivo era cierto el día que
se escribió y dejó de serlo cuando los agentes empezaron a enviarlo. Orca
muestra medidores de uso por proveedor en la pantalla principal y en el
teléfono; Meltemi los tiene en el disco y dice que no los tiene.

## What Changes

- **El plegado lee `usage_update`** como lo que es: **contexto y costo del
  agente, no consumo**. La unidad es la conversación del agente —cada proceso
  dentro del log de una sesión; `implement` abre una por tarea—: de cada una, el
  último costo acumulado que reportó y su reporte de contexto más lleno. Una
  sesión que otra de la consulta reanudó no aporta costo, porque el acumulado de
  la conversación ya lo contiene. Los contadores de tokens siguen ausentes.
- **`analytics/usage` gana un bloque aditivo `agentReport`** en la celda
  (sesiones que reportaron, contexto máximo, costo por moneda con sus sesiones) y
  en los totales (sesiones y costo por moneda, como suma de las celdas); monedas
  distintas nunca se suman ni se convierten.
- **El motivo deja de contradecir al log**: `protocol_carries_context_not_consumption`
  para las sesiones que reportaron por el protocolo; `protocol_carries_no_usage`
  queda para las que no reportaron nada, y su texto pasa a decir eso.
- **La declaración de honestidad** gana «el contexto y el costo son los que cada
  agente reportó; Meltemi solo suma el costo por moneda, no estima, no convierte,
  no pone precio».
- **Superficies**: la CLI (`meltemi usage`, humano y `--json`) y la vista Uso de
  la GUI muestran contexto máximo y costo rotulados «reportado por el agente». La
  TUI no tiene vista de uso (su entrada de paleta está reservada): cambia solo su
  descripción.
- **Un defecto al lado**: el plegado perdía el binario y el nivel de toda sesión
  con modelo declarado; se corrige.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `local-analytics`: + requisito «Uso que el agente reporta por el protocolo»;
  MODIFIED «Frontera explícita entre medido y no reportado» (un motivo nuevo y el
  texto del existente).
- `session-history`: MODIFIED «Evento local de uso en ejecuciones headless»
  (ACP no fabrica evento de uso, pero lo que el agente reporta queda en el log y
  la contabilidad lo lee).

## Impact

- Archivos: `core/meltemid/src/analytics.rs`, `core/mock-agent/src/main.rs`,
  `proto/meltemi-proto/src/lib.rs` y `proto/schemas/v1/analytics.schema.json`
  (aditivo), `tui/src/run.rs`, `tui/src/shell/palette.rs`,
  `desktop/ui/src/lib/views/Usage.svelte` y `messages.ts`,
  `docs/paridad-nucleo.md`, la referencia CLI.
- Cero dependencias nuevas.
- Riesgo: un agente que no respete la semántica acumulada del costo queda
  subreportado; el design lo declara y sigue el contrato del protocolo.

## Fuera de alcance

- Tokens de los adaptadores propios (Claude Code, Codex), que no llegan por
  ACP: `extension-acp-usage`.
- Límites de cuota, saldos o facturación: constitución §2 y `local-analytics`
  (`no-quota-balance-or-billing`).
- Una vista de uso en la TUI, y lo reportado en la barra de estado.
