# own-adapters — delta

## MODIFIED Requirements

### Requirement: Conformidad por versión y desfase rehusado
WHERE el CLI pilotado puede volcar el esquema de su cable por versión, los
tipos del adaptador SHALL validarse contra fixtures de ese esquema en la
integración continua, y el guion simulado del cable SHALL seguir la forma y el
orden de una transcripción del CLI real de la versión anclada. Todo campo que
el adaptador envía SHALL existir en el esquema volcado. El adaptador SHALL
declarar un rango soportado cuyo piso es la versión verificada contra ese
volcado. Una versión fuera del rango MUST detectarse en el handshake y
rehusarse con diagnóstico y remedio. Una versión dentro del rango y posterior
a la verificada SHALL abrir la sesión declarando esa distancia en la sesión y
en el log, y NO SHALL presentarse como verificada. El binario efectivo, la
versión del CLI pilotado y la versión verificada SHALL constar en el log de
sesión.

#### Scenario: Tipos validados contra el esquema volcado
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** los tipos del cable SHALL validarse contra el fixture del esquema de la versión soportada
- **AND** una divergencia SHALL fallar la verificación señalando el campo

#### Scenario: Un campo que el esquema no declara no viaja
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** cada clave de cada mensaje que el adaptador envía SHALL existir en las propiedades del esquema volcado
- **AND** una clave desconocida SHALL fallar la verificación nombrándola

#### Scenario: El guion simulado sigue al cable real
- **WHEN** se re-ancla el volcado vendorizado a una versión nueva
- **THEN** el guion simulado SHALL seguir la forma y el orden de una transcripción del CLI real de esa versión
- **AND** cada línea del guion SHALL validar contra el volcado

#### Scenario: Desfase de versión rehusado con remedio
- **IF** el handshake revela una versión del CLI fuera del rango soportado por el adaptador
- **THEN** el adaptador SHALL rehusar con diagnóstico que nombre ambas versiones
- **AND** el remedio SHALL indicar qué actualizar

#### Scenario: Versión posterior a la verificada declarada, no rehusada
- **WHEN** el handshake revela una versión dentro del rango y posterior a la verificada
- **THEN** la sesión SHALL abrirse con una nota visible que nombre la versión efectiva y la verificada
- **AND** el log SHALL registrar las dos

#### Scenario: Versión efectiva registrada en el log
- **WHEN** una sesión pilotada por un adaptador propio se abre con éxito
- **THEN** el log de sesión SHALL registrar el binario efectivo del CLI pilotado, su versión y la versión verificada

## ADDED Requirements

### Requirement: La versión se lee donde el proveedor la escribe

El adaptador del dialecto de servidor SHALL leer la versión del CLI solo del
primer token del user agent del handshake, y SHALL aceptar como producto de ese
token el nombre propio del CLI o el nombre con que el adaptador se presentó.
El adaptador NO SHALL presentarse con el nombre de otro cliente para obtener
una forma distinta del user agent. Cualquier otra forma SHALL rehusarse con un
diagnóstico que diga lo que se vio y las formas esperadas.

#### Scenario: El CLI escribe el nombre del cliente en su user agent
- **WHEN** el handshake responde un user agent cuyo primer token es el nombre del adaptador seguido de la versión del CLI
- **THEN** el adaptador SHALL leer esa versión

#### Scenario: Un token posterior con versión no se confunde con el CLI
- **WHEN** el user agent trae, después del primero, otro token con forma de producto y versión
- **THEN** el adaptador SHALL leer solo el primero
- **AND** SHALL rehusar si el primero no tiene un producto aceptado

#### Scenario: Una forma desconocida se rehúsa sin culpar al binario
- **IF** el primer token del user agent no tiene un producto aceptado o no trae versión
- **THEN** el adaptador SHALL rehusar con un diagnóstico que nombre lo visto y las formas esperadas

### Requirement: Las aprobaciones del servidor llegan al proxy o la sesión no abre

El adaptador del dialecto de servidor SHALL pedir al abrir el hilo que las
aprobaciones las revise el cliente, y SHALL leer de la respuesta la política
de aprobación, el revisor y el sandbox efectivos. IF el revisor efectivo no es
el cliente, o la política efectiva declara que nunca preguntará y el sandbox
efectivo permite escribir, THEN el adaptador SHALL rehusar la sesión con
diagnóstico y remedio. La política, el revisor y el sandbox efectivos SHALL
constar en el log de sesión. El adaptador NO SHALL imponer una política de
aprobación ni un sandbox distintos de los que el usuario configuró.

