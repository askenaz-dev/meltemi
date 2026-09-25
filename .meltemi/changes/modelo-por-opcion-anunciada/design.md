# Design — modelo-por-opcion-anunciada

## Context

`meltemi session "…" --agent opencode --model anthropic/claude-haiku-4-5`
respondió con el error 2006 aunque OpenCode, en el mismo handshake, había
anunciado una opción de categoría `model` con 317 valores. Leído el código antes
de diseñar, estos hechos fijan el diseño:

- **La regla vive en el nombre del binario.** `levels::levers_of`
  (`levels.rs:1037-1078`) decide por el nombre de archivo del programa:
  «codex» → modelo y esfuerzo; «claude» → modelo; «mock-agent» → modelo;
  cualquier otro → nada. `refuse_unsupported_levers` (`levels.rs:1092`) rehúsa
  con 2006 **antes** de crear la sesión (`free_session.rs:116-124`).
- **La vía de la bandera es el entorno.** `with_session_levers`
  (`levels.rs:126-152`) escribe `MELTEMI_SESSION_MODEL`/`MELTEMI_SESSION_EFFORT`
  en el entorno del subproceso; solo los adaptadores propios lo leen
  (`meltemi-adapters/src/lib.rs:37-51`, `claude/wire.rs:97`,
  `codex/mod.rs:931`). Para un agente de terceros ese entorno no significa nada.
- **La vía anunciada ya existe, pero solo en vivo.** Al abrir la sesión,
  `acp::run_session` guarda lo anunciado (`acp.rs:324-390`,
  `session_config::announced`) y el verbo `session/set-config-option`
  (`server.rs:2015-2180`) valida un valor contra el anuncio
  (`session_config::chosen_value`) y lo envía por
  `SetSessionConfigOptionRequest` sobre la conexión viva. El CLI no puede usarlo
  porque su sesión termina con el turno.
- **ACP define las categorías**: `mode`, `model`, `model_config`,
  `thought_level` y otras (`agent-client-protocol-schema` 1.4,
  `SessionConfigOptionCategory`). El modelo es la categoría `model`; el
  esfuerzo, `thought_level` («thought/reasoning level selector»). La
  correspondencia es del protocolo, no de un proveedor.
- **El perfil se pierde fuera de la sesión libre.** `resolve_session_agent`
  devuelve el `model`/`effort` del perfil en todo verbo, pero solo
  `free_session.rs:119` llama a `with_session_levers`; `propose.rs:94`,
  `sdd_flow.rs:489`, el dispatch (`server.rs:847`) y el implement
  (`server.rs:1759`) arrancan el agente sin aplicarlos, en silencio.
- **Lo que rigió se lee del evento de resolución.** `AgentResolved` lleva
  `model`/`effort` y el índice se reconstruye desde él
  (`session_index.rs:255-270`).
- **El simulado anuncia con `--config-options`**: una opción `model` (categoría
  `model`, valores `fast`/`slow`) y una `thinking` (categoría `thought_level`,
  booleana), y honra `session/set_config_option` con estado real
  (`mock-agent/src/main.rs:37-132, 239-257`). Sin la bandera no anuncia nada.
- **Los tests de hoy pinean la regla por nombre**: `e2e_permisos.rs:1241-1330`
  usa el simulado como agente que «toma modelo y no esfuerzo».

## Goals / Non-Goals

**Goals**: que el modelo y el esfuerzo pedidos lleguen al agente por la vía que
el agente de verdad tiene —su bandera documentada o la opción que anuncia—;
que un valor que el agente no anunció se rehúse nombrando lo que sí anunció;
que el perfil rija en todo verbo que arranca un agente; que el registro diga lo
que el agente confirmó; y que cada superficie ofrezca lo que se sabe de cada
agente, sin inventar.

**Non-Goals**: un catálogo de modelos propio o traducir nombres entre
proveedores (§5); cambiar de modelo a mitad de turno (`set-option` ya lo hace
entre turnos); palancas en `session/direct` al reanudar; tokens y costo
(`usage-desde-acp`); palancas para agentes de nivel 3/4 más allá de rehusarlas.

## Decisions

### D1 — Tres vías por palanca, en orden: bandera declarada, opción anunciada, rehúso

Para cada palanca pedida (modelo, esfuerzo):

1. **Bandera declarada**: si la entrada del registro la declara, viaja como hoy
   por el entorno al adaptador, opaca, sin validarse.
2. **Opción anunciada**: si no, el valor se fija con `session/set_config_option`
   sobre la opción que el agente anunció en la categoría de la palanca
   (`model`, `thought_level`), antes del primer prompt (D3).
3. **Rehúso**: si el agente no puede tenerla por ninguna de las dos, se rehúsa
   nombrando al agente y la palanca (D4).

Una entrada que declara sus palancas declara el conjunto **completo**: la que no
nombra se rehúsa antes de lanzar nada, porque se sabe que ese agente no la toma.

### D2 — El registro declara las palancas de los adaptadores propios; el resto las anuncia

- `fleet-registry.toml` gana `levers = ["model", "effort"]` por entrada,
  opcional. Presente: la bandera es la vía de esas palancas y las demás se
  rehúsan antes de lanzar. Ausente: toda palanca va por la opción anunciada.
- Lo declaran las dos entradas de adaptadores propios, que son las que leen el
  entorno: Codex (`model`, `effort`) y Claude Code (`model`). Ninguna entrada
  de terceros lo declara: su palanca es lo que anuncien.
