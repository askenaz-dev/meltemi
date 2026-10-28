# acp-session — delta

## ADDED Requirements

### Requirement: Un turno que no llevó nada se declara vacío
WHEN un turno del agente termina con el motivo de parada que Meltemi traduce
como `completed`, sin que el agente haya enviado durante ese turno un fragmento
de mensaje o de pensamiento con texto que no sea solo espacios o con contenido
no textual, una llamada a herramienta o su actualización, un plan ni una
petición de permiso, y sin que se haya pedido cancelar la sesión o interrumpir
ese turno, el daemon SHALL registrar en el log de la sesión un evento
`turn_empty` que conserve el motivo de parada. El evento SHALL registrarse para
cada turno así terminado, también en los turnos intermedios de una sesión que
corre varios turnos por instrucciones dirigidas, en un despacho y en cada tarea
de una implementación. El motivo de parada que el agente reportó NO SHALL
cambiar.

#### Scenario: Turno sin palabra ni acción declarado vacío
- **WHEN** el agente cierra su turno con `end_turn` sin haber enviado nada que lleve trabajo
- **THEN** el log de la sesión SHALL registrar `turn_empty` con el motivo de parada `completed`
- **AND** el cierre del turno SHALL conservar el motivo `completed` que el agente reportó

#### Scenario: Un turno con cualquier palabra o acción no es vacío
- **WHEN** el agente envía durante el turno al menos un fragmento de mensaje con texto, una llamada a herramienta o una petición de permiso
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Un turno que solo piensa, actualiza una herramienta o su plan no es vacío
- **WHEN** lo único que el agente envía durante el turno es un pensamiento, la actualización de una herramienta, un plan o un fragmento con contenido no textual
- **THEN** el turno NO SHALL declararse vacío

#### Scenario: Un fragmento de solo espacios no cuenta como trabajo
- **WHEN** lo único que el agente envía durante el turno es un fragmento de mensaje cuyo texto es solo espacios
- **THEN** el turno SHALL declararse vacío

#### Scenario: Lo que describe la sesión no cuenta como trabajo
- **WHEN** el agente envía durante el turno solo actualizaciones de la sesión, como sus comandos disponibles o su consumo
- **THEN** el turno SHALL declararse vacío

#### Scenario: Un turno que no terminó no se declara vacío
- **WHEN** un turno termina cancelado, rehusado o por un límite del agente
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Un turno que el humano detuvo no se declara vacío
- **WHEN** el humano cancela la sesión o interrumpe el turno y el agente lo cierra igual con `end_turn` sin haber enviado nada
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno
- **AND** la respuesta NO SHALL declarar el turno vacío

#### Scenario: Cada turno de una sesión de varios turnos se mide
- **WHEN** una sesión corre un turno con trabajo, uno vacío y otro con trabajo por instrucciones dirigidas
- **THEN** el log SHALL registrar exactamente un `turn_empty`, después del segundo prompt y antes del tercero

### Requirement: Las respuestas de sesión dicen si el turno fue vacío
La respuesta de `session/start` sin `detach` y la de `session/direct` cuando
reanuda SHALL declarar con `emptyTurn` en `true` que el turno cuyo estado
informan se declaró vacío, conservando ese estado, y SHALL omitir el campo en
cualquier otro caso. Un arranque con `detach` y una instrucción encolada o
relevada, que no informan estado, NO SHALL llevar el campo.

#### Scenario: Un arranque cuyo turno fue vacío lo declara
- **WHEN** un `session/start` sin `detach` corre un turno que se declara vacío
- **THEN** la respuesta SHALL traer `status` en `completed` y `emptyTurn` en `true`

#### Scenario: Un turno con trabajo no deja clave
- **WHEN** un `session/start` sin `detach` corre un turno que llevó trabajo
- **THEN** la respuesta NO SHALL contener la clave `emptyTurn`

#### Scenario: Sin estado no hay veredicto en la respuesta
- **WHEN** un `session/start` con `detach` arranca un turno que resulta vacío
- **THEN** la respuesta NO SHALL contener `status` ni `emptyTurn`
- **AND** el log SHALL registrar `turn_empty`
