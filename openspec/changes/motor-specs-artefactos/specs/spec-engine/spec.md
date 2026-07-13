# spec-engine — Motor de parseo y validación de `.meltemi/`

## ADDED Requirements

### Requirement: Descubrimiento del árbol `.meltemi/`
El motor SHALL descubrir, dada la raíz de un repositorio, la estructura `.meltemi/`: la constitución, los archivos de rumbo, las capacidades de la verdad viva (`specs/<capability>/spec.md`), los cambios (`changes/<name>/`) y el archivo (`changes/archive/`). Un `.meltemi/` ausente SHALL producir un árbol vacío, no un error.

#### Scenario: Repositorio con `.meltemi/` poblado
- **WHEN** el motor descubre un repositorio con `constitution.md`, `rumbo/` y `specs/<capability>/spec.md`
- **THEN** el árbol resultante enumera la constitución, los archivos de rumbo y las capacidades encontradas

#### Scenario: Repositorio sin `.meltemi/`
- **WHEN** el motor descubre un repositorio que no contiene `.meltemi/`
- **THEN** devuelve un árbol vacío sin error

### Requirement: Parseo de una spec a modelo
El motor SHALL parsear un archivo de spec en un modelo de requisitos y escenarios, reconociendo `### Requirement: <nombre>` como requisito y `#### Scenario: <nombre>` como escenario, y conservando el número de línea de cada elemento.

#### Scenario: Spec con requisitos y escenarios
- **WHEN** el motor parsea una spec con dos requisitos, cada uno con un escenario
- **THEN** el modelo contiene dos requisitos, cada uno con su escenario y su línea de origen

### Requirement: Reconocimiento del nivel de escenario
El motor SHALL reconocer como escenario únicamente los encabezados con exactamente cuatro `#`; un encabezado con tres o cinco `#` NO SHALL contar como escenario.

#### Scenario: Escenario mal nivelado
- **WHEN** una spec declara un "escenario" con tres `#` en lugar de cuatro
- **THEN** el motor no lo cuenta como escenario y el requisito queda sin escenarios reconocidos

### Requirement: Parseo del front-matter de rumbo
El motor SHALL parsear el front-matter YAML de los archivos de rumbo, extrayendo `inclusion` (`siempre` → always, `por-patrón` → on-match con su `fileMatch`, `manual`) y los metadatos de ratificación opcionales.

#### Scenario: Rumbo de inclusión por patrón
- **WHEN** el motor parsea un archivo de rumbo con `inclusion: por-patrón` y `fileMatch: ["src/**"]`
- **THEN** el modelo refleja inclusión por patrón con la lista de globs

### Requirement: Diagnóstico de requisito sin escenario
El motor SHALL emitir un diagnóstico con la ubicación del requisito cuando un `### Requirement:` no tenga ningún escenario reconocido.

#### Scenario: Requisito sin escenarios
- **WHEN** el motor valida una spec cuyo requisito no tiene escenarios
- **THEN** emite un diagnóstico con la regla "requisito sin escenario", el archivo y la línea del requisito

### Requirement: Diagnóstico de cabecera de delta no canónica
El motor SHALL emitir un diagnóstico cuando un archivo de delta use una cabecera de operación fuera del canon `## ADDED / MODIFIED / REMOVED / RENAMED Requirements`.

#### Scenario: Cabecera de delta en español
- **WHEN** el motor valida un delta con la cabecera `## AÑADIDO Requirements`
- **THEN** emite un diagnóstico de cabecera de delta no reconocida con su ubicación

### Requirement: Diagnóstico de nombre de capacidad inválido
El motor SHALL emitir un diagnóstico cuando el nombre de una capacidad de la verdad viva no sea kebab-case.

#### Scenario: Capacidad con mayúsculas
- **WHEN** existe una capacidad cuyo directorio contiene mayúsculas o espacios
- **THEN** el motor emite un diagnóstico de nombre de capacidad inválido

### Requirement: Diagnósticos con ubicación estructurada
Cada diagnóstico SHALL identificar el archivo, la línea y una regla estable legible por máquina, además del mensaje. Una validación sin problemas SHALL devolver una lista de diagnósticos vacía.

#### Scenario: Artefacto conforme
- **WHEN** el motor valida un artefacto que cumple todas las reglas estructurales
- **THEN** devuelve una lista de diagnósticos vacía

### Requirement: Conformidad de los artefactos del propio proyecto
El motor SHALL validar sin diagnósticos los artefactos del método del propio repositorio Meltemi (la constitución, los archivos de rumbo y las specs vivas del formato canónico).

#### Scenario: Dogfooding del formato
- **WHEN** el motor descubre y valida los artefactos `.meltemi/` y las specs vivas del repositorio Meltemi
- **THEN** no emite ningún diagnóstico estructural
