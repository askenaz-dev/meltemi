# Design — usage-desde-acp

## Context

Leído el código y el crate ACP pineado antes de diseñar, estos hechos fijan el
diseño:

- **`usage_update` es contrato, no extensión.** `agent-client-protocol-schema`
  1.4.0 (`src/v1/client.rs:296-332, 357-376`) define `UsageUpdate { used, size,
  cost }`: `used` son los **tokens que hay en el contexto ahora**, `size` el
  tamaño de la ventana, y `cost { amount, currency }` el **costo acumulado de la
  sesión ACP** (opcional, `amount` decimal, `currency` ISO 4217 sin validar, y un
  `cost` mal formado se lee como ausente). No trae consumo: ni tokens de entrada
  ni de salida.
- **El log ya lo guarda, sin decir de qué conversación.** `acp::forward_update`
  escribe cada `session/update` verbatim como `agent_update { update }`, sin el
  id de sesión ACP; `run_session` escribe un `agent_process` cada vez que lanza
  un agente (`acp.rs:243-249`). `carries_work` ya excluye `usage_update` del
  veredicto de turno vacío.
- **Una sesión de Meltemi no es una conversación del agente.** `implement`
  escribe un solo log y lanza un agente —con una sesión ACP nueva— por tarea
  (`server.rs:1817, 1859-1948`); reanudar una sesión (`session/direct`) crea
  otra sesión de Meltemi con su log, que hace `session/load` de la **misma**
  conversación ACP y registra `resumed_from` en su registro de índice
  (`server.rs:2439-2447, 2482, 2535, 2559`).
- **El plegado no lo lee.** `analytics::facts_of_log` solo conoce
  `usage_reported` (los contadores de nivel 3); toda sesión de nivel 1/2 cae en
  `protocol_carries_no_usage`, aunque su log diga lo contrario.
- **Un defecto al lado**: `facts_of_log` casa `AgentResolved { model: None,
  effort: None, .. }`, así que una sesión con modelo declarado pierde su binario
  y su nivel en la contabilidad (caen al índice). Se vio al cerrar
  `modelo-por-opcion-anunciada`, que hace más frecuentes esas sesiones.
- **Superficies**: la CLI (`meltemi usage`) y la vista Uso de la GUI muestran la
  contabilidad; la TUI tiene la entrada `usage` **reservada** en su paleta, sin
  vista. Los pines de la vista Uso viven en
  `core/meltemid/tests/scenarios_analytics.rs`.
- **Textos que dejan de ser ciertos**: la GUI dice del motivo
  `protocol_carries_no_usage` «el protocolo ACP no transporta uso»
  (`messages.ts:672-673, 1370-1371`), y la spec viva `session-history` dice
  «ACP, cuyo protocolo no transporta uso» (`session-history/spec.md:125-126`).
- **El simulado no emite `usage_update`.**

## Goals / Non-Goals

**Goals**: que lo que el agente reporta por ACP —contexto y costo— se lea, se
agregue y se muestre como lo que es: cifras **del agente**, relayadas, nunca
consumo y nunca un precio de Meltemi; contar el costo acumulado de cada
conversación del agente **una sola vez**; que la contabilidad deje de decir «el
agente no reportó uso» de una sesión que lo reportó; y corregir el patrón que
pierde binario y nivel.

**Non-Goals**: tokens de los adaptadores propios (`extension-acp-usage`);
convertir monedas; cuota, saldo o facturación (`no-quota-balance-or-billing`);
una vista de uso en la TUI (su entrada sigue reservada); mostrar lo reportado en
la barra de estado.

## Decisions

### D1 — Contexto y costo son otra dimensión que el consumo

`used` es ocupación, no gasto: sumarlo o ponerlo en la columna de tokens
convertiría «cuánto cabe ahora» en «cuánto se consumió», que es exactamente una
estimación. Por eso los contadores de tokens de una sesión ACP **siguen
ausentes**, y lo reportado viaja aparte, en un bloque `agentReport` de la celda
y de los totales (no `usageReported`, que ya nombra al evento de contadores
medidos de nivel 3).

### D2 — La unidad es la conversación del agente

El log de una sesión se parte en conversaciones en cada `agent_process`: un
proceso del agente es una sesión ACP. Por conversación:

- **el costo es el último que el agente reportó** —es acumulado—; un reporte
  posterior sin costo no lo borra, porque lo último que el agente dijo que costó
  sigue siendo eso;
- **el contexto es el reporte más lleno**: la mayor proporción `used/size`,
  comparada en enteros sin división (productos cruzados en `u128`), sin contar
  reportes con `size` 0, y ante empate el de más `used`. Es un máximo, no un
  estado final, y se rotula así («máx.»).

Una sesión de Meltemi suma por moneda el costo de sus conversaciones (una tarea
de `implement` es una conversación) y toma el contexto más lleno entre ellas.

### D3 — Una conversación reanudada se cuenta una vez

Una sesión que otra sesión de la consulta reanudó (su id figura como
`resumed_from` de un registro del rango) **no aporta costo**: su gasto está
dentro del costo acumulado de la conversación que la reanudó. Su contexto sí
cuenta (es un máximo; repetirlo no altera nada). Dos límites, dichos:

- el costo acumulado de una sesión que reanudó una conversación empezada antes
  del rango **incluye** ese gasto anterior;
- un registro reconstruido desde los logs (índice perdido) no conserva
  `resumed_from`, y entonces las dos sesiones cuentan su costo.

### D4 — Celda y totales

- Celda: `agentReport { sessions, contextUsed, contextSize, cost }` —cuántas
  sesiones reportaron, el reporte de contexto más lleno (D2), y `cost`: una
  entrada por moneda `{ currency, amount, sessions }`, ordenadas por moneda,
  cada una con cuántas sesiones la aportaron—. Ausente si ninguna sesión de la
  celda reportó.
