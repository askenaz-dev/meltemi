# conversational-session — delta

## ADDED Requirements

### Requirement: El stream de sesión lleva el evento completo, no solo el del agente
El stream por sesión SHALL entregar a sus destinatarios —la conexión que inició
la sesión y toda conexión que declare mirarla— **el mismo conjunto de eventos**
que el registro de sesión persiste, y no únicamente las actualizaciones del
agente: el prompt enviado, la instrucción encolada, la petición y la decisión de
permiso, el cierre de turno y los eventos de ciclo de vida SHALL llegar por el
mismo canal, con su tipo discriminado y en orden de emisión. Cada evento MUST
entregarse una sola vez por conexión, y el daemon MUST NOT introducir un método
nuevo para ello: el canal, la suscripción y la forma del evento son los
vigentes. La decisión de un permiso SHALL seguir viajando por su propio camino
decidible; el evento es su traza de auditoría, nunca su reemplazo.

#### Scenario: El cliente en vivo ve abrir y cerrar el turno
- **WHEN** una sesión en curso envía un prompt al agente y el turno concluye
- **THEN** la conexión que la inició SHALL recibir el evento de prompt enviado y el de turno completado por el stream
- **AND** SHALL poder delimitar el turno sin releer el registro de sesión

#### Scenario: Cada evento una sola vez por conexión
- **WHEN** una conexión inicia una sesión y además declara mirarla
- **THEN** SHALL recibir cada evento de esa sesión exactamente una vez

#### Scenario: El permiso sigue decidiéndose por su propio camino
- **WHEN** una petición de permiso escala al humano
- **THEN** el stream SHALL llevar el evento de petición
- **AND** la petición decidible SHALL seguir llegando por el camino de permisos vigente con sus opciones y su plazo

### Requirement: Home conversacional como vista de llegada
La superficie de escritorio SHALL presentar como vista de llegada un compositor
de instrucción con el contexto como controles dentro de él —proyecto, agente o
perfil, y modo—, con el modo **libre** por defecto y los modos del método
(proponer, explorar) ofrecidos en el mismo compositor como elección, nunca como
peaje previo. Enviar SHALL navegar hacia adentro de la sesión creada; la
superficie MUST NOT limitarse a avisar que se lanzó. Todos los puntos de entrada
vigentes —acción primaria del chrome, atajo, estados vacíos y la acción de
proponer de la vista Proyecto— SHALL rutear al compositor con su contexto
prefijado, y el lanzador modal SHALL retirarse. El compositor SHALL invocar
únicamente métodos declarados en la matriz de paridad.

#### Scenario: Llegar y escribir
- **WHEN** el usuario abre la aplicación con un proyecto activo
- **THEN** la vista de llegada SHALL presentar el compositor enfocado con proyecto, agente y modo visibles
- **AND** el modo preseleccionado SHALL ser el libre

#### Scenario: Enviar navega hacia adentro
- **WHEN** el usuario envía una instrucción desde el compositor
- **THEN** la superficie SHALL navegar a la conversación de la sesión creada
- **AND** SHALL NOT quedarse en la vista de llegada con un aviso

#### Scenario: El método está a un gesto en el mismo compositor
- **WHEN** el usuario elige el modo proponer o explorar en el compositor
- **THEN** la misma instrucción SHALL despacharse por el verbo del método correspondiente
- **AND** el método invocado SHALL declararse de forma visible antes de enviar

#### Scenario: Los puntos de entrada vigentes rutean al compositor
- **WHEN** el usuario activa la acción primaria del chrome, su atajo, o la acción de proponer de la vista Proyecto
- **THEN** la superficie SHALL abrir el compositor con el contexto de origen ya prefijado
- **AND** ningún lanzador modal SHALL aparecer

### Requirement: Compositor persistente en la conversación con estados honestos
La vista de una sesión SHALL incluir un compositor persistente que envía la
instrucción a esa sesión por el verbo de dirección vigente, con la sesión ya
fijada. Los estados SHALL decir la verdad: sobre una sesión en curso la
superficie SHALL declarar que la instrucción quedó **encolada** con su posición
—MUST NOT presentarla como atendida—; sobre una sesión terminada y reanudable
SHALL ofrecer reanudar en vez de enviar; sobre una sesión viva que no admite
dirección SHALL mostrar el diagnóstico y su remedio; y mientras la sesión espera
una decisión de permiso SHALL decir qué espera. Enviar MUST NOT interrumpir ni
cancelar el turno en curso: cancelar SHALL seguir siendo un control distinto y
explícito.

#### Scenario: Instrucción encolada se declara encolada
- **WHEN** el usuario envía una instrucción a una sesión que está ejecutando su turno
- **THEN** la conversación SHALL mostrarla como encolada con su posición
- **AND** SHALL NOT presentarla como enviada al agente

#### Scenario: Sesión terminada ofrece reanudar, no enviar
- **WHEN** el usuario abre la conversación de una sesión terminada y reanudable
- **THEN** el compositor SHALL ofrecer reanudar con la instrucción
- **AND** SHALL NOT ofrecer un envío que la sesión no puede atender

#### Scenario: Sesión no dirigible lo dice con remedio
- **IF** la sesión está viva pero conduce su propio bucle y no admite dirección
- **THEN** el compositor SHALL mostrar el diagnóstico del daemon y su remedio
- **AND** SHALL NOT ofrecer enviar

