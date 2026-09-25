# Design — verbos-del-agent-boss

## Context

Leído el código antes de diseñar, estos hechos fijan el diseño:

- **Los cinco métodos existen y no se mueven.** `permission/pending` devuelve la
  cola (`PendingPermission`: id, sesión, herramienta, título, opciones del
  agente, espera, vencimiento, `expired`); `permission/decide` resuelve por id
  con la regla de que la primera decisión gana y devuelve `applied` o
  `already_resolved`; `session/watch` declara interés en una sesión ajena y se
  responde en el propio bucle de la conexión (`server.rs:221`); `session/log`
  pagina el JSONL de a 200 líneas (`server.rs:366`) y exige la raíz del proyecto;
  `session/cancel` es una **notificación** sin respuesta.
- **`permission/decide` no valida la opción.** `PendingQueue::decide`
  (`pending.rs:194`) resuelve con `Selected { option_id }` sea cual sea el id;
  la bandeja de la TUI y la GUI nunca envían otra cosa que las opciones del
  agente porque solo pintan esas. Un verbo que acepta texto tecleado tiene que
  hacer esa comprobación él mismo.
- **La cola no lleva el comando ni las rutas.** `PendingPermission` trae la
  herramienta y el título del agente; el comando y las rutas viven en el
  `permission_requested` del registro de la sesión. Mostrarlos en la cola sería
  mover el contrato.
- **El registro se publica antes de desregistrar.** `finalize_*` escribe
  `session_ended` —y el log lo publica al hub— y **después** saca la sesión del
  registro (`session_finalize.rs:103, 141`). Hay una ventana en la que `status`
  todavía la lista viva y su cierre ya pasó por el stream.
- **El stream es acotado y calla.** El hub guarda 1024 eventos
  (`events.rs:25`); una conexión que se atrasa pierde eventos y el daemon solo
  lo anota en su propio log (`server.rs:171`): el cliente no se entera. La
  fuente completa es `session/log`.
- **El bucle de la conexión ordena.** Una notificación se despacha con `await`
  dentro del bucle (`server.rs:234`) antes de leer el siguiente mensaje; las
  peticiones se atienden en tareas aparte. Un `status` enviado después de un
  `session/cancel` por la misma conexión observa la cancelación ya aplicada.
- **`session/list` sin raíz recorre todos los proyectos** del directorio de
  datos y combina el estado vivo del registro: un id se localiza sin pedirle al
  usuario la raíz.
- **El empuje de un permiso va a la conexión que abrió la sesión**
  (`spawn_live_push`); una conexión que solo mira no recibe peticiones. Toda
  conexión inicializada cuenta como cliente, y con la política `while-connected`
  (la de omisión) una petición espera mientras haya uno.
- **La CLI tiene todo lo demás.** El aprobador (`approver.rs`) ya sabe elegir la
  opción de rechazo del agente (`reject_option`), la de aprobación de una vez
  (`allow_once_option`) y sanear texto del agente para un terminal (`clean`); el
  puente (`run::bridge`) ya escribe por el guardia de stdout que tiene `main`,
  que es lo que un flujo necesita. El código `12` de la taxonomía está definido
  y nadie lo usa todavía.

## Goals / Non-Goals

**Goals**

- Aprobar, denegar, listar, seguir, leer y cancelar desde cualquier shell —un
  script, un agente, `ssh <pc>` sin terminal—, sobre los métodos que ya existen.
- Que decidir desde la CLI no pueda ser más laxo que decidir en la bandeja: solo
  opciones que el agente ofreció, una petición por vez.
- Que `watch` no se cuelgue nunca: ni con la sesión terminada, ni con el daemon
  muerto.

**Non-Goals**

- Un servidor MCP de Meltemi, o verbos para `sdd/gate`, `sdd/review-decide` o
  `worktree/merge-file` (fuera de alcance en la propuesta).
- Mover el contrato: ni RPC nuevo, ni campo nuevo en la cola.
- Leer eventos perdidos por atraso del stream: eso es `log`.

## Decisions

### D1 — Seis verbos sobre cinco métodos que ya existen

`permissions` → `permission/pending`; `approve` y `deny` → `permission/pending`
y luego `permission/decide`; `watch` → `session/watch` (con `session/list` y
`status` para saber si hay algo que seguir); `log` → `session/list` para
localizar la sesión y `session/log` paginado; `cancel` → `session/list`, la
notificación `session/cancel` y `status`. Ninguno pide la raíz del proyecto: la
sesión se localiza por su id en el listado del daemon, que recorre todos los
proyectos. El contrato no se mueve y no nace deber de paridad: la TUI y la GUI
ya tienen casa para los cinco métodos.

### D2 — Se decide con lo que el agente ofreció para esa petición

`approve` y `deny` leen la cola antes de decidir y toman las opciones de la
petición tal como el agente las envió:

