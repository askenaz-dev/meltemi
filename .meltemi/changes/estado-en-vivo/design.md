# Design — estado-en-vivo

## Context

El 2026-09-24, con un agente cuarenta segundos dentro de un turno de 175 s
lanzado desde otro cliente, la GUI conectada al **mismo daemon** mostraba la
sesión como detenida y la barra decía «0 working». El daemon nunca se
equivocó: sabía el estado exacto en cada instante. Lo que falta no es
información, es **a quién se le cuenta**.

La causa está medida, no supuesta, y es una sola línea de arquitectura: en la
GUI todo lo que significa «esta sesión está viva» deriva del array
`$sessions`, y ese array solo se rellena cuando alguien llama a
`refreshSessions()` — al conectar, al cambiar de proyecto y al enviar
(`desktop/ui/src/lib/stores.ts:254-266`). Los cuatro indicadores cuelgan de
ahí:

| Indicador | Dónde | Cómo se calcula |
| --- | --- | --- |
| Contador «N working» | `components/StatusBar.svelte:28` | `state === "active" \|\| "starting"` |
| Cubeta de la barra lateral | `components/Sidebar.svelte:271` | `isLive(session.state)` |
| Cubetas del árbol | `lib/tree.ts:178` | `isLive(session.state)` |
| Anillo del detalle | `views/SessionDetail.svelte:12` | `isLive(session.state)` |

Eso tiene una consecuencia afortunada para esta change: **no hay cuatro
arreglos, hay uno**. Si `$sessions` se mantiene vivo, los cuatro siguen. Y la
barra lateral sigue cambiando en seco, porque ninguna de sus guardias se
toca.

La TUI tiene el mismo problema por el otro camino: `refresh_sessions`
(`tui/src/shell/conn.rs:436-445`) pregunta `session/list` entera con el
ticker, y empuja `Update::Sessions(rows)` con el resultado.

Tres cosas que **ya existen** y que esta change usa en vez de reinventar:

- El **hub de eventos** de `eventos-para-tardios`, con `session/watch`: el
  daemon ya sabe empujar a una conexión que llegó tarde.
- El módulo **`follow`** de la TUI (`tui/src/follow.rs`), que ya define qué es
  un verbo en streaming en este proyecto: un registro por línea en todo
  formato, un error a medias cierra el flujo con su objeto de error, y el
  flujo no sobrevive ni a su sesión ni a su daemon.
- El conteo de permisos en espera, que el registro de sesiones ya lleva por
  sesión (`begin_waiting` cuenta, y la primera espera fija
  `waiting_permission`).

Lo que **no** existe y hay que añadir: un `since` por transición. El
`SessionInfo` del contrato (`proto/meltemi-proto/src/lib.rs:466-501`) tiene
`started_at` y `ended_at`, pero nada que diga desde cuándo la sesión está en
el estado en que está. Sin eso, un cronómetro de `senales-de-vida` solo puede
mentir o quedarse en blanco.

## Goals / Non-Goals

**Goals**

1. Que toda conexión iniciada reciba cada transición de estado de cualquier
   sesión, sin preguntar.
2. Que una conexión que llega tarde sepa el estado **actual** antes de recibir
   su primera transición, sin carrera.
3. Que el sondeo quede solo como reconciliación al reconectar, en las dos
   superficies.
4. Que el terminal scriptable consuma lo mismo (§4), con las convenciones de
   streaming que este proyecto ya fijó.
5. Que la notificación no lleve **nada** del contenido del proyecto ni del
   prompt.

**Non-Goals**

- El progreso **dentro** del turno (qué herramienta corre ahora): viaja por
  `session/watch` y lo pinta `herramientas-a-la-vista`.
- Agregar estados de varias máquinas: un daemon por máquina
  (constitución §3). `maquinas-remotas` lista máquinas, no fusiona estados.
- Cambiar qué significa «viva»: `LIVE_STATE`
  (`desktop/ui/src/lib/session-state.ts:18`) y sus seis estados se quedan
  exactamente como están.

## Decisions

### D1 — La notificación afirma el estado entero, no un delta

`session/state` lleva el estado **completo** de una sesión: `sessionId`,
`projectRoot`, `state`, `since` y `pendingPermissions`. No lleva «pasó de X a
Y», ni número de secuencia.

Esa elección resuelve sola el problema más incómodo de esta change. Un delta
obliga a que el cliente tenga el estado anterior correcto, y por tanto a
ordenar, numerar y detectar huecos. Una afirmación completa es **idempotente**:
aplicarla dos veces da lo mismo, y aplicar dos afirmaciones en cualquier orden
converge a la última que llegó. Sin secuencias, sin huecos que reparar, y una
carrera entre la foto inicial y la primera transición deja de ser una carrera
(D3).

El precio, dicho: cada notificación es un poco más grande que un delta. Son
cinco campos escalares sobre un socket local.

Y no es un patrón nuevo en esta casa: `permission/changed` ya funciona así.
Su carga es **la cola entera**, no el cambio (`PermissionChangedParams`,
`proto/meltemi-proto/src/lib.rs:2239-2246`), y su comentario dice por qué —
«so every client reconciles to one snapshot without a round-trip». Esta change
aplica la misma idea a la unidad que corresponde: una sesión.

