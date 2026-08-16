# own-adapters — delta

## ADDED Requirements

### Requirement: Las preguntas del agente se relevan como peticiones con opciones

WHERE el CLI pilotado pide a una persona que elija entre opciones, el adaptador
SHALL relevar la pregunta como `session/request_permission` con **las opciones
del agente**, conservando el rótulo de cada una tal como el agente lo envió, y
SHALL devolver la elección por el canal de respuesta del proveedor. El rehúso
por interacción no relevable SHALL conservarse para toda herramienta que
efectivamente no pueda relevarse.

Una llamada aprobada SHALL seguir corriendo exactamente como se pidió. WHERE la
herramienta relevada es una pregunta —y solo ahí—, la respuesta elegida SHALL
poder completar el campo de respuesta que el propio input declara: en una
pregunta el input **es** el formulario, y completarlo no es reescribir lo que el
agente iba a hacer. Ninguna otra herramienta SHALL ver su input alterado.

WHERE el canal admite una sola respuesta por petición, el adaptador SHALL relevar
cada pregunta por separado y NO SHALL fingir selección múltiple.

Una petición que no ofrece forma de **rehusar** no es una petición de permiso: es
una pregunta, y sus opciones son respuestas. Las reglas de permisos NO SHALL
decidirla —no tienen opinión sobre cuál respuesta es correcta— y SHALL escalar a
una persona.

#### Scenario: Una pregunta llega con las opciones del agente

- **WHEN** el CLI pilotado pregunta con opciones
- **THEN** la petición SHALL llegar como `session/request_permission` con esas
  opciones y sus rótulos
- **AND** la elección SHALL volver al CLI por su canal de respuesta

#### Scenario: Solo una pregunta completa su propio input

- **WHEN** se aprueba una herramienta que no es una pregunta
- **THEN** su input SHALL viajar sin alteración alguna

#### Scenario: Una regla no contesta una pregunta por ti

- **WHERE** una regla de permisos resolvería toda petición
- **WHEN** llega una pregunta, que no ofrece forma de rehusar
- **THEN** SHALL escalar a una persona igualmente
- **AND** la regla NO SHALL elegir una respuesta

#### Scenario: Lo que de verdad no se puede relevar se sigue rehusando

- **WHERE** una herramienta exige una interacción que esta superficie no puede
  ofrecer
- **THEN** SHALL seguir denegándose con su motivo visible
- **AND** el adaptador SHALL NOT aprobarla por su cuenta
