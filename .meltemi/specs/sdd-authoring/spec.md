# sdd-authoring Specification

## Purpose
TBD - created by archiving change ciclo-sdd-autoria. Update Purpose after archive.
## Requirements
### Requirement: Ciclo de autoría con gates humanos
El daemon SHALL conducir la autoría de una change como secuencia de artefactos
(proposal, specs EARS, design, tasks) donde cada artefacto es redactado por el
agente, validado por el motor de specs y sometido a un gate humano con tres
salidas: aprobar, comentar para reelaborar, o abortar. Ningún artefacto MUST
avanzar al siguiente sin gate aprobado, y el estado del ciclo MUST persistir en
la change y sobrevivir reinicios del daemon.

#### Scenario: Gate aprueba y avanza
- **WHEN** el humano aprueba el gate de un artefacto validado
- **THEN** el ciclo SHALL avanzar al siguiente artefacto
- **AND** la decisión SHALL quedar registrada en la change

#### Scenario: Comentario reelabora sin reiniciar
- **WHEN** el humano comenta en un gate
- **THEN** el comentario SHALL volver al agente como instrucción de reelaboración
- **AND** el ciclo SHALL permanecer en el mismo artefacto

#### Scenario: El estado sobrevive un reinicio
- **WHEN** el daemon se reinicia con un ciclo a medias
- **THEN** el ciclo SHALL reanudarse en el artefacto y gate donde estaba

### Requirement: Validación del motor como puerta previa
Todo artefacto redactado por el agente SHALL validarse con el motor de specs
(estructura, EARS y aplicabilidad en seco de los deltas) antes de presentarse al
gate humano; un artefacto inválido MUST volver al agente con los diagnósticos
como instrucción sin consumir el gate.

#### Scenario: Inválido no llega al humano
- **WHEN** el agente entrega un delta cuyo requisito carece de escenario
- **THEN** el motor SHALL devolverlo al agente con el diagnóstico
- **AND** el gate humano SHALL NOT abrirse para esa entrega

### Requirement: Verbo constitution
El verbo `constitution` SHALL crear o editar `constitution.md` del proyecto con
plantilla guiada y gate humano final; sobre un proyecto sin `.meltemi/`, MUST
inicializar la estructura mínima de artefactos.

#### Scenario: Constitución inicial
- **WHEN** el usuario ejecuta el verbo en un repo sin `.meltemi/`
- **THEN** la estructura mínima SHALL crearse y la constitución redactada SHALL pasar por gate antes de persistir como definitiva

### Requirement: Verbo explore sin escritura
El verbo `explore` SHALL conducir deliberación con el agente (leer el repo,
sopesar opciones, proponer rumbo) en streaming, y MUST NOT escribir ni modificar
artefactos ni archivos del proyecto.

#### Scenario: Exploración inocua
- **WHEN** un turno de explore concluye
- **THEN** el árbol del proyecto SHALL quedar sin modificaciones
- **AND** la deliberación SHALL quedar solo en el log de sesión

### Requirement: Verbo plan
El verbo `plan` SHALL refinar el design y secuenciar `tasks.md` por dependencias
declaradas (incluido el solapamiento de archivos entre tareas), con gate humano
sobre el resultado.

#### Scenario: Tareas secuenciadas
- **WHEN** plan concluye sobre una change con tasks
- **THEN** `tasks.md` SHALL quedar ordenado por dependencias con el solapamiento anotado
- **AND** el humano SHALL aprobarlo en gate

### Requirement: Modo dual con criterio de proporcionalidad escrito
El ciclo SHALL ofrecer `spec-full` (gate por artefacto) y `fast-forward` (todos
los artefactos, un gate final), con un criterio de elegibilidad escrito: MUST ser
elegible para fast-forward solo una change sin capacidades nuevas y sin deltas
MODIFIED ni REMOVED; el humano MUST poder forzar cualquiera de los dos modos y la
elección SHALL registrarse en la change.

#### Scenario: Cambio grande exige spec-full
- **WHEN** una change introduce una capacidad nueva
- **THEN** fast-forward SHALL NOT ser elegible por criterio
- **AND** solo un forzado humano explícito registrado SHALL permitirlo

#### Scenario: Fast-forward con gate único
- **WHEN** una change elegible corre en fast-forward
- **THEN** los cuatro artefactos SHALL presentarse juntos en un único gate final

### Requirement: Superficies del ciclo
Las acciones del ciclo SHALL vivir en la vista Proyecto y la paleta (gates como
modales de primera clase del shell), y los verbos `explore` y `plan` SHALL ser
operativos en la CLI; en modo scriptable los gates MUST resolverse por pasos
explícitos, nunca quedando a la espera interactiva sin TTY.

#### Scenario: Gate en la TUI
- **WHEN** un artefacto llega a gate con la TUI conectada
- **THEN** el shell SHALL presentar el gate como modal con las tres salidas

#### Scenario: Scriptable sin cuelgues
- **WHEN** un gate queda pendiente y no hay TTY
- **THEN** la invocación SHALL terminar reportando el gate pendiente y cómo decidirlo
- **AND** SHALL NOT quedar a la espera de entrada


### Requirement: Cierre de sesión de los turnos de autoría
Todo turno de autoría del ciclo (explore, constitution, propose, plan y las
reelaboraciones de gate) SHALL finalizar su sesión por el finalizador
compartido de turnos únicos: eventos terminales en el log de sesión
(`turn_completed`, `session_ended`), registro de fin en el índice y baja del
registro vivo. Una sesión de autoría completada SHALL listarse como
finalizada — nunca como interrumpida — y su tiempo activo SHALL contar en la
analítica local. Un turno que falla SHALL cerrar igualmente la sesión con la
razón de error, sin inventar un estatus final.

#### Scenario: Turno de autoría finalizado queda cerrado
- **WHEN** un verbo del ciclo SDD completa su turno ACP
- **THEN** el log de la sesión SHALL contener `session_ended` y el índice un fin registrado
- **AND** `session/list` SHALL listarla como finalizada

#### Scenario: Fallo del turno también cierra
- **WHEN** el turno ACP de un verbo del ciclo falla al arrancar o ejecutar
- **THEN** la sesión SHALL cerrarse con la razón de error y sin estatus final inventado