### D2 — Un canal de estado en el hub que ya existe, y sin la regla de `delivers`

El hub de `eventos-para-tardios` es un `broadcast::Sender<Envelope>` con
capacidad 1024 (`core/meltemid/src/events.rs:24,38`). Gana un **segundo canal**
para el estado, en el mismo componente, con su propio sobre.

Lo que **no** hereda es el filtro. `delivers`
(`core/meltemid/src/events.rs:70-76`) entrega los eventos de la sesión propia y
los de otra solo mientras se la vigila. Para el estado eso sería reproducir el
defecto: «0 working» estaba mal precisamente porque la sesión la había lanzado
**otro** cliente. Así que toda conexión iniciada recibe toda transición, sin
origen y sin vigilancia.

Eso no expone nada nuevo. `session/list` ya devuelve a cualquier cliente todas
las sesiones con su raíz, sin filtrar y a propósito
(`tui/src/shell/conn.rs:437-439`, `desktop/ui/src/lib/stores.ts:260-266`), y
todos los clientes son el mismo usuario en la misma máquina sobre un socket con
permisos exclusivos (§3). La notificación no lleva instrucción, ni título, ni
contenido del proyecto: lo que ya era legible sigue siéndolo, y nada más.

El registro de clientes (`core/meltemid/src/clients.rs`) **no** se toca: solo
cuenta conexiones y no guarda emisores. El canal del hub es el fan-out, y cada
conexión sostiene su propio receptor.

### D3 — La foto primero, y la carrera deja de existir

`broadcast::subscribe()` solo entrega lo que venga después. Así que una
conexión que inicia recibe, en este orden: la **suscripción**, y después una
afirmación por cada sesión que el registro conoce.

Suscribir antes de fotografiar es deliberado. Al revés, una transición ocurrida
entre la foto y la suscripción se perdería. En este orden esa transición llega
**después** de la foto y la corrige, y por D1 aplicarla es idempotente. No hace
falta ni un número de secuencia ni un cerrojo sobre el registro mientras se
empuja.

### D4 — El rezago lo ve el servidor, así que lo repara el servidor

`broadcast` tiene capacidad finita: un receptor lento recibe
`RecvError::Lagged(n)` y pierde `n` mensajes. La corrección importante es
**quién se entera**: el `Err` lo recibe el brazo del `select!` de la conexión,
que corre en el daemon. El cliente no se entera de nada.

Así es hoy el canal de eventos, y es su punto débil: `server.rs:171-174`
registra un `tracing::warn!` en el servidor y sigue, y el cliente nunca sabe
que perdió eventos. Para el estado eso sería inaceptable: un hueco silencioso
es exactamente el defecto que esta change viene a cerrar.

Entonces, al recibir `Lagged`, la conexión **vuelve a empujar la foto entera**:
una afirmación por cada sesión conocida, con la misma función que usa al
iniciar (D3). El cliente no tiene que hacer nada, ni enterarse: recibe estado
válido y, por D1, aplicarlo converge.

Es el patrón de `permission/changed` al pie de la letra: allí `Ok(())` y
`Err(RecvError::Lagged(_))` comparten brazo y los dos responden con una foto
fresca (`server.rs:138-156`). Aquí la foto es por sesión en vez de por cola, y
el brazo del rezago la repite para todas.

Consecuencia agradable: el peor caso de una tormenta es una ráfaga de más, no
un estado falso, y el sondeo de las superficies queda libre para lo único que
debía hacer — reconciliar al reconectar.

### D5 — Sin coalescencia hasta que algo la pida, y por una razón

La propuesta señala el riesgo de tormenta con N sesiones y M clientes. El
diseño **no** añade coalescencia por sesión, y lo justifica en vez de
omitirlo: esto notifica **transiciones de ciclo de vida**, no eventos de turno.
Una sesión transita un puñado de veces por turno —arranca, activa, espera
permiso, vuelve, termina—, no una vez por token; los eventos de token ya viajan
por `session/watch` y ese canal no cambia.

La contención existe y es la de D4: capacidad acotada más «rezagarse significa
reconciliar». Añadir coalescencia antes de medir sería optimizar contra una
cifra que nadie tiene. Si la medición con el agente simulado la pide, entra con
su número.

### D6 — `since` es un instante de reloj, no un `Instant`

El registro ya tiene `idle_since: Option<Instant>`
(`core/meltemid/src/session.rs:312`), y no sirve: es monótono, no se serializa
como fecha y solo cubre la inactividad. `since` es RFC 3339 en UTC, como
`started_at` del contrato (`proto/meltemi-proto/src/lib.rs:481`), y se fija en
**cada** mutación de estado.

Sin él, un cronómetro de `senales-de-vida` solo puede quedarse en blanco o
mentir. Con él, «trabajando desde hace 40 s» es una resta que cualquier
superficie hace sola.

### D7 — Una tienda viva arregla los cuatro indicadores, y ningún componente se toca

