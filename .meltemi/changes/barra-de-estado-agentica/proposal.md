# barra-de-estado-agentica

> Vía rápida (fast-forward) candidata por criterio D7: deltas solo ADDED sobre
> capabilities existentes (`gui-shell`, `tui-shell` y, si el design lo pide,
> `local-analytics`), ninguna capability nueva, ningún MODIFIED ni REMOVED. Si
> el consumo por sesión exigiera un verbo de contrato nuevo en vez de
> campos/consultas aditivas, la ruta sube a spec-full y se anota aquí antes de
> implementar. Primer hallazgo capturado de la auditoría de intuitividad
> pendiente (docs/plan-de-cambios.md la declara deuda de la tanda UX).

## Why

El mantenedor miró la aplicación y pidió «una barra inferior de estado como
VS Code e IntelliJ». La barra **existe** —
`desktop/ui/src/lib/components/StatusBar.svelte`, anclada al borde inferior
por spec (`gui-shell`, «Arquitectura visual de aplicación de escritorio»:
conexión, versión del daemon, endpoint y resumen de sesiones\permisos) — y
esa es exactamente la noticia: una barra de estado que su propio mantenedor
no registra como tal no está cumpliendo la función de una barra de estado. La
función, en las dos referencias que él nombró, es responder de un vistazo
«qué está pasando y qué espera de mí».

Lo que no hay que copiar de las referencias es su contenido. VS Code e
IntelliJ dicen Ln/Col, encoding, EOL e indentación porque su dominio es el
archivo abierto. El dominio de Meltemi es el bucle agéntico, y su rumbo
excluye por escrito ser editor de propósito general: llenar la barra de
señales de editor sería relleno con pedigrí. Las señales de este dominio son
otras: sobre qué proyecto se trabaja, qué change del método está activa y qué
gate espera firma, cuántas sesiones corren y cuántas esperan algo del humano,
y cuánto está costando en tokens lo que corre.

Casi todo eso el daemon ya lo sabe y ninguna superficie lo dice donde el ojo
descansa: `change/list` declara `gatePending` con el artefacto que espera
(`sesion-esperando`), `session/list` trae los estados que la barra hoy aplana
en «N en curso», y `analytics/usage` pliega los tokens de los registros
locales con su frontera medido/no-reportado (`analitica-consumo-local`). Es
el patrón de `avisos-de-escritorio`: los datos existen, falta el último metro
de render.

La honestidad del segmento de consumo se dice aquí para no descubrirla
después: los niveles 1 y 2 se pilotan por ACP y **ACP no transporta usage**
(`core/meltemid/src/analytics.rs:306`, razón estable de design D4 de la
analítica), así que la mayoría de las sesiones de la flota hoy lee «no
reportado». El segmento muestra lo medido sin inventar nada; llenar ese hueco
para la flota ACP es la extensión de usage ya nombrada como change futura
(fuera de alcance de `motor-propio-byok`), no un arreglo que esta barra pueda
fingir.

## What Changes

- **Segmentos nuevos en la barra de la GUI**, ordenados del contexto al
  costo: proyecto activo (nombre corto, como ya lo muestra el sidebar), change
  activa del método con su gate cuando lo hay («artefactos-de-cada-push ·
  gate: specs», leyendo el `gatePending` existente), desglose de sesiones por
  estado en vez del plano «N en curso» (en curso / esperando permiso /
  esperando gate — los mismos estados que ya distingue `session/list`), y
  consumo de tokens de las sesiones activas donde esté medido, con el motivo
  estable cuando no («no reportado (ACP)»). Conexión, versión y endpoint se
  conservan tal cual.
- **Segmentos accionables**: clic (y foco de teclado con nombre accesible)
  navega a la vista dueña — proyecto→Proyecto, permisos→bandeja,
  sesiones→Sesiones, change→Proyecto. El patrón de las referencias: la barra
  no solo dice, lleva.
