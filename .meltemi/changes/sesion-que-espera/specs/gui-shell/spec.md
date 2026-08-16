# gui-shell — delta

## ADDED Requirements

### Requirement: La conversación sigue viva tras el turno

Terminado un turno sin que la sesión termine, la superficie SHALL declarar la
sesión en espera con símbolo y palabra —nunca con color solo— y SHALL mantener
el compositor utilizable, sin rótulo de reanudación. El indicador de trabajo NO
SHALL animarse en espera: esperar no es trabajar.

Toda superficie que declare el estado de una sesión SHALL declarar también el
estado de espera; ninguna SHALL representarlo como terminada ni omitirlo.

#### Scenario: El compositor no muere al terminar el turno

- **WHEN** un turno termina y la sesión queda esperando
- **THEN** el compositor SHALL seguir aceptando la siguiente instrucción
- **AND** NO SHALL ofrecer reanudar

#### Scenario: Esperar no enciende el indicador de trabajo

- **WHILE** una sesión espera instrucciones
- **THEN** el indicador de trabajo NO SHALL animarse

#### Scenario: Ninguna superficie omite el estado de espera

- **WHEN** se recorre cada superficie que declara estado de sesión
- **THEN** todas SHALL tener símbolo y palabra para la espera
- **AND** ninguna SHALL pintarla como terminada
