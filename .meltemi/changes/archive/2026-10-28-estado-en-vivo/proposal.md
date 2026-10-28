# estado-en-vivo

> Vía completa (proposal → design → specs → tasks): una notificación nueva
> del contrato, consumida por las dos superficies (§4), y el fin del sondeo
> en la TUI. Es la primera de la Tanda 2 porque sin ella toda señal de
> «trabajando» miente. Nace del hallazgo abierto de `sesiones-en-la-barra`
> y de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha VIS-1).

## Why

El 2026-09-24, con Claude Code cuarenta segundos dentro de un `propose` de
175 s lanzado desde otro cliente, la GUI conectada al mismo daemon mostraba
la sesión bajo **«Stopped»** y la barra de estado decía **«0 working»**. No
es un bug nuevo: el smoke de `sesiones-en-la-barra`
(`docs/qa/2026-09-16-sesiones-en-la-barra-smoke.md:96-113`) nunca vio la
cubeta «Trabajando» y lo dejó anotado como anterior a esa change.

La causa está en `desktop/ui/src/lib/stores.ts:258-262`: todo lo que dice
«activa» —la cubeta de la barra lateral, el glifo de la pestaña, el anillo de
`SessionDetail`, el contador de la barra de estado— deriva de
`refreshSessions()`, que solo corre al conectar, al cambiar de proyecto o al
enviar. Solo el anillo del Home usa una bandera local `running`. La TUI, por
su lado, sondea `session/list` con su ticker de 2 s. El daemon **sabe** el
estado de cada sesión en cada instante (`waiting_permission` contado,
`gatePending`, el hub de `eventos-para-tardios` con `session/watch`), pero
solo lo cuenta a quien pregunta por una sesión concreta.

`senales-de-vida` va a poner cronómetros y pulsos donde hoy hay texto; si la
fuente sigue siendo un sondeo ocasional, esos pulsos dirán «trabajando»
cuando no, y «detenida» cuando sí. Orca tiene «agent state heartbeats» en su
changelog por la misma razón.

## What Changes

- **Notificación `session/state`** en el contrato (`proto/schemas/v1/`,
  aditiva): el daemon la emite a **toda conexión iniciada** en cada
  transición de estado de cualquier sesión —creada, activa, esperando
  permiso, esperando instrucción, terminada, error— con `sessionId`,
  `projectRoot`, `state`, `since` y el conteo de permisos pendientes. Es
  información que `session/list` ya da; solo cambia quién pregunta. Ningún
  contenido del proyecto ni del prompt viaja en ella.
- **La GUI se alimenta de la notificación**: cubetas, pestañas, anillo,
  «N working» y el par Interrumpir/Encolar cambian al recibirla; el sondeo
  queda solo como reconciliación al reconectar. La barra lateral sigue
  cambiando en seco: las guardias de `scenarios_shell.rs` no se tocan.
- **La TUI deja de sondear el estado**: el ticker de 2 s se queda para los
  relojes; el estado llega por la notificación, igual que los eventos de
  `session/watch` hoy.
- **El CLI la expone donde aporta**: `meltemi sessions --follow` imprime las
  transiciones como líneas JSON hasta `Ctrl+C` (§4: lo que el daemon
  notifica lo consume también el terminal scriptable).
- **El hub gana un canal de estado**: el mismo componente de
  `eventos-para-tardios`, con un test que pinea que una conexión tardía
  recibe el estado actual al iniciar y las transiciones después.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «El estado de cada sesión se notifica a todo
  cliente conectado» (transiciones, sin contenido, estado inicial al
  iniciar).
- `gui-shell` y `tui-shell`: + el estado vivo como fuente de cubetas,
  pestañas, indicadores y relojes; el sondeo solo reconcilia.
- `cli-contract`: + `sessions --follow`.

## Impact

- Archivos: `core/meltemid/src/{events.rs, session.rs, server.rs,
  clients.rs}`, `proto/schemas/v1/session-state.schema.json` (nuevo),
  registros de paleta de TUI y GUI (el gate de paridad exige casa para todo
  método del contrato; el design decide cómo se registra una notificación),
  `desktop/ui/src/lib/{stores.ts, session-state.ts, session-tabs.ts,
  components/Sidebar.svelte, components/StatusBar.svelte}`,
  `tui/src/shell/{conn.rs, state.rs, live.rs}`, `tui/src/run.rs`,
  `docs/paridad-nucleo.md`.
- Cero dependencias nuevas.
- Riesgo real: tormenta de notificaciones con N sesiones y M clientes; el
  design fija coalescencia por sesión y mide con el agente simulado. Y el
  orden entre `session/state` y los eventos de `session/watch` de la misma
  sesión: el estado inicial al iniciar evita la carrera de la conexión
  tardía.

## Fuera de alcance

- Notificar el progreso **dentro** del turno (herramienta en curso): eso
  viaja por `session/watch` y lo pinta `herramientas-a-la-vista`.
- Estado entre máquinas: un daemon por máquina, un túnel por daemon
  (constitución §3); `maquinas-remotas` lista máquinas, no agrega estados.
