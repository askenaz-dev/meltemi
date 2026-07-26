# mobile-companion — delta

## MODIFIED Requirements

### Requirement: Alcance del compañero móvil
La superficie móvil de Meltemi SHALL ser el puesto remoto del **Agent Boss**:
una superficie compañera limitada a cuatro verbos — **monitorear** (estado de
la flota, sesiones, tareas y lo que espera decisión), **aprobar** (peticiones
de permiso y planes pendientes), **revisar** (leer los diffs de una carrera y
decidir sobre el trabajo: gates, checklist de revisión, adopción o reversión
de archivos con confirmación explícita) y **dirigir** (enviar instrucciones a
sesiones existentes). Revisar es decidir, no redactar: la superficie móvil
NO SHALL ofrecer autoría de código ni de specs (edición libre de contenido);
adoptar o revertir bajo confirmación es una decisión gobernada y trazable, no
edición. Toda ampliación de este alcance exige enmienda fundacional previa.

#### Scenario: Aprobación de una petición de permiso desde el móvil
- **WHEN** un agente solicita un permiso y el usuario responde desde la superficie móvil
- **THEN** la aprobación o denegación es explícita por petición, con el mismo efecto que si se emitiera desde la TUI o la GUI

#### Scenario: Adopción de un archivo de un competidor desde el móvil
- **WHEN** el usuario revisa una carrera desde la superficie móvil y adopta el archivo de un competidor
- **THEN** la adopción exige confirmación explícita y queda trazada como cualquier decisión de merge
- **AND** la superficie móvil NO SHALL ofrecer editar el contenido adoptado

#### Scenario: Propuesta de autoría en el móvil
- **WHEN** una propuesta de cambio introduce autoría de código o specs (edición libre de contenido) en la superficie móvil
- **THEN** la propuesta se rechaza salvo enmienda fundacional aprobada que amplíe el alcance del compañero
