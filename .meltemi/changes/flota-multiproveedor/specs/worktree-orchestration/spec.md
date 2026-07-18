## ADDED Requirements

### Requirement: Despacho de competidores con su propio proveedor
El daemon SHALL despachar el turno de un agente o perfil sobre el worktree de su
asignación — checkpoint previo, turno bajo las reglas de permisos vigentes y
commit con trazabilidad — resolviendo el binario de **ese** competidor desde la
flota; despachos de competidores distintos MUST poder correr en paralelo, cada
uno con su binario y contexto propios, y un despacho MUST NOT marcar la tarea en
`tasks.md`: el competidor no la posee, la fusión asistida decide.

#### Scenario: Carrera de dos proveedores distintos
- **WHEN** se despachan dos competidores de proveedores distintos sobre la misma tarea asignada
- **THEN** cada sesión SHALL lanzar el binario de su propio proveedor en su worktree aislado
- **AND** ambos resultados SHALL quedar comparables como diff contra la base común

#### Scenario: El despacho no marca la tarea
- **WHEN** un despacho concluye con su commit de trazabilidad
- **THEN** `tasks.md` SHALL permanecer sin marcar para esa tarea
- **AND** el commit SHALL llevar los trailers de trazabilidad de la tarea
