# propose-flow — delta

## ADDED Requirements

### Requirement: La propuesta dice si el turno del agente fue vacío
WHEN el turno del agente que redacta la propuesta se declara vacío, el
resultado de `propose` SHALL traer `emptyTurn` en `true` junto a su estado, y
SHALL omitir el campo cuando el turno llevó trabajo.

#### Scenario: Propuesta con un turno vacío
- **WHEN** el agente cierra el turno de la propuesta sin decir ni hacer nada
- **THEN** el resultado de `propose` SHALL traer `emptyTurn` en `true`
