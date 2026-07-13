# artifact-format — Formato canónico de los artefactos `.meltemi/`

## ADDED Requirements

### Requirement: Política de idioma híbrida
Los artefactos del método SHALL usar palabras clave estructurales y normativas en inglés y prosa descriptiva en español neutro. El motor de specs SHALL reconocer únicamente las palabras clave inglesas al parsear; el idioma de la prosa no afecta al parseo.

#### Scenario: Estructura en inglés, prosa en español
- **WHEN** el motor parsea un `spec.md` con cabeceras y palabras EARS en inglés y descripciones en español
- **THEN** lo acepta como válido y extrae requisitos y escenarios sin depender del idioma de la prosa

#### Scenario: Palabra clave estructural en español
- **WHEN** un artefacto usa una cabecera de delta en español (p. ej. `## AÑADIDO Requirements`)
- **THEN** el motor la reporta como no reconocida y señala la cabecera canónica esperada

### Requirement: Cabeceras de operación de delta
Un archivo de delta SHALL agrupar sus cambios bajo las cabeceras canónicas `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` y `## RENAMED Requirements`, y NO SHALL usar otras variantes.

#### Scenario: Delta con requisitos añadidos
- **WHEN** un cambio introduce capacidades nuevas
- **THEN** sus requisitos aparecen bajo `## ADDED Requirements`

#### Scenario: Cabecera de delta desconocida
- **WHEN** un delta usa una cabecera fuera del canon (p. ej. `## NEW Requirements`)
- **THEN** el motor la rechaza con un error que nombra las cabeceras válidas

### Requirement: Sintaxis de requisito y escenario
Cada requisito SHALL declararse como `### Requirement: <nombre>` y contener **al menos un** escenario declarado como `#### Scenario: <nombre>` con exactamente cuatro `#`. Un requisito sin escenarios SHALL ser inválido.

#### Scenario: Requisito con un escenario
- **WHEN** un requisito declara `### Requirement:` seguido de al menos un `#### Scenario:`
- **THEN** el motor lo acepta

#### Scenario: Requisito sin escenarios
- **WHEN** un requisito no declara ningún `#### Scenario:`
- **THEN** el motor lo reporta como inválido

#### Scenario: Escenario con nivel de encabezado incorrecto
- **WHEN** un escenario se declara con tres `#` en lugar de cuatro
- **THEN** el motor no lo reconoce como escenario y el requisito queda sin escenarios válidos

### Requirement: Canon de palabras clave EARS
Los requisitos normativos SHALL expresarse con los patrones EARS en inglés — `WHEN` (evento), `WHILE` (estado), `IF … THEN` (no deseado), `WHERE` (opcional) y el ubicuo — usando el verbo normativo `SHALL` o `MUST`, y SHOULD evitar `should`/`may`. Los pasos de escenario SHALL usar viñetas con marcadores `**WHEN**` y `**THEN**`.

#### Scenario: Escenario en forma WHEN/THEN
- **WHEN** un escenario lista una condición `- **WHEN** …` y un resultado `- **THEN** …`
- **THEN** el motor lo reconoce como un caso verificable

### Requirement: Estructura y nombres de artefactos
El directorio `.meltemi/` SHALL seguir la estructura canónica: `constitution.md`; `rumbo/` con `product.md`, `tech.md`, `structure.md` y otros `*.md`; `specs/<capability>/spec.md` como verdad viva; `changes/<change-name>/` con `proposal.md`, `requirements.md`, `design.md`, `specs/` y `tasks.md`; y `changes/archive/<YYYY-MM-DD-change-name>/`. Los nombres de capacidad SHALL ser kebab-case.

#### Scenario: Capacidad en la verdad viva
- **WHEN** existe `specs/<capability>/spec.md` con `<capability>` en kebab-case
- **THEN** el motor la reconoce como una capacidad de la verdad viva

#### Scenario: Nombre de capacidad no kebab-case
- **WHEN** un directorio de capacidad usa espacios o mayúsculas
- **THEN** el motor lo reporta como nombre inválido

### Requirement: Front-matter de los archivos de rumbo
Cada archivo de `rumbo/` SHALL declarar front-matter YAML con `inclusion: siempre | por-patrón | manual`. Cuando `inclusion` sea `por-patrón`, el front-matter SHALL incluir `fileMatch` con una lista de globs. Los campos `ratificado` y `ratificador` son opcionales.

#### Scenario: Rumbo de inclusión siempre
- **WHEN** un archivo de rumbo declara `inclusion: siempre`
- **THEN** el motor lo incluye como contexto en toda sesión

#### Scenario: Inclusión por patrón sin fileMatch
- **WHEN** un archivo declara `inclusion: por-patrón` pero omite `fileMatch`
- **THEN** el motor lo reporta como front-matter inválido

### Requirement: Reglas de contenido de delta
Un delta `MODIFIED` SHALL incluir el bloque completo del requisito actualizado; un delta `REMOVED` SHALL incluir los campos **Reason** y **Migration**; un delta `RENAMED` SHALL usar el formato `FROM:` / `TO:`.

#### Scenario: Requisito modificado con bloque parcial
- **WHEN** un delta `MODIFIED` incluye solo un fragmento del requisito en lugar del bloque completo
- **THEN** el motor lo reporta como delta inválido

#### Scenario: Requisito eliminado sin migración
- **WHEN** un delta `REMOVED` omite el campo **Migration**
- **THEN** el motor lo reporta como delta inválido
