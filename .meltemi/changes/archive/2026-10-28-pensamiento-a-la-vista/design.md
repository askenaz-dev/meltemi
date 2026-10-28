# Design — pensamiento-a-la-vista

## Context

Verificado en el código: **la cadena que transporta el pensamiento está
completa y funciona**. Los dos adaptadores propios lo mapean en streaming
(`claude/mapping.rs:135-138` con dedup en `:161-165`; `codex/mapping.rs:113-137`),
el daemon reenvía cada `SessionUpdate` verbatim al log y al hub
(`acp.rs:192-197, 372-376`), el contrato lo lleva dentro de `agent_update`
(«forwarded verbatim», `session-event.schema.json:84-95`) y la GUI lo acumula
chunk a chunk (`conversation.ts:124-131, 247-254`).

Lo que falla es lo último:

- **GUI**: el pensamiento se pinta en `<details class="thought">` **sin
  `open`** (`SessionDetail.svelte:649-652`). El stream existe y crece; hay que
  abrirlo a mano, turno tras turno.
- **TUI**: `summarize_event` reduce **cada** evento a `[sessionId] tipo`
  (`tui/src/shell/conn.rs:694-705`). Ni prosa, ni pensamiento, ni herramientas:
  el drill-in muestra una lista de nombres de tipo.

Dos datos que deciden la forma:

1. El turno del agente ya sabe si está en vuelo: `AgentTurn.closed`
   (`conversation.ts:62`, puesto en `:304, :310`).
2. El nivel 3 **no mapea reasoning** (`levels.rs:368-390`): el pensamiento en
   vivo es de los niveles ACP, y la superficie no debe fingir lo contrario.

## Goals / Non-Goals

**Goals**: ver el pensamiento mientras ocurre sin un gesto por turno; que el
drill-in del terminal lea como una conversación y no como un registro de tipos.

**Non-Goals**: reasoning en nivel 3; resumir o acortar el pensamiento;
configurarlo para apagarlo; el ciclo de vida de la sesión.

## Decisions

### D1 — El pensamiento se abre mientras el turno está en vuelo

`open={!item.closed}` en el `<details>`. Abierto mientras el turno corre,
plegado cuando cierra — el registro queda ordenado sin que nadie lo pliegue.

Detalle que hace que esto funcione y no moleste: Svelte solo escribe el
atributo cuando **la expresión cambia**. Mientras el turno corre, `!closed`
sigue siendo `true`, así que si el usuario pliega el bloque a mano **no se le
vuelve a abrir** en el siguiente chunk. El único cambio automático es el
plegado al terminar, que es la política declarada.

### D2 — El terminal pliega la conversación en vez de nombrar tipos

`summarize_event` deja de rendir `[id] tipo` para los eventos que llevan
contenido y rinde lo que dicen: prosa del agente, pensamiento **marcado como
tal**, y llamadas a herramienta con su estado. Los eventos que no llevan
contenido (arranque, resolución, checkpoints) conservan su línea de tipo, que
es exactamente lo que son.

Se hace en el lugar donde ya se decide qué se muestra, y **no** se reconstruye
el pliegue de turnos de `conversation.ts`: agrupar por turno en el shell es una
estructura nueva con su propio estado, y lo que falta aquí es que las líneas
digan algo. Si más adelante el shell quiere burbujas, esa es su change.

### D3 — El pensamiento se distingue de la prosa, sin depender del color

Prefijo de palabra en la línea, como el resto del shell hace con glifo+palabra:
quien lee el terminal debe poder separar lo que el agente **dijo** de lo que
**pensó** sin más ayuda que el texto. Sin color como único portador y con
gemelo ASCII, que es requisito vigente del shell.

### D4 — Lo que no llega, no se dibuja

Sin chunk de pensamiento no hay sección, ni en la GUI ni en el terminal: nunca
un encabezado vacío ni un «pensando…» de relleno. Un agente de nivel 3 no
reporta reasoning y su transcript debe leerse como lo que es, no como uno al
que le falta algo.

## Risks / Trade-offs

- **Densidad en el terminal**: el pensamiento es largo por naturaleza. La línea
  se entrega al paneo que la lista ya tiene y al tope de líneas vigente del
  transcript; no se añade un recorte propio que pelee con ellos.
- Abrir por defecto empuja el resto del turno hacia abajo mientras crece. Es el
  comportamiento de las referencias y el precio de ver el trabajo; el plegado
  automático al cerrar el turno lo devuelve.

## Migration / Rollout

Solo presentación: `desktop/ui` y `tui/`. Cero daemon, cero contrato, cero
dependencias.
