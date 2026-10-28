

### Requirement: Puente ACP gobernado sobre el binario oficial
Cada adaptador propio SHALL ser un binario que habla ACP por stdio hacia el
daemon y pilota exclusivamente el CLI oficial del proveedor como
subproceso, con la autenticación que ese CLI gestiona por su cuenta. El
adaptador MUST NOT embeber el runtime del proveedor como librería, MUST NOT
enlazar pila HTTP ni TLS alguna, MUST NOT leer, almacenar ni reenviar
material de autenticación, y MUST NOT escuchar en puerto alguno. WHEN el
CLI oficial no se encuentra o no puede lanzarse como subproceso, el
adaptador MUST rehusar con diagnóstico que nombre la capa ausente y su
remedio, y MUST NOT degradar a ninguna vía alternativa. El nombre con que el
catálogo declara ese CLI SHALL resolverse al archivo que la plataforma
ejecuta realmente, con el mismo criterio con que el catálogo lo detecta, de
modo que un CLI que el catálogo reporta presente sea lanzable por el
adaptador; el binario efectivamente resuelto SHALL constar en el log de
sesión.

#### Scenario: Adaptador sin pila de red
- **WHEN** se audita el árbol de dependencias del crate de adaptadores
- **THEN** ningún binario adaptador SHALL enlazar cliente HTTP ni pila TLS
- **AND** la auditoría de dependencias de CI SHALL verificar esa ausencia

#### Scenario: La autenticación queda en el binario oficial
- **WHEN** el CLI pilotado reporta un fallo de autenticación
- **THEN** el adaptador SHALL propagar el error tal cual a la sesión
- **AND** SHALL NOT inyectar, leer ni persistir credencial alguna

#### Scenario: CLI oficial ausente al lanzar
- **IF** el CLI oficial del proveedor no puede lanzarse como subproceso
- **THEN** el adaptador SHALL rehusar con diagnóstico y remedio que nombren la capa ausente
- **AND** SHALL NOT intentar ninguna vía alternativa de pilotaje

#### Scenario: CLI que el catálogo encuentra, lanzable por el adaptador
- **WHEN** el CLI oficial está instalado bajo una forma que el catálogo reconoce como lanzable
- **THEN** el adaptador SHALL resolver el nombre declarado al archivo que la plataforma ejecuta
- **AND** un CLI presente para el catálogo SHALL NOT rehusarse como ausente al lanzarlo

### Requirement: Sesión headless de eventos JSON como sesión ACP
El adaptador del dialecto de sesión headless SHALL pilotar el CLI oficial
en su modo documentado de sesión con eventos JSON delimitados por líneas y
mapear esos eventos a la sesión ACP en streaming, incluidos los deltas
parciales y los transcripts de subagentes cuando el CLI los ofrezca. El
adaptador SHALL detectar las capacidades que el CLI anuncia en su evento
inicial y MUST rehusar con diagnóstico y remedio cuando la superficie
requerida — el modo de sesión con la cuenta ya iniciada — no esté
disponible; MUST NOT cambiar en silencio a un modo que exija clave de API.
La proyección MCP vigente de la sesión SHALL entregarse al CLI por su
canal de configuración documentado, y la reanudación de sesión del CLI
SHALL mapearse a la carga de sesión ACP.

#### Scenario: Eventos de sesión mapeados en streaming
- **WHEN** el CLI pilotado emite eventos de sesión, incluidos deltas parciales
- **THEN** la sesión ACP SHALL reflejarlos como actualizaciones en streaming
- **AND** los transcripts de subagentes que el CLI emita SHALL conservarse en la sesión

#### Scenario: Rehúso ante modo que exige clave de API
- **IF** el CLI indica que operará en un modo que exige clave de API en lugar de la sesión iniciada
- **THEN** el adaptador SHALL rehusar con diagnóstico y remedio
- **AND** SHALL NOT inyectar credencial alguna ni continuar en silencio

#### Scenario: Proyección MCP entregada al CLI
- **WHEN** la sesión tiene perfiles MCP proyectados
- **THEN** el adaptador SHALL entregarlos al CLI por su canal de configuración documentado
- **AND** las herramientas MCP SHALL quedar disponibles dentro de la sesión pilotada