- Totales: `agentReport { sessions, cost }` como **suma** de las celdas
  devueltas, igual que el resto de los totales; el contexto no está, porque un
  máximo no es una suma.
- Monedas distintas **nunca** se suman ni se convierten; la moneda se relaya
  como llegó. Nada ausente se rellena con cero.

### D5 — El motivo deja de contradecir al log (MODIFIED)

Una sesión cuyo agente reportó por el protocolo declara sus tokens de consumo
ausentes con el motivo nuevo `protocol_carries_context_not_consumption`; el
motivo `protocol_carries_no_usage` queda —con su clave estable— para las
sesiones ACP cuyo agente no reportó nada, y su texto pasa a decir eso: «el
agente no reportó uso por el protocolo». Como el requisito vivo «Frontera
explícita entre medido y no reportado» enumera los motivos, cambia, y lo mismo
la frase de `session-history` que da por hecho que ACP no transporta uso:
**la change deja la vía rápida** que anunciaba su propuesta y va por la completa.

### D6 — Cifras del agente, dichas como tales

La declaración de honestidad gana la clave `agent-reported-context-and-cost`:
el contexto y el costo son los que cada agente reportó por el protocolo;
Meltemi solo suma el costo por moneda —no estima, no convierte, no pone
precio—, un costo puede ser una estimación **del agente**, y un costo acumulado
puede incluir gasto anterior al rango (D3). Las superficies lo rotulan
«reportado por el agente».

### D7 — Superficies

- **CLI**: bajo cada celda con reporte, una línea —contexto máx. `used/size`,
  costo por moneda y de cuántas sesiones—; bajo los totales, el costo por
  moneda; la línea de la declaración nueva. El `--json` sigue siendo un objeto,
  con `agentReport`.
- **GUI**: la vista Uso gana las columnas «Contexto máx.» y «Costo (del
  agente)», con la marca de ausente (símbolo y palabra, nunca cero); el costo se
  formatea por el locale y, si el navegador no conoce la moneda, como cifra y
  código. El texto ES/EN del motivo nuevo, el del motivo reescrito, y la línea de
  la paleta («solo lo medido o lo que el agente reporta»).
- **TUI**: sin vista de uso (entrada reservada); cambia solo la descripción de
  la entrada, que hoy dice «solo lo medido».

### D8 — El patrón que perdía binario y nivel

`facts_of_log` casa `AgentResolved { binary, level, .. }` sin filtrar por modelo
ni esfuerzo. Una prueba lo pinea con un log cuyo `agent_resolved` lleva modelo.

### D9 — El simulado reporta uso

Interruptores apagados por omisión: `--usage` envía un `usage_update` al cerrar
cada turno con el contexto creciendo y el costo acumulado del proceso en
fracciones binarias exactas (`0.125` por turno), para que las sumas en `f64` se
comparen exactas; `--usage-no-cost` lo envía sin costo; `--usage-currency
<código>` cambia la moneda (por omisión `USD`). Lo que el simulado no puede
mezclar en una sola sesión —un reporte sin costo tras uno con costo, una
reanudación que continúe el acumulado— se pinea con logs e índices fixture en
pruebas unitarias del plegado.

## Risks / Trade-offs

- **R1 — Un agente que viole la semántica acumulada** (que mande el costo del
  turno y no el de la sesión) quedaría subreportado al tomar el último; uno que
  reinicie el acumulado al recargar una sesión quedaría subreportado por D3. Se
  sigue el contrato del protocolo; sumar reportes sería el error contrario, y
  peor.
- **R2 — El costo del agente puede ser su estimación** con sus tablas de
  precios: se dice en la declaración (D6) y en el rótulo; Meltemi no lo corrige.
- **R3 — `f64` para montos**: son los del protocolo; la suma entre sesiones puede
  arrastrar error de coma flotante en el último decimal. Las superficies
  redondean al mostrar.

## Migration

Aditivo en los campos (`agentReport` en celdas y totales). La celda y los
totales dejan de derivar `Eq` porque llevan montos decimales. El motivo y la
clave de declaración nuevos son variantes de enums cerrados: un cliente de una
versión anterior no los lee, así que el aditivo vale para clientes de la misma
versión que el daemon —como viajan en la misma release—. Una sesión ACP que no
reportó nada se lee como hoy.

## Revisión adversarial (2026-09-25)

Un revisor contra el código (corrección, honestidad, testabilidad), y el plan se
rehízo:

- **D2 y D3 nacen de los dos bloqueantes**: el plan contaba «el último reporte
  de la sesión», que en `implement` se queda con la última tarea y en una
  reanudación suma dos veces el acumulado de la misma conversación. La unidad
  pasa a ser la conversación del agente, y una sesión reanudada por otra no
  aporta costo.
- **D4**: el costo lleva sus sesiones por moneda; el contexto es un máximo
  rotulado y sale de los totales, que siguen siendo sumas de las celdas.
- **D5 y D6**: los textos que dejaban de ser ciertos (la GUI, la frase de
  `session-history`) cambian con la change, y la declaración dice lo que Meltemi
  sí hace —sumar por moneda— en vez de negar todo cálculo.
- **La vía**: dos MODIFIED sacan a la change de la vía rápida; la propuesta se
  reescribió para no contradecir al design.
- **Tareas**: la 1.1 deja compilando el workspace; los pines de la vista Uso van
  en `scenarios_analytics.rs`; lo que el simulado no puede mezclar en una sesión
  se pinea con logs fixture; el bloque se llama `agentReport` para no chocar con
  el evento `usage_reported`.
