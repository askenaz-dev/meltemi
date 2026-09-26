# v01-acceptance — delta

## ADDED Requirements

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
