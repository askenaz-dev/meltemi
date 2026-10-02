# tui-shell — delta

## ADDED Requirements

### Requirement: Las máquinas se eligen desde el terminal

La paleta del terminal SHALL ofrecer listar las máquinas configuradas y hacer
activa a otra, con descripción en ambos idiomas, y SHALL decir en su cabecera
en qué máquina está. Cambiar de máquina SHALL reconectar sin reiniciar el
proceso. Un nombre que no está en la lista SHALL rehusarse nombrando los que sí
están.

#### Scenario: Máquinas desde la paleta

- **WHEN** se invoca el verbo de máquinas desde la paleta
- **THEN** el terminal SHALL mostrar cada máquina con su endpoint y cuál es la
  activa

#### Scenario: Cambiar de máquina desde la paleta reconecta

- **WHEN** se nombra otra máquina de la lista
- **THEN** el terminal SHALL reconectar contra su endpoint sin reiniciarse
- **AND** un nombre que no está SHALL rehusarse nombrando los que sí están
