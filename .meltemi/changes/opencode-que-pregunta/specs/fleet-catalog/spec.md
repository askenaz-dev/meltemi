# fleet-catalog — delta

## ADDED Requirements

### Requirement: Entorno de lanzamiento declarado por el registro
Una entrada del registro MAY declarar el entorno con que se lanza su agente. El
daemon SHALL aplicarlo en toda resolución de ese agente —por perfil, por id de
catálogo y como agente configurado del proyecto— y SHALL NOT aplicarlo a un
comando literal configurado por el usuario. Una variable del mismo nombre
presente en el entorno del usuario o en el perfil SHALL prevalecer. La entrada
de OpenCode SHALL declarar la configuración en línea que hace que pida permiso
antes de editar, ejecutar comandos, ir a la red, lanzar subagentes y tocar
rutas fuera del proyecto.

#### Scenario: OpenCode se lanza de modo que pregunte
- **WHEN** una sesión resuelve OpenCode por su id, por un perfil suyo o como agente del proyecto
- **THEN** su lanzamiento SHALL llevar la configuración en línea que pone esos permisos en `ask`

#### Scenario: Lo que el usuario fijó manda
- **WHERE** el usuario ya tiene en su entorno o en su perfil una variable del mismo nombre
- **WHEN** se resuelve el agente
- **THEN** el lanzamiento SHALL llevar el valor del usuario y no el del registro

#### Scenario: Un comando literal no hereda el entorno del registro
- **WHERE** el proyecto configura un comando literal para su agente
- **WHEN** se resuelve el agente configurado
- **THEN** el lanzamiento SHALL NOT llevar el entorno de ninguna entrada del registro
