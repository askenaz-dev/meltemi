## ADDED Requirements

### Requirement: Árbol Proyecto → Sesiones en el sidebar
El sidebar SHALL presentar el árbol de los proyectos conocidos con sus sesiones,
y cada sesión SHALL mostrar el avatar de su agente, el nombre de su suscripción
como pill y su badge de estado, honrando la densidad y la escala de elevación del
design system normativo (`design-system/`). El árbol MUST NOT animar su layout
cuando llegan o terminan sesiones: nada se mueve bajo el cursor. WHERE la raíz de
un proyecto ya no existe en disco, su nodo SHALL mostrarse marcado como ausente
con su remedio y MUST NOT desaparecer sin aviso.

#### Scenario: Árbol con dos proyectos y sus sesiones
- **WHEN** hay sesiones en dos proyectos distintos
- **THEN** el sidebar SHALL mostrar un nodo por proyecto con sus sesiones debajo
- **AND** cada sesión SHALL mostrar agente y suscripción sin abrir el detalle

#### Scenario: Dos suscripciones del mismo agente distinguibles
- **WHEN** un proyecto tiene dos sesiones del mismo agente lanzadas con perfiles distintos
- **THEN** ambas SHALL mostrar el mismo avatar de agente
- **AND** SHALL distinguirse por el nombre de su suscripción

#### Scenario: Proyecto ausente en disco marcado en el árbol
- **WHEN** el árbol incluye un proyecto cuya raíz ya no existe
- **THEN** su nodo SHALL renderizarse marcado como ausente con símbolo y palabra
- **AND** SHALL ofrecer el siguiente paso en vez de desaparecer

#### Scenario: El árbol no salta bajo el cursor
- **WHILE** el usuario tiene el puntero sobre una sesión del árbol
- **WHEN** otra sesión arranca o termina
- **THEN** los controles bajo el puntero SHALL NOT desplazarse por animación de layout

### Requirement: Ámbito de proyecto conmutable y persistente
El conmutador de proyecto del sidebar SHALL fijar el proyecto activo de la
superficie a partir de los proyectos conocidos; el directorio de trabajo del
proceso SHALL ser únicamente el ámbito inicial por omisión y MUST NOT seguir
siendo una jaula. Toda invocación de un método con ámbito de proyecto SHALL usar
el proyecto activo, el ámbito activo MUST persistir en el directorio de datos del
usuario, y sin proyecto activo las vistas Sesiones, Permisos y Flota MUST seguir
plenamente usables.

#### Scenario: Conmutar el proyecto activo reenfoca las vistas
- **WHEN** el usuario elige otro proyecto en el conmutador del sidebar
- **THEN** las vistas con ámbito de proyecto SHALL consultar el nuevo proyecto activo
- **AND** el chrome SHALL reflejar el ámbito vigente

#### Scenario: El ámbito activo sobrevive al reinicio
- **WHEN** el usuario fija un proyecto activo distinto del cwd y reabre la aplicación
- **THEN** la superficie SHALL restaurar ese proyecto como activo

#### Scenario: Sin proyecto activo las vistas globales siguen vivas
- **WHERE** no hay proyecto activo seleccionado
- **THEN** Sesiones, Permisos y Flota SHALL seguir operativas
- **AND** solo lo que exige un proyecto SHALL mostrar su estado vacío con el siguiente paso

### Requirement: Selector de proyecto en el lanzador de sesión
El lanzador de "Nueva sesión" SHALL ofrecer, además de agente o perfil y modo, la
elección del proyecto entre los conocidos, con el proyecto activo preseleccionado,
y MUST NOT introducir métodos nuevos del contrato para lanzar el trabajo.

#### Scenario: Lanzar en otro proyecto sin conmutar antes
- **WHEN** el usuario abre el lanzador y elige un proyecto distinto del activo
- **THEN** la sesión SHALL lanzarse sobre la raíz de ese proyecto
- **AND** SHALL invocar únicamente métodos existentes del contrato

#### Scenario: Suscripción elegible por nombre en el lanzador
- **WHEN** el usuario despliega la selección de agente en el lanzador
- **THEN** los perfiles SHALL ofrecerse por su nombre de suscripción junto a su agente subyacente