En la GUI la notificación entra como un caso más del `switch` que ya despacha
los empujones del daemon (`desktop/ui/src/lib/stores.ts:475`, junto a
`permission/changed` y `permission/request`). Su cuerpo parchea `allSessions`
por `sessionId` y vuelve a estrechar `sessions` con `scopedTo`.

Eso es todo. El contador «N working» (`components/StatusBar.svelte:28`), la
cubeta de la barra lateral (`components/Sidebar.svelte:271`), las del árbol
(`lib/tree.ts:178`) y el anillo del detalle derivan de ese array, así que los
cuatro pasan a ser ciertos sin editar ninguno de los cuatro. Y por eso las
guardias de `desktop/tests/scenarios_shell.rs` siguen intactas: la barra
lateral sigue cambiando en seco.

En la TUI, `refresh_sessions` (`tui/src/shell/conn.rs:436-445`) deja de ser la
fuente y pasa a ser la reconciliación de D4; la transición llega como un
`Update` propio que parchea la fila por su id. El ticker de 2 s se queda para
los relojes, que es lo que siempre debió hacer.

### D8 — El verbo de terminal sigue el precedente, no inventa uno

`sessions --follow` es una bandera sobre el verbo `sessions` que ya existe
(`tui/src/cli.rs:850`), no un verbo nuevo. Y se implementa con las convenciones
que `follow` ya fijó para `watch` y `log` (`tui/src/follow.rs:1-31`): un
registro por línea en todo formato, un documento YAML por registro bajo
`--yaml`, un error a medias cierra el flujo con su objeto de error, y el flujo
no sobrevive a su daemon.

Una diferencia declarada: `watch` no sobrevive a **su sesión**, porque vigila
una. `sessions --follow` no tiene una sesión a la que sobrevivir, así que
termina cuando el daemon se va o cuando el usuario corta.

### D9 — Lo que los gates exigen de una notificación, dicho antes de escribirla

Una notificación del daemon al cliente no se registra como un método
invocable, y el gate de paridad lo sabe. `tui/tests/parity.rs` compara el
contrato contra la paleta de la TUI y el registro de la GUI **en las dos
direcciones**, y excluye el tráfico iniciado por el daemon con una lista
explícita (`DAEMON_INITIATED`, `tui/tests/parity.rs:19-24`). Así que
`session/state`:

1. Añade su constante al módulo `methods` del contrato, en **una** línea con
   la forma `= "session/state"` — el gate parsea texto, no Rust.
2. Se añade a `DAEMON_INITIATED`. Sin eso, los dos primeros tests fallan por
   «método del contrato sin casa en la paleta».
3. **No** entra en la paleta de la TUI ni en el registro de la GUI. Si entrara,
   fallarían las aserciones de la dirección contraria («declara métodos fuera
   del contrato»).
4. Gana su fila en `docs/paridad-nucleo.md`, en la tabla de tráfico iniciado
   por el daemon, con el método entre acentos graves: el test de la matriz
   recorre el contrato **sin** exclusiones.

`sessions --follow` no es un método nuevo, es una bandera sobre `sessions`, así
que no toca ninguno de los cuatro puntos.

La conformidad es un gate aparte: el esquema vive como entrada de `$defs` en un
archivo de `proto/schemas/v1/` —los de notificación no tienen archivo propio,
viven en el de su dominio— y se ata con un `assert_conforms` en
`proto/meltemi-proto/tests/conformance.rs`.

## Risks / Trade-offs

- **Tormenta de notificaciones.** Acotada por D4 y D5, y medida con el agente
  simulado antes de cerrar: varias sesiones a la vez contra varios clientes,
  comprobando que nadie se rezaga en uso normal y que quien se rezaga
  reconcilia.
- **La foto inicial con muchas sesiones.** Un registro con cientos de sesiones
  produce una ráfaga al conectar. El registro solo guarda lo que conoce en
  memoria, no el histórico en disco, así que la ráfaga es del tamaño de lo vivo
  más lo recién terminado. Se mide; si estorba, la foto se acota a lo vivo y se
  dice.
- **Dos fuentes para lo mismo.** Mientras el sondeo siga existiendo como
  reconciliación, hay dos caminos por los que un estado puede llegar. Por D1
  convergen, y ese es justamente el motivo por el que la notificación afirma el
  estado entero.
- **Un estado nuevo en el contrato.** `LIVE_STATE` es un `Record` sobre la
  unión a propósito: añadir un estado rompe la compilación una vez en vez de
  desincronizar cuatro listas. Esta change no añade estados, y esa propiedad se
  conserva.

## Migration Plan

Aditivo entero. La notificación es nueva, así que ningún cliente anterior la
espera y ninguno se rompe por recibirla; un cliente que la ignore se queda
exactamente con el comportamiento de hoy, sondeo incluido. No hay formato
persistido que migrar: el estado vive en memoria y el log de sesión no cambia.

## Open Questions

- Si la medición de la ráfaga inicial pide acotar la foto a las sesiones vivas,
  ¿queda dicho en la spec o basta el design? Se resuelve con el número en la
  mano, no antes.
