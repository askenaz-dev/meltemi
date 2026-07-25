## ADDED Requirements

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
