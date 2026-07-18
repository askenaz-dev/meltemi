# community-governance Specification

## Purpose
TBD - created by archiving change gobernanza-comunidad. Update Purpose after archive.
## Requirements
### Requirement: Documentos de gobernanza presentes y completos
El repositorio SHALL contener en su raíz GOVERNANCE, CONTRIBUTING,
CODE_OF_CONDUCT y SECURITY con las secciones mínimas que esta capacidad define, y
un lint de CI MUST verificar su presencia y secciones. GOVERNANCE SHALL declarar
el modelo vigente (mantenedor fundador, ratificación de enmiendas conforme a
`method-bootstrap`) y los criterios declarados para incorporar mantenedores.

#### Scenario: Lint de presencia y secciones
- **WHEN** corre el lint de gobernanza en CI
- **THEN** SHALL fallar si falta un documento o una sección mínima

#### Scenario: Gobernanza refleja la realidad
- **WHEN** un lector consulta GOVERNANCE
- **THEN** SHALL encontrar quién decide, cómo se ratifican enmiendas y cómo se llega a mantenedor

### Requirement: Contribución spec-driven
CONTRIBUTING SHALL establecer que toda funcionalidad entra como propuesta de
cambio con sus artefactos, con la vía corta explícita para correcciones
triviales; la plantilla de PR MUST incluir la checklist de calidad (change
enlazada, clippy/fmt/tests en tres plataformas, cabecera SPDX, convención de
commits) y MUST declarar la prohibición de trailers de co-autoría.

#### Scenario: PR de feature sin change
- **WHEN** un PR de funcionalidad llega sin propuesta de cambio enlazada
- **THEN** la plantilla SHALL requerirla explícitamente antes de la revisión

#### Scenario: Vía corta para lo trivial
- **WHERE** la contribución es una corrección trivial declarada
- **THEN** CONTRIBUTING SHALL permitirla sin artefactos completos

### Requirement: Texto del CLA acotado
El repositorio SHALL contener el texto del acuerdo de contribución acotado:
licencia de la contribución bajo Apache-2.0 con concesión de patentes y sin
cesión de copyright; el mecanismo de firma queda fuera y su decisión SHALL
constar como pendiente del mantenedor.

#### Scenario: CLA presente y acotado
- **WHEN** un contribuidor consulta el CLA
- **THEN** SHALL encontrar el alcance Apache-2.0 sin cesión de copyright

### Requirement: Política de seguridad publicada
SECURITY SHALL documentar la divulgación responsable (canal privado, alcance
alineado al modelo de amenaza del documento fundacional, tiempos de respuesta
honestos) y MUST NOT prometer plazos o programas inexistentes.

#### Scenario: Reporte responsable encaminado
- **WHEN** alguien encuentra una vulnerabilidad
- **THEN** SECURITY SHALL indicarle el canal privado y qué esperar

