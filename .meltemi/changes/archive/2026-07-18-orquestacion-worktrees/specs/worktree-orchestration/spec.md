## ADDED Requirements

### Requirement: Worktrees gestionados con ciclo de vida
El daemon SHALL crear, listar y eliminar worktrees gestionados con nomenclatura
estable por change, tarea y agente, usando el git del usuario; MUST NOT tocar
worktrees que no creó, y la limpieza de uno con cambios sin commitear MUST exigir
confirmación explícita.

#### Scenario: Creación con nomenclatura estable
- **WHEN** se asigna una tarea a un agente
- **THEN** el daemon SHALL crear el worktree y la rama gestionados con el nombre derivado de change, tarea y agente

#### Scenario: Limpieza segura
- **IF** un worktree gestionado tiene cambios sin commitear
- **THEN** su eliminación SHALL exigir confirmación explícita
- **AND** un worktree ajeno SHALL NOT ser tocado jamás

### Requirement: Sesión ligada a su worktree
Una sesión asignada SHALL ejecutar su agente con el worktree como directorio de
trabajo, y la tabla de Sesiones SHALL mostrar worktree y rama en su columna. Una
sesión sin asignación SHALL conservar el flujo actual, y el daemon MUST advertir
cuando más de una sesión simultánea comparte el mismo árbol.

#### Scenario: Agente confinado a su worktree
- **WHEN** arranca una sesión asignada
- **THEN** el subproceso del agente SHALL tener el worktree como cwd
- **AND** la tabla SHALL mostrar worktree y rama

#### Scenario: Colisión de árbol advertida
- **WHEN** dos sesiones sin worktree corren sobre el mismo árbol
- **THEN** el daemon SHALL emitir la advertencia visible en ambas sesiones

### Requirement: Asignación de tareas a agentes
El daemon SHALL asignar N agentes sobre M tareas creando un worktree por
asignación desde una base común fijada, y MUST serializar automáticamente las
tareas con solapamiento de archivos declarado, informando la serialización.

#### Scenario: Paralelo sin solapamiento
- **WHEN** dos tareas sin archivos compartidos se asignan a dos agentes
- **THEN** ambas sesiones SHALL correr en paralelo en worktrees distintos desde la misma base

#### Scenario: Solapamiento serializado
- **WHERE** dos tareas declaran archivos compartidos
- **THEN** el daemon SHALL ejecutarlas en secuencia
- **AND** SHALL informar el motivo

### Requirement: Carreras de agentes sobre la misma tarea
El daemon SHALL soportar asignar la misma tarea a dos o más agentes en worktrees
separados desde la misma base, etiquetando las sesiones como competidoras; al
concluir, cada resultado SHALL quedar disponible como diff contra la base común.

#### Scenario: Carrera etiquetada
- **WHEN** una tarea se asigna a dos agentes en carrera
- **THEN** ambas sesiones SHALL correr aisladas desde la misma revisión base
- **AND** SHALL quedar vinculadas como competidoras de esa tarea

### Requirement: Merge asistido por humano
La superficie SHALL presentar los resultados en competencia como diffs lado a
lado (con reflow del shell en anchos menores), permitiendo elegir una base y
aplicar selectivamente por archivo; ninguna mezcla MUST aplicarse sin decisión
humana explícita.

#### Scenario: Elección y aplicación selectiva
- **WHEN** el usuario compara dos resultados de una carrera
- **THEN** SHALL poder elegir uno como base y aplicar archivos del otro selectivamente
- **AND** cada aplicación SHALL requerir su decisión explícita

### Requirement: Degradación honesta sin git
La orquestación por worktrees SHALL declararse no disponible, con remedio, sobre
un directorio que no es repositorio git, y las sesiones SHALL seguir funcionando
sin aislamiento con la advertencia correspondiente.

#### Scenario: Proyecto sin git
- **WHEN** se intenta asignar tareas en un directorio sin repositorio git
- **THEN** el daemon SHALL rehusar la orquestación con diagnóstico y remedio
- **AND** la sesión simple SHALL seguir disponible con advertencia
