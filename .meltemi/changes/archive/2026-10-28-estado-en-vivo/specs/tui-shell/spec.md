# tui-shell — delta

## ADDED Requirements

### Requirement: La TUI deja de sondear el estado de las sesiones

La fila de una sesión en la TUI SHALL reflejar el estado que el daemon notifica,
y la TUI NO SHALL depender de su reloj periódico para que ese estado sea
cierto. El reloj periódico SHALL seguir existiendo para lo que mide tiempo
—relojes y duraciones— y para reconciliar la lista al reconectar, nunca como
fuente del estado.

#### Scenario: La fila cambia al llegar la transición

- **WHEN** una sesión transita de estado mientras la TUI está abierta
- **THEN** su fila pasa a ese estado sin esperar el siguiente tic del reloj
- **AND** una sesión abierta desde otro cliente aparece y avanza igual que una abierta en la TUI

#### Scenario: El reloj sigue moviendo los relojes

- **WHEN** pasa un tic del reloj periódico sin que ninguna sesión transite
- **THEN** las duraciones mostradas avanzan
- **AND** no se consulta la lista de sesiones para saber en qué estado están
