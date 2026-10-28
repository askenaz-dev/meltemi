# remote-access — delta

## ADDED Requirements

### Requirement: Decidir y seguir por SSH sin terminal interactiva
Los verbos de la CLI que listan y deciden peticiones de permiso y los que siguen
una sesión SHALL funcionar bajo una ejecución remota de `ssh` sin terminal
interactiva: MUST NOT leer el teclado, preguntar ni exigir un tamaño de
pantalla, y la decisión SHALL ser exactamente la que dice la línea de órdenes.
Una petición decidida así SHALL resolverse con la regla de que la primera
decisión gana, como desde cualquier superficie. La documentación de acceso
remoto SHALL dar la receta de aprobar desde otro equipo sin la TUI —listar,
decidir una petición por su id y seguir la sesión— y SHALL declarar que esa
ruta no necesita túnel: la orden corre en la máquina del daemon por el SSH del
propio usuario, y ningún verbo decide en bloque.

#### Scenario: Aprobar sin terminal desde otra conexión
- **WHEN** una sesión iniciada por otra superficie espera un permiso y se invocan `permissions` y luego `approve <petición>` sin terminal interactiva
- **THEN** la petición SHALL resolverse con la opción elegida, decidida por el cliente
- **AND** la sesión SHALL continuar su turno

#### Scenario: Seguir sin terminal
- **WHEN** se invoca `watch` sin terminal interactiva sobre una sesión viva
- **THEN** SHALL emitir sus registros a medida que ocurren
- **AND** SHALL terminar cuando la sesión termina

#### Scenario: La receta de aprobar desde el teléfono está documentada
- **WHEN** un usuario consulta la documentación de acceso remoto
- **THEN** SHALL encontrar la receta `ssh <pc> meltemi permissions`, luego `ssh <pc> meltemi approve <petición>`, y `watch` para seguir
- **AND** SHALL encontrar declarado que no hace falta túnel y que ningún verbo decide en bloque
