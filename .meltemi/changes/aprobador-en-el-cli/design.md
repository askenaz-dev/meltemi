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
  `allow-session`, `codex/permission.rs:34-38`; Claude y el simulado, no). Esa
  opción saca del proxy las peticiones parecidas que vengan después: el agente
  deja de preguntar.
- **Lo que el agente envía varía por adaptador.** Claude manda clase, título
  (la herramienta y la primera línea del comando) y la entrada entera como
  texto en `content`, sin `rawInput` (`claude/permission.rs:98-123`). Codex, en
  una aprobación de comando simple, no manda ni título ni comando: el comando
  viajó antes, en la `tool_call` del mismo id (`codex/permission.rs:155-199`,
  `codex/mapping.rs:626-635`). El simulado manda `rawInput.path`. La conexión
  que arrancó la sesión ya recibe esas `tool_call` por `session/event`.
- **Las reglas del daemon leen hechos del push**: la herramienta es la clase o,
  sin ella, el título; el comando es `rawInput.command`; la ruta,
  `rawInput.path` o la primera ubicación (`permissions.rs:39-66`). Sin clase ni
  título la cola etiqueta la petición con el marcador `permission`
  (`acp.rs:935-938`), que ninguna regla llega a igualar.
- **La sugerencia anti-fatiga es herramienta más prefijo de comando**
  (`pending.rs:329-340`): con los adaptadores de hoy, que no mandan
  `rawInput`, queda en herramienta sola.
- **Denegar, en la bandeja, elige la opción de rechazo del agente** y cancela
  solo si no hay ninguna (`live.rs:497-510`). Responder `cancelled` no es lo
  mismo: los adaptadores le dicen al agente «el turno se canceló antes de
  decidirlo» (`claude/permission.rs:236`) en vez de «rehusado».
- **Cancelar la sesión con una petición en espera la retira de la cola** y el
  log la anota `decidedBy: interrupted` (`acp.rs:1001-1008`, `769-776`);
  `session/cancel` es una notificación (`server.rs:2837-2847`).
- **El daemon atiende cada petición de una sesión antes de la siguiente**
  (`acp.rs:292-296`): una sesión nunca tiene dos preguntas vivas a la vez.
- **Las instantáneas de `permission/changed` salen en el orden en que cada
  conexión las toma** (`server.rs:138-150`), pero el push sale de otra tarea
  (`acp.rs:1098`) y las peticiones del cliente se atienden en tareas aparte
  (`server.rs:225-230`): una instantánea tomada justo antes de registrar la
  petición puede llegar después del push.
- **Cada verbo aborta su manejador de mensajes en cuanto su llamada vuelve**
  (`peer.close(); background.abort();`, 36 veces en `run.rs`) y `main.rs`
  termina con `process::exit`, que no corre destructores: lo que el manejador
  deje a medias queda a medias.
- **`main.rs` sostiene los candados de stdout y stderr** durante todo el
  despacho en un runtime de un hilo (`main.rs:13-22`); escribir desde otro hilo
  del sistema se bloquea (medido y escrito en `run.rs:1644-1650`).
- **Las pruebas en proceso comparten el stdin y el stderr reales**: si la
  biblioteca detectara el terminal por su cuenta, una prueba corrida desde un
  terminal se quedaría esperando una tecla.
- **`--mode` existe solo en `session`** (`cli.rs:630-632`), y en cualquier
  modo un comando escala aunque una regla del usuario lo permita
  (`permissions.rs:229-249`).

## Goals / Non-Goals

**Goals**: que un humano en un terminal pueda contestar las preguntas del
agente desde el mismo comando que las provocó, viendo lo que el agente envió,
con las opciones del agente; que recordar una decisión escriba la regla más
específica posible y nunca una más amplia que lo que se vio; que sin terminal
el agente reciba un rechazo y el aviso diga la verdad; que Ctrl+C pare de
verdad y el registro lo diga; que el terminal vuelva siempre a su estado; que
stdout siga siendo el canal de `--json`; y que ninguna bandera nueva conceda
nada.

