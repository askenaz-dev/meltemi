# Design — modelo-por-opcion-anunciada

## Context

`meltemi session "…" --agent opencode --model anthropic/claude-haiku-4-5`
respondió con el error 2006 aunque OpenCode, en el mismo handshake, había
anunciado una opción de categoría `model` con 317 valores. Leído el código antes
de diseñar, estos hechos fijan el diseño:

- **La regla vive en el nombre del binario.** `levels::levers_of` decide por el
  nombre de archivo del programa: «codex» → modelo y esfuerzo; «claude» →
  modelo; «mock-agent» → modelo; cualquier otro → nada. `refuse_unsupported_levers`
  rehúsa con 2006 antes de crear la sesión (`free_session.rs:116-124`).
- **La vía de la bandera es el entorno.** `with_session_levers` escribe
  `MELTEMI_SESSION_MODEL`/`MELTEMI_SESSION_EFFORT` en el entorno del
  subproceso; solo los adaptadores propios lo leen
  (`meltemi-adapters/src/lib.rs:37-51`).
- **La vía anunciada ya existe, pero solo en vivo.** `acp::run_session` guarda lo
  anunciado y la conexión antes del primer prompt (`acp.rs:324-397`); el verbo
  `session/set-config-option` valida con `session_config::chosen_value` y envía
  `SetSessionConfigOptionRequest` (`server.rs:2015-2180`), cuya respuesta es el
  conjunto **completo** de opciones tal como quedó.
- **ACP define las categorías** —`mode`, `model`, `model_config`,
  `thought_level`— como dato de interfaz, y dice que un cliente no debe
  depender de ellas para ser correcto (`agent-client-protocol-schema` 1.4,
  `src/v1/agent.rs:2470-2473, 2523-2527`).
- **El perfil se pierde fuera de la sesión libre**: solo `free_session.rs:119`
  aplica sus palancas.
- **Los verbos crean cosas antes de resolver el agente**: `propose` escribe su
  andamio (`propose.rs:62-79`) antes de resolver (`:94`); `dispatch` crea
  worktree, rama y checkpoint (`server.rs:807-824`) antes de `run_session`.
- **Un fallo de `run_session` se vuelve «el agente falló»**: `acp.rs:609-610`
  lo aplana en texto; la sesión libre responde 2002 y cierra con razón `error`;
  `dispatch` lo convierte en `status: refused` y responde bien
  (`server.rs:958-961`); `implement` descarta el resultado (`server.rs:1845`);
  `finalize_ok` siempre escribe `turn_completed`.
- **`session/list` lee el índice**: el registro de inicio lleva el modelo
  pedido, los de cierre `None`, y la fusión no toma el modelo de un registro
  posterior (`session_index.rs:154-189`); la GUI ofrece de vuelta todo modelo
  visto en el índice como «conocido» (`Home.svelte:118-127`).
- **La cancelación no mira antes del primer prompt**: `cancel()` notifica solo a
  quien ya espera (`session.rs:537-548`), y la espera del prompt se registra
  dentro de su `select!` (`acp.rs:437-445`).
- **Todo verbo rehúsa los niveles 3 y 4 antes de mirar palancas**
  (`level_not_pilotable`).
- **El simulado** anuncia con `--config-options` un selector `model`
  (`fast`/`slow`, `fast` por omisión) y un interruptor `thinking`
  (`thought_level`), responde `set_config_option` con exactamente lo pedido e
  ignora el entorno.

## Goals / Non-Goals

**Goals**: que el modelo y el esfuerzo pedidos lleguen al agente por la vía que
de verdad tiene —la bandera que su adaptador documenta o la opción que anuncia—;
que **ningún prompt salga** si el agente no confirmó exactamente lo pedido; que
el rehúso llegue a quien lo pidió, en todo verbo, como dato; que el perfil rija
en todo verbo que arranca un agente; y que el histórico diga lo que rigió y
nunca lo que se rehusó.

**Non-Goals**: un catálogo de modelos propio (§5); recordar lo que cada agente
anunció y un selector que lo ofrezca —propuesta aparte
(`selector-de-modelo-que-sabe`), porque su clave, sus cotas y su contenido
publicado merecen su propio diseño—; cambiar de modelo a mitad de turno
(`set-option`); palancas al reanudar con `session/direct`; tokens y costo.

## Decisions

### D1 — Tres vías por palanca: bandera declarada, opción anunciada, rehúso

1. **Bandera declarada**: si la entrada del agente la declara (D2), viaja como
   hoy por el entorno al adaptador, opaca, sin validarse.
2. **Opción anunciada**: si la entrada no declara palancas, se fija sobre la
   opción que el agente anunció en la categoría de la palanca —`model`,
   `thought_level`—, antes del primer prompt, y debe confirmarse (D3).
3. **Rehúso**: la entrada declara sus palancas y no esta → antes de crear nada
   (2006); el agente no la anuncia o no confirma → al abrir, sin prompt (2007).

