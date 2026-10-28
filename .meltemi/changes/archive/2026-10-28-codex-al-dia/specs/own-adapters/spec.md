# own-adapters — delta

## MODIFIED Requirements

### Requirement: Conformidad por versión y desfase rehusado
WHERE el CLI pilotado puede volcar el esquema de su cable por versión, los
tipos del adaptador SHALL validarse contra fixtures de ese esquema en la
integración continua, todo campo que el adaptador envía SHALL existir en el
esquema volcado, el guion simulado del cable SHALL seguir el orden de una
transcripción del CLI real de la versión anclada salvo las diferencias que su
cabecera declare, y el adaptador SHALL declarar un rango soportado cuyo piso es
la versión verificada contra ese volcado. Una versión fuera del rango MUST
detectarse en el handshake y rehusarse con diagnóstico y remedio. WHERE el
adaptador declara una versión verificada, una versión dentro del rango y
posterior a la verificada SHALL abrir la sesión declarando esa distancia en la
sesión y en el log, NO SHALL presentarse como verificada, y un valor de
gobierno que el adaptador no reconozca SHALL rehusar la sesión. El binario
efectivo y la versión del CLI pilotado SHALL constar en el log de sesión.

#### Scenario: Tipos validados contra el esquema volcado
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** los tipos del cable SHALL validarse contra el fixture del esquema de la versión soportada
- **AND** una divergencia SHALL fallar la verificación señalando el campo

#### Scenario: Un campo que el esquema no declara no viaja
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** cada clave de los parámetros y resultados que el adaptador envía SHALL existir en las propiedades del esquema volcado
- **AND** una clave desconocida SHALL fallar la verificación nombrándola

#### Scenario: El guion simulado sigue al cable real
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** la secuencia de métodos del guion simulado SHALL ser una subsecuencia ordenada de la transcripción vendorizada de la versión anclada
- **AND** cada línea del guion SHALL validar contra el volcado

#### Scenario: Desfase de versión rehusado con remedio
- **IF** el handshake revela una versión del CLI fuera del rango soportado por el adaptador
- **THEN** el adaptador SHALL rehusar con diagnóstico que nombre ambas versiones
- **AND** el remedio SHALL indicar qué actualizar

#### Scenario: Versión posterior a la verificada declarada, no rehusada
- **WHERE** el adaptador declara una versión verificada
- **WHEN** el handshake revela una versión dentro del rango y posterior a la verificada
- **THEN** la sesión SHALL abrirse con una fila visible que nombre la versión efectiva y la verificada
- **AND** el log SHALL registrar las dos

#### Scenario: Los valores de gobierno del esquema están pineados
- **WHEN** corre la verificación de conformidad de un adaptador que exige valores de gobierno
- **THEN** los valores posibles de política de aprobación, revisor y sandbox del volcado SHALL coincidir con los que el adaptador conoce
- **AND** un valor nuevo SHALL fallar la verificación nombrándolo

#### Scenario: Versión efectiva registrada en el log
- **WHEN** una sesión pilotada por un adaptador propio se abre con éxito
- **THEN** el log de sesión SHALL registrar el binario efectivo del CLI pilotado y su versión

### Requirement: Passthrough de permisos con compuerta dura y pérdidas visibles
Las peticiones de permiso del CLI pilotado SHALL relevarse a
`session/request_permission` de la sesión ACP del adaptador, de modo que
las decida el proxy de permisos vigente, y el adaptador SHALL configurar
además la compuerta dura nativa del CLI, la más estricta que el CLI ofrezca,
para que ninguna llamada a herramienta proceda sin haber pasado por esa vía,
incluso en los modos permisivos del propio CLI. WHERE la compuerta nativa del
CLI no somete a aprobación una clase de herramientas, el adaptador SHALL
declarar esa clase en la sesión al abrirla, como pérdida visible, y SHALL
nombrar en la sesión cada servidor de herramientas que el CLI arranque por su
cuenta. WHERE la superficie del proveedor no puede relevar una interacción
(herramientas solo interactivas), la denegación automática MUST mostrarse en
la sesión con su motivo y el adaptador MUST NOT aprobarla por su cuenta. El
relevo MUST NOT abrir transporte nuevo alguno en el daemon. El canal privado
de una sesión SHALL ser privado del usuario y SHALL retirarse al cerrarse la
sesión en orden; WHERE el adaptador termina sin poder ejecutar ese cierre, los
canales que ninguna sesión puede estar usando SHALL retirarse al abrirse el
canal siguiente, de modo que la configuración resuelta de una sesión no se
acumule sin límite. Ese retiro MUST NOT alcanzar el canal de una sesión que
pueda seguir viva, y su peor efecto posible SHALL ser una denegación y nunca
una aprobación.

#### Scenario: Permiso decidido por el proxy vigente
- **WHEN** el CLI pilotado solicita aprobación para una herramienta
- **THEN** la petición SHALL llegar como `session/request_permission` de la sesión del adaptador
- **AND** la decisión SHALL ser la que el proxy de permisos resuelva, sin atajo alguno

