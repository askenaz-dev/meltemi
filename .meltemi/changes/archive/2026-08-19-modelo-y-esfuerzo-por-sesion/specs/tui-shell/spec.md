# tui-shell — delta

## ADDED Requirements

### Requirement: El terminal declara el modelo efectivo

La TUI SHALL mostrar el modelo y el esfuerzo efectivos de una sesión donde
muestra su estado, y SHALL omitirlos cuando la sesión no declaró ninguno en vez
de mostrar un valor inventado.

#### Scenario: El terminal muestra el modelo efectivo

- **WHEN** una sesión corre con modelo declarado
- **THEN** la TUI SHALL mostrarlo
- **AND** una sesión sin modelo declarado NO SHALL mostrar uno
