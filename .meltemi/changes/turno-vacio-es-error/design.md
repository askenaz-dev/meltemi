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
  no lo escriben nunca, y los turnos intermedios de una sesión que corre varios
  turnos (`acp.rs:477-528`) no tienen evento de cierre: el siguiente
  `PromptSent` es el único borde. Un campo en `TurnCompleted` no mediría «cada
  turno».
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

La revisión adversarial del plan (17 hallazgos verificados de 20) corrigió
seis cosas que este design ya incorpora: un turno detenido por el humano no es
vacío aunque el agente conteste `end_turn` (D2); `sdd/gate` corre la mayoría de
los turnos de autoría (D3); el código de salida sigue al campo y no a una lista
de verbos (D4); el contrato de las respuestas necesita sus propios requisitos
(D3); el detalle histórico de la TUI también lo dice (D6); y el simulado tiene
que poder hacer trabajo, callar y volver a trabajar (D8).

## Goals / Non-Goals

**Goals**: que todo turno que el agente cierra como terminado sin haber dicho
ni hecho nada, y que nadie detuvo, quede declarado vacío en el log, sea cual sea
el runner; que las respuestas que informan el estado de un turno lo digan; que
el CLI no salga con `0`; que la GUI y la TUI lo muestren como estado, con
palabra y remedio; y que la analítica lo cuente.

**Non-Goals**: diagnosticar por qué el agente no dijo nada (leer su `stderr`
es otra change), reintentar el turno, cambiar el motivo de parada que el agente
reportó, marcar la fila de la barra lateral, o tocar los adaptadores propios,
que ya propagan los rechazos del proveedor con sus palabras.

## Decisions

### D1 — Se mide dentro de cada turno, con un clasificador puro

Una función pura, `carries_work(&SessionUpdate) -> bool`, decide qué
actualización lleva trabajo del agente:

- `agent_message_chunk` y `agent_thought_chunk` con texto que no sea solo
  espacios, **o con contenido no textual** (imagen, audio, recurso): un agente
  que responde solo con una imagen respondió;
- `tool_call`, `tool_call_update` y `plan`.

No llevan trabajo `user_message_chunk` (el eco del prompt),
`available_commands_update`, `current_mode_update`, `config_option_update`,
`session_info_update` ni `usage_update`: describen la sesión, no el turno.

`HandlerState` gana un contador del turno (`Arc<AtomicU32>`, como `denials`).
`forward_update` lo incrementa cuando `carries_work` dice que sí, y
`passthrough_permission` por cada petición de permiso. Se pone a cero al
escribir cada `PromptSent` y se lee en cuanto la respuesta al prompt resuelve;
el contexto garantiza que entonces ya contó todo. El clasificador puro es lo
que los unit tests fijan: `HandlerState` exige un `Peer` vivo y el simulado no
envía planes ni turnos de solo pensamiento.

- **Descartado: contar después, leyendo el log.** Obligaría a releer el
  archivo por turno o a llevar un índice del último `PromptSent`; el manejador
  ya ve cada actualización en el orden correcto.

### D2 — El veredicto es un evento propio por turno, y nunca para un turno detenido

Un turno se declara vacío cuando se cumplen las cuatro condiciones:

1. su estado es `completed`;
2. su contador está en cero;
3. durante ese turno no se pidió detenerlo: el brazo de cancelación del
   `select!` de `acp.rs:418-425` no disparó (cubre cancelar la sesión,
   interrumpir el turno y el apagado del daemon, que son el mismo aviso);
4. la sesión no está cancelada y el turno no fue interrumpido con relevo.

Entonces el daemon escribe `turn_empty { stopReason }` en un bloque nuevo
**después** de tomar el relevo (`acp.rs:448-451`) y **antes** de la rama que
cierra por cancelación (`acp.rs:452`), la cola y la espera. Por eso lo tienen
también los turnos intermedios, `worktree/dispatch` y cada tarea de
`sdd/implement`: todos pasan por ese bucle de `run_session`.