**Non-Goals**: una concesión «para esta sesión» inventada por Meltemi (no
existe en el daemon y crearla es cambiar la política), mostrar rutas o diffs
que el agente no envió, verbos para decidir por id desde un script
(`verbos-del-agent-boss`), avisar fuera del terminal (`aviso-de-espera-opt-in`),
las preguntas con texto libre del agente (`preguntas-del-agente`), y corregir
la regla que propone la bandeja (propuesta aparte, D5).

## Decisions

### D1 — Se pregunta solo donde un terminal puede contestar

El aprobador está activo cuando **stdin y stderr son terminales y el terminal
admite el modo crudo**, y eso se decide una sola vez, en `main.rs`: la prueba
entra y sale del modo crudo. En Windows las teclas se leen de la consola y no
de stdin, así que lo que prueba que la pregunta podrá contestarse es esa
entrada, no `is_terminal`. La biblioteca (`execute`, `dispatch`) deniega por
omisión, como hoy: ninguna prueba en proceso puede quedarse esperando una tecla.

**Un terminal no prueba que haya una persona.** Un script que asigna un
pseudoterminal (`script`, `expect`, `ssh -tt`) pasa la puerta; D7 detecta la
entrada que llega más rápido de lo que una persona lee y la deniega.

**Sin terminal**, toda petición se deniega con **la opción de rechazo del
agente**, o cancelando si no ofreció ninguna —la misma elección que la tecla de
denegar—, y el aviso dice que no hay terminal donde preguntar y nombra **las
reglas de permisos** (`.meltemi/permissions.toml`) como la manera de conceder
sin humano: la única que sirve para todo verbo. `--mode` no se nombra: solo
existe en `session`, ningún modo concede comandos, y con un modo puesto un
comando escala aunque una regla lo permita.

El log anota `decidedBy: client`, que es lo que ocurre: el cliente conectado
respondió. Distinguir la tecla de una persona de la política de un cliente sin
terminal es una procedencia nueva para todas las superficies; queda como
propuesta aparte.

### D2 — La pregunta muestra lo que el agente envió, y nada del agente manda en el terminal

En stderr, antes de leer la tecla:

```
permission requested by session <id>
  tool     <kind, o el título si no hay kind>
  title    <título, si añade algo>
  command  <rawInput.command, si viene>
  path     <rawInput.path o locations[0].path, si viene>
  content  <el texto de los bloques de contenido, línea a línea>
  1 <nombre de la opción>  (<clase>)
  2 …
  d deny   r allow and remember   Ctrl+C stop the session
```

- **Lo que se muestra es del agente**: los campos de la petición, completados
  con los de la última `tool_call` o `tool_call_update` que la sesión
  transmitió con el mismo `toolCallId`. Así Codex muestra el comando que envió
  antes y Claude su entrada entera. Nada se deduce ni se inventa; si nada dice
  qué haría, la pregunta lo dice.
- **El contenido** se muestra hasta 40 líneas; lo que se omite se cuenta.
  El comando nunca se recorta.
- **Ningún texto del agente llega crudo al terminal.** Los caracteres de
  control (C0, DEL, C1), los de reordenación bidireccional y los de formato
  invisibles se escriben escapados (`\u{…}`, `\n`): un título con
  `ESC[2K` y un retorno de carro podría redibujar la pregunta, y una consulta
  al terminal podría hacer que este «tecleara» una respuesta.
- Las opciones son **las del agente**, en su orden, con su nombre y su clase.
  **No hay tecla por omisión**: Enter no decide nada.

### D3 — Una tecla, y cada tecla hace lo que la bandeja haría

