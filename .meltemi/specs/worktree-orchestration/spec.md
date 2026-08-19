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

### Requirement: Taller de change en su propia rama

El daemon SHALL ofrecer, a demanda, el taller de una change: por defecto, una
rama con el nombre de la change creada desde la punta de la rama por defecto, y
un worktree gestionado con nomenclatura estable dentro de la raíz gestionada
del proyecto. La petición MAY nombrar la rama del taller — si la nombrada no
existe se crea desde la punta de la rama por defecto; si existe, la elección
explícita SHALL entenderse como consentimiento para trabajar sobre ella. La
petición MAY pedir en su lugar un taller único: rama y worktree con un sufijo
único, de modo que varios talleres de la misma change coexistan sin pisarse.
La petición por defecto SHALL ser idempotente — si el taller ya existe y es
gestionado, se devuelve con su ruta y su rama, declarando que es un
reencuentro; un taller único SHALL ser siempre una creación nueva. WHERE exista
una rama homónima que el daemon no creó y la petición no la haya nombrado
explícitamente, la petición SHALL rehusarse con diagnóstico y remedio, y el
daemon MUST NOT tocarla. La raíz gestionada SHALL quedar excluida del estado de
git del árbol principal por vía local, sin modificar el `.gitignore` versionado
del usuario.

#### Scenario: El primer taller se crea desde la rama por defecto

- **WHEN** se pide el taller de una change que no lo tiene
- **THEN** el daemon SHALL crear la rama con el nombre de la change desde la
  punta de la rama por defecto
- **AND** SHALL crear su worktree gestionado y devolver ruta y rama

#### Scenario: Pedirlo de nuevo reencuentra, no falla

- **WHEN** se pide el taller de una change que ya lo tiene
- **THEN** el daemon SHALL devolver el existente con su ruta y su rama
- **AND** SHALL declarar que es un reencuentro, no una creación

#### Scenario: El taller sobre una rama elegida

- **WHEN** se pide el taller nombrando una rama
- **THEN** el worktree SHALL crearse sobre esa rama, creándola desde la punta
  de la rama por defecto si no existe
- **AND** nombrarla explícitamente SHALL valer como consentimiento aunque el
  daemon no la haya creado

#### Scenario: Un taller único no colisiona con nadie

- **WHEN** se pide un taller único para una change
- **THEN** su rama y su worktree SHALL llevar un sufijo único
- **AND** varios talleres de la misma change SHALL coexistir sin pisarse
- **AND** la respuesta SHALL declararlo creación, nunca reencuentro

#### Scenario: La rama ajena se rehúsa sin tocarse

- **WHERE** existe una rama con el nombre de la change que el daemon no creó
- **THEN** la petición SHALL rehusarse con diagnóstico y remedio
- **AND** la rama NO SHALL modificarse

#### Scenario: El taller no ensucia el estado del árbol principal

- **WHEN** existe al menos un taller gestionado
- **THEN** el estado de git del árbol principal NO SHALL listar la raíz
  gestionada como contenido sin seguimiento
- **AND** el `.gitignore` versionado del usuario NO SHALL haberse modificado

### Requirement: Aterrizaje del taller con decisión explícita

El daemon SHALL fusionar la rama del taller en la rama por defecto únicamente
con confirmación explícita; sin ella, SHALL previsualizar qué commits
aterrizarían y qué archivos tocan. La fusión SHALL conservar la forma de la
change en el grafo. El daemon SHALL rehusarse con diagnóstico y remedio cuando
el taller tenga cambios sin commitear, y cuando la fusión produzca conflictos —
en cuyo caso SHALL abortar la fusión dejando la rama por defecto intacta, y
MUST NOT resolver conflicto alguno por su cuenta.

#### Scenario: Sin confirmación, la previsualización

- **WHEN** se pide aterrizar sin confirmación
- **THEN** el daemon SHALL responder los commits que aterrizarían y los
  archivos que tocan
- **AND** NO SHALL fusionar nada

#### Scenario: Con confirmación, el aterrizaje limpio

- **WHEN** se pide aterrizar con confirmación y la fusión aplica limpia
- **THEN** la rama del taller SHALL quedar fusionada en la rama por defecto
- **AND** la forma de la change SHALL quedar visible en el grafo

#### Scenario: El conflicto se rehúsa y no deja el árbol a medias

- **WHERE** la fusión produce conflictos
- **THEN** el daemon SHALL abortar la fusión dejando la rama por defecto
  intacta
- **AND** SHALL rehusar con diagnóstico y el remedio de resolver en el git del
  usuario

#### Scenario: El taller sucio no aterriza

- **WHERE** el taller tiene cambios sin commitear
- **THEN** el aterrizaje SHALL rehusarse con diagnóstico y remedio
- **AND** nada SHALL fusionarse

### Requirement: El taller sin aterrizar no se pierde en silencio

Retirar el taller de una change cuya rama contiene commits que la rama por
defecto no alcanza SHALL exigir confirmación explícita, y el aviso SHALL decir
cuántos commits quedarían solo en la rama. Retirar el taller SHALL retirar el
worktree y NO SHALL borrar la rama.

#### Scenario: Retirar con commits sin aterrizar exige confirmación

- **WHERE** la rama del taller tiene commits que la rama por defecto no alcanza
- **WHEN** se pide retirar el taller sin confirmación
- **THEN** el daemon SHALL rehusar diciendo cuántos commits quedarían solo en
  la rama

#### Scenario: Retirar el taller conserva la rama

- **WHEN** se retira el taller de una change
- **THEN** el worktree gestionado SHALL desaparecer
- **AND** la rama SHALL permanecer
