# cli-contract — delta

## ADDED Requirements

### Requirement: Todo verbo acepta el endpoint con forma de comando

Cualquier verbo del CLI SHALL operar con un endpoint que nombre un comando,
igual que con uno que nombre un socket, sin bandera adicional y sin sintaxis
propia por verbo.

#### Scenario: Un verbo cualquiera opera a través del comando

- **WHEN** se declara un endpoint con forma de comando y se corre un verbo que consulta al daemon
- **THEN** el verbo responde lo mismo que respondería contra un daemon local
