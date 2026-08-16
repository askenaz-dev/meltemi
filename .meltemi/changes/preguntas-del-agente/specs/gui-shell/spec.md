# gui-shell — delta

## ADDED Requirements

### Requirement: La pregunta se contesta donde se escribe

Mientras una sesión espera una decisión, la zona del compositor SHALL presentar
la petición y sus opciones como un listado recorrible con teclado y activable
con Enter, con el rótulo de cada opción tal como el agente lo envió. SHALL
decidir por el mismo verbo que la bandeja, y NO SHALL constituir una segunda
cola. La tarjeta del transcript SHALL conservarse: el registro es la verdad.

La aparición NO SHALL animar el layout: nada se mueve bajo el cursor mientras se
decide. WHERE las opciones no caben, el listado SHALL desplazarse dentro de sí
mismo y NO SHALL desplazar el panel.

La última opción SHALL ser una salida de texto libre, y su rótulo SHALL decir
qué hará realmente según el canal de la sesión: responder la pregunta, o
interrumpir el turno y relevarlo con el texto.

#### Scenario: La pregunta aparece en el compositor y se contesta con teclado

- **WHILE** una sesión espera una decisión
- **THEN** el compositor SHALL presentar sus opciones recorribles con teclado
- **AND** activar una SHALL decidir por el mismo verbo que la bandeja

#### Scenario: La pregunta aparece sin mover nada

- **WHEN** la petición llega
- **THEN** NO SHALL animarse el layout
- **AND** ningún control SHALL desplazarse bajo el cursor

#### Scenario: Muchas opciones se desplazan dentro del listado

- **WHERE** las opciones exceden el alto disponible
- **THEN** el listado SHALL desplazarse dentro de sí mismo
- **AND** el panel NO SHALL desplazarse

#### Scenario: La salida de texto libre dice lo que hará

- **WHEN** se ofrece la salida de texto libre
- **THEN** su rótulo SHALL decir si responde la pregunta o interrumpe el turno
  para relevarlo
