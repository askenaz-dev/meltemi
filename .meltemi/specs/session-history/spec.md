# session-history Specification

## Purpose
TBD - created by archiving change sesiones-reanudables. Update Purpose after archive.
## Requirements

### Requirement: Metadatos persistentes de sesión
El daemon SHALL persistir los metadatos de cada sesión (id, agente, proyecto,
estado final, marcas de inicio y fin, ruta del log) en un índice apend-only por
proyecto, y MUST poder reconstruir el índice desde los logs cuando falte o esté
dañado: los logs son la fuente de verdad.

#### Scenario: Índice reconstruible
- **WHEN** el índice de sesiones falta y existen logs JSONL
- **THEN** el daemon SHALL reconstruir el índice desde los logs
- **AND** el listado SHALL reflejar esas sesiones

### Requirement: Sesiones interrumpidas tras una caída
El daemon SHALL marcar como interrumpida, al arrancar, toda sesión del índice sin
fin registrado, y MUST NOT presentarla como activa. El estado interrumpido SHALL
ser visible en el listado y en las superficies.

#### Scenario: La caída no deja fantasmas
- **WHEN** el daemon arranca tras una terminación abrupta con sesiones a medias
- **THEN** esas sesiones SHALL listarse como interrumpidas
- **AND** ninguna SHALL aparecer como activa

### Requirement: Listado histórico por contrato
El daemon SHALL exponer `session/list` con activas e históricas (filtros de
proyecto y estado, límite y orden por recencia), de modo que ningún cliente
necesite leer el disco del daemon. WHERE la consulta no fija filtro de proyecto,
el listado SHALL abarcar todos los proyectos conocidos y cada sesión SHALL
declarar su raíz de proyecto, de modo que una sola consulta baste para agregar el
árbol Proyecto → Sesiones sin repetir la llamada por proyecto.

#### Scenario: Históricas listadas
- **WHEN** un cliente invoca `session/list` sobre un proyecto con sesiones finalizadas
- **THEN** la respuesta SHALL incluirlas con su estado final y marcas temporales

#### Scenario: Listado global agregable por proyecto
- **WHEN** un cliente invoca `session/list` sin filtro de proyecto habiendo sesiones en dos proyectos
- **THEN** la respuesta SHALL incluir las sesiones de ambos
- **AND** cada sesión SHALL declarar su raíz de proyecto para poder agruparlas

### Requirement: Lectura del registro por contrato
El daemon SHALL exponer `session/log` con lectura paginada del registro JSONL de
una sesión (por rango de líneas), apta para transcripts largos y acceso remoto
por túnel.

#### Scenario: Transcript paginado
- **WHEN** un cliente pide la última página del log de una sesión finalizada
- **THEN** la respuesta SHALL contener ese rango de eventos
- **AND** ofrecer continuidad hacia atrás por offset

### Requirement: Reanudación negociada con degradación honesta
El daemon SHALL ofrecer reanudar una sesión únicamente cuando el agente anunció
la capacidad de carga de sesiones en su handshake, abriendo una sesión nueva que
solicita la carga de la anterior; sin la capacidad, las superficies MUST
presentar la sesión como inspeccionable pero no reanudable, y la acción de
reanudar MUST advertir que el estado del repositorio pudo cambiar.

#### Scenario: Reanudar con capacidad anunciada
- **WHEN** el usuario reanuda una sesión de un agente que anuncia carga de sesiones
- **THEN** el daemon SHALL abrir una sesión nueva solicitando la carga de la anterior
- **AND** la nueva sesión SHALL quedar vinculada a la original en sus metadatos

#### Scenario: Sin capacidad, honestidad
- **WHERE** el agente no anuncia carga de sesiones
- **THEN** la superficie SHALL mostrar la sesión como no reanudable e inspeccionable
- **AND** SHALL NOT ofrecer una reanudación que no puede cumplir

### Requirement: Histórico en las superficies
La vista de Sesiones SHALL dar acceso a las históricas (filtro/etiqueta con la
línea base de accesibilidad) y el drill-in de una finalizada SHALL mostrar su
transcript desde la lectura por contrato. El subcomando CLI `sessions` SHALL
listar activas e históricas con variante `--json` de un objeto.

#### Scenario: Drill-in de una finalizada
- **WHEN** el usuario abre una sesión finalizada desde la tabla
- **THEN** el detalle SHALL mostrar su transcript leído por `session/log`
- **AND** SHALL indicar su estado final con glifo y palabra

#### Scenario: CLI sessions
- **WHEN** se invoca `meltemi sessions --json`
- **THEN** el binario SHALL emitir exactamente un objeto JSON con el listado

### Requirement: Agente y suscripción resueltos en los metadatos de sesión
Los metadatos de cada sesión SHALL registrar el id del agente resuelto y el
nombre del perfil de lanzamiento —la suscripción— cuando la resolución los nombró,
y el listado por contrato SHALL exponerlos, de modo que una sesión se lea como
«agente · suscripción» sin adivinar el agente desde la ruta del binario. WHERE el
índice falte, esos datos MUST reconstruirse desde el evento de resolución del log
de sesión. Los metadatos MUST NOT contener la sobrecapa de entorno del perfil ni
ningún material de autenticación (constitución §2).

#### Scenario: Sesión de un perfil declara su suscripción
- **WHEN** una sesión se lanza nombrando un perfil de lanzamiento
- **THEN** sus metadatos SHALL registrar el id del agente resuelto y el nombre del perfil
- **AND** el listado por contrato SHALL exponer ambos

#### Scenario: Suscripción reconstruida desde el log
- **WHEN** el índice de sesiones falta y el log conserva el evento de resolución
- **THEN** el agente y el perfil SHALL reconstruirse desde ese evento
- **AND** la sesión SHALL listarse con su suscripción visible

#### Scenario: El listado nunca lleva el contexto de autenticación
- **WHEN** una sesión de perfil se lista o se inspecciona
- **THEN** la respuesta SHALL llevar solo el nombre del perfil
- **AND** SHALL NOT incluir la sobrecapa de entorno ni material de autenticación

#### Scenario: Sesión sin perfil se lista sin campos inventados
- **WHERE** la sesión resolvió su agente sin perfil (id de catálogo o etiqueta libre)
- **THEN** el listado SHALL omitir el nombre de perfil
- **AND** SHALL NOT presentar una suscripción que no existió

## MODIFIED Requirements