#### Scenario: Compuerta dura sobre modo permisivo del CLI
- **WHEN** el CLI pilotado corre en su modo más permisivo
- **THEN** la compuerta dura configurada por el adaptador SHALL seguir denegando toda llamada no aprobada
- **AND** ninguna herramienta fuera de las clases declaradas como pérdida SHALL ejecutarse sin decisión del proxy

#### Scenario: Lo que la compuerta del CLI no cubre se declara al abrir
- **WHERE** la compuerta nativa del CLI deja clases de herramientas sin aprobación
- **WHEN** se abre una sesión del adaptador
- **THEN** la sesión SHALL mostrar, antes del primer turno, una fila que nombre esas clases

#### Scenario: Un servidor de herramientas que el CLI arranca se nombra
- **WHEN** el CLI pilotado informa que arranca un servidor de herramientas de su propia configuración
- **THEN** la sesión SHALL mostrar una fila que lo nombre, una sola vez por sesión

#### Scenario: Interacción no relevable denegada con motivo visible
- **WHERE** el proveedor auto-deniega una herramienta que exige interacción directa
- **THEN** la sesión SHALL mostrar la denegación con su motivo
- **AND** el adaptador SHALL NOT aprobarla ni ocultarla

#### Scenario: Canal de una sesión muerta retirado en la apertura siguiente
- **WHERE** un adaptador terminado sin cierre dejó su canal con la configuración resuelta de su sesión
- **THEN** la apertura del canal de una sesión nueva SHALL retirar los canales que ninguna sesión puede estar usando
- **AND** SHALL NOT retirar el canal de una sesión que pueda seguir viva

## ADDED Requirements

### Requirement: La versión se lee donde el proveedor la escribe

El adaptador del dialecto de servidor SHALL leer la versión del CLI solo del
primer token del user agent del handshake, y SHALL aceptar como producto de ese
token el nombre propio del CLI o el nombre con que el adaptador se presentó.
El adaptador SHALL presentarse con su propio nombre y NO SHALL presentarse con
el nombre de otro cliente. Cualquier otra forma SHALL rehusarse con un
diagnóstico que diga lo que se vio y las formas esperadas. WHEN el hilo abierto
declara la versión del CLI que lo creó y esta difiere de la leída en el
handshake, el adaptador SHALL rehusar la sesión.

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

#### Scenario: El adaptador se presenta con su propio nombre
- **WHEN** el adaptador envía el handshake
- **THEN** el nombre del cliente SHALL ser el nombre propio del adaptador
- **AND** NO SHALL ser el nombre de ningún cliente del proveedor

#### Scenario: El hilo contradice la versión leída
- **IF** la respuesta del hilo declara una versión del CLI distinta de la leída en el handshake
- **THEN** el adaptador SHALL rehusar la sesión con diagnóstico que nombre las dos

### Requirement: La compuerta del servidor se pide, se verifica y falla cerrada

El adaptador del dialecto de servidor SHALL pedir al abrir el hilo la política
de aprobación y el sandbox más estrictos que el CLI ofrece y al cliente como
revisor de las aprobaciones, y SHALL leer de la respuesta los valores efectivos.
IF alguno de los valores efectivos no es exactamente el pedido, o falta, o el
sandbox efectivo permite la red, THEN el adaptador SHALL rehusar la sesión con
diagnóstico y remedio. Los valores efectivos SHALL constar en el log de sesión.
IF el CLI informa que su control remoto no está desactivado, THEN el adaptador
SHALL rehusar la sesión o, durante un turno, SHALL interrumpirlo y rehusarlo con
diagnóstico y remedio.

#### Scenario: El adaptador pide la compuerta más estricta
- **WHEN** el adaptador abre el hilo de una sesión
- **THEN** la petición SHALL declarar la política de aprobación más estricta, el sandbox de solo lectura y al cliente como revisor

#### Scenario: Un valor efectivo distinto del pedido rehúsa la sesión
- **IF** la respuesta del hilo declara una política, un revisor o un sandbox distinto del pedido
- **THEN** el adaptador SHALL rehusar la sesión con diagnóstico y remedio

#### Scenario: Un valor desconocido o ausente rehúsa la sesión
- **IF** la respuesta del hilo omite un valor de gobierno o trae uno que el adaptador no conoce
- **THEN** el adaptador SHALL rehusar la sesión

#### Scenario: La compuerta efectiva consta en el log
- **WHEN** una sesión del dialecto de servidor se abre con éxito
- **THEN** el log SHALL registrar la política de aprobación, el revisor y el sandbox efectivos

#### Scenario: Un control remoto activo rehúsa la sesión
- **IF** el CLI informa un control remoto que no está desactivado
- **THEN** el adaptador SHALL rehusar la sesión, o interrumpir y rehusar el turno en curso, con diagnóstico y remedio

### Requirement: Un turno solo lo mueve lo que es suyo

