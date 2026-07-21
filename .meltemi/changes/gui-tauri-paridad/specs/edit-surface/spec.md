## MODIFIED Requirements

### Requirement: Advertencia por sesión de agente activa
La política de concurrencia humano↔agente sobre un mismo worktree SHALL ser de
bloqueo suave, nunca bloqueo duro. El daemon SHALL exponer el estado del
worktree en tres niveles — libre, sesión activa sin turno en vuelo, turno en
vuelo — y las superficies de edición SHALL comportarse según ese estado: con
turno en vuelo, el guardado MUST exigir confirmación reforzada que advierta el
riesgo de conflicto; con sesión activa sin turno en vuelo, el guardado SHALL
advertir y pedir confirmación simple; con worktree libre, el guardado procede
sin fricción. Toda edición aplicada SHALL registrarse como `human_edit`, y el
daemon MUST anteponer al siguiente turno del agente una nota con los archivos
editados por el humano desde su último turno — la nota viaja en el prompt del
turno; no se inventa una notificación push fuera de ACP.

#### Scenario: Edición con turno en vuelo
- **WHEN** el usuario guarda una edición in situ en un worktree cuyo agente tiene un turno en vuelo
- **THEN** la superficie SHALL exigir confirmación reforzada advirtiendo el riesgo de conflicto
- **AND** al confirmar, la escritura SHALL aplicarse vía el daemon y registrarse como `human_edit`

#### Scenario: Edición entre turnos
- **WHEN** el usuario guarda una edición in situ con sesión activa pero sin turno en vuelo
- **THEN** la superficie SHALL advertir y pedir confirmación simple antes de aplicar

#### Scenario: Worktree libre sin fricción
- **WHEN** el usuario guarda una edición in situ en un worktree sin sesión de agente activa
- **THEN** la escritura SHALL aplicarse sin confirmación adicional
- **AND** SHALL registrarse igualmente como `human_edit`

#### Scenario: Nota al siguiente turno del agente
- **WHEN** el agente inicia su siguiente turno tras ediciones humanas en su worktree
- **THEN** el daemon SHALL anteponer al turno una nota con los archivos editados desde el último turno
- **AND** la nota SHALL quedar evidenciada en el log de sesión
