# method-navigation — delta

## MODIFIED Requirements

### Requirement: Listado de changes con estado agregado
El daemon SHALL listar las changes del método — activas y archivadas — con su
estado agregado desde lo ya persistido: artefactos presentes, progreso de tareas,
estado de review y de verify, y **el gate de autoría pendiente con el artefacto
que lo espera**, por change; el listado MUST ser de solo lectura y
MUST estar disponible en todas las superficies por igual, con salida
scriptable.

#### Scenario: Listado con estado por change
- **WHEN** un cliente consulta el listado de changes
- **THEN** cada change activa SHALL aparecer con sus artefactos presentes, sus tareas completadas sobre el total, y el estado de review y verify
- **AND** ninguna consulta SHALL modificar el árbol del método

#### Scenario: Gate pendiente descubrible en el listado
- **WHEN** una change tiene un gate de autoría esperando decisión humana
- **THEN** el listado SHALL declararlo con el artefacto que espera
- **AND** un cliente que no invocó el verbo SHALL poder descubrirlo sin invocar nada más

#### Scenario: Archivadas consultables
- **WHEN** el listado incluye el histórico
- **THEN** cada change archivada SHALL aparecer con su nombre y su fecha de archivado

#### Scenario: Estado parcial honesto
- **WHERE** una change carece de un artefacto o de deltas
- **THEN** el listado SHALL reflejar la ausencia tal cual
- **AND** SHALL NOT inventar estado para completar columnas