#### Scenario: Reanudación mapeada a la sesión ACP
- **WHEN** el daemon solicita cargar una sesión previa del adaptador
- **THEN** el adaptador SHALL reanudar la sesión correspondiente del CLI pilotado
- **AND** el ámbito de la reanudación SHALL quedar acotado al directorio del proyecto y sus worktrees

### Requirement: Passthrough de permisos con compuerta dura y pérdidas visibles
Las peticiones de permiso del CLI pilotado SHALL relevarse a
`session/request_permission` de la sesión ACP del adaptador, de modo que
las decida el proxy de permisos vigente, y el adaptador SHALL configurar
además la compuerta dura nativa del CLI para que ninguna llamada a
herramienta proceda sin haber pasado por esa vía, incluso en los modos
permisivos del propio CLI. WHERE la superficie del proveedor no puede
relevar una interacción (herramientas solo interactivas), la denegación
automática MUST mostrarse en la sesión con su motivo y el adaptador MUST
NOT aprobarla por su cuenta. El relevo MUST NOT abrir transporte nuevo
alguno en el daemon. El canal privado de una sesión SHALL ser privado del
usuario y SHALL retirarse al cerrarse la sesión en orden; WHERE el
adaptador termina sin poder ejecutar ese cierre, los canales que ninguna
sesión puede estar usando SHALL retirarse al abrirse el canal siguiente, de
modo que la configuración resuelta de una sesión no se acumule sin límite.
Ese retiro MUST NOT alcanzar el canal de una sesión que pueda seguir viva,
y su peor efecto posible SHALL ser una denegación y nunca una aprobación.

#### Scenario: Permiso decidido por el proxy vigente
- **WHEN** el CLI pilotado solicita aprobación para una herramienta
- **THEN** la petición SHALL llegar como `session/request_permission` de la sesión del adaptador
- **AND** la decisión SHALL ser la que el proxy de permisos resuelva, sin atajo alguno

#### Scenario: Compuerta dura sobre modo permisivo del CLI
- **WHEN** el CLI pilotado corre en su modo más permisivo
- **THEN** la compuerta dura configurada por el adaptador SHALL seguir denegando toda llamada no aprobada
- **AND** ninguna herramienta SHALL ejecutarse sin decisión del proxy

#### Scenario: Interacción no relevable denegada con motivo visible
- **WHERE** el proveedor auto-deniega una herramienta que exige interacción directa
- **THEN** la sesión SHALL mostrar la denegación con su motivo
- **AND** el adaptador SHALL NOT aprobarla ni ocultarla

#### Scenario: Canal de una sesión muerta retirado en la apertura siguiente
- **WHERE** un adaptador terminado sin cierre dejó su canal con la configuración resuelta de su sesión
- **THEN** la apertura del canal de una sesión nueva SHALL retirar los canales que ninguna sesión puede estar usando
- **AND** SHALL NOT retirar el canal de una sesión que pueda seguir viva

### Requirement: Servidor JSON-RPC del proveedor como sesión ACP
El adaptador del dialecto de servidor SHALL lanzar el CLI oficial en su
modo servidor JSON-RPC 2.0 documentado, con delimitación por líneas sobre
stdio, y mapear sus primitivas de conversación a la sesión ACP en
streaming. Las aprobaciones que el servidor solicite SHALL relevarse a
`session/request_permission`, y la cancelación ACP MUST propagarse al
turno en curso del servidor.

#### Scenario: Conversación del servidor mapeada en streaming
- **WHEN** el servidor pilotado emite los ítems de un turno en curso
- **THEN** la sesión ACP SHALL reflejarlos como actualizaciones en streaming
- **AND** el cierre del turno SHALL cerrar la respuesta de la sesión

#### Scenario: Aprobación del servidor decidida por el proxy
- **WHEN** el servidor pilotado solicita aprobación para una operación
- **THEN** la petición SHALL relevarse a `session/request_permission`
- **AND** la operación SHALL proceder solo con la decisión del proxy

#### Scenario: Cancelación propagada al servidor
- **WHEN** el daemon cancela la sesión ACP del adaptador
- **THEN** el adaptador SHALL interrumpir el turno en curso del servidor pilotado
- **AND** SHALL terminar el subproceso de forma limpia si el servidor no responde

