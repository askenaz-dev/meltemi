# cli-contract — delta

## MODIFIED Requirements

### Requirement: Taxonomía de códigos de salida
El binario SHALL usar una taxonomía de códigos de salida estable: `0` éxito, `1`
error interno inesperado, `2` error de uso, `10` daemon inalcanzable, `11` error
de contrato, `12` operación rechazada por política, `13` operación cancelada,
`14` validación con hallazgos, `15` turno del agente terminado sin decir ni
hacer nada. Todo subcomando MUST terminar con el código que corresponde a su
desenlace.

#### Scenario: Éxito termina en cero
- **WHEN** un subcomando operativo completa su cometido sin error
- **THEN** el binario SHALL terminar con el código `0`

#### Scenario: Daemon inalcanzable
- **WHEN** un subcomando respaldado por RPC no logra conectar ni arrancar el daemon
- **THEN** el binario SHALL terminar con el código `10`

#### Scenario: Respuesta de error del contrato
- **WHERE** el daemon responde con un error JSON-RPC o rechaza la versión de protocolo
- **THEN** el binario SHALL terminar con el código `11`

#### Scenario: Validación con hallazgos distinguible
- **WHEN** una validación concluye con diagnósticos
- **THEN** el binario SHALL terminar con el código `14`
- **AND** una validación limpia SHALL terminar con el código `0`

#### Scenario: Turno vacío distinguible
- **WHEN** un subcomando recibe una respuesta que declara vacío el turno que informa
- **THEN** el binario SHALL terminar con el código `15`
- **AND** el mismo subcomando con un turno que llevó trabajo SHALL terminar con el código `0`

## ADDED Requirements

### Requirement: Un turno vacío se advierte con remedio
WHEN la respuesta de un subcomando declara vacío el turno que informa, el
binario SHALL escribir en stderr, en modo humano, una advertencia que diga que
el agente terminó sin decir ni hacer nada y un remedio en condicional: probar el
agente por su cuenta y revisar su proveedor y sus credenciales. En modo `--json`
la respuesta SHALL viajar íntegra por stdout, con el campo que declara el turno
vacío. La regla SHALL depender del campo de la respuesta y no de una lista de
subcomandos.

#### Scenario: La advertencia va a stderr con su remedio
- **WHEN** `meltemi session` termina con un turno vacío en modo humano
- **THEN** stderr SHALL contener la advertencia y el remedio
- **AND** stdout SHALL contener solo la salida útil del comando

#### Scenario: La salida de máquina lleva el veredicto
- **WHEN** un subcomando que arranca sesión termina con un turno vacío en modo `--json`
- **THEN** stdout SHALL contener la respuesta con `emptyTurn` en `true`

#### Scenario: Un verbo de autoría sigue la misma regla
- **WHEN** `meltemi plan` recibe una respuesta con `emptyTurn` en `true`
- **THEN** el binario SHALL advertir en stderr y terminar con el código `15`
