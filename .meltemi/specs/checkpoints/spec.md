# checkpoints Specification

## Purpose
TBD - created by archiving change checkpoints-rollback. Update Purpose after archive.
## Requirements
### Requirement: Checkpoint automático antes de cada tarea
El daemon SHALL crear un checkpoint del worktree (incluido lo no rastreado no
ignorado) antes de que un agente ejecute cada tarea, como ref técnica fuera de
las ramas del usuario, y MUST registrar su creación en el log de sesión. Las
ramas y refs del usuario MUST NOT ser modificadas por la creación de checkpoints.

#### Scenario: Checkpoint pre-tarea
- **WHEN** una tarea va a ejecutarse en su worktree
- **THEN** el daemon SHALL crear el checkpoint bajo la ref técnica de esa tarea
- **AND** el evento SHALL quedar en el JSONL

### Requirement: Listado de checkpoints
El daemon SHALL listar los checkpoints por change y tarea (ref, momento, tarea,
worktree), consultable por contrato para las superficies.

#### Scenario: Checkpoints consultables
- **WHEN** un cliente consulta los checkpoints de una change
- **THEN** la respuesta SHALL incluir por tarea su ref y momento de creación

### Requirement: Reversión granular por tarea
El daemon SHALL revertir una tarea restaurando su worktree al checkpoint (estado
rastreado y limpieza de lo no rastreado creado después), sin tocar otros
worktrees ni reescribir jamás ramas del usuario; la reversión MUST exigir
confirmación explícita mediante la superficie modal del shell.

#### Scenario: Revertir una tarea no toca a las demás
- **WHEN** el usuario revierte la tarea T con otras tareas en curso en otros worktrees
- **THEN** el worktree de T SHALL volver a su checkpoint
- **AND** los demás worktrees SHALL permanecer intactos

#### Scenario: Confirmación obligatoria
- **WHEN** se solicita una reversión
- **THEN** el shell SHALL exigir confirmación explícita antes de ejecutarla

### Requirement: Alcance honesto de la reversión
La UX de reversión SHALL declarar, antes de confirmar, las operaciones aprobadas
durante la tarea que actúan fuera del árbol (según la clasificación de sus
peticiones de permiso) como irreversibles; el daemon MUST acumularlas junto al
checkpoint y MUST NOT presentar la reversión como total cuando existan.

#### Scenario: Irreversibles declaradas
- **WHERE** durante la tarea se aprobó la ejecución de un comando fuera del árbol
- **THEN** la confirmación de reversión SHALL listar esa operación como irreversible
- **AND** SHALL NOT anunciar una reversión total

#### Scenario: Reversión limpia
- **WHERE** la tarea no tuvo operaciones fuera del árbol
- **THEN** la confirmación SHALL declarar la reversión como completa del árbol de trabajo

