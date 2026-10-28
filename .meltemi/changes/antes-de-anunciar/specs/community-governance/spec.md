# community-governance — delta

## ADDED Requirements

### Requirement: Plantillas de issue por tipo
El repositorio SHALL ofrecer una plantilla de issue para cada tipo de
contribución —un error, una pregunta y una propuesta de cambio— con las
secciones que cada tipo necesita, y SHALL NOT ofrecer el issue en blanco. La
plantilla de propuesta SHALL nombrar los artefactos del método en el orden de
la constitución, y la configuración de plantillas SHALL encaminar los reportes
de seguridad a la política publicada.

#### Scenario: Cada tipo tiene su plantilla con sus secciones
- **WHEN** corre el lint de gobernanza
- **THEN** SHALL fallar si falta la plantilla de error, la de pregunta o la de propuesta, o una de sus secciones

#### Scenario: La seguridad no va a un issue público
- **WHEN** alguien abre un issue
- **THEN** la configuración SHALL ofrecer el camino de la política de seguridad
- **AND** SHALL NOT ofrecer un issue en blanco
