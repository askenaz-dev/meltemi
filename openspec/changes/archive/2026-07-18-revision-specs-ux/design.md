## Context

"La revisión de specs es la obsesión" (§4.9). El ciclo (#14) produce artefactos
con gates; falta que revisarlos sea placentero en terminal: diff de deltas por
requisito, diagnósticos presentados donde duelen, checklist interactiva, y
comentarios que vuelven al agente. La detección semántica diferida desde
`motor-ears-deltas` aterriza aquí como diagnósticos nuevos del motor.

## Goals / Non-Goals

**Goals:** render de deltas por requisito/escenario; diagnósticos semánticos
acotados y verificables en `meltemi-spec`; checklist `/review` persistente;
comentario→instrucción integrado al bucle del ciclo.
**Non-Goals:** revisión de código línea a línea con LSP (GUI fase 2); edición in
situ de specs (cerca `edit-surface`).

## Decisions

### D1 — Diff semántico de deltas, no de líneas
El render agrupa por operación (ADDED/MODIFIED/REMOVED/RENAMED) y por requisito:
para MODIFIED muestra el antes/después **por escenario y por statement**
(alineación por nombre), no un diff textual. Accesibilidad baseline: operación
como palabra+glifo, jamás solo color.

### D2 — Diagnósticos semánticos acotados (motor)
Tres detectores nuevos en `meltemi-spec`, deterministas y testeables:
(a) **duplicado**: requisito ADDED cuyo nombre normalizado ya existe en la
capacidad; (b) **no-op**: MODIFIED cuyo contenido es idéntico al vivo;
(c) **referencia colgante**: mención `«Requirement: X»` a un requisito
inexistente tras aplicar el delta. Nada de NLP: reglas exactas, cero falsos
positivos por diseño. Lo difuso queda para el humano (para eso es la checklist).

### D3 — Checklist persistente por requisito
`/review` recorre los requisitos del delta con estados (aprobado / comentado /
rechazado) persistidos en la change (reanudable); los diagnósticos del motor se
anclan al requisito que los produce. Cerrar la review exige todos los ítems
decididos.

### D4 — Comentario→instrucción por el bucle existente
Un comentario de review se despacha como reelaboración del artefacto specs (el
mismo bucle del ciclo #14), con el requisito citado; el gate de specs se reabre.

### D5 — Superficie
TUI: review como flujo de la vista Proyecto (lista de requisitos → detalle con
diff y diagnósticos → decisión por ítem). CLI: `review` operativo con salida por
pasos y `--json` (estado de la checklist).

## Risks / Trade-offs

- **Detección semántica ambiciosa** → acotada a 3 reglas exactas; ampliar es
  delta futuro con evidencia.
- **Reviews eternas** → estado persistente + reanudable; el conteo pendiente
  visible en la vista Proyecto.

## Migration Plan

Aditivo: reglas nuevas en el motor (variantes de `Rule`), verbo nuevo, render
nuevo. Nada existente cambia de significado.

## Open Questions

- Normalización de nombres para (a): casefold+espacios; ¿acentos? — fijar en tasks.