- **Un dígito** elige la opción del agente con ese número. Si su clase es
  «permitir siempre», el agente dejará de preguntar por peticiones parecidas
  en esta sesión, fuera del proxy: la elección pide una segunda tecla (`y`)
  que lo confirme, y lo dice.
- **`d`** (y Esc) deniega con la opción de rechazo del agente, o cancela si no
  ofreció ninguna: la misma regla que la bandeja, en una función compartida.
- **`r`** aprueba con la opción «permitir una vez» del agente y recuerda la
  decisión como regla (D5), tras una segunda tecla de confirmación (`y`); con
  cualquier otra tecla vuelve a la pregunta.
- **Ctrl+C** para la sesión (D11). En modo crudo llega como tecla: no hace
  falta manejador de señales.
- Cualquier otra tecla no hace nada.

### D4 — El push lleva el id de la cola

`PermissionRequestParams` gana `requestId`, aditivo: el mismo id que la entrada
de la cola. Es lo que permite al cliente que responde el push decidir por id y
persistir una regla (`permission/decide`), confirmar que su petición sigue en
pie (`permission/pending`) y reconocer en `permission/changed` que dejó de
estarlo. La TUI y la GUI lo reciben y hoy no lo usan: su conducta no cambia. Es
un campo del contrato de permisos (`permission-rules`).

- **Descartado: emparejar por sesión**, como hace la GUI. Una sesión puede
  tener dos peticiones seguidas; emparejar por sesión elige la equivocada.

### D5 — Recordar es la regla más específica posible, y solo cuando lo es

El requisito vivo pide proponer «la regla más específica posible». `r` propone
la que permiten **los hechos que el daemon lee del push** (nunca lo completado
desde la sesión, que es solo para mostrar):

- la herramienta con **el comando exacto** como prefijo, si el push trae
  comando;
- si no, la herramienta con **la ruta exacta** como prefijo, si trae ruta;
- o la sugerencia anti-fatiga del daemon cuando es al menos igual de específica
  (trae comando o ruta).

**No se ofrece** —y la pregunta no muestra `r`— cuando:

- la petición no nombra herramienta (sin clase ni título: la regla llevaría el
  marcador de la cola y no gobernaría nada);
- no trae ni comando ni ruta: una regla de herramienta sola, como `execute`,
  permitiría todo comando;
- la ruta está dentro de `.meltemi/`, donde viven las reglas: una regla ahí
  dejaría al agente escribir sus propios permisos;
- el agente no ofreció opción de rechazo (es una pregunta suya, no una
  aprobación) o no ofreció «permitir una vez» (aprobar con «permitir siempre»
  haría además que el agente recordara);
- el push no trae `requestId` (un daemon anterior);
- el comando trabaja en un worktree de tarea (`implement`, `dispatch`): la
  regla se escribiría en el worktree, ninguna sesión siguiente la cargaría y el
  commit de la tarea se la llevaría. Solo `session` y `propose`, que trabajan
  en la raíz del proyecto, ofrecen recordar.

La confirmación nombra el efecto, las condiciones, el ámbito y el archivo, y
dice que la regla rige **desde la siguiente sesión abierta sin `--mode`**: las
reglas se cargan al abrir la sesión, y bajo un modo los comandos escalan
aunque una regla los permita. Se persiste con `permission/decide {requestId,
optionId, persistRule}` y después se responde el push con la misma opción (la
primera resolución gana; la segunda recibe «ya resuelta» sin efecto).

- **La bandeja no cambia aquí.** La TUI propone, sin sugerencia, la herramienta
  sola —más amplia que el requisito vivo— y la GUI solo la sugerencia. Se
  anota como propuesta aparte; lo compartido entre el shell y el CLI es elegir
  la opción de permiso y la de rechazo.
- **Límites conocidos**: `permission/decide` no informa si la persistencia
  falló, y la creación de una regla no queda en ningún log de sesión. Se
  declaran y se dejan como propuesta aparte.
