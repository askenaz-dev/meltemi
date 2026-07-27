# acp-session — delta

## ADDED Requirements

### Requirement: Sesión bloqueada por una decisión humana
Mientras una petición de permiso de una sesión espera la decisión humana, el
daemon SHALL declarar esa sesión en estado `waiting_permission` en `status` y
en el listado de sesiones, y SHALL restituirla a `active` cuando la petición
se resuelva por cualquier vía (decisión, vencimiento de la cota o denegación
sin clientes). Con varias peticiones simultáneas de la misma sesión, la
restitución SHALL ocurrir solo cuando ninguna quede pendiente.

#### Scenario: Sesión esperando se declara esperando
- **WHEN** una petición de permiso de una sesión activa escala al humano
- **THEN** el listado de sesiones SHALL mostrar esa sesión en `waiting_permission`

#### Scenario: Resuelta la petición, la sesión vuelve a activa
- **WHEN** la petición pendiente se resuelve
- **THEN** el listado SHALL volver a mostrar la sesión en `active`

#### Scenario: Varias esperas simultáneas
- **WHEN** una sesión tiene más de una petición esperando y se resuelve una
- **THEN** la sesión SHALL seguir declarándose `waiting_permission` mientras quede alguna pendiente
