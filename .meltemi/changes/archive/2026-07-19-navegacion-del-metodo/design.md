## Context

El método vive en `.meltemi/` y sus verbos operativos existen (`propose` →
`archive`/`implement`), pero no hay navegación: listar, mostrar, validar. Todo
el estado necesario **ya está persistido** (artefactos en disco, checklist de
review, `.verify.jsonl`, ticks de `tasks.md`, registro de archivadas); esta
change son lectores agregados + un verbo de validación que ya existe como
función interna. Auditoría de paridad: OpenSpec local 1.6.0 == upstream 1.6.0
(2026-07-18); huecos cubribles = list/show/status/validate.

## Goals / Non-Goals

**Goals:** listado con estado agregado; show de changes y specs vivas; validate
independiente con señal CI; `--json` en todo; solo lectura.
**Non-Goals:** dashboard TUI (delta futuro de `tui-shell` que consume estos
métodos); stores/worksets/schemas; completions; mutación alguna del árbol.

## Decisions

### D1 — Métodos del daemon, jamás parsing en el cliente
`change/list`, `change/show`, `spec/list`, `spec/show`, `sdd/validate` viven en
el daemon (paridad §4): la TUI/GUI consumen lo mismo que la CLI. El cliente solo
renderiza. Nombres consistentes con la familia existente (`session/list`,
`fleet/list`).

### D2 — El estado se agrega de lo persistido; cero estado nuevo
Por change, el listado computa: artefactos presentes (existencia de archivos),
tareas `x/y` (parser de `implement`), review `decididos/total` (estado del
checklist de `revision-specs-ux`), verify `verificados/total` (lector de
`verify`). Archivadas: nombre + fecha del directorio datado. Ninguna escritura;
un listado jamás muta (a diferencia de `review`, que persiste checklist — el
listado usa el lector, no el handler).

### D3 — `sdd/validate` = motor + fusión en seco, extraídos de `archive`
Por change: validación del motor (estructura + EARS por delta) +
`dry_run_diagnostics` (aplicación en seco contra la verdad viva) — exactamente
los pasos 1–2 del gate de `archive`, expuestos sin el resto. Sin argumento:
valida la verdad viva completa (cada spec parsea y es estructuralmente
conformante — doctor-lite). Los diagnósticos salen legibles y en `--json`.

### D4 — Código de salida `14`: hallazgos no son errores del canal
`validate` con diagnósticos no es error interno (1), ni de uso (2), ni de
contrato (11): es un resultado. CI necesita distinguirlo con código propio →
`14` (validación con hallazgos), extendiendo la taxonomía estable por su vía
declarada (delta de `cli-contract`). Éxito limpio = `0`.

### D5 — Show sin re-render: los artefactos son la verdad
`change/show` devuelve los artefactos tal cual (contenido de proposal/design/
tasks + deltas por capacidad); `spec/show` devuelve la spec viva parseada
(requisitos y escenarios) — el render bonito es del cliente. Nada se reescribe
ni normaliza al mostrar.

## Risks / Trade-offs

- **Listados grandes** (histórico crece) → `limit`/filtro por estado en params
  desde el día uno (patrón `session/list`).
- **Estados parciales** (change sin tasks.md, sin deltas) → columnas honestas
  (`—` / ausente), jamás inventadas; el listado refleja lo que hay.
- **Deriva con la gramática de cli-contract** (drift preexistente: la lista de
  subcomandos del requisito no incluye los verbos post-implement) → esta change
  no lo agrava (verbos en su propia capacidad) y el saneamiento queda anotado
  como cleanup futuro.

## Migration Plan

Aditivo puro y de solo lectura. Reversión: retirar métodos y subcomandos.

## Open Questions

- ¿CLI para decidir (`review-decide`/`verify-mark`) en un delta hermano?
- ¿Filtro `--state` del listado (activas|archivadas|revisables) en v1 o después?
