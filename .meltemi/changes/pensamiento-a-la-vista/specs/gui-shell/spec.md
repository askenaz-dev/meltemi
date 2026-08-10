# gui-shell — delta

## ADDED Requirements

### Requirement: El pensamiento del turno se ve mientras ocurre

Mientras un turno está en vuelo, el pensamiento que el agente emite SHALL
mostrarse desplegado, sin exigir un gesto por turno; al cerrarse el turno SHALL
plegarse. El usuario SHALL poder plegarlo o desplegarlo en cualquier momento, y
plegarlo mientras el turno sigue en vuelo NO SHALL deshacerse solo. Cuando un
turno no emite pensamiento NO SHALL mostrarse sección alguna en su lugar.

#### Scenario: El pensamiento se ve mientras el turno corre

- **WHEN** un turno en vuelo emite pensamiento
- **THEN** SHALL mostrarse desplegado
- **AND** al cerrarse el turno SHALL quedar plegado

#### Scenario: Plegarlo a mano no se deshace solo

- **WHILE** un turno sigue en vuelo
- **WHEN** el usuario pliega su pensamiento
- **THEN** SHALL permanecer plegado aunque sigan llegando fragmentos

#### Scenario: Sin pensamiento no hay sección

- **WHEN** un turno no emite pensamiento
- **THEN** NO SHALL mostrarse encabezado ni marcador alguno en su lugar