- **Con los adaptadores de hoy `r` casi no aparece**: Claude y Codex no mandan
  comando ni ruta en `rawInput`. Es lo buscado: una regla demasiado amplia es
  peor que ninguna, y el día que el adaptador mande los hechos, `r` aparece.

### D6 — Una pregunta a la vez, retirada cuando deja de estar en pie

Una sesión nunca tiene dos preguntas vivas; si un comando llevara varias
sesiones, los pushes esperan su turno en orden. Cuando le toca a una:

1. `permission/pending` **confirma que sigue en pie**. Si ya no está, no se
   pregunta y se dice que se resolvió en otra parte; si está vencida, que
   venció. Esa respuesta es posterior al registro de la petición, porque el
   push lo es.
2. **Mientras está abierta**, las instantáneas de `permission/changed` llegan
   en el orden en que el daemon las tomó. Visto el id una vez —en la
   confirmación o en una instantánea—, una instantánea posterior **sin** él
   significa que se resolvió en otra parte (otra superficie la decidió, o su
   sesión paró), y una que la muestra **vencida**, que venció. Una instantánea
   sin el id **antes** de haberlo visto no dice nada: puede ser anterior a la
   petición.
3. Con un daemon sin `requestId`, solo `permission/timeout` de la misma sesión
   y la misma llamada retira la pregunta.

Un push retirado se responde igual (con cancelación, sin efecto: ya hubo
primera resolución). El CLI no tiene temporizador propio: la espera es la del
daemon (`[permissions] wait`).

### D7 — Las teclas se leen en modo crudo, sin pantalla completa, y solo cuando la pregunta pudo leerse

Con crossterm, que ya llega por la reexportación de `ratatui`: modo crudo sin
pantalla alternativa, en un hilo bloqueante que sondea en tramos cortos para
que una retirada termine la lectura. La pregunta se escribe antes de entrar en
modo crudo y la respuesta después, desde el hilo del runtime, que es el que
tiene el candado de stderr.

- **Una espera de 400 ms** sigue a cada pregunta (y a cada confirmación) antes
  de aceptar teclas. Lo tecleado antes o durante esa espera se descarta: una
  tecla tecleada de antemano no contesta una pregunta que nadie vio. Si en tres
  esperas seguidas llegan teclas, la petición se deniega como desatendida: la
  entrada llega más rápido de lo que una persona lee. Ctrl+C sí se atiende
  aunque llegue durante la espera, porque solo deniega.
- **Solo cuenta la pulsación.** En Windows, una pulsación de una tecla que ya
  estaba abajo es una repetición y se ignora. En Unix se pide el pegado entre
  corchetes, y un pegado se ignora diciendo por qué.
- **La fuente de teclas es un trait** (entrar y salir del modo crudo, descartar
  lo pendiente, leer una tecla): las pruebas la programan, con teclas
  «tecleadas antes», esperas hasta la retirada y un recuento de entradas y
  salidas del modo crudo.

### D8 — Ninguna bandera nueva concede permisos

Esta change no añade `--yes`, `--allow` ni nada parecido: la constitución (§3)
no admite el atajo, y quien necesite conceder sin humano tiene reglas, que se
leen y se auditan. `--mode` sigue siendo la única postura por bandera, y nunca
levanta un deny del usuario ni lo irreversible. La prueba de la gramática fija
**el conjunto exacto** de banderas globales que acepta el analizador: añadir
cualquiera obliga a tocar esa prueba, a la vista de quien revise.

### D9 — stdout queda intacto

El aprobador escribe solo en stderr (una salida inyectable para las pruebas).
Con `--json`, stdout sigue llevando exactamente un objeto; una prueba lo
consume con el aprobador activo.

### D10 — El simulado aprende a pedir con nombre

`mock-agent --titled-permission` pone `kind: edit` y un título en su petición
de permiso. Con `PROPOSAL_PATH:` en la instrucción, la petición lleva además
esa ruta, fuera de `.meltemi/`: el camino de la regla se prueba de punta a
punta. Apagado por omisión: ninguna prueba existente cambia lo que lee.

