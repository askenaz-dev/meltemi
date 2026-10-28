# tui-shell — delta

## ADDED Requirements

### Requirement: Diagnóstico desde la paleta
La paleta SHALL ofrecer `doctor` (y `diagnostico`), que muestre el mismo informe
que el subcomando, con estado en símbolo y palabra, también cuando el daemon no
responde.

#### Scenario: La paleta muestra el diagnóstico, también sin daemon
- **WHEN** el usuario escribe `doctor` en la paleta
- **THEN** la TUI SHALL mostrar el informe
- **AND** con el daemon caído SHALL mostrarlo igual, con esa fila en falla
