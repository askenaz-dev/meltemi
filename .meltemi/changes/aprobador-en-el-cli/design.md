# Design — aprobador-en-el-cli

## Context

El 2026-09-24 `meltemi session … --agent claude-code` terminó `completed` con
doce permisos denegados: el CLI responde toda petición de permiso con
`cancelled` y la frase «interactive approval arrives in a later release»
(`tui/src/run.rs:220-245`). El proxy funciona; el cliente no tiene con qué
contestar.

Leído el código antes de diseñar, estos hechos fijan el diseño:

- **El push llega al cliente que arrancó el flujo** (`acp.rs:1084,1097`) como
  `permission/request {sessionId, toolCall, options}`, y su respuesta alimenta
  la cola como una decisión más (`acp.rs:1078-1126`), primera en ganar. **El
  push no lleva el id de la cola**, que sí existe en ese punto
  (`acp.rs:949-970`): sin él, quien responde el push no puede usar
  `permission/decide` (el único camino que persiste una regla,
  `server.rs:2778-2805`) ni reconocer en `permission/changed` que su petición
  se resolvió por otra vía.
- **«Permitir en esta sesión» no existe en el daemon.** Las reglas tienen
  ámbito `global | project` y se cargan al abrir la sesión
  (`free_session.rs:210-215`, `propose.rs:194`); la única concesión que dura
  una sesión es la opción que el propio agente ofrece (Codex ofrece
  `allow-session`, `codex/permission.rs:34-38`; Claude y el simulado, no).
- **La bandeja muestra herramienta, título y opciones**, no rutas ni diff: la
  cola no tiene campos para ellos. Lo que sí viaja en el push es el `toolCall`
  del agente, con `rawInput.command`, `rawInput.path` o `locations` cuando el
  agente los da — que son justo los hechos que las reglas leen
  (`permissions.rs:39-66`).
- **Denegar, en la bandeja, elige la opción de rechazo del agente** y cancela
  solo si no hay ninguna (`live.rs:497-510`). Responder `cancelled` no es lo
  mismo: los adaptadores le dicen al agente «el turno se canceló antes de
  decidirlo» (`claude/permission.rs:236`) en vez de «rehusado».
- **`main.rs` sostiene los candados de stdout y stderr** durante todo el
  despacho en un runtime de un hilo (`main.rs:13-22`); escribir desde otro hilo
  del sistema se bloquea (medido y escrito en `run.rs:1644-1650`).
- **Las pruebas en proceso comparten el stdin y el stderr reales**: si la
  biblioteca detectara el terminal por su cuenta, una prueba corrida desde un
  terminal se quedaría esperando una tecla.
- **`--mode` ya existe** (`cli.rs:47-51`): la propuesta lo daba por futuro.

## Goals / Non-Goals

**Goals**: que un humano en un terminal pueda contestar las preguntas del
agente desde el mismo comando que las provocó, con las opciones del agente y
con la misma regla que ofrece la bandeja; que sin terminal nada cambie salvo la
honestidad del aviso; que Ctrl+C pare de verdad; que stdout siga siendo el
canal de `--json`; y que ninguna bandera conceda nada.

**Non-Goals**: una concesión «para esta sesión» inventada por Meltemi (no
existe en el daemon y crearla es cambiar la política), mostrar rutas o diffs
que el agente no envió, verbos para decidir por id desde un script
(`verbos-del-agent-boss`), avisar fuera del terminal (`aviso-de-espera-opt-in`),
y las preguntas con texto libre del agente (`preguntas-del-agente`).

## Decisions

### D1 — Se pregunta solo donde un humano puede contestar

El aprobador está activo cuando **stdin y stderr son terminales**, y eso se
decide una sola vez, en `main.rs`. La biblioteca (`execute`, `dispatch`)
deniega por omisión, como hoy: ninguna prueba en proceso puede quedarse
esperando una tecla. Con stdin redirigido (un script que alimenta el comando)
no hay humano del otro lado y no se pregunta.

Sin terminal, la denegación sigue igual y el aviso deja de prometer una release
futura: dice que no hay terminal donde preguntar y nombra las dos maneras de
conceder sin humano, una regla en `.meltemi/permissions.toml` o `--mode`.

### D2 — La pregunta muestra lo que decide y nada más

En stderr, antes de leer la tecla:

```
permission requested by session <id>
  tool     <kind, o el título si no hay kind>
  title    <título>
  command  <rawInput.command, si viene>
  path     <rawInput.path o locations[0].path, si viene>
  1 <nombre de la opción>  (<clase>)
  2 …
  d deny   r allow and remember   Ctrl+C stop the session
```

Las líneas `command` y `path` aparecen solo si el agente las envió. Las opciones
son **las del agente**, en su orden, con su nombre y su clase: «permitir en esta
sesión» aparece exactamente cuando el agente la ofrece. **No hay tecla por
omisión**: Enter no decide nada.

### D3 — Una tecla, y cada tecla hace lo que la bandeja haría

- **Un dígito** elige la opción del agente con ese número.
- **`d`** (y Esc) deniega con la opción de rechazo del agente, o cancela si no
  ofreció ninguna: la misma regla que la bandeja, en una función compartida.
