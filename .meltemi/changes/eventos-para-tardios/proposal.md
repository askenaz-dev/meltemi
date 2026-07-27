# eventos-para-tardios

> Tercero y último de los prerrequisitos de daemon del Agent Boss
> (`enmienda-agent-boss`, meltemi.md v1.4), tras `espera-humana` y
> `sesion-esperando`. Vía rápida con design completo; gate del mantenedor
> dado el 2026-07-26.

## Why

Las dos changes anteriores consiguieron que una petición espere al humano y
que las superficies puedan ver **qué** espera. Falta lo que convierte al
teléfono en un puesto de trabajo y no en una bandeja de avisos: **ver al
agente trabajar**.

Hoy no se puede si no fuiste tú quien lanzó la sesión. `session/event` se
emite contra el `Peer` capturado cuando la sesión arrancó (`acp.rs`), es
decir contra **una sola conexión**: la que invocó el verbo. Un cliente que
conecta después — el caso literal del Agent Boss: dejaste la sesión corriendo
en el escritorio y abres el móvil desde la calle — no recibe una sola
actualización. Su único recurso es sondear `session/log` en bucle: una
llamada completa por cada vistazo, sin push, sobre un túnel móvil. La cola de
permisos ya resolvió esto para su dominio (vive en el daemon, la ve cualquier
cliente); el transcript de la sesión sigue atado a un socket.

Hay un segundo efecto silencioso: como el push va a la conexión iniciadora,
si esa conexión muere el turno sigue corriendo y **nadie** ve sus eventos
hasta que termine, aunque haya tres clientes conectados mirando.

## What Changes

- **El evento deja de escribirse contra una conexión y pasa a publicarse.**
  Un hub de eventos en el daemon — la misma forma que la cola de permisos ya
  usa para sus cambios — recibe cada `session/event` y lo reparte. El
  fan-out es una sola ruta, así que nadie recibe nada dos veces.
- **Cada conexión declara qué sesiones mira**, con un método nuevo
  `session/watch` (`{sessionId, watch}` → `{sessionId, watching}`): un
  cliente que abre el detalle de una sesión que no inició pide el stream y
  lo recibe en vivo; al cerrarlo, deja de pedirlo. Sobre un túnel móvil eso
  importa: el teléfono no paga el tráfico de sesiones que nadie está
  mirando.
- **La conexión que inició la sesión sigue recibiendo su stream sin pedir
  nada** — mismo comportamiento observable que hoy. Se consigue sellando
  cada evento con la conexión que lo originó: el hub entrega a esa conexión
  siempre, y a las demás solo si miran esa sesión.
- **Paridad ×3**: el método entra en el registro de la paleta de la TUI, en
  el registro de métodos de la GUI y en la matriz `docs/paridad-nucleo.md`
  (el gate de CI lo exige), y la GUI lo usa de verdad: al abrir el detalle
  de una sesión pide el stream y lo suelta al salir.
- **La TUI deja de mezclar transcripts**: hoy pinta cualquier
  `session/event` que llegue, sin mirar de qué sesión es — inofensivo
  mientras solo llegaban los propios, un error en cuanto llegan de varias.
  Pasa a filtrar por la sesión que está mostrando.

## Capabilities

### Modified Capabilities

- `acp-session`: ~ «Prompt con streaming de actualizaciones» (el stream deja
  de ser exclusivo de la conexión iniciadora: cualquier cliente puede
  suscribirse a una sesión en curso).

## Impact

- `proto/`: método `session/watch` + sus tipos y esquema; conformance.
- `core/meltemi-client`: `Peer` gana un identificador de conexión (contador
  monótono en proceso) para que el hub distinga origen de destino.
- `core/meltemid`: `events.rs` nuevo (hub), `acp.rs` (publica en vez de
  notificar), `server.rs` (conjunto de sesiones miradas por conexión, brazo
  de fan-out en el bucle y el handler del método nuevo).
- Superficies: registro de la paleta TUI, registro GUI, matriz de paridad;
  filtro por sesión en la TUI; suscripción real en el detalle de sesión de
  la GUI.
- Tests: unit del hub (entrega al origen sin suscripción, a un tercero solo
  si mira, a nadie más), y e2e donde un segundo cliente conecta a mitad de
  turno, pide el stream y recibe eventos que él no provocó.
- Sin dependencias nuevas.

## Fuera de alcance

- **Formas asíncronas de los RPC que bloquean el turno entero**
  (`sdd/gate`, `sdd/review-decide`, `worktree/dispatch`): la auditoría los
  nombró junto al stream, pero cambiarlos toca una promesa ratificada — los
  gates son «pasos scriptables» que reportan el gate pendiente en su propia
  respuesta (`sdd-authoring`), y volverlos asíncronos cambia el contrato de
  toda superficie y de los scripts. Además el stream de esta change los
  mitiga: quien pierde la respuesta por un corte reconecta, mira los eventos
  y consulta `change/list`, que desde `sesion-esperando` ya declara el gate.
  Entra como change propia si la evidencia de uso lo pide, con su prueba
  escrita.
- Reproducción del histórico al suscribirse (el «replay» de lo ya emitido):
  `session/log` ya lo da paginado y con offset; duplicarlo en el stream
  sería una segunda fuente del mismo dato.
- Persistencia o encolado de eventos para clientes desconectados: la
  frontera de `remote-access` es explícita — sin conexión no hay
  notificación.
