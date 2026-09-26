# cli-contract — delta

## ADDED Requirements

### Requirement: El harness efectivo se consulta desde la línea de comandos

La CLI SHALL ofrecer un verbo que muestre el harness efectivo, opcionalmente
acotado a un agente, con la capa de origen de cada pieza y con lo pisado y lo
inválido incluidos. SHALL respetar la disciplina de salida vigente: legible por
defecto, estructurada bajo `--json`, y los mismos códigos de salida que el
resto de los verbos de lectura.

#### Scenario: El verbo muestra de qué capa viene cada pieza

- **WHEN** se consulta el harness desde la CLI
- **THEN** cada pieza SHALL mostrarse con su capa de origen
- **AND** lo pisado y lo inválido SHALL aparecer con su motivo