- **`r`** aprueba con la primera opción de permiso del agente y recuerda la
  decisión como regla (D5), tras una segunda tecla de confirmación (`y`); con
  cualquier otra tecla vuelve a la pregunta.
- **Ctrl+C** deniega esta petición (cancelada), envía `session/cancel` y el
  comando termina con `13`, que la taxonomía ya define como «operación
  cancelada». No hace falta manejador de señales: en modo crudo Ctrl+C llega
  como tecla.
- Cualquier otra tecla no hace nada.

Toda decisión queda en el log con `decidedBy: client`, como las de la bandeja:
el daemon no distingue superficies.

### D4 — El push lleva el id de la cola

`PermissionRequestParams` gana `requestId`, aditivo: el mismo id que la entrada
de la cola. Es lo que permite al cliente que responde el push decidir por id y
persistir una regla (`permission/decide`), pedir la sugerencia del daemon
(`permission/pending`) y reconocer en `permission/changed` que su petición ya no
está pendiente. La TUI y la GUI lo reciben y hoy no lo usan: su conducta no
cambia. Es un campo del contrato de permisos (`permission-rules`).

- **Descartado: emparejar por sesión**, como hace la GUI. Una sesión puede
  tener dos peticiones a la vez; emparejar por sesión elige la equivocada.

### D5 — Recordar es la regla de la bandeja, por el camino de la bandeja

`r` propone **la misma regla que la bandeja**: la sugerencia anti-fatiga del
daemon si la entrada la trae, y si no, permitir esa herramienta en el ámbito
del proyecto. La lógica pasa a una función compartida entre el shell y el CLI.
Se persiste con `permission/decide {requestId, optionId, persistRule}` y después
se responde el push con la misma opción (la primera resolución gana; la segunda
recibe «ya resuelta» sin efecto).

- **Rige desde la siguiente sesión**: las reglas se cargan al abrir la sesión
  (requisito vivo), y el aprobador lo dice al confirmar.
- **Solo se ofrece con herramienta**: una petición sin `kind` ni título daría
  una regla sin herramienta, más amplia que cualquier cosa que se sepa. Tampoco
  se ofrece si el push no trae `requestId` (un daemon anterior).
- **Límite conocido**: `permission/decide` no informa si la persistencia
  falló (hoy solo lo anota el daemon, para todas las superficies). Se declara y
  se deja como propuesta aparte.

### D6 — Una pregunta a la vez, retirada cuando deja de estar en pie

El aprobador atiende los pushes en orden: si llega otro mientras hay una
pregunta abierta, espera su turno. Mientras espera la tecla sigue leyendo la
conexión, y retira la pregunta, diciendo por qué, cuando:

- llega `permission/timeout` de esa sesión (y de esa herramienta, si la
  nombra): venció por la política de espera del daemon;
- llega un `permission/changed` en el que su `requestId` ya no está: se decidió
  en otra superficie.

El CLI no tiene temporizador propio: la espera es la del daemon
(`[permissions] wait`), que el CLI no reimplementa.

### D7 — Las teclas se leen en modo crudo, sin pantalla completa

Con crossterm, que ya llega por la reexportación de `ratatui`: modo crudo sin
pantalla alternativa, en un hilo bloqueante, sondeando en tramos cortos para
que una retirada termine la lectura y restaure el terminal. Al entrar se
descartan las teclas escritas antes de que la pregunta apareciera: una tecla
tecleada de antemano no contesta una pregunta que nadie vio. En Windows solo
cuenta la pulsación, no la liberación. La pregunta se escribe antes de entrar
en modo crudo y la respuesta después, desde el hilo del runtime, que es el que
tiene el candado de stderr. La fuente de teclas es un trait: las pruebas la
programan.

### D8 — Ninguna bandera concede permisos

No hay `--yes`, `--allow` ni nada parecido, y una prueba de la gramática lo
fija: la constitución (§3) no admite el atajo, y quien necesite conceder sin
humano tiene reglas y modos, que se leen y se auditan.

### D9 — stdout queda intacto

El aprobador escribe solo en stderr (una salida inyectable para las pruebas).
Con `--json`, stdout sigue llevando exactamente un objeto; una prueba lo
consume con el aprobador activo.

### D10 — El simulado aprende a pedir con nombre

`mock-agent --titled-permission` pone `kind: edit` y un título en su petición
de permiso, para que el camino de la regla se pruebe de punta a punta. Apagado
por omisión: ninguna prueba existente cambia lo que lee.

## Risks / Trade-offs

- **R1 — El terminal queda en modo crudo si el proceso muere leyendo.** El
  guardia restaura al salir; una muerte forzada no se puede atender, como en
  cualquier TUI.
- **R2 — Sin terminal en CI.** La lectura de teclas se prueba por su
  traducción pura y el resto de punta a punta con la fuente programada. El
  camino con terminal real queda para una verificación manual del mantenedor.
- **R3 — Una regla pedida que no se escribe.** D5, límite conocido.

## Migration

Aditivo: un campo en el push que los clientes previos ignoran. En un terminal,
el CLI pasa de denegar a preguntar; sin terminal, deniega como siempre.
