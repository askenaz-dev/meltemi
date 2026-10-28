# cli-contract — delta

## ADDED Requirements

### Requirement: El arranque scriptable sigue esperando por defecto

El verbo de arranque de la CLI SHALL seguir esperando el desenlace de la sesión
por defecto. El desacople SHALL ser opt-in explícito, y mientras la CLI no tenga
superficie para el stream de eventos ni para el registro de sesión, la ayuda del
verbo SHALL decir qué NO se verá al desacoplar.

#### Scenario: Arrancar desde la CLI sigue mostrando el desenlace

- **WHEN** se arranca una sesión desde la CLI sin pedir desacople
- **THEN** el comando SHALL esperar y SHALL mostrar el desenlace del turno