El adaptador del dialecto de servidor SHALL ignorar, a efectos del turno, toda
notificación de un hilo distinto del de su sesión y, una vez conocido el turno
en curso, toda notificación de un turno distinto o ya terminado; una
notificación que no declara hilo o turno NO SHALL filtrarse por ese eje. El
adaptador SHALL declinar sin preguntar al proxy, y a la vista, toda aprobación
de otro hilo o de otro turno, y SHALL declarar en la sesión cada hilo que el CLI
abra como hijo del de la sesión. El eco del mensaje del usuario NO SHALL
mostrarse en la sesión. Un turno que el servidor reporta como fallido SHALL
rehusarse con el mensaje del proveedor. Los avisos que el proveedor escribe
para el humano SHALL anotarse en el registro del adaptador, también durante el
handshake, y NO SHALL mostrarse como palabra del agente.

#### Scenario: Lo que dice otro hilo no cierra el turno en curso
- **WHEN** llega el cierre de un turno de otro hilo mientras corre un turno de la sesión
- **THEN** el turno de la sesión SHALL seguir en curso

#### Scenario: El cierre rezagado de un turno anterior no cierra el siguiente
- **WHEN** el cierre de un turno ya terminado llega durante el turno siguiente
- **THEN** el turno siguiente SHALL seguir en curso

#### Scenario: Una aprobación de otro hilo o turno se declina a la vista
- **WHEN** llega una aprobación de un hilo o de un turno que no es el de la sesión en curso
- **THEN** el adaptador SHALL declinarla sin preguntar al proxy
- **AND** la sesión SHALL mostrar una herramienta fallida que diga por qué

#### Scenario: Un hilo hijo se declara en la sesión
- **WHEN** el CLI abre un hilo cuyo padre es el hilo de la sesión
- **THEN** la sesión SHALL mostrar una fila que lo declare

#### Scenario: El eco del mensaje del usuario no se muestra
- **WHEN** el servidor devuelve el mensaje del usuario como ítem del turno
- **THEN** la sesión NO SHALL mostrarlo

#### Scenario: Un turno fallido se explica con las palabras del proveedor
- **WHEN** el servidor cierra un turno como fallido con un mensaje de error
- **THEN** el rehúso del turno SHALL incluir ese mensaje

#### Scenario: Los avisos del proveedor se anotan y no se dicen como agente
- **WHEN** el servidor emite un aviso para el humano, durante el handshake o durante un turno
- **THEN** el adaptador SHALL anotarlo en su registro sin rutas del estado del CLI
- **AND** la sesión NO SHALL mostrarlo como palabra del agente

### Requirement: Toda petición del servidor tiene una respuesta declarada

El adaptador del dialecto de servidor SHALL declarar, para cada petición que
el esquema volcado permite al servidor, si la releva al proxy o la rehúsa.
Una petición rehusada SHALL responderse con el error de método no
implementado del protocolo y SHALL mostrarse en la sesión como una
herramienta fallida que nombre la petición, sin sus parámetros. Las peticiones
que piden credenciales o atestiguar la identidad del cliente NO SHALL llegar
jamás al proxy ni registrar sus parámetros. Una aprobación relevada SHALL
mostrar al proxy lo que de verdad concede: el host y el protocolo de un acceso
de red, la escritura en una terminal en curso como distinta de un comando
nuevo, y la raíz bajo la que un cambio de archivo pide escribir por el resto de
la sesión.

#### Scenario: Una petición nueva del proveedor falla la conformidad
- **WHEN** el esquema volcado declara una petición del servidor sin respuesta declarada en el adaptador
- **THEN** la verificación de conformidad SHALL fallar nombrando la petición

#### Scenario: Una petición no relevada se rehúsa a la vista
- **WHEN** el servidor hace una petición que el adaptador no releva
- **THEN** el adaptador SHALL responder el error de método no implementado
- **AND** la sesión SHALL mostrar una herramienta fallida que nombre la petición

#### Scenario: Credenciales y atestación jamás llegan al proxy
- **WHEN** el servidor pide refrescar credenciales o atestiguar al cliente
- **THEN** el adaptador SHALL responder con error sin preguntar al proxy
- **AND** NO SHALL registrar los parámetros de la petición

#### Scenario: Una aprobación de red muestra a qué se conecta
- **WHEN** el servidor pide aprobación para un acceso de red
- **THEN** la petición de permiso SHALL mostrar el host y el protocolo

#### Scenario: Una escritura en una terminal se distingue de un comando
- **WHEN** el servidor pide aprobación para escribir en una terminal en curso
- **THEN** la petición de permiso SHALL decirlo y no presentarla como un comando nuevo

#### Scenario: Una raíz de escritura para toda la sesión se muestra
- **WHEN** el servidor pide aprobar un cambio de archivo junto con escribir bajo una raíz por el resto de la sesión
- **THEN** la petición de permiso SHALL mostrar esa raíz y ese alcance
