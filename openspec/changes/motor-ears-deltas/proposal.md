# Propuesta: Motor EARS y de deltas

## Why

`spec-engine` ya descubre, parsea y valida la **estructura** de `.meltemi/`, pero le faltan las dos capacidades que convierten al motor en el corazón del ciclo SDD: **validar EARS en vivo** (que los requisitos sean normativos y los escenarios tengan forma verificable) y **parsear y fundir deltas** (`## ADDED/MODIFIED/REMOVED/RENAMED Requirements`) sobre la verdad viva — que es exactamente lo que `/archive` hace. Con esto, Meltemi podrá aplicar sus propios cambios sobre `.meltemi/specs/`, y quedará listo para cerrar el bootstrap en dos etapas (migrar de `openspec/` a `.meltemi/`).

## What Changes

- **Validación EARS** (amplía `spec-engine`): un escenario SHALL tener al menos un marcador EARS reconocido (`WHEN/WHILE/IF/THEN/WHERE`); un requisito SHALL ser normativo (su descripción usa `SHALL`/`MUST`, no `should`/`may`). Nuevos diagnósticos con ubicación.
- **Parser de deltas** (nueva capacidad `spec-merge`): parsea un delta spec en operaciones estructuradas — `Added`/`Modified`/`Removed`/`Renamed` — con el bloque completo de cada requisito, los campos `Reason`/`Migration` de las eliminaciones y el `FROM:`/`TO:` de los renombres.
- **Aplicación/fusión de deltas** (`spec-merge`): dado el spec vivo de una capacidad y un delta, computa el spec resultante y reporta diagnósticos de deltas inválidos (añadir un requisito que ya existe; modificar/eliminar/renombrar uno inexistente; eliminar sin `Reason`/`Migration`; renombrar a un nombre ya usado).
- **Dogfooding**: tests que aplican deltas reales (fixtures) y confirman que la fusión reproduce lo que `openspec archive` produjo, y que las specs vivas pasan la validación EARS.

## Capabilities

### New Capabilities

- `spec-merge`: el parser y aplicador de deltas — convierte un delta spec en operaciones estructuradas y las funde sobre el spec vivo de su capacidad, con diagnósticos de deltas inválidos y referencias colgantes. Es la operación que `/archive` ejecutará sobre `.meltemi/specs/`.

### Modified Capabilities

- `spec-engine`: se añaden requisitos de **validación EARS** (marcador de escenario y verbo normativo de requisito), que amplían la validación estructural existente sin cambiar sus reglas actuales.

## Impact

- **Código**: amplía el crate `core/meltemi-spec` con módulos `ears` (validación) y `delta` (parser + aplicador). Sin dependencias nuevas.
- **Documentos**: ninguno. No toca `meltemi.md` — sin checkpoint de gobernanza.
- **Verdad viva**: nueva capacidad `spec-merge`; requisitos añadidos a `spec-engine`.
- **Posible ajuste de dogfooding**: si alguna spec viva no cumpliera la validación EARS recién añadida, se normaliza esa spec (la validación cazándose a sí misma — el valor del dogfooding).
- **Fuera de alcance** (changes posteriores): **detección semántica de contradicciones y huecos** entre requisitos (más allá de referencias colgantes de deltas), que requiere análisis de significado; el ciclo de comandos `/review`·`/verify`·`/archive` y su integración en el daemon (`ciclo-sdd-autoria`, `comandos-verify-archive`); la migración `openspec/ → .meltemi/` (`migracion-openspec-a-meltemi`).
