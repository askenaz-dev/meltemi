

### Requirement: Listado de changes con estado agregado
El daemon SHALL listar las changes del método — activas y archivadas — con su
estado agregado desde lo ya persistido: artefactos presentes, progreso de tareas,
estado de review y de verify por change; el listado MUST ser de solo lectura y
MUST estar disponible en todas las superficies por igual, con salida
scriptable.

#### Scenario: Listado con estado por change
- **WHEN** un cliente consulta el listado de changes
- **THEN** cada change activa SHALL aparecer con sus artefactos presentes, sus tareas completadas sobre el total, y el estado de review y verify
- **AND** ninguna consulta SHALL modificar el árbol del método

#### Scenario: Archivadas consultables
- **WHEN** el listado incluye el histórico
- **THEN** cada change archivada SHALL aparecer con su nombre y su fecha de archivado

#### Scenario: Estado parcial honesto
- **WHERE** una change carece de un artefacto o de deltas
- **THEN** el listado SHALL reflejar la ausencia tal cual
- **AND** SHALL NOT inventar estado para completar columnas

### Requirement: Mostrar changes y specs vivas
El daemon SHALL mostrar una change (sus artefactos y sus deltas por capacidad)
y la verdad viva (las capacidades existentes, y una spec con sus requisitos y
escenarios) sin reescribir ni normalizar nada al mostrar; un nombre inexistente
MUST rehusarse con diagnóstico y remedio.

#### Scenario: Mostrar una change con sus artefactos
- **WHEN** un cliente pide mostrar una change existente
- **THEN** la respuesta SHALL incluir sus artefactos y sus deltas por capacidad tal como están en disco

#### Scenario: Mostrar una spec viva
- **WHEN** un cliente pide mostrar una capacidad de la verdad viva
- **THEN** la respuesta SHALL incluir sus requisitos y escenarios parseados

#### Scenario: Nombre inexistente rehúsa con remedio
- **IF** el nombre no corresponde a una change ni a una capacidad
- **THEN** la petición SHALL rehusarse con diagnóstico
- **AND** el remedio SHALL orientar al listado

### Requirement: Validación independiente del archivado
El daemon SHALL validar una change sin archivarla — la validación del motor y la
aplicación en seco de sus deltas contra la verdad viva, con los diagnósticos
legibles — y SHALL validar la verdad viva completa cuando no se nombra change;
la validación MUST NOT mutar nada, y su desenlace MUST ser distinguible de forma
scriptable entre limpio y con hallazgos.

#### Scenario: Validar una change sin tocarla
- **WHEN** un cliente valida una change con deltas aplicables
- **THEN** el resultado SHALL declarar la validación limpia
- **AND** la verdad viva y la change SHALL permanecer intactas

#### Scenario: Hallazgos reportados sin archivar
- **WHERE** un delta de la change no aplica sobre la verdad viva
- **THEN** la validación SHALL reportar los diagnósticos del conflicto
- **AND** SHALL NOT bloquear ni modificar nada más allá del reporte

#### Scenario: Verdad viva validada sin argumento
- **WHEN** un cliente valida sin nombrar change
- **THEN** cada spec de la verdad viva SHALL validarse estructuralmente
- **AND** los diagnósticos SHALL señalar capacidad y ubicación
