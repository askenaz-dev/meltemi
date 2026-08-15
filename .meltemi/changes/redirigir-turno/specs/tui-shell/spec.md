# tui-shell — delta

## ADDED Requirements

### Requirement: Interrumpir y enviar desde la conversación

El shell SHALL permitir dirigir una sesión pidiendo interrumpir el turno en
vuelo, y SHALL decir cuál de los dos desenlaces ocurrió: la instrucción quedó
encolada, o relevó al turno interrumpido.

#### Scenario: El shell dice si encoló o relevó

- **WHEN** se dirige una sesión pidiendo interrumpir
- **THEN** el shell SHALL decir si la instrucción quedó encolada o relevó al
  turno