- **`approve <id>`** elige la aprobación de una sola vez **cuando el agente
  ofreció exactamente una**. Con varias (una pregunta del agente con dos
  caminos, por ejemplo) rehúsa listándolas: elegir la primera sería adivinar,
  y el terminal tampoco lo hace —pregunta por número—. Sin ninguna, rehúsa
  igual. Una aprobación permanente (`allow_always`) no se elige nunca por
  omisión.
- **`deny <id>`** elige el rechazo del agente con la misma función que la
  bandeja y el terminal (`reject_option`), y si el agente no ofreció rechazo,
  cancela: exactamente lo que hace denegar en las otras superficies.
- **`--option <id>`** nombra una opción concreta —nombrarla es consentir, como
  `--branch`—, y solo de la clase del verbo: `approve` una aprobación, `deny` un
  rechazo. Una opción que el agente no ofreció, o de la clase contraria, se
  rehúsa listando las del agente, y no se decide nada.

Como el daemon no valida la opción (Context), la CLI jamás envía un id que no
esté en la lista de esa petición.

### D3 — Una decisión por id, nunca en bloque

`approve` y `deny` aceptan exactamente un id. No hay `--all`, ni lista, ni
comodín: dos ids son un error de uso cuyo mensaje dice cómo se concede sin que
pregunten —las reglas de permisos o el modo de la sesión—, y un comodín no es
más que un id que la cola no tiene. Un script que aprueba en bucle cada
petición que aparece está escribiendo una regla a mano, y la regla es el lugar
donde eso queda escrito, auditado y con alcance.

### D4 — Los códigos de salida, sin ampliar la taxonomía

- **`12`** cuando la decisión no se aplica porque la petición ya no espera: la
  resolvió otra superficie, venció, su sesión paró, no existió nunca, o el
  daemon respondió `already_resolved` porque otra decisión llegó un instante
  antes. Es la regla del proxy —la primera decisión gana— rechazando la tardía,
  y es el primer uso del código que la taxonomía reservó para el proxy de
  permisos. `approve x && …` no puede seguir como si hubiera aprobado.
- **`11`** cuando el daemon no conoce la sesión que se nombra: el mismo hecho
  que `direct` recibe del daemon como `session_not_found` (2004) sale con el
  mismo código aunque aquí lo detecte el cliente en la respuesta del listado.
- **`2`** para dos ids, una opción ausente o de la clase contraria, o varias
  aprobaciones sin `--option`: la línea de órdenes no alcanza para decidir.
- **`10`** cuando el daemon cierra la conexión durante `watch`.
- **`0`** para `watch` y `cancel` de una sesión que ya no corre: el estado que
  se buscaba —que termine, que no siga— ya se cumple, y un script que hace
  `direct` y luego `watch` no puede fallar porque el turno fue rápido.

### D5 — `watch` y `log` son flujos: un registro por línea (dos MODIFIED)

La salida útil de ambos es el registro de la sesión, que ya es una sucesión de
objetos. Emiten **JSON Lines** —un registro por línea— en todo formato: en modo
humano porque «el registro tal cual» es ese (la propuesta), y bajo `--json`
porque un flujo en vivo no puede ser un objeto único sin dejar de ser en vivo.
Bajo `--yaml`, un documento YAML por registro (`---`), que es la forma estándar
de un flujo en YAML. Es una excepción a dos reglas vigentes de `cli-contract` —un
objeto bajo `--json`, un documento bajo `--yaml`—, y se escribe en esas reglas
como MODIFIED en vez de dejarla implícita. Un error antes del primer registro
sigue las reglas de siempre; un error a mitad del flujo, en formato de máquina,
cierra el flujo con un objeto de error con su código, y en modo humano va a
stderr.

### D6 — `watch` no sobrevive a su sesión ni a su daemon

1. Declara el interés (`session/watch`) **antes** de mirar si la sesión corre:
   al revés, un cierre entre la consulta y la declaración se perdería.
2. Localiza la sesión (`session/list` y `status`): desconocida, `11`; conocida y
   no viva, termina con `0` y una nota en stderr que remite a `log`.
3. Emite cada registro de esa sesión a medida que llega, y termina tras emitir
   `session_ended`.
4. Cada 2 s consulta `status`: si la sesión ya no está viva y su cierre no llegó
   —la ventana entre publicar y desregistrar, o un cierre perdido por atraso—,
   termina con `0` remitiendo a `log`. Consultar el registro local cada dos
   segundos cuesta nada; colgarse para siempre cuesta la confianza en el verbo.
5. Si la conexión se cierra, termina de inmediato con `10`.

`Ctrl+C` (o matar el proceso) solo deja de seguir: la conexión se cae, el
daemon olvida el interés y la sesión sigue corriendo. Es distinto, a propósito,
de `Ctrl+C` ante una pregunta del aprobador, que cancela: mirar no es gobernar.
No hace falta manejar la señal —nada que restaurar, ningún modo crudo—, y por
eso no se añade la feature `signal` de tokio.

