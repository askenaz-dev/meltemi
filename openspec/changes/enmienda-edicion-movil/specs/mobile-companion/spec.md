# Delta: mobile-companion (enmienda-edicion-movil)

## ADDED Requirements

### Requirement: Alcance del compañero móvil
La superficie móvil de Meltemi SHALL ser un compañero reducido limitado a tres verbos: **monitorear** (estado de la flota, sesiones y tareas), **aprobar** (peticiones de permiso y planes pendientes) y **dirigir** (enviar instrucciones a sesiones existentes). La superficie móvil NO SHALL ofrecer edición de código ni de specs; toda ampliación de este alcance exige enmienda fundacional previa.

#### Scenario: Aprobación de una petición de permiso desde el móvil
- **WHEN** un agente solicita un permiso y el usuario responde desde la superficie móvil
- **THEN** la aprobación o denegación es explícita por petición, con el mismo efecto que si se emitiera desde la TUI o la GUI

#### Scenario: Propuesta de edición en el móvil
- **WHEN** una propuesta de cambio introduce capacidades de edición de código o specs en la superficie móvil
- **THEN** la propuesta se rechaza salvo enmienda fundacional aprobada que amplíe el alcance del compañero

### Requirement: Regla de subconjunto respecto de TUI y GUI
Toda capacidad del daemon consumida por la superficie móvil SHALL estar disponible también desde la TUI y la GUI. La superficie móvil NO SHALL ser la única consumidora de ninguna capacidad del daemon (constitución §4: ninguna funcionalidad del daemon accesible desde una sola superficie).

#### Scenario: Nueva capacidad del daemon motivada por el móvil
- **WHEN** una propuesta de cambio añade al daemon una capacidad destinada a la superficie móvil
- **THEN** la propuesta incluye su consumo desde la TUI y la GUI, o se rechaza

### Requirement: Acceso remoto exclusivamente por túnel SSH
La superficie móvil SHALL conectarse al daemon únicamente a través de un túnel SSH del usuario que desemboque en el socket local (constitución §3). El daemon NO SHALL abrir transporte de red ni distinguir una conexión tunelizada de una local.

#### Scenario: Conexión del compañero móvil
- **WHEN** la superficie móvil establece conexión con `meltemid`
- **THEN** la conexión llega al socket local a través de un túnel SSH gestionado por el usuario, y el daemon la atiende como a cualquier cliente local

#### Scenario: Propuesta de transporte de red para el móvil
- **WHEN** una propuesta de cambio requiere que el daemon escuche en un puerto de red o dependa de un servicio de retransmisión en la nube para servir al móvil
- **THEN** la propuesta se rechaza (constitución §3 y rumbo tech: sin puertos de red, jamás; sin servicio en la nube)