- `levers_of` deja de leer nombres de binario. Un agente sin entrada —un
  `agent.command` pelado, como el simulado en las pruebas— va por la opción
  anunciada; las pruebas de la vía de la bandera usan un registro de prueba que
  la declara.
- Un valor desconocido en `levers` es un diagnóstico del registro, no un error:
  la entrada queda sin él, como un campo inválido cualquiera.

### D3 — La opción anunciada se fija antes del primer prompt

`SessionParams` gana las palancas que van por anuncio. En `run_session`, tras
`session/new` y antes del primer prompt:

- se busca, por palanca, la opción anunciada de su categoría; el valor se
  valida con `session_config::chosen_value` —la misma validación del verbo en
  vivo, sin catálogo propio—;
- se envía `SetSessionConfigOptionRequest` con una espera acotada (30 s): un
  agente que no contesta no deja una sesión colgada;
- lo que el agente devuelve es el registro: se actualiza la configuración viva,
  se anota `config_options_announced` y un evento nuevo `levers_applied` con el
  valor **que el agente confirmó** (el `current_value` de su respuesta), no el
  pedido.

El índice y `session/list` toman el modelo y el esfuerzo de `levers_applied`
cuando existe, y de `agent_resolved` cuando no (vía de la bandera).

### D4 — Se rehúsa cuando la verdad se conoce, y nunca después del primer prompt

- **Antes de crear nada** cuando ya se sabe: la entrada declara sus palancas y
  no nombra la pedida, o el agente es de nivel 3/4 (no anuncia). Es el 2006 de
  hoy, con el mismo texto.
- **Al abrir, antes del primer prompt**, cuando solo el anuncio puede decirlo:
  el agente no anunció la categoría, o el valor no está entre los anunciados.
  La sesión termina sin turno con la razón `lever_refused`, y el verbo responde
  un error que **lista lo anunciado** («anunció: fast, slow») o dice que no
  anunció ninguno. La sesión queda en el histórico tal como fue: pedida,
  abierta, rehusada.
- Nunca después: si el agente acepta, recién entonces sale el prompt.

La sesión libre toma su punto de restauración antes de abrir: una sesión
rehusada al abrir deja esa ref técnica, como cualquier sesión que termina sin
tocar nada. Se declara; no se reordena.

### D5 — Todos los verbos que arrancan un agente aplican las palancas del perfil

`propose`, `explore`/`plan`/`constitution` (flujo SDD), `dispatch` e
`implement` aplican el `model`/`effort` del perfil con la misma función que la
sesión libre (`with_session_levers`), con las tres vías de D1 y el mismo
registro en `agent_resolved`/`levers_applied`. Precedencia sin cambios: lo
explícito de la sesión pisa el perfil; un perfil que no declara no impone.
Estos verbos no ganan banderas propias.

### D6 — La flota recuerda lo último que cada agente anunció

- Cada apertura que anuncia opciones, con un agente de la flota resuelto
  (`agent_id`), guarda el anuncio en el directorio de datos
  (`fleet-announced.json`: por id, la fecha y las opciones tal cual).
- `fleet/list` lo devuelve por entrada (`announced`, aditivo) junto a las
  palancas declaradas (`levers`, aditivo).
- Es un recuerdo, no una promesa: sirve para que las superficies ofrezcan, y la
  decisión sigue siendo la del agente al abrir (D4). Un `agent.command` pelado
  no tiene id y no se recuerda.

### D7 — Cada superficie ofrece lo que se sabe, y nada más

- **GUI**, el selector de modelo del compositor, según el agente elegido: la
  lista anunciada la última vez (con búsqueda, y entrada libre advertida «no
  estaba en el último anuncio: se confirma al arrancar»); una palanca por
  bandera (entrada libre, como hoy); «este agente no toma modelo» cuando su
  último anuncio no traía la categoría y no la declara; y, sin nada sabido,
  entrada libre con «se confirma al arrancar».
- **TUI**, la vista Flota, y el **CLI**, `meltemi fleet`: por agente, qué
  palanca toma por bandera y qué anunció la última vez (cuántos valores y cuándo).
- Ninguna muestra precios ni un catálogo propio.

### D8 — El simulado sigue siendo la única pieza de las pruebas

Vía anunciada: el simulado con `--config-options` (acepta `fast`/`slow`).
Vía de la bandera: un registro de prueba que declara `levers` para una entrada
del simulado. Rehúso al abrir: el simulado sin `--config-options`. Ninguna
prueba toca un agente real ni la red.

## Risks / Trade-offs

- **R1 — Un viaje de ida y vuelta más antes del primer turno**, solo cuando se
  pide una palanca por anuncio; acotado a 30 s.
- **R2 — Un rehúso al abrir cuesta un lanzamiento** y deja una sesión corta en
  el histórico: es el precio de no decidir por nombre de binario. El recuerdo
  de D6 permite a las superficies avisar antes.
- **R3 — Lo que OpenCode 1.18 hace con un `set_config_option` a `ollama/…`**
  solo lo confirma una corrida real (`corrida-integrada`).
- **R4 — Las pruebas que pineaban el simulado como «toma modelo»** cambian a un
  registro de prueba: el comportamiento que pinean no cambia.

## Migration

Aditivo en el contrato (`levers`, `announced`, `levers_applied`). Codex y Claude
Code se comportan como hoy. Un agente de terceros que anuncia la palanca pasa
de rehusarse a aceptarla; uno que no la anuncia pasa de rehusarse antes de
lanzar a rehusarse al abrir, sin turno.
