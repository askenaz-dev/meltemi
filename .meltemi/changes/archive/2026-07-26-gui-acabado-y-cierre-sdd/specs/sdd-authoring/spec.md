# sdd-authoring — delta

## ADDED Requirements

### Requirement: Cierre de sesión de los turnos de autoría
Todo turno de autoría del ciclo (explore, constitution, propose, plan y las
reelaboraciones de gate) SHALL finalizar su sesión por el finalizador
compartido de turnos únicos: eventos terminales en el log de sesión
(`turn_completed`, `session_ended`), registro de fin en el índice y baja del
registro vivo. Una sesión de autoría completada SHALL listarse como
finalizada — nunca como interrumpida — y su tiempo activo SHALL contar en la
analítica local. Un turno que falla SHALL cerrar igualmente la sesión con la
razón de error, sin inventar un estatus final.

#### Scenario: Turno de autoría finalizado queda cerrado
- **WHEN** un verbo del ciclo SDD completa su turno ACP
- **THEN** el log de la sesión SHALL contener `session_ended` y el índice un fin registrado
- **AND** `session/list` SHALL listarla como finalizada

#### Scenario: Fallo del turno también cierra
- **WHEN** el turno ACP de un verbo del ciclo falla al arrancar o ejecutar
- **THEN** la sesión SHALL cerrarse con la razón de error y sin estatus final inventado