### Requirement: Cancelación que llega y turno que dice la verdad
Una cancelación aceptada para un turno en vuelo SHALL seguir vigente aunque
llegue antes de que ese turno emita su primera instrucción: el adaptador
MUST NOT registrar una cancelación y descartarla después al arrancar el
turno al que iba dirigida. Todo turno que termine tras una cancelación
SHALL responderse como cancelado, incluso WHERE la cancelación misma sea lo
que impidió que el turno prosiguiera; el motivo original SHALL quedar en el
log de sesión en lugar de perderse. WHERE la superficie del proveedor solo
documenta el fin de la entrada como paro, el adaptador SHALL cerrar esa
entrada de modo que el proceso proveedor lo perciba —terminando por su
cuenta en vez de ser matado al agotarse la gracia— y SHALL rehusar con
diagnóstico y remedio todo turno posterior de esa sesión, en lugar de darlo
por enviado. WHERE el proceso proveedor sigue trabajando pese a ese fin de
entrada, el adaptador MUST terminarlo al agotarse la gracia y responder el
turno igualmente como cancelado: quien pidió parar MUST NOT quedar
esperando el turno que canceló, ni la sesión sostener su worktree abierto.

#### Scenario: Cancelación temprana no perdida entre la aceptación y el turno
- **WHEN** el daemon cancela un turno ya aceptado cuyo trabajo aún no ha comenzado
- **THEN** la cancelación SHALL seguir vigente cuando el turno arranque
- **AND** SHALL alcanzar al proveedor por el mecanismo que su superficie documenta

#### Scenario: Turno cancelado reportado como cancelado
- **IF** un turno termina en error porque la cancelación cortó su vía al proveedor
- **THEN** la respuesta al daemon SHALL ser «cancelado» y no un fallo
- **AND** el motivo original SHALL quedar registrado en el log de sesión

#### Scenario: Fin de entrada percibido por el proceso proveedor
- **WHEN** el adaptador cierra la entrada del subproceso proveedor
- **THEN** un proveedor que termina con su entrada SHALL terminar por su cuenta
- **AND** SHALL NOT depender de que se le mate al agotarse la gracia

#### Scenario: Turno cancelado terminado aunque el proveedor no lo atienda
- **WHERE** el proceso proveedor sigue trabajando pese al fin de entrada que lo cancela
- **THEN** el adaptador SHALL terminarlo al agotarse la gracia y responder el turno como cancelado
- **AND** ninguna sesión SHALL quedar activa sosteniendo el worktree de ese turno

#### Scenario: Turno posterior a una cancelación rehusado con remedio
- **WHERE** la cancelación acabó con la entrada que la superficie documenta como único paro
- **THEN** un turno posterior de esa sesión SHALL rehusarse con diagnóstico y remedio
- **AND** el remedio SHALL indicar cómo seguir sin perder lo ya conversado

### Requirement: Conformidad por versión y desfase rehusado
WHERE el CLI pilotado puede volcar el esquema de su cable por versión, los
tipos del adaptador SHALL validarse contra fixtures de ese esquema en la
integración continua. El desfase de versión entre adaptador y CLI MUST
detectarse en el handshake y rehusarse con diagnóstico y remedio, nunca
asumirse compatible; el binario efectivo y la versión del CLI pilotado
SHALL constar en el log de sesión.

#### Scenario: Tipos validados contra el esquema volcado
- **WHEN** corre la verificación de conformidad del adaptador
- **THEN** los tipos del cable SHALL validarse contra el fixture del esquema de la versión soportada
- **AND** una divergencia SHALL fallar la verificación señalando el campo

#### Scenario: Desfase de versión rehusado con remedio
- **IF** el handshake revela una versión del CLI fuera del rango soportado por el adaptador
- **THEN** el adaptador SHALL rehusar con diagnóstico que nombre ambas versiones
- **AND** el remedio SHALL indicar qué actualizar

#### Scenario: Versión efectiva registrada en el log
- **WHEN** una sesión pilotada por un adaptador propio se abre con éxito
- **THEN** el log de sesión SHALL registrar el binario efectivo del CLI pilotado y su versión

### Requirement: Cada adaptador traduce lo que su proveedor documenta

Cada adaptador propio SHALL traducir modelo y esfuerzo a lo que su proveedor
acepta oficialmente, en el punto donde ese proveedor lo acepta, y SHALL rehusar
con diagnóstico lo que no. NO SHALL inventar una traducción no verificada
contra la versión pineada del proveedor.

