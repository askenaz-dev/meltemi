# Design — usage-desde-acp

## Context

Leído el código y el crate ACP pineado antes de diseñar, estos hechos fijan el
diseño:

- **`usage_update` es contrato, no extensión.** `agent-client-protocol-schema`
  1.4.0 (`src/v1/client.rs:302-332, 357-376`) define `UsageUpdate { used, size,
  cost }`: `used` son los **tokens que hay en el contexto ahora**, `size` el
  tamaño de la ventana, y `cost { amount, currency }` el **costo acumulado de la
  sesión** (opcional, `amount` decimal, `currency` ISO 4217, y un `cost` mal
  formado se lee como ausente). No trae consumo: ni tokens de entrada ni de
  salida.
- **El log ya lo guarda.** `acp::forward_update` escribe cada `session/update`
  verbatim como `agent_update { update }`; la sesión de OpenCode del 2026-09-24
  dejó `{"sessionUpdate":"usage_update","used":12267,"size":200000,"cost":…}`.
  `carries_work` ya lo excluye del veredicto de turno vacío.
- **El plegado no lo lee.** `analytics::facts_of_log` solo conoce
  `usage_reported` (los contadores de nivel 3); toda sesión de nivel 1/2 cae en
  `unreported_kind` → `protocol_carries_no_usage`, aunque su log diga lo
  contrario.
- **Un defecto al lado**: `facts_of_log` casa `AgentResolved { model: None,
  effort: None, .. }`, así que una sesión con modelo declarado pierde su binario
  y su nivel en la contabilidad (caen al índice). Se vio al cerrar
  `modelo-por-opcion-anunciada`, que hace más frecuentes esas sesiones.
- **Superficies**: la CLI (`meltemi usage`) y la vista Uso de la GUI muestran la
  contabilidad; la TUI tiene la entrada `usage` **reservada** en su paleta, sin
  vista.
- **El simulado no emite `usage_update`.**

## Goals / Non-Goals

**Goals**: que lo que el agente reporta por ACP —contexto y costo— se lea, se
agregue y se muestre como lo que es: cifras **del agente**, relayadas, nunca
consumo y nunca un precio de Meltemi; que la contabilidad deje de decir «el
protocolo no transporta uso» de una sesión que lo transportó; y corregir el
patrón que pierde binario y nivel.

**Non-Goals**: tokens de los adaptadores propios (`extension-acp-usage`);
convertir monedas; cuota, saldo o facturación (`no-quota-balance-or-billing`);
una vista de uso en la TUI (su entrada sigue reservada).

## Decisions

### D1 — Contexto y costo son otra dimensión que el consumo

`used` es ocupación, no gasto: sumarlo o ponerlo en la columna de tokens
convertiría «cuánto cabe ahora» en «cuánto se consumió», que es exactamente una
estimación. Por eso los contadores de tokens de una sesión ACP **siguen
ausentes**, y lo reportado viaja aparte, en un bloque `reported` de la celda y
de los totales.

### D2 — Qué se toma de cada sesión, y cómo se agrega

- **Por sesión, el último reporte**: `used` y `size` son un estado; el costo es
  acumulado. Sumar reportes de una misma sesión contaría dos veces. Un reporte
  sin costo no borra un costo anterior de la misma sesión: el último costo
  reportado sigue siendo lo último que el agente dijo que costó.
- **Por celda y en los totales**:
  - `sessions`: cuántas sesiones reportaron por el protocolo;
  - `contextUsed`/`contextSize`: el reporte más lleno entre ellas —la sesión que
    más cerca estuvo de su ventana—, porque sumar contextos no significa nada;
  - `cost`: por moneda, la suma de los últimos costos de cada sesión, y
    `costSessions`: cuántas lo aportaron. Monedas distintas **nunca** se suman
    ni se convierten.
- Todo campo ausente se omite; nada se rellena con cero.

### D3 — El motivo deja de contradecir al log (MODIFIED)

Una sesión cuyo agente reportó por el protocolo declara sus tokens de consumo
ausentes con un motivo nuevo y estable, `protocol_carries_context_not_consumption`
—el protocolo trajo contexto y costo, no consumo—, y ya no
`protocol_carries_no_usage`, que queda para las sesiones ACP que no reportaron
nada. Como el requisito vivo «Frontera explícita entre medido y no reportado»
enumera los motivos, cambia (MODIFIED): **la change deja la vía rápida** que
anunciaba su propuesta y va por la completa.

### D4 — Cifras del agente, dichas como tales

La declaración de honestidad gana la clave
`context-and-cost-as-the-agent-reports`: el contexto y el costo son los que el
agente reportó, relayados sin calcular ni convertir; un costo puede ser una
estimación **del agente**, y Meltemi no pone precio a nada. Las superficies lo
rotulan como «reportado por el agente», no como gasto medido.

### D5 — Superficies

- **CLI**: bajo cada celda con reporte, una línea: contexto `used/size`, costo
  por moneda y de cuántas sesiones; el `--json` sigue siendo un objeto, con el
  bloque `reported`.
- **GUI**: la vista Uso gana las columnas «Contexto» y «Costo (del agente)» en
  filas y totales, con la marca de ausente (símbolo y palabra, nunca cero) y la
  moneda formateada por el locale; el costo en moneda desconocida para el
  navegador se muestra como cifra y código.
- **TUI**: sin vista de uso (entrada reservada); nada cambia.

### D6 — El patrón que perdía binario y nivel

`facts_of_log` casa `AgentResolved { binary, level, .. }` sin filtrar por modelo
ni esfuerzo. Una prueba lo pinea con un log cuyo `agent_resolved` lleva modelo.

### D7 — El simulado reporta uso

Interruptores apagados por omisión: `--usage` envía un `usage_update` al cerrar
cada turno con el contexto creciendo y el costo acumulado en `USD`;
`--usage-no-cost` lo envía sin costo. Las pruebas de extremo a extremo leen la
contabilidad de sesiones reales del simulado.

## Risks / Trade-offs

- **R1 — Un agente que viole la semántica acumulada** (que mande el costo del
  turno y no el de la sesión) quedaría subreportado al tomar el último. Se sigue
  el contrato del protocolo; sumar reportes sería el error contrario, y peor.
- **R2 — El costo del agente puede ser su estimación** con sus tablas de
  precios: se dice en la declaración (D4) y en el rótulo; Meltemi no lo corrige.
- **R3 — `f64` para montos**: los montos son los del protocolo; la suma entre
  sesiones puede arrastrar error de coma flotante en el último decimal. Las
  superficies redondean al mostrar.

## Migration

Aditivo en el contrato (`reported` en celdas y totales, un motivo y una clave
nuevos); los tipos de la celda y los totales dejan de derivar `Eq` porque llevan
montos decimales. Una sesión ACP que no reportó nada se lee como hoy.
