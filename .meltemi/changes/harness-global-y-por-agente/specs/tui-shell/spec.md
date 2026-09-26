# tui-shell — delta

## ADDED Requirements

### Requirement: El terminal también lee el harness efectivo

La superficie de terminal SHALL ofrecer la misma lectura del harness efectivo
que las demás superficies, con la capa de origen de cada pieza y con lo pisado
y lo inválido incluidos.

#### Scenario: El terminal muestra el harness con su origen

- **WHEN** se consulta el harness desde el terminal
- **THEN** SHALL mostrarse cada pieza con su capa de origen
