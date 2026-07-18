# method-bootstrap Specification

## Purpose
TBD - created by archiving change enmiendas-fundacionales-v1. Update Purpose after archive.
## Requirements
### Requirement: Excepción interina del método (bootstrap en dos etapas)
La etapa de bootstrap sobre `openspec/` SHALL quedar cerrada: los artefactos del
método del propio proyecto (specs vivas, changes activas e histórico) viven en
`.meltemi/` y toda modificación de los documentos fundacionales (`meltemi.md`,
`.meltemi/constitution.md`, `.meltemi/rumbo/`) SHALL tramitarse como una change
en `.meltemi/changes/` mediante el ciclo de Meltemi. La verdad viva migrada MUST
haber sido verificada por el motor como idéntica (requisitos y escenarios) a la
de la etapa anterior, y el histórico MUST preservarse íntegro.

#### Scenario: Enmienda fundacional post-migración
- **WHEN** se necesita modificar un documento fundacional tras la migración
- **THEN** el cambio SHALL entrar como change en `.meltemi/changes/` y seguir el ciclo de Meltemi con sus gates

#### Scenario: Verdad viva idéntica tras migrar
- **WHEN** la migración concluye
- **THEN** cada spec de `.meltemi/specs/` SHALL ser idéntica en requisitos y escenarios a su origen
- **AND** el histórico SHALL conservar fechas y contenido

### Requirement: Ratificación de enmiendas fundacionales
Toda enmienda a un documento fundacional ratificado SHALL requerir la aprobación explícita del mantenedor fundador antes de aplicarse, y NO SHALL auto-ratificarse por la herramienta que la redacta.

#### Scenario: Enmienda pendiente de ratificación
- **WHEN** una propuesta de cambio modifica un documento fundacional ratificado
- **THEN** la nueva versión del documento se marca como "pendiente de ratificación" hasta que el mantenedor fundador la ratifique explícitamente

### Requirement: El método del proyecto es su propio producto
El desarrollo de Meltemi SHALL usar los comandos de Meltemi (`propose`, `review`,
`verify`, `archive`) sobre `.meltemi/` como único método, y la herramienta
prestada de la etapa de bootstrap MUST quedar retirada de la configuración del
repositorio, sin referencias operativas restantes.

#### Scenario: Dogfooding definitivo
- **WHEN** se crea una change del propio proyecto tras la migración
- **THEN** SHALL crearse y tramitarse con los comandos de Meltemi sobre `.meltemi/`

#### Scenario: Sin referencias operativas a la etapa anterior
- **WHEN** corre el barrido de referencias en CI
- **THEN** SHALL NOT quedar invocaciones operativas de la herramienta prestada en el repositorio