- **Por qué la condición 3.** El código ya trata la cancelación como
  independiente del motivo de parada (comentario de `acp.rs:433-434`), y el
  simulado ignora la cancelación a propósito porque «es el caso que el daemon
  tiene que sobrevivir» (`mock-agent/src/main.rs:134-140`). Un agente que
  contesta `end_turn` a una cancelación sin haber dicho nada no falló: lo
  pararon. Declararlo vacío convertiría una decisión humana en un fallo del
  proveedor.
- **Solo `completed`.** Un turno `cancelled`, `refused`, `max_tokens` o
  `max_turn_requests` ya dice que no terminó; `stopReason` viaja igual para que
  el evento se lea solo y para que ampliar el criterio sea aditivo.
- **El motivo del agente no cambia.** `TurnCompleted` y el `status` de las
  respuestas siguen diciendo lo que el agente dijo.
- **No se persisten los conteos**: cada actualización ya está en el log, y
  repetir su número sería una segunda fuente que puede discrepar.
- **Descartado: un campo en `TurnCompleted`** (contexto).

### D3 — Las respuestas dicen si el turno que informan fue vacío

`SessionOutcome` gana `last_turn_empty: bool` y `empty_turns: u32`, con la
misma regla de D2. Las respuestas que informan el estado de un turno ganan
`emptyTurn: true`, aditivo y omitido cuando es falso:

| Método | Resultado | Qué dice `emptyTurn` |
| --- | --- | --- |
| `session/start` (sin `detach`) | `SessionStartResult` | el turno cuyo `status` informa |
| `propose` | `ProposeResult` | ídem |
| `session/direct` (reanudación) | `SessionDirectResult` | ídem |
| `worktree/dispatch` | `WorktreeDispatchResult` | ídem |
| `sdd/explore`, `sdd/propose`, `sdd/plan`, `sdd/constitution`, `sdd/gate` | `SddResult` | algún turno de autoría de la llamada (la vía rápida corre cuatro; `sdd/gate` corre el siguiente artefacto o el retrabajo) |

`run_turn` y `author_artifact` (`sdd_flow.rs`) dejan de tirar el
`SessionOutcome` y devuelven si el turno fue vacío.

**Sin `emptyTurn`, por declaración**: un arranque con `detach`, una instrucción
encolada o relevada (no informan estado), `sdd/review-decide` (descarta el
resultado del gate que dispara, `review.rs:216`) y `sdd/implement` (su tarea ya
queda `pending`). En todos ellos el evento del log es la fuente, y es la que
leen la GUI y la TUI; `docs/paridad-nucleo.md` lo declara.

Cada método tiene su requisito en la capacidad que posee su respuesta:
`acp-session` (`session/start`, `session/direct`), `propose-flow`,
`worktree-orchestration` y `sdd-authoring`.

### D4 — El CLI sigue al campo: advierte en stderr y termina con `15`

La taxonomía de códigos de salida gana `15`: *el turno del agente terminó sin
decir ni hacer nada*. La regla vive en un solo lugar, `tui/src/lib.rs`, junto
a la de `validate` (`lib.rs:91-111`): todo subcomando cuya respuesta trae
`emptyTurn: true` termina con `15` y, en modo humano, escribe en stderr:

```
warning: the agent ended its turn without saying or doing anything — if that
was not expected, try the agent on its own and check its provider and credentials
```

Así la cumplen `session`, `propose`, `direct`, `dispatch`, `explore`, `plan`,
`constitution` y cualquier verbo futuro cuya respuesta lleve el campo. En
`--json` la respuesta viaja íntegra por stdout y el código es el mismo. La
decisión es una función pura (`exit_for(&Command, &Value)`) para que un unit
test la fije sin daemon.

- **Por qué un código propio y no `0` con aviso.** Un script lee el código, no
  stderr; un `0` es exactamente el éxito que esta change deja de afirmar.
- **Por qué no `1` ni `11`.** Ni el binario falló ni el daemon rompió el
  contrato: el turno ocurrió y no llevó nada.
- **Enmienda**: la taxonomía es un requisito vivo de `cli-contract` que
  enumera los códigos; añadir uno es un MODIFIED. Por eso esta change deja la
  vía rápida y sigue la completa.

### D5 — La GUI lo pinta como estado del turno