#### Scenario: El adaptador manda la palanca donde su proveedor la acepta

- **WHEN** una sesión declara modelo para un adaptador que lo admite
- **THEN** SHALL viajar en el punto que el protocolo de ese proveedor define

#### Scenario: Lo no verificado se rehúsa en vez de inventarse

- **WHERE** el adaptador no tiene verificado que su proveedor acepte una palanca
- **THEN** SHALL rehusarla con ese motivo
- **AND** NO SHALL enviar una traducción adivinada

### Requirement: Las preguntas del agente se relevan como peticiones con opciones

WHERE el CLI pilotado pide a una persona que elija entre opciones, el adaptador
SHALL relevar la pregunta como `session/request_permission` con **las opciones
del agente**, conservando el rótulo de cada una tal como el agente lo envió, y
SHALL devolver la elección por el canal de respuesta del proveedor. El rehúso
por interacción no relevable SHALL conservarse para toda herramienta que
efectivamente no pueda relevarse.

Una llamada aprobada SHALL seguir corriendo exactamente como se pidió. WHERE la
herramienta relevada es una pregunta —y solo ahí—, la respuesta elegida SHALL
poder completar el campo de respuesta que el propio input declara: en una
pregunta el input **es** el formulario, y completarlo no es reescribir lo que el
agente iba a hacer. Ninguna otra herramienta SHALL ver su input alterado.

WHERE el canal admite una sola respuesta por petición, el adaptador SHALL relevar
cada pregunta por separado y NO SHALL fingir selección múltiple.

Una petición que no ofrece forma de **rehusar** no es una petición de permiso: es
una pregunta, y sus opciones son respuestas. Las reglas de permisos NO SHALL
decidirla —no tienen opinión sobre cuál respuesta es correcta— y SHALL escalar a
una persona.

#### Scenario: Una pregunta llega con las opciones del agente

- **WHEN** el CLI pilotado pregunta con opciones
- **THEN** la petición SHALL llegar como `session/request_permission` con esas
  opciones y sus rótulos
- **AND** la elección SHALL volver al CLI por su canal de respuesta

#### Scenario: Solo una pregunta completa su propio input

- **WHEN** se aprueba una herramienta que no es una pregunta
- **THEN** su input SHALL viajar sin alteración alguna

#### Scenario: Una regla no contesta una pregunta por ti

- **WHERE** una regla de permisos resolvería toda petición
- **WHEN** llega una pregunta, que no ofrece forma de rehusar
- **THEN** SHALL escalar a una persona igualmente
- **AND** la regla NO SHALL elegir una respuesta

#### Scenario: Lo que de verdad no se puede relevar se sigue rehusando

- **WHERE** una herramienta exige una interacción que esta superficie no puede
  ofrecer
- **THEN** SHALL seguir denegándose con su motivo visible
- **AND** el adaptador SHALL NOT aprobarla por su cuenta

### Requirement: Terminar al proveedor es terminar todo lo que lanzó

Cuando el adaptador termina al proceso proveedor —al agotarse la gracia
tras cerrar su entrada, o al cerrarse la sesión— SHALL terminar también a
todo proceso que ese proveedor haya necesitado para existir. WHERE la
plataforma resuelva el CLI oficial a un intermediario que crea el proceso
real, terminar al proveedor SHALL terminar al intermediario y a cuanto creó,
y el adaptador NO SHALL responder el turno como terminado mientras alguno
siga vivo. WHERE la plataforma permita atar la vida de esos procesos a la del
adaptador, el adaptador SHALL atarla al lanzar, de modo que un fin abrupto
del adaptador tampoco deje huérfanos. El adaptador NO SHALL ganar
dependencia externa alguna para ello.

#### Scenario: El intermediario del proveedor no deja huérfanos

- **WHERE** el CLI oficial se resuelve a un intermediario que crea el proceso real
- **WHEN** el proveedor ignora el fin de su entrada y la gracia se agota
- **THEN** el adaptador SHALL terminar al intermediario y a todo proceso que creó
- **AND** SHALL responder el turno como cancelado solo entonces

#### Scenario: El adaptador cae y el proveedor no le sobrevive

- **WHERE** la plataforma permite atar la vida del proveedor a la del adaptador
- **WHEN** el proceso del adaptador termina sin ejecutar su apagado
- **THEN** el proveedor y cuanto lanzó SHALL terminar igualmente
