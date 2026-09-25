# Design — turno-vacio-es-error

## Context

La prueba integrada del 2026-09-24 terminó una sesión libre de OpenCode 1.14.33
con `{"status":"completed","deniedPermissions":0}` y un log de diez eventos sin
una palabra ni una acción del agente: sus credenciales estaban rotas y su ACP
cerró el turno con `end_turn` igual. Meltemi lo llamó éxito.

Leído el código antes de diseñar, tres hechos corrigen la propuesta:

- **`TurnCompleted` se escribe una vez por sesión, no por turno.** Solo
  `session_finalize.rs:68-122` lo escribe, con el estado del último turno
  (medido en el smoke de `lanzador-conversacional`, H4). `worktree/dispatch`
  (`server.rs:958-998`) y el bucle de `sdd/implement` (`server.rs:1844-1921`)
  no lo escriben nunca, y los turnos intermedios de una sesión que espera
  (`acp.rs:477-528`) no tienen evento de cierre: el siguiente `PromptSent` es
  el único borde. Un campo en `TurnCompleted` no mediría «cada turno».
- **«Sin salida» no se pinta en el caso que la propuesta describe.** El
  pliegue de la GUI (`conversation.ts:160-321`) abre un turno del agente solo
  cuando llega una parte clasificable; un turno vacío no abre ninguno, así que
  `parts.length === 0` (`SessionDetail.svelte:901`) no se alcanza. Lo que se ve
  hoy son líneas de sistema neutras y `■ turn_completed {"stopReason":"completed"}`.
- **La GUI arranca sesiones separadas** (`detach: true`, `Home.svelte:284`):
  nunca recibe el estado en la respuesta, así que solo el evento puede decírselo.

Y un hecho a favor: el despacho de la conexión ACP procesa los mensajes uno a
uno y **espera a que cada manejador termine** antes del siguiente
(`agent-client-protocol` 1.2.0, `src/concepts/ordering.rs`). Cuando la
respuesta a `session/prompt` resuelve, todas las notificaciones que el agente
envió antes ya pasaron por `forward_update`. Contar allí no tiene carrera.

## Goals / Non-Goals

**Goals**: que todo turno que el agente cierra como terminado sin haber dicho
ni hecho nada quede declarado vacío en el log, sea cual sea el runner; que las
respuestas de los verbos que informan el estado de un turno lo digan; que el
CLI no salga con `0`; que la GUI y la TUI lo muestren como estado, con palabra
y remedio; y que la analítica lo cuente.

**Non-Goals**: diagnosticar por qué el agente no dijo nada (leer su `stderr`
es otra change), reintentar el turno, cambiar el motivo de parada que el agente
reportó, o tocar los adaptadores propios, que ya propagan los rechazos del
proveedor con sus palabras.

## Decisions

### D1 — Se mide dentro de cada turno, en el manejador de actualizaciones

`HandlerState` gana un contador del turno (`Arc<TurnTally>`, atómicos como
`denials`). `forward_update` suma lo que **lleva** trabajo del agente:

- `agent_message_chunk` y `agent_thought_chunk` cuyo texto no es solo espacios;
- `tool_call` y `tool_call_update`;
- `plan`.

`passthrough_permission` suma cada petición de permiso. No suman
`user_message_chunk` (el eco del prompt), `available_commands_update`,
`current_mode_update`, `config_option_update`, `session_info_update` ni
`usage_update`: describen la sesión, no el trabajo del turno. El contador se
pone a cero al escribir cada `PromptSent` y se lee en cuanto la respuesta al
prompt resuelve; D1 del contexto garantiza que entonces ya contó todo.

- **Descartado: contar después, leyendo el log.** Obligaría a releer el
  archivo por turno o a llevar un índice del último `PromptSent`; el manejador
  ya ve cada actualización en el orden correcto.

### D2 — El veredicto es un evento propio por turno: `turn_empty`

Cuando un turno termina con estado `completed` y su contador está en cero, el
daemon escribe `turn_empty { stopReason }` inmediatamente después de leer la
respuesta, antes de decidir el siguiente turno. Por eso lo tienen también los
turnos intermedios de una sesión que espera, `worktree/dispatch` y cada tarea
de `sdd/implement`: todos pasan por el mismo bucle de `run_session`.

- **Solo `completed`.** Un turno `cancelled`, `refused`, `max_tokens` o
  `max_turn_requests` ya dice que no terminó su trabajo; declararlo además
  vacío sería repetir. `stopReason` viaja igual, para que el evento se lea solo
  y para que ampliar el criterio sea aditivo.
- **El motivo del agente no cambia.** `TurnCompleted` y el `status` de las
  respuestas siguen diciendo lo que el agente dijo. Meltemi no reinterpreta al
  agente: lo mide y lo declara al lado.
- **Corrección a la propuesta**: no se persisten los conteos. Cada
  actualización ya está en el log; repetir su número sería una segunda fuente
  que puede discrepar. Se persiste el veredicto.
- **Descartado: un campo en `TurnCompleted`.** Se escribe una vez por sesión y
  dos runners no lo escriben (contexto).

