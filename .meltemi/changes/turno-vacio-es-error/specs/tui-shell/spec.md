# tui-shell — delta

## ADDED Requirements

### Requirement: El turno vacío se dice en el transcript
WHEN el transcript en vivo de una sesión recibe un turno vacío, o el detalle de
una sesión finalizada lee uno de su log, la TUI SHALL mostrarlo con palabras,
en el idioma de la interfaz, junto al remedio en condicional, y NO SHALL
reducirlo al nombre crudo del evento.

#### Scenario: El transcript nombra el turno vacío
- **WHEN** llega un evento `turn_empty` de una sesión en vivo
- **THEN** la línea del transcript SHALL decir «turno vacío» en español, o su equivalente en inglés, con el remedio

#### Scenario: El detalle histórico nombra el turno vacío
- **WHEN** el detalle de una sesión finalizada lee una línea `turn_empty` de su log
- **THEN** la línea SHALL decir «turno vacío» con el remedio, no el nombre crudo del evento
