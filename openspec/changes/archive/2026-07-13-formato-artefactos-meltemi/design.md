# Diseño: Formato canónico de los artefactos `.meltemi/`

## Context

`fase-0-fundacion` produjo las primeras specs vivas usando un formato híbrido (estructura en inglés, prosa en español) heredado de la práctica OpenSpec, mientras `meltemi.md` §5.1/§2.1 seguía describiendo un formato en español. El motor de specs de Fase 1 no puede construirse sobre esa ambigüedad. Esta change fija el contrato de formato como spec de la verdad viva y alinea el documento fundacional. No implementa el motor: define lo que el motor deberá cumplir.

## Goals / Non-Goals

**Goals:**
- Un formato canónico, testable y sin ambigüedad para todos los artefactos `.meltemi/`.
- Política de idioma explícita: estructura/normativa en inglés, prosa en español.
- Reconciliar `meltemi.md` con la práctica, dejando la ratificación de la nueva versión al mantenedor.

**Non-Goals:**
- No se implementa parser, validador, detección de contradicciones ni el ciclo de comandos (changes posteriores).
- No se cambian los requisitos de las capacidades de código existentes.
- No se define el formato de artefactos que no son del método (código, `brand/`, etc.).

## Decisions

### F1 — Política de idioma híbrida (estructura EN / prosa ES)
Palabras clave estructurales y normativas en inglés; prosa descriptiva en español neutro. Canon:
- Cabeceras de delta: `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`, `## RENAMED Requirements`.
- Requisito: `### Requirement: <nombre>` (nombre en español).
- Escenario: `#### Scenario: <nombre>` (exactamente 4 `#`).
- EARS: `WHEN`, `WHILE`, `IF … THEN`, `WHERE`, y el verbo normativo `SHALL`/`MUST` (evitar `should`/`may`).
- Pasos de escenario: viñetas `- **WHEN** …` / `- **THEN** …`.
*Alternativas descartadas*: todo-español (rompe el canon EARS y el ecosistema; obliga a reescribir las specs vivas) y todo-inglés (rompe constitución §11). El híbrido es la práctica de facto y la elección del mantenedor.

### F2 — Estructura y nombres de artefactos
Se canoniza el árbol de §5.1 de `meltemi.md`:
- `constitution.md` — principios no negociables.
- `rumbo/{product,tech,structure}.md` y `rumbo/*.md` — contexto persistente, con front-matter.
- `specs/<capability>/spec.md` — verdad viva por capacidad (kebab-case; nombre en inglés, como las capacidades existentes).
- `changes/<change-name>/` con `proposal.md`, `requirements.md`, `design.md`, `specs/` (deltas) y `tasks.md`.
- `changes/archive/<YYYY-MM-DD-change-name>/` — histórico.
- `hooks/` — automatizaciones (fase 2).

### F3 — Front-matter de `rumbo/`
YAML front-matter con `inclusion: siempre | por-patrón | manual`. Para `por-patrón`, un campo `fileMatch: [<globs>]`. Los campos de ratificación (`ratificado`, `ratificador`) son opcionales y los estampa el mantenedor. Ejemplo canónico: los `rumbo/*.md` actuales.

### F4 — Reglas de delta
- Cada delta describe solo lo que cambia; no reescribe la spec completa.
- `MODIFIED` incluye el **bloque completo** del requisito actualizado (no fragmentos).
- `REMOVED` incluye **Reason** y **Migration**.
- `RENAMED` usa formato `FROM:` / `TO:`.
- Todo requisito tiene **al menos un escenario**.
- Al archivar, los deltas se funden en `specs/<capability>/spec.md` y el andamiaje pasa a `archive/`.

### F5 — Enmienda consecuente a `meltemi.md` (v1.1 → v1.2)
- §2.1: los patrones EARS de ejemplo se muestran en el canon inglés (`WHEN`/`WHILE`/`IF…THEN`/`WHERE` + `SHALL`), aclarando que la prosa va en español.
- §5.1: la línea de deltas del árbol pasa a `## ADDED / ## MODIFIED / ## REMOVED Requirements`; el editor de §6.1 se describe con las cabeceras inglesas.
- Cabecera → v1.2, con nota de enmienda y ratificación pendiente del mantenedor (no auto-ratificar; regla `method-bootstrap`).
El texto exacto de cada edición se fija en `tasks.md`.

## Risks / Trade-offs

- **[Estructura EN vs identidad ES]** → Mitigación: la prosa —lo que el humano lee— es español; solo las palabras clave que el parser reconoce son inglesas, alineadas con EARS/ACP. Se documenta el porqué en la spec `artifact-format`.
- **[Editar un documento ratificado]** → Cada edición se especifica textualmente en tasks; requiere aprobación del mantenedor al aplicar; la ratificación de v1.2 queda pendiente.

## Open Questions

- ¿Se permite front-matter también en `constitution.md` y `specs/*/spec.md` (además de `rumbo/`)? Propuesta: opcional y solo para metadatos de ratificación; se confirma al implementar el motor (`motor-specs-artefactos`).
