# session-history — delta

## ADDED Requirements

### Requirement: El registro de una sesión de calle se lee por su calle
Una sesión que corrió en un worktree gestionado —una calle— SHALL poder leerse
por `session/log` con la raíz que el listado da para ella, la del worktree, y
también con la del proyecto. Reanudarla SHALL dejar el registro de la sesión
nueva junto al de la original, de modo que el listado del proyecto la muestre y
su registro se lea por la misma calle. La raíz que el índice guarda para la
calle SHALL seguir siendo el worktree.

#### Scenario: El registro de un despacho se lee por su calle
- **WHEN** un cliente pide `session/log` de una sesión de `worktree/dispatch` con la raíz que el listado le dio
- **THEN** la respuesta SHALL traer el registro de esa sesión, no una página vacía
- **AND** pedirlo con la raíz del proyecto SHALL traer el mismo registro

#### Scenario: Reanudar una calle deja el registro con su proyecto
- **WHEN** se reanuda una sesión de calle
- **THEN** el listado del proyecto SHALL mostrar la sesión nueva con la calle como raíz
- **AND** su registro SHALL leerse por esa calle