- **Prioridad declarada al encoger**: con ancho insuficiente se trunca
  primero el endpoint (hoy ya truncado a 40ch), luego versión, luego
  proyecto; conexión y permisos pendientes no se caen jamás. La prioridad es
  la misma jerarquía de señales vigente del shell, no una nueva.
- **TUI a la par en señales**: el header del chrome (que ya dice conexión,
  versión, uptime, sesiones y `⚑ N esperando`) gana change+gate y consumo
  medido con glifo+palabra y gemelos ASCII, como el resto del shell. El
  footer sigue siendo de atajos y eco de foco: su spec lo fija así y esta
  change no lo toca.
- **Daemon y contrato solo si el design lo pide**: si el consumo de sesiones
  activas no se puede derivar razonablemente de `analytics/usage` +
  `session/list` desde el cliente, se añade consulta o campo **aditivo** (p.
  ej. filtro por sesión en `analytics/usage`), que nace ×3 con la matriz de
  paridad al día (§4). Ningún verbo nuevo sin subir la ruta a spec-full.
- **Exclusión escrita**: sin Ln/Col, encoding, EOL ni indentación — señales
  de la superficie de edición utilitaria que el rumbo excluye del producto.
  Si el editor las necesita algún día, viven en el editor, no en el chrome
  global.

## Capabilities

### Modified Capabilities

- `gui-shell`: + requisitos ADDED — señales del bucle en la barra de estado
  (proyecto, change+gate, sesiones por estado, consumo con frontera de
  honestidad), segmentos accionables y prioridad de truncamiento. El
  requisito existente de la barra (conexión, versión, endpoint, resumen) no
  se modifica: lo nuevo se suma, no lo reescribe.
- `tui-shell`: + requisito ADDED — las mismas señales nuevas en el chrome
  persistente, con los gemelos ASCII y la prioridad de señales vigente.
- `local-analytics` (condicional al design): + consulta/campo aditivo para
  el consumo de sesiones activas, con la misma frontera medido/no-reportado.

### New Capabilities

- Ninguna.

## Impact

- Archivos: `desktop/ui/src/lib/components/StatusBar.svelte`,
  `desktop/ui/src/lib/messages.ts` (textos es/en), `App.svelte` (datos que el
  shell ya posee), `tui/src/shell/render.rs` y `tui/src/shell/messages.rs`;
  quizá `core/meltemid/src/analytics.rs` y `proto/` (solo aditivo), y en ese
  caso `docs/paridad-nucleo.md`. Tests por escenario en las suites de shell
  existentes; smoke visual CDP sobre el binario release con medidas, como las
  changes de la tanda UX.
- Cero dependencias nuevas. Cero telemetría: todo se calcula en local, como
  ya lo hace la analítica (§9).
- Riesgo nombrado: densidad — la barra es una línea; si los segmentos no
  caben en la ventana mínima (900px), decide la prioridad declarada, y el
  design fija los anchos con la escala del design system (`fs-caption`).
- Lo que solo el uso confirmará: si «consumo de sesiones activas» se
  entiende mejor como agregado de las activas o como el de la sesión
  enfocada; el design elige uno y lo escribe, el smoke lo valida.

## Fuera de alcance

- **La extensión ACP de usage** que llenaría el «no reportado» de los niveles
  1 y 2: change futura ya nombrada en `motor-propio-byok`, con su prueba §6
  (extensión abierta y documentada que cualquier agente pueda implementar).
  Esta barra muestra lo medido; no lo fabrica.
- **Configurabilidad de la barra** (mostrar/ocultar segmentos): futuro con
  evidencia de que alguien lo necesita.
- **Señales de editor** (Ln/Col, encoding, EOL, indentación): excluidas por
  rumbo, no diferidas.
- **La auditoría de intuitividad completa**: esta change captura su primer
  hallazgo, no la sustituye; el barrido sigue siendo deuda declarada.
- **Avisos al escritorio**: tienen su change abierta
  (`avisos-de-escritorio`); la barra no duplica ese canal.
