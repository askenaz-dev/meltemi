# pensamiento-a-la-vista

> Vía rápida (fast-forward) candidata: deltas solo ADDED sobre `gui-shell` y
> `tui-shell`, cero daemon, cero contrato, cero dependencias — verificado: la
> cadena entera del pensamiento ya se transporta; esto es presentación. Si el
> pliegue conversacional de la TUI creciera más de lo previsto, la ruta sube a
> spec-full y se anota aquí.

## Why

El mantenedor pidió «ver la ejecución y la cadena de pensamiento en vivo como
lo hacen Codex, Claude Code, Antigravity, Copilot». La auditoría del código
trajo la mejor noticia posible: **la cadena está construida de punta a punta
y funciona** — lo que falta es destaparla.

Los hechos, eslabón por eslabón. Los dos adaptadores propios ya reenvían el
pensamiento: el de Claude pide `--include-partial-messages` y mapea cada
`thinking_delta` a `AgentThoughtChunk` en streaming, con deduplicación del
bloque final (`core/meltemi-adapters/src/claude/mapping.rs:135-138, 161-165`;
test en `:474-480`); el de Codex mapea los deltas de reasoning del app-server
(`codex/wire.rs:83-85`, `codex/mapping.rs:113-137`) y descarta el resumen
final a propósito para no repetir. El daemon no discrimina: reenvía cada
`SessionUpdate` verbatim al log, y el log publica al hub
(`core/meltemid/src/acp.rs:192-197, 372-376`). El contrato lo transporta
dentro de `agent_update` («forwarded verbatim»,
`proto/schemas/v1/session-event.schema.json:84-95`). Y la GUI **ya pinta en
streaming**: `session/watch` + pliegue reactivo donde las burbujas crecen
chunk a chunk (`desktop/ui/src/lib/conversation.ts:124-131, 247-254`).

Los dos eslabones flojos, exactos:

1. **GUI**: el pensamiento se pinta dentro de un `<details>` **colapsado**
   (`SessionDetail.svelte:637-641`). El stream vivo existe; hay que
   expandirlo a mano, turno por turno. El efecto «Claude Code» está a un
   atributo de distancia.
2. **TUI**: descarta todo el contenido — `summarize_event` reduce cada evento
   a `[id] tipo` (`tui/src/shell/conn.rs:693-705`) y el drill-in muestra esos
   one-liners. Ni prosa, ni pensamiento, ni tool calls legibles. Los datos ya
   llegan al proceso; falta el pliegue conversacional que la GUI ya tiene.

## What Changes

- **GUI — el pensamiento se muestra abierto mientras el turno está en
  vuelo**: el bloque de pensamiento del turno activo se renderiza expandido
  durante el streaming y el usuario puede plegarlo; al cerrar el turno, la
  política de reposo (queda como estaba / se pliega) la fija el gate con el
  design system. Sin animación de layout: aparece y crece como texto, que es
  lo que es.
- **TUI — pliegue conversacional en el drill-in de sesión**: espejo en Rust
  del fold de `conversation.ts` — prosa del agente, pensamiento marcado como
  tal, tool calls con su estado — con el tope de líneas que la TUI ya tiene,
  glifo+palabra y gemelos ASCII. El eco crudo de eventos actual sigue
  disponible (es el log del operador); el gate fija si conmutado o apilado.
- **Honestidad de nivel 3**: las salidas headless no mapean reasoning hoy
  (`core/meltemid/src/levels.rs:368-390`) — el pensamiento en vivo es de los
  niveles ACP (1 y 2) y la superficie no finge lo contrario: sin chunk, no
  hay sección, jamás un placeholder.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «El pensamiento del turno se ve mientras ocurre» —
  expandido en vivo, plegable, sin animación de layout.
- `tui-shell`: + requisito «El drill-in de sesión lee como conversación» —
  pliegue de prosa/pensamiento/herramientas con tope y gemelos ASCII.

## Impact

- Archivos: `desktop/ui/src/lib/views/SessionDetail.svelte` (atributo y
  política de apertura), `tui/src/shell/conn.rs` + render del drill-in (el
  pliegue nuevo, probablemente módulo propio testeable), i18n es/en para los rótulos
  del pliegue TUI. Cero daemon, cero `proto/`, cero dependencias.
- Verificación: tests de componente y del módulo de pliegue por escenario +
  smoke visual CDP (GUI) y smoke de TUI sobre el binario release, con una
  sesión del mock-agent emitiendo thought chunks (el mock ya ejercita el
  canal ACP; si le falta un escenario con pensamiento, se añade al fixture —
  sigue sin red y sin agentes reales).
- Riesgo: bajo. Lo único con juicio es la densidad del pliegue TUI en
  terminales angostas; el tope de líneas existente y el suelo de tamaño del
  chrome ya acotan el problema.

## Fuera de alcance

- **Reasoning en nivel 3 (headless)**: cuando la salida JSON oficial de un
  agente lo traiga, es change propia con su mapeo verificado.
- **Resumir o acortar el pensamiento**: se muestra lo que llega, como llega.
- **Ocultar el pensamiento por configuración**: futuro con evidencia de que
  alguien lo quiere apagado.
- **El estado «esperando instrucciones» y la vida de la sesión**: eso es
  `sesion-que-espera`; esta change no toca el ciclo de vida.
