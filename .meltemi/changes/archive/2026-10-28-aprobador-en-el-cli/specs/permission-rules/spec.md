# permission-rules — delta

## ADDED Requirements

### Requirement: La petición en vivo lleva su id en la cola
La petición de permiso que el daemon empuja al cliente SHALL llevar el id de esa
petición en la cola de pendientes, de modo que el mismo cliente pueda decidirla
o recordarla por id y reconocer cuándo se resolvió por otra vía.

#### Scenario: La petición en vivo nombra su id
- **WHEN** el daemon empuja una petición de permiso a un cliente
- **THEN** la petición SHALL llevar el id que la cola de pendientes le asignó

### Requirement: La regla creada desde el terminal es la más específica posible
WHEN el humano aprueba desde el aprobador del terminal pidiendo recordar la
decisión, el binario SHALL proponer la regla más específica que permiten los
hechos que el daemon lee de la petición —la herramienta con el comando exacto
o, sin comando, con la ruta exacta, o la sugerencia del daemon cuando es al
menos igual de específica—, SHALL nombrar su efecto, sus condiciones, su ámbito
y su archivo, SHALL exigir una segunda tecla de confirmación, SHALL persistirla
por la misma vía que la bandeja y SHALL decir que rige desde la siguiente
sesión abierta sin `--mode`. El binario NO SHALL ofrecer regla cuando la
petición no nombra herramienta, cuando no trae ni comando ni ruta, cuando la
ruta está dentro de `.meltemi/`, cuando el agente no ofreció opción de rechazo
ni «permitir una vez», o cuando el comando trabaja en un worktree de tarea.

#### Scenario: La regla del terminal gobierna la siguiente sesión
- **WHEN** el humano aprueba y recuerda desde el terminal una petición que nombra su herramienta y su ruta
- **THEN** la regla SHALL persistirse en el ámbito del proyecto con esa herramienta y esa ruta
- **AND** la siguiente sesión sin `--mode` SHALL resolver la misma petición por esa regla sin preguntar

#### Scenario: Sin herramienta no se ofrece regla
- **WHEN** la petición no trae ni clase ni título
- **THEN** el aprobador NO SHALL ofrecer recordar la decisión

#### Scenario: Sin comando ni ruta no se ofrece regla
- **WHEN** la petición nombra su herramienta pero no trae ni comando ni ruta
- **THEN** el aprobador NO SHALL ofrecer recordar la decisión

#### Scenario: Ninguna regla del terminal alcanza la configuración de Meltemi
- **WHEN** la ruta de la petición está dentro de `.meltemi/`
- **THEN** el aprobador NO SHALL ofrecer recordar la decisión

#### Scenario: En un worktree de tarea no se ofrece regla
- **WHEN** el comando trabaja en un worktree de tarea
- **THEN** el aprobador NO SHALL ofrecer recordar la decisión