### D7 — Lo que emite `watch` es lo que guarda `log`

Cada línea de `watch` es el **registro** (`{v, ts, type, payload}`), no el
sobre de la notificación: la sesión ya se nombró al invocar, y así las líneas de
`watch` son las mismas que `log` mostrará después, con el mismo contenido y en
el mismo orden. No son idénticas byte a byte —la CLI vuelve a serializar un
valor y el orden de las claves puede diferir—; la prueba compara valores.

### D8 — El texto del agente, sin poder sobre el terminal

En las líneas de `watch` y `log`, todo carácter que un terminal obedecería
—controles C0 y C1, DEL, los de dirección y los invisibles de formato— se
escribe como escape de JSON (`‮`). Dentro de una línea JSON esos
caracteres solo pueden estar en una cadena, así que la línea sigue siendo JSON
válido y leída como JSON conserva **el mismo valor**: `jq` ve lo que el agente
escribió y el terminal no ve nada que obedecer. El listado de `permissions`
usa `approver::clean`, como el aprobador.

### D9 — `cancel` es la `x` de la TUI, con respuesta

Localiza la sesión; si no corre, responde que no corre (`0`). Si corre, envía la
notificación y a continuación, por la misma conexión, `status`: el bucle de la
conexión despacha la notificación antes de leer la petición (Context), así que
la respuesta dice si la sesión seguía viva —el agente cierra su turno— o ya se
había ido. No espera el drenaje: seguirlo es `watch`, y `cancel X && watch X`
compone las dos cosas.

### D10 — `permissions` muestra lo que la cola lleva

Encabezado con los totales («N esperan una decisión · M vencidas»), columnas con
anchos del contenido (id, espera, vencimiento, sesión, herramienta, título) y,
bajo cada petición, sus opciones con el id que `--option` toma. El comando y
las rutas no están en la cola (Context): el pie remite a `meltemi log
<sesión>`, donde el `permission_requested` trae la llamada completa. La
propuesta prometía rutas en el listado; se corrige en vez de mover el contrato.

### D11 — Por SSH, sin terminal y sin túnel

Ninguno de los seis verbos lee el teclado, pregunta ni exige tamaño de
pantalla. `ssh <pc> meltemi permissions` y `ssh <pc> meltemi approve <id>`
corren **en la máquina del daemon** por el SSH del propio usuario: no hace falta
túnel ni puente, y funciona igual contra el named pipe de Windows porque la
orden lo alcanza localmente. La conexión de un `watch` cuenta como cliente, así
que bajo `while-connected` una petición sigue esperando mientras alguien mira
—igual que con la GUI abierta—, y eso es lo que da tiempo a aprobar desde el
teléfono. La receta entra en `docs/acceso-remoto.md`.

### D12 — Pruebas

- **Unitarias**: la gramática (seis verbos, `--option` solo en `approve`/`deny`,
  un id), la elección de opción, el listado, el escape de las líneas y el
  escritor por formato.
- **E2e contra el daemon en proceso y el simulado**: una sesión abierta por un
  cliente crudo que nunca responde el empuje —la bandeja de otra superficie
  abierta—, decidida desde la CLI; una sesión `detach` seguida por el binario
  real sin TTY de principio a fin (pregunta, aprobación, turno, `cancel`,
  cierre), comparada con `log`.
- **E2e con la prueba como daemon** (el patrón del puente): el daemon que muere
  a mitad del flujo y la sesión que desaparece sin cierre.
- El pin de `FLAGS` gana `--option`; la referencia CLI se regenera.

### D13 — La vía

Las dos reglas de formato cambian (D5): dos MODIFIED sacan a la change de la vía
rápida. La propuesta se corrige en eso, en el número de verbos (seis sobre
cinco métodos), en `deny --option` y en las rutas del listado (D10).

## Risks / Trade-offs

- **R1 — Eventos perdidos sin aviso.** Si una conexión se atrasa más de 1024
  eventos, el daemon los descarta y el cliente no se entera: `watch` es el en
  vivo y `log` la fuente completa, y así lo dice su ayuda. Avisar del atraso
  exigiría un evento nuevo en el contrato.
- **R2 — Un `watch` olvidado mantiene esperando.** Bajo `while-connected`, una
  petición espera mientras haya un cliente, y un `watch` en un `tmux` olvidado lo
  es —como una GUI abierta—. Es la misma regla para toda superficie.
- **R3 — La consulta de vida.** Un `status` cada 2 s por cada `watch` abierto:
  local y trivial frente a quedar colgado.
- **R4 — JSON en modo humano.** `watch` y `log` imprimen registros también sin
  `--json`: es el registro tal cual, y un humano que quiere leerlo cómodo tiene
  la TUI y la GUI.

## Migration

Aditivo: seis verbos y el flag `--option` entran a la gramática y a su pin; el
código `12` tiene su primer uso, con la descripción de la taxonomía al día. El
contrato no cambia; un daemon de la misma versión sirve todo.
