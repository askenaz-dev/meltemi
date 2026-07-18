## Context

La proyección de constitución+rumbo a los formatos de instrucciones de cada
agente es hoy manual (`AGENTS.md` lo declara y ya se desincronizó una vez). El
motor `meltemi-spec` parsea constitución, rumbo (con front-matter de inclusión) y
changes; falta el compilador y su escritura segura. Es además el vehículo del
nivel 4 (#10).

## Goals / Non-Goals

**Goals:** compilación determinista artefactos→documento; bloques gestionados que
jamás pisan contenido del usuario; variantes por destino con nombres en datos
(neutralidad de marca en artefactos); regeneración bajo demanda; dogfooding
inmediato en este repo.
**Non-Goals:** mapa del repo y `@refs` (#12); selección dinámica de contexto por
tarea (fase 2+); inyección MCP (#13).

## Decisions

### D1 — Compilador puro en `meltemi-spec`
`project(tree, change_activa?) -> Documento` como función pura: constitución
íntegra + rumbo según regla de inclusión (`siempre` entra; `por-patrón` y
`manual` se listan como disponibles, no se inyectan) + resumen de la change
activa (proposal + deltas). Determinista: mismo árbol → mismo texto.

### D2 — Bloques gestionados con huella
La escritura ocurre solo entre marcadores `<!-- meltemi:context:begin -->` /
`<!-- meltemi:context:end -->` con una huella (hash de fuentes + versión). Todo
contenido fuera de los marcadores se preserva byte a byte; si los marcadores no
existen, se anexan al final. Escritura atómica (temp + rename) vía daemon.

### D3 — Destinos declarados en datos
Un mapa de destinos (archivo de datos, como el registro de flota) declara los
nombres/ubicaciones de instrucciones que consume cada agente del catálogo; la
base siempre es `AGENTS.md`. Los artefactos del método no nombran productos; los
datos sí (interoperabilidad factual).

### D4 — Disparadores: bajo demanda ahora, hook al archivar después
RPC `context/project` + subcomando CLI `project` (delta acumulativo de gramática:
asume `fleet` ya operativa). La regeneración automática post-archivado se engancha
cuando exista `/archive` (#19); hasta entonces, manual — decisión provisional
anotada.

### D5 — Dogfooding inmediato
Al aplicarse, este repositorio pasa a proyección generada: se retira la
advertencia "proyección manual" de `AGENTS.md` y el contenido queda gestionado.

## Risks / Trade-offs

- **Pisar contenido del usuario** → invariante verificable: fuera de marcadores,
  bytes idénticos (test property-ish con documentos adversos).
- **Deriva entre destinos** → una sola compilación, N escrituras; huella común.
- **Formatos de terceros cambian** → viven en datos versionados, no en código.

## Migration Plan

Aditivo (método + subcomando). El primer `project` sobre un archivo existente
anexa los marcadores sin tocar lo demás. Reversión: borrar bloques gestionados.

## Open Questions

- Presupuesto de tamaño del bloque (truncar rumbo `por-patrón` listado si crece).
- Nombre final del subcomando (`project` vs `context`) — fijado en tasks como `project`.