#### Scenario: El adaptador pide que las aprobaciones las revise el cliente
- **WHEN** el adaptador abre el hilo de una sesión
- **THEN** la petición SHALL declarar como revisor de aprobaciones al cliente

#### Scenario: Un revisor distinto del cliente rehúsa la sesión
- **IF** la respuesta del hilo declara un revisor efectivo distinto del cliente
- **THEN** el adaptador SHALL rehusar la sesión con diagnóstico y remedio

#### Scenario: Una política que nunca pregunta con sandbox que escribe rehúsa la sesión
- **IF** la respuesta del hilo declara una política que nunca pregunta y un sandbox que no es de solo lectura
- **THEN** el adaptador SHALL rehusar la sesión con diagnóstico y remedio

#### Scenario: La política efectiva consta en el log
- **WHEN** una sesión del dialecto de servidor se abre con éxito
- **THEN** el log SHALL registrar la política de aprobación, el revisor y el sandbox efectivos

### Requirement: Un turno solo lo mueve lo que es suyo

El adaptador del dialecto de servidor SHALL ignorar toda notificación de un
hilo distinto del de su sesión y, una vez conocido el turno en curso, toda
notificación de un turno distinto o ya terminado, y SHALL declinar sin
preguntar al proxy toda aprobación de otro hilo o de otro turno. El eco del
mensaje del usuario NO SHALL mostrarse en la sesión. Un turno que el servidor
reporta como fallido SHALL rehusarse con el mensaje del proveedor.

#### Scenario: Lo que dice otro hilo no cierra el turno en curso
- **WHEN** llega el cierre de un turno de otro hilo mientras corre un turno de la sesión
- **THEN** el turno de la sesión SHALL seguir en curso

#### Scenario: El cierre rezagado de un turno anterior no cierra el siguiente
- **WHEN** el cierre de un turno ya terminado llega durante el turno siguiente
- **THEN** el turno siguiente SHALL seguir en curso

#### Scenario: Una aprobación de otro turno se declina sin preguntar
- **WHEN** llega una aprobación de un turno que no es el turno en curso
- **THEN** el adaptador SHALL declinarla
- **AND** NO SHALL preguntar al proxy

#### Scenario: El eco del mensaje del usuario no se muestra
- **WHEN** el servidor devuelve el mensaje del usuario como ítem del turno
- **THEN** la sesión NO SHALL mostrarlo

#### Scenario: Un turno fallido se explica con las palabras del proveedor
- **WHEN** el servidor cierra un turno como fallido con un mensaje de error
- **THEN** el rehúso del turno SHALL incluir ese mensaje

### Requirement: Toda petición del servidor tiene una respuesta declarada

El adaptador del dialecto de servidor SHALL declarar, para cada petición que
el esquema volcado permite al servidor, si la releva al proxy o la rehúsa.
Una petición rehusada SHALL responderse con el error de método no
implementado del protocolo y SHALL mostrarse en la sesión como una
herramienta fallida que nombre la petición. Las peticiones que piden
credenciales o atestiguar la identidad del cliente NO SHALL llegar jamás al
proxy. Una aprobación de comando SHALL mostrar al proxy el host y el protocolo
de una aprobación de red, y SHALL distinguir la escritura en una terminal en
curso de la ejecución de un comando.

#### Scenario: Una petición nueva del proveedor falla la conformidad
- **WHEN** el esquema volcado declara una petición del servidor sin respuesta declarada en el adaptador
- **THEN** la verificación de conformidad SHALL fallar nombrando la petición

#### Scenario: Una petición no relevada se rehúsa a la vista
- **WHEN** el servidor hace una petición que el adaptador no releva
- **THEN** el adaptador SHALL responder el error de método no implementado
- **AND** la sesión SHALL mostrar una herramienta fallida que nombre la petición

#### Scenario: Credenciales y atestación jamás llegan al proxy
- **WHEN** el servidor pide refrescar credenciales o atestiguar al cliente
- **THEN** el adaptador SHALL responder con error
- **AND** NO SHALL preguntar al proxy

#### Scenario: Una aprobación de red muestra a qué se conecta
- **WHEN** el servidor pide aprobación para un acceso de red
- **THEN** la petición de permiso SHALL mostrar el host y el protocolo
