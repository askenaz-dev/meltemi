# method-bootstrap — Gobernanza del método durante el bootstrap

## ADDED Requirements

### Requirement: Excepción interina del método (bootstrap en dos etapas)
Mientras el motor de specs de Fase 1 no permita a Meltemi hospedar sus propios cambios, toda modificación de los documentos fundacionales (`meltemi.md`, `.meltemi/constitution.md`, `.meltemi/rumbo/`) SHALL tramitarse como una propuesta de cambio en `openspec/changes/`, y NO en `.meltemi/changes/`. La migración del método a `.meltemi/` SHALL realizarse mediante una change dedicada (`migracion-openspec-a-meltemi`).

#### Scenario: Enmienda a un documento ratificado durante el bootstrap
- **WHEN** se necesita modificar `meltemi.md` o cualquier artefacto ratificado de `.meltemi/`
- **THEN** el cambio entra como una propuesta en `openspec/changes/`, se aprueba por el mantenedor fundador y se aplica desde ahí

#### Scenario: Cierre de la etapa de bootstrap
- **WHEN** el motor de specs de Fase 1 tiene el ciclo `/archive` operativo sobre `.meltemi/`
- **THEN** la migración de `openspec/` a `.meltemi/` se tramita como la change `migracion-openspec-a-meltemi`, tras la cual las enmiendas fundacionales pasan a vivir en `.meltemi/changes/`

### Requirement: Ratificación de enmiendas fundacionales
Toda enmienda a un documento fundacional ratificado SHALL requerir la aprobación explícita del mantenedor fundador antes de aplicarse, y NO SHALL auto-ratificarse por la herramienta que la redacta.

#### Scenario: Enmienda pendiente de ratificación
- **WHEN** una propuesta de cambio modifica un documento fundacional ratificado
- **THEN** la nueva versión del documento se marca como "pendiente de ratificación" hasta que el mantenedor fundador la ratifique explícitamente
