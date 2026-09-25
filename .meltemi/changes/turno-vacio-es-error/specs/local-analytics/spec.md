# local-analytics — delta

## ADDED Requirements

### Requirement: Los turnos vacíos se cuentan
La actividad de cada celda de la contabilidad local SHALL incluir cuántos
turnos se declararon vacíos en los registros que agrega, y las superficies que
muestran la actividad SHALL mostrarlo cuando no es cero.

#### Scenario: La actividad cuenta los turnos vacíos
- **WHEN** se agregan registros de sesión que contienen turnos vacíos
- **THEN** la actividad de la celda SHALL reportar su número
- **AND** una celda sin turnos vacíos SHALL reportar cero