#### Scenario: Enviar no interrumpe
- **WHILE** una sesión ejecuta su turno
- **WHEN** el usuario envía una instrucción desde el compositor
- **THEN** el turno en curso SHALL continuar intacto
- **AND** la cancelación SHALL seguir siendo un control aparte

### Requirement: Burbujas de turno como lectura del log, jamás en su lugar
El transcript SHALL ofrecer una lectura conversacional plegada sobre el mismo
flujo de eventos, y un conmutador al log de operador completo y crudo. Las
reglas de plegado SHALL ser: el prompt enviado abre un turno humano; la
instrucción encolada se muestra como turno humano pendiente hasta que su prompt
llegue; las actualizaciones de texto del agente se acumulan en el turno del
agente; el pensamiento se muestra plegado y separado de la prosa; las llamadas a
herramienta se muestran como elementos actualizables en su sitio; y el cierre de
turno cierra la burbuja mostrando su motivo. Todo evento que el plegado no sepa
clasificar —incluidos los tipos desconocidos y las formas de actualización no
reconocidas— SHALL renderizarse **en su lugar** como línea neutra de sistema con
su nombre, y MUST NOT omitirse. Conmutar entre lecturas MUST NOT perder ni
descartar eventos: el número de eventos del log de operador SHALL ser igual al
número de eventos recibidos.

#### Scenario: Un turno se pliega de prompt a cierre
- **WHEN** la conversación recibe el prompt enviado, las actualizaciones de texto del agente y el cierre de turno
- **THEN** SHALL presentarlos como un turno humano seguido de un turno del agente
- **AND** el cierre SHALL mostrar su motivo

#### Scenario: Evento no clasificable cae a la vista, no al olvido
- **WHEN** llega un evento cuyo tipo el plegado no conoce
- **THEN** SHALL renderizarse en su posición como línea neutra con su nombre crudo
- **AND** SHALL seguir presente en el log de operador

#### Scenario: El conmutador no pierde nada
- **WHEN** el usuario conmuta de la lectura conversacional al log de operador y vuelve
- **THEN** el log de operador SHALL contener tantos eventos como se recibieron
- **AND** ninguna lectura SHALL inventar eventos que el registro no tenga

#### Scenario: El pensamiento no se mezcla con la respuesta
- **WHEN** el agente emite pensamiento y prosa en el mismo turno
- **THEN** el pensamiento SHALL mostrarse plegado y distinguible
- **AND** SHALL NOT concatenarse dentro del texto de la respuesta

### Requirement: Peticiones de permiso en línea dentro de la conversación
Una petición de permiso SHALL renderizarse como tarjeta en su posición dentro de
la conversación, con el contenido mínimo vigente de la petición y sus opciones,
y SHALL decidirse por los métodos de permiso ya existentes: la conversación es
otra vista de la misma cola, MUST NOT ser otra cola. Una tarjeta cuya petición ya
no está pendiente —decidida en otra superficie, vencida por plazo, o denegada por
ausencia de clientes— SHALL mostrarse resuelta con su resultado y quién lo
decidió, y MUST NOT ofrecer controles accionables. La bandeja de permisos vigente
SHALL seguir siendo la vista completa, y MUST NOT perder ninguna petición por el
hecho de que exista la tarjeta.

#### Scenario: Decidir un permiso sin salir de la conversación
- **WHEN** el agente pide un permiso durante una conversación abierta
- **THEN** la conversación SHALL mostrar la tarjeta con la petición y sus opciones
- **AND** decidir desde ella SHALL resolver la misma petición de la cola del proxy

#### Scenario: Tarjeta ya resuelta no es accionable
- **WHEN** la petición de una tarjeta visible se decide en otra superficie o vence
- **THEN** la tarjeta SHALL pasar a mostrar el resultado y quién lo decidió
- **AND** SHALL NOT conservar controles que ya no deciden nada

#### Scenario: La bandeja sigue siendo la vista completa
- **WHILE** hay una tarjeta de permiso en una conversación
- **THEN** la bandeja de permisos SHALL seguir listando esa petición
- **AND** su prioridad de señales SHALL comportarse como hoy

### Requirement: Dirección interactiva de una sesión desde la TUI
La TUI SHALL ofrecer dirigir una instrucción a la sesión abierta desde su
drill-in, con una entrada de texto que **preserve el texto tal cual se escribe**
—mayúsculas incluidas—, y MUST presentar los mismos estados honestos que la
superficie de escritorio: encolada con su posición, reanudación cuando la sesión
terminó y es reanudable, y diagnóstico con remedio cuando no admite dirección.
El verbo de dirección MUST dejar de estar anunciado como reservado en la paleta,
porque una capacidad anunciada y no cableada es una promesa que el shell no
cumple.

#### Scenario: Instrucción dirigida desde el drill-in
- **WHEN** el usuario dirige una instrucción a la sesión abierta desde la TUI
- **THEN** el shell SHALL enviarla a esa sesión
- **AND** SHALL informar si quedó encolada, con su posición, o si reanudó la sesión

#### Scenario: El texto de la instrucción no se altera
- **WHEN** el usuario escribe una instrucción con mayúsculas y signos
- **THEN** la instrucción enviada SHALL ser idéntica a la tecleada
- **AND** SHALL NOT normalizarse a minúsculas por el camino de la paleta

#### Scenario: El verbo deja de anunciarse como reservado
- **WHEN** el usuario abre la paleta y localiza el verbo de dirección
- **THEN** SHALL presentarse como operativo
- **AND** activarlo SHALL abrir su entrada de instrucción, nunca cerrar el overlay sin efecto
