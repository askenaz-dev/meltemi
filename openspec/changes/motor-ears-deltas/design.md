# Diseño: Motor EARS y de deltas

## Context

`core/meltemi-spec` provee el modelo, el parser line-oriented y la validación estructural. Esta change añade la capa semántica ligera (EARS) y la operación de fusión de deltas, que es la mecánica de `/archive`. El parser actual clasifica los marcadores EARS de cada paso y reconoce las cabeceras de delta, pero no asocia requisitos con su operación de delta ni aplica nada. Aquí se construye eso encima, sin dependencias nuevas y con el mismo estilo line-oriented.

## Goals / Non-Goals

**Goals:**
- Validación EARS: escenarios con marcador reconocido; requisitos normativos.
- Parser de deltas estructurado (operación → requisitos, con Reason/Migration y FROM/TO).
- Aplicación de un delta sobre el spec vivo → spec fundido + diagnósticos.
- Paridad con `openspec archive`: la fusión reproduce el resultado esperado (verificado con fixtures y contra el histórico real).

**Non-Goals:**
- Detección semántica de contradicciones/huecos entre requisitos (análisis de significado): change posterior.
- Persistencia/escritura del resultado fundido y el ciclo de comandos: `comandos-verify-archive`.
- Migración `openspec/ → .meltemi/`.

## Decisions

### D1 — Módulos nuevos en `core/meltemi-spec`
- `ears`: reglas de validación EARS (amplían `validate`). 
- `delta`: modelo de delta estructurado (`DeltaSpec`, `DeltaOp`), parser y aplicador (`apply_delta`).
Ambos reutilizan el parser line-oriented y el tipo `Diagnostic` existentes.

### D2 — Validación EARS (amplía `spec-engine`)
- **Marcador de escenario**: un `Scenario` sin ningún `Step` con marcador `When/While/If/Then/Where` → diagnóstico `ScenarioWithoutEarsMarker` en la línea del escenario. Los pasos ya se clasifican en `spec-engine`.
- **Verbo normativo**: la descripción de un `Requirement` que no contiene `SHALL` ni `MUST` → diagnóstico `RequirementWithoutNormativeVerb` en la línea del requisito. Se detecta como palabra completa para no confundir con subcadenas.
- Nuevas variantes en el enum `Rule`; los diagnósticos siguen el formato con ubicación de M6.

### D3 — Modelo de delta estructurado
```
DeltaSpec { capability, operations: Vec<DeltaOp>, source }
DeltaOp = Added(Requirement)
        | Modified(Requirement)          // bloque completo
        | Removed { name, reason, migration }
        | Renamed { from, to }
```
El parser agrupa el contenido bajo cada `## <OP> Requirements`: los `### Requirement:` de una sección `ADDED`/`MODIFIED` se parsean como requisitos completos (reutilizando el parser de specs); en `REMOVED` se extraen `**Reason**:` y `**Migration**:`; en `RENAMED`, las líneas `FROM:` / `TO:`.

### D4 — Aplicación de deltas
`apply_delta(living: &Spec, delta: &DeltaSpec) -> (Spec, Vec<Diagnostic>)`:
- **Added**: si el nombre ya existe en `living` → diagnóstico `AddedRequirementExists`; si no, se añade.
- **Modified**: si el nombre no existe → `ModifiedRequirementMissing`; si existe, se reemplaza el bloque completo.
- **Removed**: si falta `Reason` o `Migration` → `RemovedWithoutReasonOrMigration`; si el nombre no existe → `RemovedRequirementMissing`; si todo bien, se elimina.
- **Renamed**: si `from` no existe → `RenamedFromMissing`; si `to` ya existe → `RenamedToExists`; si no, se renombra.
El resultado es el `Spec` fundido (con las operaciones válidas aplicadas); los diagnósticos listan las inválidas. Determinista: preserva el orden de los requisitos vivos, añadidos al final.

### D5 — Paridad con `openspec archive`
Un test toma una change **archivada** real (p. ej. `fase-0-fundacion`) y su delta, aplica la fusión sobre un spec vivo vacío, y asevera que el resultado coincide (en requisitos y escenarios) con el `openspec/specs/<capability>/spec.md` actual. Esto ancla la semántica de fusión a la herramienta que hoy hace de motor durante el bootstrap.

### D6 — Ubicación de la validación EARS en el pipeline
`validate_spec` se amplía para incluir las reglas EARS tras las estructurales. Así `validate_tree` (y el dogfood) las aplican automáticamente. Se verifica que las specs vivas actuales las cumplen; si alguna no, se normaliza en esta change.

## Risks / Trade-offs

- **[La validación EARS marca specs vivas]** → Es el dogfooding trabajando: si una spec no es normativa, se corrige. Se revisa antes de cerrar (tarea de conformidad).
- **[Parser de bloque de requisito en MODIFIED]** → Reutiliza el parser de specs sobre el fragmento de la sección; se delimita cada requisito hasta el siguiente `### Requirement:` o el fin de la sección.
- **[Fusión vs `openspec` real]** → El test de paridad detecta divergencias de semántica; casos borde de formato de `openspec` que no usemos quedan documentados, no soportados.

## Migration Plan

Aditivo: nuevos módulos en un crate existente. Rollback = quitar `ears` y `delta` y revertir la ampliación de `validate`.

## Open Questions

- ¿La validación EARS debe exigir además un escenario con forma condición→resultado (un `WHEN/WHILE/IF` y un `THEN`), o basta con “algún marcador EARS”? Propuesta: por ahora “algún marcador”, para no marcar escenarios ubicuos legítimos; se endurece si hace falta.
- ¿`apply_delta` valida también EARS del resultado fundido? Propuesta: sí, se puede correr `validate_spec` sobre el resultado, pero la fusión en sí no lo exige.
