# own-adapters — delta

## ADDED Requirements

### Requirement: Puente ACP gobernado sobre el binario oficial
Cada adaptador propio SHALL ser un binario que habla ACP por stdio hacia el
daemon y pilota exclusivamente el CLI oficial del proveedor como
subproceso, con la autenticación que ese CLI gestiona por su cuenta. El
adaptador MUST NOT embeber el runtime del proveedor como librería, MUST NOT
enlazar pila HTTP ni TLS alguna, MUST NOT leer, almacenar ni reenviar
material de autenticación, y MUST NOT escuchar en puerto alguno. WHEN el
CLI oficial no se encuentra o no puede lanzarse como subproceso, el
adaptador MUST rehusar con diagnóstico que nombre la capa ausente y su
remedio, y MUST NOT degradar a ninguna vía alternativa.

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
alguno en el daemon.

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
por enviado.

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