El pliegue de la conversación convierte `turn_empty` en un aviso propio
(`kind: "empty"`), que la vista pinta con glifo `∅`, la palabra «turno vacío» y
el remedio en condicional: «el agente terminó sin decir ni hacer nada; si no
lo esperabas, pruébalo por su cuenta y revisa su proveedor y sus
credenciales». Nunca solo color; claves en español e inglés. En el log de
operador, `EVENT_STYLE` gana `turn_empty: { glyph: "∅", tone: "warn" }`: el
test que exige glifo y tono a todo tipo de evento declarado
(`desktop/tests/scenarios_shell.rs`, `every_declared_event_type_has_a_glyph_and_a_tone`)
lo pide en el mismo commit que añade el evento.

**La barra lateral no se toca**, y es una decisión: una sesión que termina tras
su turno pasa a «Detenidas», y una sesión libre de la GUI, que arranca
separada, sigue en «Listas para tu instrucción» hasta su espera; el veredicto
aparece en la conversación, no en la fila. Marcar la fila es otra change si la
evidencia lo pide.

### D6 — La TUI lo dice en el transcript y en el detalle histórico

`summarize_event` (`tui/src/shell/conn.rs:789-806`) convierte `turn_empty` en
`[<sesión>] turno vacío — <remedio>`, y `summarize_log_line` (`conn.rs:529-539`)
hace lo mismo en el detalle de una sesión finalizada: el texto sale del
catálogo `Msg` en el idioma de la interfaz, que ambas funciones reciben. Sin
glifo: ninguna de las dos recibe la presentación, y la palabra ya cumple
«nunca solo color» también en ASCII.

### D7 — La analítica cuenta los turnos vacíos

`facts_of_log` cuenta cada `turn_empty`; `UsageActivity` gana `emptyTurns`
(aditivo, cero por omisión) y el esquema lo declara bajo `properties`, porque
`activity` es `additionalProperties: false`. Lo muestran las dos superficies
que muestran la actividad: la vista Consumo de la GUI, como una columna más
junto a las otras (que ya muestra sus ceros), y `meltemi usage`, que solo lo
imprime cuando es mayor que cero. La casa `usage` de la TUI sigue reservada.

### D8 — El agente simulado aprende a callar

Dos interruptores, apagados por omisión como todos los del simulado:

- `--silent-turn` calla todos los turnos;
- `--silent-turn-at <N>` calla solo el turno `N` (desde cero), para correr
  trabajo → vacío → trabajo en una sesión de varios turnos.

Un turno callado respeta `--turn-delay-ms` antes de responder, para que un
test pueda cancelarlo o interrumpirlo en vuelo, y responde con el mismo motivo
que un turno normal (`end_turn`, o `cancelled` si `--honor-cancel` y hubo
cancelación). La decisión es una función pura, `silent_in(args, turn)`, con su
unit test.

## Risks / Trade-offs

- **R1 — Un turno legítimamente callado.** Un prompt que solo cambia opciones,
  o un agente que acierta a no hacer nada. El veredicto describe el turno, no
  lo juzga, y el remedio habla en condicional. El código `15` deja que el
  script decida.
- **R2 — Un agente que solo emite `usage_update`.** Es exactamente el caso
  medido: gastó cero tokens y no dijo nada. Se declara vacío.
- **R3 — Orden de las notificaciones.** Si una versión futura del crate ACP
  despachara manejadores en paralelo, un turno con salida podría leerse vacío.
  La garantía está documentada en el crate pineado, y los e2e con el simulado
  normal exigen que ningún turno con salida se declare vacío.
- **R4 — Una cancelación que llega justo cuando el turno termina solo.** Si el
  brazo de cancelación dispara, el turno no se declara vacío aunque el agente
  no llegara a leerla. Se prefiere callar un vacío dudoso a culpar al
  proveedor de una decisión humana.

## Migration

Todo es aditivo: un evento nuevo en el log (los clientes previos lo muestran
como línea de sistema neutra), un campo opcional en cinco respuestas y en la
actividad, y un código de salida nuevo que antes era `0`. Esto último es el
cambio visible: un script que hoy recibe `0` de un turno vacío recibirá `15`,
y es a propósito.
