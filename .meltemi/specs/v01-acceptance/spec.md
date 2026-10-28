# v01-acceptance Specification

## Purpose
TBD - created by archiving change hito-v01-aceptacion. Update Purpose after archive.
## Requirements

### Requirement: El guion del hito como verificación ejecutable
El proyecto SHALL mantener una verificación ejecutable del hito v0.1 que recorra
el ciclo completo sobre un repositorio fixture: proponer una idea, revisar specs
con al menos una reelaboración por comentario, implementar con dos agentes de
perfiles distintos en paralelo sobre worktrees, verificar contra los escenarios y
archivar fundiendo la verdad viva; en CI MUST ejecutarse con agentes simulados y
sin red.

#### Scenario: Ciclo completo en terminal
- **WHEN** corre la verificación del hito
- **THEN** el fixture SHALL terminar con la funcionalidad implementada, verificada y archivada
- **AND** todos los pasos SHALL haberse ejecutado por las superficies del producto

#### Scenario: Paralelismo real de dos agentes
- **WHILE** la fase de implementación corre
- **THEN** dos agentes de perfiles distintos SHALL trabajar en paralelo en worktrees separados
- **AND** sus commits SHALL conservar la trazabilidad por tarea

### Requirement: Validación manual del mantenedor documentada
El guion equivalente con agentes reales SHALL estar documentado paso a paso para
la validación manual del mantenedor, y su resultado MUST registrarse en el
informe de aceptación; la aceptación del hito MUST incluir ambas corridas
(automatizada y manual).

#### Scenario: Corrida manual registrada
- **WHEN** el mantenedor completa el guion con agentes reales
- **THEN** el informe SHALL registrar fecha, agentes usados y resultado por paso

### Requirement: Métricas del hito verificadas
Los presupuestos aplicables del documento fundacional SHALL verificarse en el
pipeline de release (arranque y tamaño del binario de la TUI) y sus valores MUST
constar en el informe de aceptación.

#### Scenario: Presupuestos en el informe
- **WHEN** se genera el informe de aceptación
- **THEN** SHALL incluir los valores medidos frente a sus presupuestos

### Requirement: Informe de aceptación reproducible
La corrida de aceptación SHALL producir un informe reproducible (qué se ejecutó,
versiones, resultado por criterio, desviaciones) que acompaña al tag v0.1;
regenerarlo desde el mismo commit MUST producir el mismo veredicto.

#### Scenario: Informe acompaña al tag
- **WHEN** se publica v0.1
- **THEN** el informe de aceptación SHALL publicarse junto al tag con veredicto por criterio

### Requirement: El informe de la corrida manual está publicado
El informe de la corrida manual con agentes reales SHALL publicarse en
`docs/qa/` con su fecha, los agentes y sus versiones, la versión del producto,
un veredicto por cada criterio del guion (C1–C6) y las desviaciones que la
corrida encontró; `docs/hito-v01.md` SHALL enlazarlo en vez de quedarse en
plantilla.

#### Scenario: El informe de aceptación existe con su veredicto
- **WHEN** se consulta el informe de aceptación v0.1
- **THEN** SHALL registrar fecha, agentes y sus versiones
- **AND** SHALL dar un veredicto por cada criterio del guion

#### Scenario: El hito enlaza su informe
- **WHEN** se consulta `docs/hito-v01.md`
- **THEN** SHALL enlazar el informe de la corrida manual con agentes reales
