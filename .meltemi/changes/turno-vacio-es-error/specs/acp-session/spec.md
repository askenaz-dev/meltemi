# acp-session — delta

## ADDED Requirements

### Requirement: Un turno que no llevó nada se declara vacío
WHEN un turno del agente termina con el motivo de parada que Meltemi traduce
como `completed` sin que el agente haya enviado, durante ese turno, un
fragmento de mensaje o de pensamiento con texto, una llamada a herramienta o su
actualización, un plan ni una petición de permiso, el daemon SHALL registrar en
el log de la sesión un evento `turn_empty` que conserve el motivo de parada. El
evento SHALL registrarse para cada turno así terminado, también en los turnos
intermedios de una sesión que espera, en un despacho y en cada tarea de una
implementación. El motivo de parada que el agente reportó NO SHALL cambiar.

#### Scenario: Turno sin palabra ni acción declarado vacío
- **WHEN** el agente cierra su turno con `end_turn` sin haber enviado nada que lleve trabajo
- **THEN** el log de la sesión SHALL registrar `turn_empty` con el motivo de parada `completed`
- **AND** el cierre del turno SHALL conservar el motivo `completed` que el agente reportó

#### Scenario: Un turno con cualquier palabra o acción no es vacío
- **WHEN** el agente envía durante el turno al menos un fragmento de mensaje con texto, una llamada a herramienta o una petición de permiso
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Lo que describe la sesión no cuenta como trabajo
- **WHEN** el agente envía durante el turno solo actualizaciones de la sesión, como sus comandos disponibles o su consumo
- **THEN** el turno SHALL declararse vacío

#### Scenario: Un turno que no terminó no se declara vacío
- **WHEN** un turno termina cancelado, rehusado o por un límite del agente
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Cada turno de una sesión que espera se mide
- **WHEN** una sesión que espera corre varios turnos y uno intermedio termina vacío
- **THEN** el log SHALL registrar `turn_empty` después del prompt de ese turno y antes del prompt del siguiente
