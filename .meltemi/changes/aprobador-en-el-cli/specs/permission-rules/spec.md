# permission-rules — delta

## ADDED Requirements

### Requirement: La petición en vivo lleva su id en la cola
La petición de permiso que el daemon empuja al cliente SHALL llevar el id de esa
petición en la cola de pendientes, de modo que el mismo cliente pueda decidirla
o recordarla por id y reconocer cuándo se resolvió por otra vía.

#### Scenario: La petición en vivo nombra su id
- **WHEN** el daemon empuja una petición de permiso a un cliente
- **THEN** la petición SHALL llevar el id que la cola de pendientes le asignó

### Requirement: La regla creada desde el terminal es la de la bandeja
WHEN el humano aprueba desde el aprobador del terminal pidiendo recordar la
decisión, el binario SHALL proponer la misma regla que la bandeja —la sugerencia
del daemon o, sin ella, permitir la herramienta en el ámbito del proyecto—,
SHALL exigir una segunda tecla de confirmación, SHALL persistirla por la misma
vía que la bandeja y SHALL decir que rige desde la siguiente sesión. Una
petición sin herramienta conocida NO SHALL ofrecer regla.

#### Scenario: La regla del terminal gobierna la siguiente sesión
- **WHEN** el humano aprueba y recuerda la decisión desde el terminal
- **THEN** la regla SHALL persistirse en el ámbito del proyecto con el formato de la bandeja
- **AND** en la siguiente sesión la misma petición SHALL resolverse por esa regla sin preguntar

#### Scenario: Sin herramienta no se ofrece regla
- **WHEN** la petición no trae ni clase ni título
- **THEN** el aprobador NO SHALL ofrecer recordar la decisión
