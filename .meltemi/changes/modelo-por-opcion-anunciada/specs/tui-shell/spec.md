# tui-shell — delta

## MODIFIED Requirements

### Requirement: El terminal declara el modelo efectivo

La TUI SHALL mostrar el modelo y el esfuerzo efectivos de una sesión donde
muestra su estado, y SHALL omitirlos cuando la sesión no declaró ninguno en vez
de mostrar un valor inventado. Cuando una palanca se rehusó al abrir, la
transcripción SHALL decir la palanca, lo pedido y el motivo, en español y en
inglés, y el modelo rehusado NO SHALL mostrarse como el que rigió.

#### Scenario: El terminal muestra el modelo efectivo

- **WHEN** una sesión corre con modelo declarado
- **THEN** la TUI SHALL mostrarlo
- **AND** una sesión sin modelo declarado NO SHALL mostrar uno

#### Scenario: El terminal dice la palanca rehusada

- **WHEN** una sesión termina rehusando una palanca al abrir
- **THEN** la transcripción SHALL decir la palanca, lo pedido y el motivo
