# worktree-orchestration Specification

## Purpose
TBD - created by archiving change orquestacion-worktrees. Update Purpose after archive.
## Requirements

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

### Requirement: Despacho de competidores con su propio proveedor
El daemon SHALL despachar el turno de un agente o perfil sobre el worktree de su
asignación — checkpoint previo, turno bajo las reglas de permisos vigentes y
commit con trazabilidad — resolviendo el binario de **ese** competidor desde la
flota; despachos de competidores distintos MUST poder correr en paralelo, cada
uno con su binario y contexto propios, y un despacho MUST NOT marcar la tarea en
`tasks.md`: el competidor no la posee, la fusión asistida decide.

#### Scenario: Carrera de dos proveedores distintos
- **WHEN** se despachan dos competidores de proveedores distintos sobre la misma tarea asignada
- **THEN** cada sesión SHALL lanzar el binario de su propio proveedor en su worktree aislado
- **AND** ambos resultados SHALL quedar comparables como diff contra la base común

#### Scenario: El despacho no marca la tarea
- **WHEN** un despacho concluye con su commit de trazabilidad
- **THEN** `tasks.md` SHALL permanecer sin marcar para esa tarea
- **AND** el commit SHALL llevar los trailers de trazabilidad de la tarea

### Requirement: La carrera consultable con procedencia

El resultado del diff por competidor SHALL declarar, por calle y como
campos aditivos opcionales, la procedencia de su último despacho — fuente
de resolución, perfil cuando aplique y nivel de integración —, la sesión
que corrió ese despacho, su estado de commit (sha cuando exista) y la base
fijada de esa calle. El resultado del despacho SHALL nombrar la sesión que
abrió. La omisión de todos los campos nuevos MUST serializar byte a byte
igual que antes de esta change: un cliente anterior no se rompe.

#### Scenario: La calle declara procedencia, sesión y estado

- **WHEN** un cliente consulta el diff de la carrera después de un despacho
- **THEN** cada calle despachada SHALL traer su fuente de resolución, su
  perfil cuando lo hubo, su nivel, la sesión del despacho y su estado de
  commit
- **AND** el resultado del despacho SHALL nombrar esa misma sesión

#### Scenario: Los campos aditivos no rompen al cliente anterior

- **WHEN** los campos nuevos se omiten por no existir procedencia registrada
- **THEN** la serialización del resultado SHALL ser idéntica a la forma
  previa a esta change
- **AND** una calle sin despacho registrado SHALL presentarse sin
  procedencia, nunca con procedencia inventada

#### Scenario: Bases divergentes visibles por calle

- **IF** dos calles de la misma tarea se crearon con bases distintas
- **THEN** cada calle SHALL declarar su propia base
- **AND** el resultado MUST NOT fundir las bases en una base única: cada
  calle conserva la suya