**La categoría es la llave, y es una elección deliberada.** ACP la presenta como
dato de interfaz; aquí es lo único que dice cuál opción es «el modelo» sin
adivinar por id o por valor, y adivinar es lo que §5 prohíbe. Por eso:
exactamente **una** opción de la categoría —ninguna o dos se rehúsan, listando
las opciones anunciadas (id, nombre, categoría) y remitiendo a `set-option
<id> <valor>` en una sesión viva—; y un **interruptor** se fija con
`true`/`false`, las palabras del propio agente, y otro valor se rehúsa diciendo
que la opción es un interruptor.

### D2 — El registro declara las palancas de los adaptadores propios

- `fleet-registry.toml` gana `levers = ["model", "effort"]` por entrada,
  opcional. Presente: la bandera es la vía de esas palancas y las demás se
  rehúsan antes de lanzar (conjunto completo). Ausente: toda palanca va por la
  opción anunciada.
- Lo declaran los dos adaptadores propios, los únicos que leen el entorno: Codex
  (`model`, `effort`) y Claude Code (`model`). Una prueba sobre el registro
  embebido lo pinea y hereda el escenario de `own-adapters` que hoy vive en la
  prueba por nombre.
- `levers_of` desaparece: nada se decide por el nombre del binario. Un
  `agent.command` literal —aunque haya un `agent.id` al lado— no hereda la
  declaración de ninguna entrada, porque el comando es el que corre.
- Un nombre de palanca desconocido o repetido invalida el documento, como un
  nivel fuera de escala: un override inválido se ignora con aviso y rige el
  embebido.
- La declaración es dato interno del daemon en esta change: exponerla en
  `fleet/list` es de `selector-de-modelo-que-sabe`.

### D3 — La palanca anunciada se confirma antes del primer prompt, o no hay prompt

En `run_session`, tras `session/new` y antes del primer prompt:

1. **El modelo primero.** Su opción (D1) se valida con `chosen_value` y se envía
   `SetSessionConfigOptionRequest`.
2. **El esfuerzo después**, validado contra la lista que devolvió el paso
   anterior: fijar un modelo puede cambiar las opciones de esfuerzo.
3. **La última respuesta decide.** Cada palanca pedida debe aparecer en ella con
   exactamente el valor pedido. Si el agente devolvió otro —lo ajustó a otro
   modelo, cayó a uno por omisión— o no lo devolvió, **se rehúsa sin prompt**,
   nombrando lo pedido y lo que el agente dejó. Un prompt lleva el contenido del
   repositorio: sale solo hacia el modelo que se eligió.
4. **El modo no se mueve de lado.** Si la respuesta cambió la opción de categoría
   `mode`, se rehúsa: el modo decide qué hace el agente sin preguntar, y nadie
   lo pidió. Cualquier otra opción que haya cambiado queda en
   `config_options_announced`.
5. **La espera está acotada** (30 s por palanca; parámetro de la sesión, que las
   pruebas acortan) y **se puede cancelar**: cada envío compite con la
   cancelación, y la bandera de cancelado se mira justo antes del primer prompt.
   Una cancelación ahí termina la sesión sin prompt, como cancelada.

Confirmado todo, se anota `levers_applied` con lo que rigió —que es lo pedido,
porque otra cosa se rehúsa— y recién entonces sale el prompt.

### D4 — El rehúso es dato, llega a quien lo pidió, y nunca crea antes de decidir

- **Antes de crear nada** (2006) cuando la entrada declara sus palancas y no la
  pedida. Cada verbo resuelve el agente y reparte las palancas **antes** de sus
  efectos: `propose` antes de su andamio, `dispatch` e `implement` antes del
  worktree.
- **Al abrir** (2007), en los casos de D1 y D3. `run_session` anota un evento
  `lever_refused` —la palanca, lo pedido, la razón (`not_announced`,
  `ambiguous`, `not_a_value`, `not_confirmed`, `mode_changed`, `no_answer`), lo
  que el agente anunció o dejó, a lo sumo 20 valores y cuántos más— y devuelve un
  resultado tipado, no un texto de fallo. Un cierre propio escribe
  `session_ended` con razón `lever_refused` y **sin** `turn_completed`.
- **Cada verbo lo entrega como el 2007 de siempre**, con esos datos en el error:
  la sesión libre adjunta, `propose`, los verbos de autoría, `dispatch` e
  `implement` (que deja de desplegar). La sesión libre desacoplada ya respondió:
  lo dicen su log y sus superficies. Nunca «el agente se negó a continuar».
- **Lo que queda**: `propose` retira su andamio si sigue intacto, para que el
  reintento no choque con «la change ya existe»; el worktree de `dispatch` e
  `implement` y el punto de restauración de la sesión libre quedan como queda
  todo lo de una sesión que termina sin tocar nada.
- **Las superficies lo dicen en español e inglés**: la conversación de la GUI y
  la transcripción de la TUI muestran la palanca rehusada con su motivo, y el
  CLI imprime el error con la lista acotada y saneada (`approver::clean`): lo
  anunciado es texto del agente.

