# acp-session — delta

## MODIFIED Requirements

### Requirement: Prompt con streaming de actualizaciones
`meltemid` SHALL enviar prompts a la sesión ACP y reenviar, en orden y en
tiempo real, todas las actualizaciones de sesión emitidas por el agente hasta
la finalización del turno. El reenvío SHALL alcanzar a la conexión que inició
la sesión sin que ésta pida nada, y a cualquier otra conexión que declare
mirar esa sesión; una conexión que no la mire NO SHALL recibir sus
actualizaciones. Cada actualización SHALL entregarse una sola vez por
conexión. El stream entrega lo emitido desde la suscripción: el histórico se
lee por el registro de sesión, y el daemon NO SHALL encolar eventos para
conexiones cerradas.

#### Scenario: Turno con actualizaciones
- **WHEN** el cliente envía un prompt y el agente emite actualizaciones de progreso antes de completar el turno
- **THEN** el cliente recibe cada actualización en el orden emitido, seguida de la señal de fin de turno

#### Scenario: Cliente que llega a mitad de sesión
- **WHEN** un cliente conecta con una sesión ya en curso que él no inició y declara mirarla
- **THEN** SHALL recibir las actualizaciones emitidas a partir de ese momento
- **AND** el cliente que inició la sesión SHALL seguir recibiéndolas sin declarar nada

#### Scenario: Sin declarar interés no llega el stream
- **WHEN** un cliente conectado no declara mirar una sesión que no inició
- **THEN** NO SHALL recibir las actualizaciones de esa sesión
