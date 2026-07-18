# Propuesta: Motor de specs — parser y validador de `.meltemi/`

## Why

El contrato de formato (`artifact-format`, 7 requisitos) ya está en la verdad viva, pero nada lo implementa: Meltemi todavía no puede **leer ni validar sus propios artefactos**. Este es el primer código que cierra ese hueco — el motor que descubre la estructura `.meltemi/`, parsea specs y archivos de rumbo a un modelo en memoria, y valida su conformidad estructural, reportando diagnósticos con ubicación. Es el cimiento sobre el que se construirán la validación EARS, los deltas y el ciclo de comandos (changes posteriores de Fase 1). Marca el cruce de "spec-driven a mano" a "spec-driven con motor".

## What Changes

- **Nuevo crate `core/meltemi-spec`**: librería de parseo y validación de artefactos `.meltemi/`, consumible por el daemon y por el tooling (paridad de núcleo: es una librería, no un binario).
- **Modelo en memoria**: tipos para una spec parseada (`Spec`, `Requirement`, `Scenario`), un archivo de rumbo con su front-matter (`RumboFile`, `Inclusion`), y el árbol `.meltemi/` descubierto (`MeltemiTree`: constitución, rumbo, capacidades de la verdad viva, cambios, archivo).
- **Parser line-oriented**: reconoce `### Requirement:`, `#### Scenario:` (exactamente 4 `#`), pasos `- **WHEN**`/`- **THEN**`, y las cabeceras de delta `## ADDED/MODIFIED/REMOVED/RENAMED Requirements`. Front-matter YAML mínimo (`inclusion`, `fileMatch`) sin nueva dependencia pesada.
- **Validación estructural** (subconjunto de `artifact-format` que no requiere semántica EARS ni deltas): todo requisito tiene ≥1 escenario; escenarios con nivel de encabezado correcto; nombres de capacidad en kebab-case; front-matter de rumbo presente y bien formado; cabeceras de delta reconocidas.
- **Diagnósticos con ubicación**: cada violación reporta archivo, línea y regla incumplida, en un tipo de error estructurado.
- **Dogfooding**: un test valida los propios artefactos del proyecto — `.meltemi/constitution.md`, `.meltemi/rumbo/*` y las specs vivas — confirmando que Meltemi cumple su propio formato.

## Capabilities

### New Capabilities

- `spec-engine`: el motor que descubre, parsea y valida estructuralmente los artefactos `.meltemi/` contra el contrato `artifact-format`, produciendo un modelo en memoria y diagnósticos con ubicación. Es la implementación fundacional del método; la validación semántica EARS, la aplicación de deltas y la detección de contradicciones se construyen encima en changes posteriores.

### Modified Capabilities

<!-- Ninguna. `artifact-format` define las reglas; este motor las implementa sin cambiarlas. No toca `daemon-lifecycle`, `acp-session`, `propose-flow` ni `method-bootstrap`. -->

## Impact

- **Código nuevo**: crate `core/meltemi-spec` (librería + tests). Añadido al workspace Cargo (`Cargo.toml` raíz).
- **Dependencias**: idealmente ninguna nueva (el formato es line-oriented; el front-matter, mínimo). Si el parseo de front-matter con listas lo justifica, una dependencia pequeña y pineada, argumentada en el design.
- **Documentos**: ninguno. No toca `meltemi.md` — sin checkpoint de gobernanza.
- **Fuera de alcance** (changes posteriores): validación de palabras clave EARS y detección de contradicciones/huecos (`motor-ears-deltas`); parseo y **aplicación/fusión de deltas** (`motor-ears-deltas`); el ciclo de comandos `/review`·`/plan`·`/verify`·`/archive` (`ciclo-sdd-autoria`, `comandos-verify-archive`); la migración de `openspec/` a `.meltemi/` (`migracion-openspec-a-meltemi`).
