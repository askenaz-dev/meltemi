# worktree-orchestration — delta

## ADDED Requirements

### Requirement: El despacho dice si el turno fue vacío
WHEN el turno del agente despachado se declara vacío, el resultado de
`worktree/dispatch` SHALL traer `emptyTurn` en `true` junto a su estado, y
SHALL omitir el campo cuando el turno llevó trabajo.

#### Scenario: Despacho con un turno vacío
- **WHEN** el agente despachado cierra su turno sin decir ni hacer nada
- **THEN** el resultado del despacho SHALL traer `emptyTurn` en `true`
- **AND** el log de la sesión del despacho SHALL registrar `turn_empty`