### D11 — Ctrl+C para de verdad, y el registro lo dice

1. El aprobador envía `session/cancel` **primero**.
2. Espera —con cota, cinco segundos— a que el daemon retire la petición: la
   cancelación la saca de la cola y el log la anota `decidedBy: interrupted`,
   siempre, en vez de depender de qué llega antes.
3. Responde el push (cancelación, sin efecto).
4. No pregunta nada más: todo push posterior se deniega sin preguntar.

El comando sigue esperando la respuesta de su verbo —el turno termina con la
cancelación; un segundo Ctrl+C, ya fuera del modo crudo, termina el proceso
como siempre— y la muestra; el binario sale con `13`. Con `--json`, stdout
lleva un único objeto: la respuesta del verbo, cuyo estado dice cómo terminó el
turno.

### D12 — El terminal vuelve siempre a su estado

El aprobador no se aborta nunca a media pregunta. Cada verbo, al volver su
llamada, **avisa al manejador y lo espera** (con cota) antes de cerrar la
conexión: el manejador atiende lo que ya llegó —una retirada con su motivo—,
para la lectura, sale del modo crudo y termina. Después, `dispatch_with`
asienta el aprobador: si algún lector siguiera activo, sale del modo crudo
desde el hilo del runtime. Solo una muerte forzada deja el terminal en modo
crudo.

## Risks / Trade-offs

- **R1 — Una muerte forzada deja el terminal en modo crudo.** D12 cubre todo
  final ordenado, incluidos los que llegan con una pregunta abierta; un
  `kill -9` no se puede atender, como en cualquier TUI.
- **R2 — Sin terminal en CI.** La traducción de eventos, las esperas y todo el
  bucle se prueban con la fuente programada y un flujo en memoria; el camino
  con terminal real queda para una verificación manual del mantenedor.
- **R3 — Una regla pedida que no se escribe.** D5, límite conocido.
- **R4 — Un pseudoterminal pasa la puerta.** D1 lo declara; D7 lo acota.
- **R5 — `r` casi no aparece con los adaptadores de hoy.** D5: preferido a una
  regla más amplia de lo que se vio.

## Migration

Aditivo: un campo en el push que los clientes previos ignoran. En un terminal,
el CLI pasa de denegar a preguntar; sin terminal, deniega con el rechazo del
agente en vez de cancelar, y el aviso deja de prometer una release futura.

## Revisión adversarial (2026-09-25)

Tres revisores (corrección, seguridad, testabilidad) contra el código y la
constitución; cada hallazgo verificado por separado. Lo que cambió respecto del
plan revisado:

- D2 escapa todo texto del agente y completa la pregunta con la `tool_call` de
  la sesión (Claude no manda `rawInput`, Codex no manda el comando).
- D5 pasa de «la regla de la bandeja» (herramienta sola, más amplia que el
  requisito vivo) a la más específica posible, con los casos en que no se
  ofrece: `.meltemi/`, preguntas del agente, worktrees.
- D1 deja de nombrar `--mode` en el aviso y deniega con el rechazo del agente;
  declara que un terminal no prueba una persona.
- D3 pide confirmación para «permitir siempre»; D7 añade la espera, el
  descarte, las repeticiones y el pegado.
- D6 retira por `requestId` y solo tras haberlo visto (una instantánea puede
  ser anterior a la petición).
- D8 deja de afirmar que ninguna bandera concede: `--mode` concede en el árbol.
- D11 cancela antes de responder, para que el log diga `interrupted` siempre.
- D12 nace: los verbos abortaban el manejador y `process::exit` no corre
  destructores.

Descartados tras verificar: la cola de preguntas no reconciliadas (una sesión
no tiene dos peticiones vivas) y la falta de saneado (el núcleo ya lo hacía).
