# implement-command Specification

## Purpose
TBD - created by archiving change comando-implement. Update Purpose after archive.
## Requirements
### Requirement: Despliegue de agentes sobre las tareas
El verbo `implement` SHALL ejecutar las tareas de la change en orden de
dependencias, componiendo por tarea: checkpoint previo, turno del agente en su
worktree con las reglas de permisos vigentes, verificación rápida cuando la tarea
declare comando, commit con trazabilidad y marca de la tarea; el progreso MUST
persistir en la change y sobrevivir reinicios.

#### Scenario: Tarea completa el ciclo compuesto
- **WHEN** implement ejecuta una tarea elegible
- **THEN** SHALL crearse su checkpoint, correr el agente en su worktree y cometer con trazabilidad
- **AND** la tarea SHALL quedar marcada en tasks.md

#### Scenario: Progreso sobrevive reinicio
- **WHEN** el daemon se reinicia con un implement a medias
- **THEN** el despliegue SHALL reanudarse en la primera tarea no completada

### Requirement: Modo planificar o actuar por tarea
El despliegue SHALL soportar planificar (el agente propone el plan de la tarea y
un gate humano lo aprueba antes de tocar nada) y actuar (ejecución directa); el
modo SHALL elegirse por change con override por tarea, y en planificar ningún
cambio MUST aplicarse antes del gate aprobado.

#### Scenario: Plan aprobado antes de actuar
- **WHEN** una tarea corre en modo planificar
- **THEN** el plan del agente SHALL presentarse en gate
- **AND** el árbol SHALL permanecer intacto hasta la aprobación

### Requirement: Autonomía solo dentro de guardarraíles
El modo autónomo SHALL operar únicamente con reglas de permisos definidas y
commit directo registrado; sin reglas aplicables al proyecto, el despliegue MUST
degradar a supervisado con aviso visible — jamás autonomía por accidente.

#### Scenario: Sin reglas no hay autonomía
- **WHEN** se solicita implement autónomo en un proyecto sin reglas de permisos
- **THEN** el despliegue SHALL correr supervisado
- **AND** SHALL avisar el motivo de la degradación

### Requirement: Progreso vivo e interrupción segura
La sesión de implement SHALL emitir eventos de progreso por tarea visibles en las
superficies (tarea actual, completadas, restantes); la interrupción entre tareas
SHALL dejar estado consistente (completadas cometidas y marcadas), y la
interrupción a mitad de tarea SHALL cancelar solo esa tarea dejando su worktree
disponible para revertir o inspeccionar.

#### Scenario: Interrupción entre tareas
- **WHEN** el usuario interrumpe el despliegue entre dos tareas
- **THEN** las completadas SHALL quedar cometidas y marcadas
- **AND** ninguna tarea nueva SHALL arrancar

#### Scenario: Interrupción a mitad de tarea
- **WHEN** el usuario interrumpe durante el turno de una tarea
- **THEN** la sesión de esa tarea SHALL cancelarse con confirmación
- **AND** su worktree SHALL quedar disponible para reversión o inspección

