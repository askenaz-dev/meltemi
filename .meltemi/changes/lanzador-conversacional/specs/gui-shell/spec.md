# gui-shell — delta

## MODIFIED Requirements

### Requirement: La sesión como acción primaria; proponer como herramienta
La superficie SHALL exponer "Nueva sesión" como acción primaria del chrome —a un
clic y con atajo— llevando al **compositor conversacional**, no a un lanzador
modal: allí se elige agente o perfil de la flota, proyecto y modo, con el modo
libre por defecto y los modos del método (explorar, proponer) ofrecidos en el
mismo compositor. `propose` MUST seguir alcanzable a una tecla (paleta y vista
Proyecto) como herramienta, MUST NOT ser la única entrada visible al trabajo, y
su activación SHALL rutear al compositor con el modo ya elegido. El despacho de
una tarea a un competidor SHALL conservar su entrada desde la superficie del
método. La superficie SHALL invocar únicamente métodos declarados en la matriz de
paridad —incluido el verbo de arranque de sesión libre— y MUST NOT inventar
rutas fuera del contrato. Todo estado vacío SHALL ofrecer su siguiente paso como
control ejecutable y enfocable, nunca solo como texto (Proyecto sin `.meltemi/`
SHALL ofrecer inicializar la constitución; Flota sin agentes SHALL ofrecer
refrescar la detección).

#### Scenario: Nueva sesión desde el chrome
- **WHEN** el usuario activa la acción primaria del chrome
- **THEN** la superficie SHALL abrir el compositor con proyecto, agente/perfil y modo
- **AND** el modo preseleccionado SHALL ser el libre

#### Scenario: Proponer sigue a una tecla
- **WHEN** el usuario abre la paleta o la vista Proyecto
- **THEN** `propose` SHALL estar disponible como herramienta directa
- **AND** activarlo SHALL llevar al compositor con ese modo ya elegido

#### Scenario: El compositor no inventa contrato
- **WHEN** el compositor despacha una instrucción en cualquiera de sus modos
- **THEN** SHALL invocar un método declarado en la matriz de paridad

#### Scenario: Inicializar desde el vacío de Proyecto
- **WHEN** la vista Proyecto muestra que el directorio no es un proyecto
- **THEN** SHALL ofrecer un control ejecutable que inicia `sdd/constitution`

#### Scenario: Flota vacía ofrece refrescar
- **WHEN** la vista Flota no detecta agentes
- **THEN** SHALL ofrecer un control que reejecuta la detección

### Requirement: Transcript de primera clase
El transcript SHALL renderizar cada evento con glifo y tono según su tipo (los
tipos desconocidos caen a neutro con su nombre crudo), SHALL permitir expandir el
texto completo de un evento, conmutar timestamps, copiar una línea o todo lo
cargado, y buscar dentro de lo cargado. El transcript SHALL ofrecer además una
**lectura conversacional plegada** sobre el mismo flujo de eventos, con un
conmutador explícito a este log de operador: el log es la verdad y la lectura
conversacional es una vista de él. Ninguna de las dos lecturas MUST omitir un
evento recibido, y conmutar entre ellas MUST NOT descartar nada de lo cargado.

#### Scenario: Evento con texto expandible
- **WHEN** un evento con texto excede la línea
- **THEN** el usuario SHALL poder expandirlo completo en su lugar

#### Scenario: Buscar en el transcript
- **WHEN** el usuario busca un término en el transcript
- **THEN** las líneas coincidentes SHALL resaltarse y ser navegables

#### Scenario: Tipo desconocido no rompe
- **WHEN** llega un evento de tipo no catalogado
- **THEN** SHALL renderizarse en tono neutro con su nombre crudo

#### Scenario: Conmutar entre conversación y log de operador
- **WHEN** el usuario conmuta desde la lectura conversacional al log de operador
- **THEN** el log SHALL mostrar todos los eventos cargados en orden de llegada
- **AND** el conteo de eventos SHALL coincidir con el recibido

### Requirement: Árbol Proyecto → Sesiones en el sidebar
El sidebar SHALL presentar de forma **persistente** una sección de proyectos con
el árbol de los proyectos conocidos y sus sesiones —siempre visible, no solo tras
abrir un modal—, y cada sesión SHALL mostrar el avatar de su agente, el nombre de
su suscripción como pill y su badge de estado, honrando la densidad y la escala
de elevación del design system normativo (`design-system/`). Cada proyecto SHALL
ofrecer conmutar el ámbito y una acción rápida para iniciar allí una sesión, que
lleva al compositor con el proyecto ya prefijado; la sección SHALL ofrecer además
dar de alta un directorio. El árbol MUST NOT animar su layout cuando llegan o
terminan sesiones: nada se mueve bajo el cursor. WHERE la raíz de un proyecto ya
no existe en disco, su nodo SHALL mostrarse marcado como ausente con su remedio y
MUST NOT desaparecer sin aviso.

#### Scenario: Árbol con dos proyectos y sus sesiones
- **WHEN** hay sesiones en dos proyectos distintos
- **THEN** el sidebar SHALL mostrar un nodo por proyecto con sus sesiones debajo
- **AND** cada sesión SHALL mostrar agente y suscripción sin abrir el detalle

#### Scenario: La sección de proyectos está siempre visible
- **WHEN** el usuario abre la aplicación con proyectos registrados
- **THEN** la sección de proyectos SHALL estar presente en el sidebar sin abrir ningún modal

#### Scenario: Acción rápida por proyecto lleva al compositor
- **WHEN** el usuario activa la acción de iniciar sesión en un proyecto del árbol
- **THEN** la superficie SHALL abrir el compositor con ese proyecto prefijado

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

### Requirement: Selector de proyecto en el lanzador de sesión
El compositor conversacional —el lanzador de sesión de esta superficie— SHALL
ofrecer, además de agente o perfil y modo, la
elección del proyecto entre los conocidos, con el proyecto activo preseleccionado,
y SHALL ofrecer **abrir una carpeta** que no esté registrada, dándola de alta por
el método del contrato antes de lanzar nada. El diálogo nativo de selección de
carpeta SHALL vivir en el cliente: la superficie MUST NOT pedir al daemon que
abra ventanas ni que enumere el sistema de archivos, y el daemon SHALL recibir
únicamente la ruta elegida. La superficie SHALL invocar únicamente métodos
declarados en la matriz de paridad para lanzar el trabajo y para registrar el
proyecto.

#### Scenario: Lanzar en otro proyecto sin conmutar antes
- **WHEN** el usuario abre el compositor y elige un proyecto distinto del activo
- **THEN** la sesión SHALL lanzarse sobre la raíz de ese proyecto
- **AND** SHALL invocar únicamente métodos declarados en la matriz de paridad

#### Scenario: Abrir una carpeta la registra antes de lanzar
- **WHEN** el usuario elige una carpeta no registrada desde el compositor
- **THEN** la superficie SHALL darla de alta por el método del contrato
- **AND** SHALL quedar disponible como proyecto en el sidebar

#### Scenario: El diálogo vive en el cliente
- **WHEN** la superficie abre el diálogo nativo de selección de carpeta
- **THEN** el daemon SHALL recibir únicamente la ruta resultante
- **AND** SHALL NOT participar en la presentación del diálogo

#### Scenario: Suscripción elegible por nombre en el lanzador
- **WHEN** el usuario despliega la selección de agente en el compositor
- **THEN** los perfiles SHALL ofrecerse por su nombre de suscripción junto a su agente subyacente
