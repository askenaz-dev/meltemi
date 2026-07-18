# Tareas: Motor EARS y de deltas

> Amplía `core/meltemi-spec`. Sin tocar documentos ratificados. Cada tarea deja el workspace compilando y con clippy/fmt limpios (constitución §7). Decisiones: `design.md` (D1–D6).

## 1. Validación EARS (amplía `spec-engine`)

- [x] 1.1 Módulo `ears`: regla `ScenarioWithoutEarsMarker` (escenario sin ningún paso `When/While/If/Then/Where`) con ubicación (D2)
- [x] 1.2 Módulo `ears`: regla `RequirementWithoutNormativeVerb` (descripción sin `SHALL`/`MUST` como palabra completa) con ubicación (D2)
- [x] 1.3 Integrar las reglas EARS en `validate_spec` (tras las estructurales) y ampliar el enum `Rule`

## 2. Modelo y parser de deltas (`spec-merge`)

- [x] 2.1 Modelo `DeltaSpec`/`DeltaOp` (`Added`/`Modified`/`Removed{name,reason,migration}`/`Renamed{from,to}`) (D3)
- [x] 2.2 Parser de deltas: agrupar requisitos bajo su cabecera de operación; extraer `Reason`/`Migration` en `REMOVED` y `FROM:`/`TO:` en `RENAMED` (D3)

## 3. Aplicación de deltas

- [x] 3.1 `apply_delta(living, delta) -> (Spec, Vec<Diagnostic>)` con las reglas de D4 (Added/Modified/Removed/Renamed + diagnósticos de duplicado, inexistente, sin reason/migration, renombre a nombre en uso)
- [x] 3.2 Determinismo: preservar orden de requisitos vivos, añadidos al final; nuevas variantes de `Rule`

## 4. Pruebas

- [x] 4.1 Tests de validación EARS (escenario sin marcador; requisito no normativo; casos conformes)
- [x] 4.2 Tests del parser de deltas (operaciones mixtas; REMOVED con reason/migration; RENAMED from/to)
- [x] 4.3 Tests de `apply_delta` por regla (aplicar y cada diagnóstico), escenario por escenario del spec `spec-merge`
- [x] 4.4 Test de paridad (D5): aplicar el delta de un cambio archivado real sobre el spec vivo previo y comparar con la verdad viva resultante

## 5. Conformidad y cierre

- [x] 5.1 Verificar que las specs vivas actuales cumplen la validación EARS recién añadida; normalizar cualquier spec no normativa (dogfooding)
- [x] 5.2 `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo test` verdes en todo el workspace; verificación escenario por escenario de `spec-engine` (EARS) y `spec-merge`
