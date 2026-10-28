# own-adapters — delta

## ADDED Requirements

### Requirement: Terminar al proveedor es terminar todo lo que lanzó

Cuando el adaptador termina al proceso proveedor —al agotarse la gracia
tras cerrar su entrada, o al cerrarse la sesión— SHALL terminar también a
todo proceso que ese proveedor haya necesitado para existir. WHERE la
plataforma resuelva el CLI oficial a un intermediario que crea el proceso
real, terminar al proveedor SHALL terminar al intermediario y a cuanto creó,
y el adaptador NO SHALL responder el turno como terminado mientras alguno
siga vivo. WHERE la plataforma permita atar la vida de esos procesos a la del
adaptador, el adaptador SHALL atarla al lanzar, de modo que un fin abrupto
del adaptador tampoco deje huérfanos. El adaptador NO SHALL ganar
dependencia externa alguna para ello.

#### Scenario: El intermediario del proveedor no deja huérfanos

- **WHERE** el CLI oficial se resuelve a un intermediario que crea el proceso real
- **WHEN** el proveedor ignora el fin de su entrada y la gracia se agota
- **THEN** el adaptador SHALL terminar al intermediario y a todo proceso que creó
- **AND** SHALL responder el turno como cancelado solo entonces

#### Scenario: El adaptador cae y el proveedor no le sobrevive

- **WHERE** la plataforma permite atar la vida del proveedor a la del adaptador
- **WHEN** el proceso del adaptador termina sin ejecutar su apagado
- **THEN** el proveedor y cuanto lanzó SHALL terminar igualmente
