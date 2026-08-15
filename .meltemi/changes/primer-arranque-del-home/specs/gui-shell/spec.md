# gui-shell — delta

## ADDED Requirements

### Requirement: Primer arranque del compositor

Cuando la flota ha respondido y no hay ningún agente lanzable, el selector de
agente del compositor SHALL decirlo en su cara con tono de advertencia, antes
de que un envío sea rehusado, y NO SHALL ofrecer como elegible el agente por
defecto del proyecto. Su menú SHALL permitir abrir la vista de flota, no solo
nombrarla. En la primera llegada con agentes lanzables, el selector SHALL
reconocer cuántos hay una sola vez, y ese reconocimiento NO SHALL repetirse en
arranques posteriores.

#### Scenario: La flota vacía se dice antes de fallar

- **WHEN** la flota ha respondido y ningún agente es lanzable
- **THEN** el selector de agente SHALL decirlo en su cara con tono de
  advertencia
- **AND** NO SHALL ofrecer el agente por defecto como elegible

#### Scenario: El menú vacío abre la flota

- **WHEN** el menú del selector no ofrece ningún agente
- **THEN** SHALL permitir abrir la vista de flota desde él

#### Scenario: El reconocimiento se dice una vez

- **WHEN** el compositor llega por primera vez con agentes lanzables
- **THEN** SHALL mostrar cuántos hay
- **AND** NO SHALL volver a mostrarlo en arranques posteriores