### D5 — Todos los verbos que arrancan un agente aplican las palancas del perfil

La sesión libre, `propose`, los verbos de autoría (`explore`, `plan`,
`constitution`), `dispatch` e `implement` aplican el `model`/`effort` del perfil
con la misma función, las tres vías de D1, la confirmación de D3 y el rehúso de
D4. Precedencia sin cambios: lo explícito de la sesión pisa el perfil; un perfil
que no declara no impone. Estos verbos no ganan banderas propias.

### D6 — El histórico dice lo que rigió, nunca lo que se rehusó

- El evento de resolución y el registro de inicio llevan **solo** lo que viajó
  por bandera; lo anunciado aún no rigió.
- Lo confirmado viaja en el resultado de `run_session` hasta el registro de
  cierre, y la fusión del índice toma modelo y esfuerzo de un registro
  posterior; la reconstrucción desde el log lee `levers_applied`.
- Una sesión rehusada no lista modelo: lo pedido vive en `lever_refused`, no en
  lo que `session/list` presenta como lo que rigió, ni en lo que la GUI ofrece
  como «visto en este proyecto».
- La respuesta de `session/start` gana el modelo y el esfuerzo que rigieron
  (aditivo), y el CLI los imprime.

### D7 — El simulado aprende a demostrar lo que el diseño afirma

Interruptores nuevos, apagados por omisión: `--config-clamp` (responde con otro
valor que el pedido), `--config-hang` (nunca responde a `set_config_option`),
`--config-mode-shift` (fijar el modelo le cambia el modo), `--config-twins`
(anuncia dos opciones de categoría `model`), `--echo-levers`
(dice en su primer mensaje lo que leyó del entorno de palancas) y `--reasoning`
(anuncia un selector `reasoning` de categoría `thought_level`, `low`/`high`, en
lugar del interruptor). Las pruebas piden `slow`, que no es el valor por
omisión; la vía de la bandera se prueba con un registro de prueba que la
declara para una entrada del simulado.

## Risks / Trade-offs

- **R1 — Un viaje de ida y vuelta más** antes del primer turno, solo cuando se
  pide una palanca por anuncio; acotado y cancelable.
- **R2 — Un rehúso al abrir cuesta un lanzamiento** y deja una sesión corta en
  el histórico; recordar lo anunciado para avisar antes es de
  `selector-de-modelo-que-sabe`.
- **R3 — La categoría como llave** deja fuera a un agente que anuncia su selector
  sin categoría: se dice y se remite a `set-option`.
- **R4 — Rehusar lo que el agente ajusta** puede rechazar un alias legítimo (un
  agente que confirma `sonnet-4.5` al pedir `sonnet`): es el precio de no mandar
  el repositorio a un modelo que no se eligió, y el rehúso nombra lo que el
  agente dejó para que se pida eso.
- **R5 — Lo que OpenCode 1.18 hace con `ollama/…`** solo lo confirma una corrida
  real (`corrida-integrada`).

## Migration

Aditivo en el contrato (`levers_applied`, `lever_refused`, el modelo y el
esfuerzo en la respuesta de `session/start`, los datos del 2007). Codex y
Claude Code se comportan como hoy. Un agente de terceros que anuncia la palanca
pasa de rehusarse a aceptarla; uno que no la anuncia pasa de rehusarse antes de
lanzar a rehusarse al abrir, sin turno. `propose`, `dispatch` e `implement`
pasan a aplicar el modelo del perfil que hoy ignoran.

## Revisión adversarial (2026-09-25)

Tres revisores (corrección, seguridad, testabilidad) contra el código, y el plan
se rehízo:

- **D3 nace** del bloqueante: el plan mandaba el prompt aunque el agente hubiera
  dejado otro modelo; ahora lo confirmado debe ser lo pedido, el modo no se
  mueve, la espera se acota y se cancela.
- **D4 entero**: el rehúso no llegaba en la sesión desacoplada, `dispatch` ni
  `implement`, se registraba como fallo del agente, y «antes de crear nada» era
  falso en `propose` y `dispatch`.
- **D6**: `session/list` lee el índice, no la reconstrucción; lo rehusado ya no
  aparece como lo que rigió.
- **D1**: la dependencia de la categoría se declara, con respuesta para la
  ambigüedad, los selectores sin categoría y los interruptores.
- **D2**: un nombre inválido invalida el documento; el embebido queda pineado.
- **D7 nace**: el simulado no podía distinguir lo confirmado de lo pedido, ni
  probar la espera, la cancelación, la bandera ni el esfuerzo por anuncio.
- **Sale del alcance** la memoria de anuncios y el selector que la ofrece: su
  clave (agente y perfil), sus cotas, su escritura y su contenido publicado
  (§9) son un diseño propio → `selector-de-modelo-que-sabe`.
- Se retira el rehúso por nivel 3/4: todo verbo los rehúsa antes.
