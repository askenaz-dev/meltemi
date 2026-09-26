# cli-contract — delta

## ADDED Requirements

### Requirement: `sessions --follow` imprime las transiciones de estado

El verbo `sessions` SHALL aceptar `--follow`, y con él SHALL escribir cada
afirmación de estado que el daemon emita como **un registro por línea**, en
todo formato de salida —un documento YAML por registro bajo el formato YAML—,
hasta que el usuario corte o el daemon se vaya. Un error a mitad del flujo
SHALL cerrarlo con su objeto de error, como los demás flujos del CLI. El
flujo SHALL comenzar por el estado actual de cada sesión conocida, de modo que
quien lo lea desde el primer momento no necesite consultar nada más.

`sessions` sin `--follow` SHALL seguir respondiendo exactamente como hoy: la
lista completa, una vez.

#### Scenario: Cada transición es una línea

- **WHEN** se corre `sessions --follow` y una sesión transita de estado
- **THEN** se escribe un registro por línea con el estado afirmado
- **AND** el primer bloque de registros es el estado actual de cada sesión conocida

#### Scenario: El flujo termina con su daemon

- **WHEN** el daemon se detiene mientras `sessions --follow` corre
- **THEN** el flujo termina con el código que el CLI reserva para un daemon que se fue
- **AND** no queda el proceso esperando indefinidamente

#### Scenario: Sin la bandera nada cambia

- **WHEN** se corre `sessions` sin `--follow`
- **THEN** responde la lista completa una vez y termina, igual que antes de esta change
