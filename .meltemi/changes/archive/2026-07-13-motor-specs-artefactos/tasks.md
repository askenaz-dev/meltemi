# Tareas: Motor de specs — parser y validador de `.meltemi/`

> Change de código, sin tocar documentos ratificados. Cada tarea deja el workspace compilando y con clippy/fmt limpios (constitución §7). Referencia de decisiones: `design.md` (M1–M7).

## 1. Andamiaje del crate

- [x] 1.1 Crear el crate `core/meltemi-spec` (librería, cabecera SPDX) y añadirlo al workspace Cargo raíz
- [x] 1.2 Definir el modelo (M2): `Spec`, `Requirement`, `Scenario`, `Step`/`StepMarker`, `RumboFile`, `Inclusion`, `MeltemiTree`, `ChangeDir`

## 2. Parseo

- [x] 2.1 Parser line-oriented de specs (M3): `### Requirement:`, `#### Scenario:` (exactamente 4 `#`), pasos `- **WHEN|WHILE|IF|THEN|WHERE|AND**`, con número de línea por elemento
- [x] 2.2 Reconocimiento de cabeceras de delta `## ADDED/MODIFIED/REMOVED/RENAMED Requirements` (clasificar, no aplicar)
- [x] 2.3 Parser mínimo de front-matter de rumbo (M4): `inclusion` (`siempre`/`por-patrón`/`manual`), `fileMatch: [globs]`, `ratificado`/`ratificador`; sin nueva dependencia
- [x] 2.4 Descubrimiento del árbol `.meltemi/` (M2): constitución, rumbo, specs vivas, changes, archive; árbol vacío si falta `.meltemi/`

## 3. Validación y diagnósticos

- [x] 3.1 Tipo `Diagnostic { file, line, rule, message }` con enum `Rule` estable (M6); mensajes en inglés
- [x] 3.2 Reglas estructurales (M5): requisito sin escenario; escenario mal nivelado; nombre de capacidad no kebab-case; front-matter ausente o `por-patrón` sin `fileMatch`; cabecera de delta no canónica
- [x] 3.3 API de validación: `validate_tree(&MeltemiTree) -> Vec<Diagnostic>` y `validate_spec(&Spec) -> Vec<Diagnostic>`

## 4. Pruebas

- [x] 4.1 Tests unitarios del parser (requisitos/escenarios, nivel de encabezado, pasos) y del front-matter
- [x] 4.2 Tests de cada regla de validación (caso conforme + caso que dispara el diagnóstico), escenario por escenario del spec `spec-engine`
- [x] 4.3 Test de dogfooding (M7): descubrir y validar `.meltemi/` + las specs vivas del repo → **cero diagnósticos**

## 5. Cierre

- [x] 5.1 `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo test` verdes en todo el workspace
- [x] 5.2 Verificación escenario por escenario del spec `spec-engine` y ajustes finales
