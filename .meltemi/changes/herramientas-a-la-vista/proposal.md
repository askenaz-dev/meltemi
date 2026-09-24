# herramientas-a-la-vista

> Vía completa (proposal → design → specs → tasks): toca los dos adaptadores
> propios, el render del transcript en la GUI y en la TUI, y las calles del
> tablero. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas D1, VIS-2, AI-4,
> D9 y VIS-8).

## Why

El log de la sesión de Claude Code del 2026-09-24 tiene 144 `agent_update`:
lecturas con sus rutas, un `Grep`, un `Glob`, un subagente lanzado para leer
por él, 37 fragmentos de pensamiento. El transcript de la GUI los muestra
como píldoras «título · estado». El daemon persiste **todo** lo que ACP
transporta en una llamada a herramienta —`kind`, `status`, `content`,
`locations`, el diff cuando el agente lo envía— porque escribe el `update`
verbatim (`core/meltemid/src/acp.rs:739-742`); ni la GUI
(`desktop/ui/src/lib/conversation.ts:133-141`) ni la TUI
(`tui/src/shell/conn.rs:835-845`) lo pintan. Y los adaptadores propios
envían Edit y Write como llamadas sin `ToolCallContent::Diff`, así que ni
siquiera hay diff que pintar cuando el agente es Claude Code o Codex.

Orca lo muestra en su landing porque es lo que un humano necesita para
confiar sin leer todo: `Read(src/x.ts) ⎿ Read 180 lines`, `Update(…) ⎿ Added
8 lines, removed 3 lines`, el hunk inline, los lotes agrupados, la actividad
de subagentes, el progreso en vivo. Meltemi tiene la misma información en el
disco y una promesa fundacional de «sesiones inspeccionables» (meltemi.md
§6.11). La regla de la casa decide la forma: estructura ACP genérica —clase,
contenido, ubicaciones, estado— para cualquier agente; `rawInput` solo
plegado bajo demanda; nunca una tarjeta que dependa del proveedor
(constitución §5).

## What Changes

- **Fila de herramienta**: glifo por estado (○ pendiente, ◔ en curso, ✓ hecho,
  ✕ fallido; con palabra), título en monoespaciada, cronómetro `0:07` de la
  llamada, línea de resultado `⎿` derivada del contenido («180 líneas», «+8
  −3»), ubicaciones como enlaces que abren el Editor en `archivo:línea`,
  diff plegable **cerrado por defecto** cuando hay `ToolCallContent::Diff`,
  y lotes: llamadas consecutivas de la misma clase se pliegan en «Leyó 4
  archivos». Usa las clases de llegada y cambio de estado de
  `senales-de-vida`.
- **Los adaptadores propios envían el diff**: `meltemi-claude-acp` mapea
  Edit/Write/MultiEdit a `ToolCallContent::Diff` (ruta, texto viejo, texto
  nuevo) y `meltemi-codex-acp` hace lo mismo con sus ítems de parche; la
  conformidad manual gana la comprobación «el diff llega estructurado».
- **Subagentes con su sitio**: una llamada que anida actividad (el `Agent` de
  Claude Code) se pinta como grupo plegable con sus propias filas, con lo que
  ACP transporte; no se inventa jerarquía donde el agente no la envía.
- **La TUI, misma información**: `◔ 0:07 Read src/x.rs` y la línea `⎿`, con
  gemelos ASCII; el diff se abre con la tecla de detalle.
- **Calles vivas en el tablero**: cada calle muestra la última fila de
  herramienta mientras su despacho corre, alimentada por `session/watch`
  (y por `estado-en-vivo` cuando el despacho lo lanzó otro cliente).
- **`rawInput` bajo demanda**: la entrada cruda de la herramienta se pliega
  detrás de un «ver entrada», nunca abierta por defecto.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: + requisito «Las ediciones viajan como contenido de diff
  ACP» en ambos adaptadores.
- `gui-shell` y `tui-shell`: + la fila de herramienta con resultado, diff
  plegable, ubicaciones y lotes; + grupos de subagente.
- `worktree-orchestration`: + escenario «la calle muestra su herramienta en
  curso».
- `integration-levels`: + comprobación del diff estructurado en la
  conformidad manual.

## Impact

- Archivos: `core/meltemi-adapters/src/{claude,codex}/`, `core/mock-agent`
  (emite diffs y grupos para los e2e), `desktop/ui/src/lib/{conversation.ts,
  views/SessionDetail.svelte, views/Review.svelte}`, `tui/src/shell/{conn.rs,
  render.rs, glyphs.rs}`, `docs/conformidad-manual.md`, `docs/qa/` (smoke
  visual).
- Cero dependencias nuevas; ningún RPC nuevo.
- Riesgo real: transcripts largos con cientos de filas plegables; el design
  fija virtualización o paginación por turno y lo mide con el presupuesto de
  RAM de la GUI.

## Fuera de alcance

- Anotar líneas del diff y devolverlas al agente: `anotar-el-diff`.
- Volver a un turno anterior: `volver-a-este-turno`.
- Un render específico por proveedor: nunca (§5).