### D3 — Las respuestas dicen si el turno que informan fue vacío

`SessionOutcome` gana `last_turn_empty: bool` y `empty_turns: u32`. Las
respuestas que informan el estado de un turno ganan `emptyTurn: true`,
aditivo y omitido cuando es falso:

| Método | Resultado | Qué dice `emptyTurn` |
| --- | --- | --- |
| `session/start` (esperando) | `SessionStartResult` | el turno cuyo `status` informa |
| `propose` | `ProposeResult` | ídem |
| `session/direct` (reanudación) | `SessionDirectResult` | ídem |
| `worktree/dispatch` | `WorktreeDispatchResult` | ídem |
| `sdd/explore`, `sdd/propose`, `sdd/plan`, `sdd/constitution` | `SddResult` | algún turno de la llamada (la vía rápida corre cuatro) |

Donde no hay estado —arranque separado, instrucción encolada o relevada— no
hay `emptyTurn`: el evento del log es la única fuente, y es la que leen la GUI y
la TUI.

### D4 — El CLI advierte en stderr y termina con `15`

La taxonomía de códigos de salida gana `15`: *el turno del agente terminó sin
decir ni hacer nada*. `session`, `propose`, `direct`, `dispatch` y `explore`
terminan con `15` cuando la respuesta trae `emptyTurn: true`, y escriben en
stderr, en modo humano:

```
warning: the agent ended its turn without saying or doing anything — if that
was not expected, try the agent on its own and check its provider and credentials
```

En `--json` la respuesta viaja íntegra por stdout, con `emptyTurn`, y el código
es el mismo. El precedente existe: `validate` termina con `14` sobre un
resultado exitoso con hallazgos (`tui/src/lib.rs:91-111`).

- **Por qué un código propio y no `0` con aviso.** Un script lee el código, no
  stderr; un `0` es exactamente el éxito que esta change deja de afirmar.
- **Por qué no `1` ni `11`.** Ni el binario falló ni el daemon rompió el
  contrato: el turno ocurrió y no llevó nada. Un código propio deja que un
  script lo distinga y reintente, o no.
- **Enmienda**: la taxonomía es un requisito vivo de `cli-contract` que
  enumera los códigos; añadir uno es un MODIFIED. Por eso esta change deja la
  vía rápida y sigue la completa.

### D5 — La GUI lo pinta como estado del turno

El pliegue de la conversación convierte `turn_empty` en un aviso propio
(`kind: "empty"`), que la vista pinta con glifo `◌`, la palabra «turno vacío» y
el remedio en condicional: «el agente terminó sin decir ni hacer nada; si no
lo esperabas, pruébalo por su cuenta y revisa su proveedor y sus
credenciales». Nunca solo color. Las claves van en español e inglés. La barra
lateral no cambia: la sesión sigue «detenida».

### D6 — La TUI lo dice en el transcript

`summarize_event` (`tui/src/shell/conn.rs:789-806`) convierte `turn_empty` en
`[<sesión>] turno vacío — <remedio>`, con el texto del catálogo `Msg` en el
idioma de la sesión. Sin glifo: esa función no recibe la presentación, y la
palabra ya cumple «nunca solo color» también en ASCII.

### D7 — La analítica cuenta los turnos vacíos

`facts_of_log` cuenta cada `turn_empty`; `UsageActivity` gana `emptyTurns`
(aditivo, cero por omisión) y el esquema lo declara bajo `properties`, porque
`activity` es `additionalProperties: false`. Lo muestran las dos superficies
que muestran la actividad: la vista Consumo de la GUI y `meltemi usage` en el
CLI (solo cuando es mayor que cero). La casa `usage` de la TUI sigue reservada.

### D8 — El agente simulado aprende a callar

`mock-agent --silent-turn` termina cada turno con `end_turn` sin enviar
actualización ni pedir permiso. Apagado por omisión, como todo interruptor del
simulado: ninguna prueba existente cambia de log. Con él se prueban de punta a
punta el evento, la respuesta, el código de salida y la analítica, sin agente
real (constitución §5).

## Risks / Trade-offs

- **R1 — Un turno legítimamente callado.** Un prompt que solo cambia opciones,
  o un agente que acierta a no hacer nada. El veredicto describe el turno, no
  lo juzga, y el remedio habla en condicional («si no lo esperabas»). El
  código `15` deja que el script decida.
- **R2 — Un agente que solo emite `usage_update`.** Es exactamente el caso
  medido: gastó cero tokens y no dijo nada. Se declara vacío.
- **R3 — Orden de las notificaciones.** Si una versión futura del crate ACP
  despachara manejadores en paralelo, un turno con salida podría leerse vacío.
  La garantía está documentada en el crate pineado, y el e2e con el simulado
  normal exige que ningún turno con salida se declare vacío.

## Migration

Todo es aditivo: un evento nuevo en el log (los clientes previos lo muestran
como línea de sistema neutra), un campo opcional en cinco respuestas y en la
actividad, y un código de salida nuevo que antes era `0`. Esto último es el
cambio visible: un script que hoy recibe `0` de un turno vacío recibirá `15`,
y es a propósito.
