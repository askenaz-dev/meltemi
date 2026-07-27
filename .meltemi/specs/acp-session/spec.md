# acp-session Specification

## Purpose
TBD - created by archiving change fase-0-fundacion. Update Purpose after archive.
## Requirements

### Requirement: Lanzamiento del binario oficial configurado
`meltemid` SHALL lanzar como subproceso el binario de agente definido en la configuración del usuario (`agent.command`), sin empaquetar, modificar ni sustituir ningún agente, y sin leer ni almacenar credenciales del agente.

#### Scenario: Binario configurado ausente
- **WHEN** se solicita una sesión y el comando configurado no existe en el sistema
- **THEN** el cliente recibe un error claro que identifica el comando faltante y cómo configurarlo, y no se crea sesión

### Requirement: Handshake e inicio de sesión ACP
`meltemid` SHALL completar el handshake ACP (`initialize` con negociación de versión y capacidades, seguido de la creación de sesión) antes de aceptar prompts, y SHALL reportar al cliente un error informativo si la negociación falla.

#### Scenario: Handshake exitoso
- **WHEN** el subproceso del agente responde al `initialize` con una versión de protocolo compatible
- **THEN** `meltemid` crea una sesión ACP y la reporta al cliente como lista para recibir prompts

#### Scenario: Versión de protocolo incompatible
- **WHEN** el agente responde con una versión de protocolo no soportada
- **THEN** el subproceso se termina ordenadamente y el cliente recibe un error que incluye ambas versiones

### Requirement: Prompt con streaming de actualizaciones
`meltemid` SHALL enviar prompts a la sesión ACP y reenviar al cliente, en orden y en tiempo real, todas las actualizaciones de sesión emitidas por el agente hasta la finalización del turno.

#### Scenario: Turno con actualizaciones
- **WHEN** el cliente envía un prompt y el agente emite actualizaciones de progreso antes de completar el turno
- **THEN** el cliente recibe cada actualización en el orden emitido, seguida de la señal de fin de turno

### Requirement: Passthrough de peticiones de permiso
`meltemid` SHALL reenviar al cliente conectado toda petición de permiso emitida por el agente y devolver al agente la decisión del cliente; si no hay cliente conectado a la sesión, `meltemid` SHALL responder con denegación.

#### Scenario: Cliente aprueba
- **WHEN** el agente emite una petición de permiso y el cliente responde aprobando
- **THEN** el agente recibe la aprobación y continúa la operación

#### Scenario: Sin cliente conectado
- **WHEN** el agente emite una petición de permiso y ningún cliente está conectado a la sesión
- **THEN** el agente recibe una denegación

### Requirement: Contenido mínimo de la petición de permiso
Toda petición de permiso reenviada al cliente SHALL incluir la información disponible para decidir con fundamento: la herramienta u operación, el comando o ruta afectada, y la clasificación de efecto externo cuando el agente la provea.

#### Scenario: Petición con contexto completo
- **WHEN** el agente emite una petición de permiso para ejecutar un comando
- **THEN** el cliente recibe la operación y el comando exacto a autorizar antes de decidir

### Requirement: Cliente que no responde
Si el cliente conectado no responde a una petición de permiso dentro del plazo configurado, `meltemid` SHALL responder al agente con denegación y notificar al cliente el vencimiento.

#### Scenario: Timeout de aprobación
- **WHEN** una petición de permiso no recibe respuesta del cliente dentro del plazo configurado
- **THEN** el agente recibe una denegación y el cliente recibe una notificación del vencimiento

### Requirement: Registro persistente de sesión
`meltemid` SHALL registrar cada evento de la sesión (mensajes, actualizaciones, permisos y decisiones, terminación) en un log JSONL apend-only en el directorio de datos del usuario, inspeccionable tras finalizar la sesión.

#### Scenario: Auditoría posterior
- **WHEN** una sesión ha finalizado
- **THEN** su log JSONL contiene la secuencia completa de eventos en orden, incluida cada petición de permiso con su decisión

### Requirement: Terminación sin huérfanos
`meltemid` SHALL garantizar que el subproceso del agente termina cuando la sesión se cancela, finaliza o el daemon se apaga.

#### Scenario: Cancelación de sesión
- **WHEN** el cliente cancela una sesión activa
- **THEN** el subproceso del agente recibe la cancelación ACP y, de no terminar en un plazo razonable, es terminado por el daemon; no queda ningún proceso huérfano

### Requirement: Dirección de una sesión existente
El daemon SHALL aceptar instrucciones dirigidas a una sesión existente
(`session/direct`): sobre una sesión activa la instrucción SHALL encolarse y
despacharse como el siguiente turno de la misma sesión del agente al concluir el
turno en curso, sin interrumpirlo; sobre una sesión terminada y reanudable SHALL
reanudarse con la instrucción como prompt; sobre una sesión inexistente o no
reanudable MUST rehusarse con diagnóstico y remedio. Cada instrucción MUST
registrarse en el log de sesión al encolarse y al despacharse, y el verbo SHALL
ser consumible desde todas las superficies por igual.

#### Scenario: Instrucción a una sesión activa se despacha como siguiente turno
- **WHEN** un cliente dirige una instrucción a una sesión activa
- **THEN** la instrucción SHALL encolarse sin interrumpir el turno en curso
- **AND** al concluir ese turno SHALL despacharse como el siguiente prompt de la misma sesión del agente
- **AND** el encolado y el despacho SHALL constar en el JSONL

#### Scenario: Instrucción a una sesión reanudable la reanuda
- **WHEN** un cliente dirige una instrucción a una sesión terminada cuyo agente anunció capacidad de reanudación
- **THEN** el daemon SHALL reanudar esa sesión con la instrucción como prompt
- **AND** la sesión nueva SHALL quedar enlazada a la original como reanudación

#### Scenario: Sesión no dirigible rehúsa con remedio
- **IF** la sesión no existe o no es reanudable
- **THEN** la dirección SHALL rehusarse con diagnóstico
- **AND** el remedio SHALL orientar a listar las sesiones disponibles

#### Scenario: Dirigir no interrumpe ni cancela
- **WHILE** una sesión ejecuta su turno
- **WHEN** llegan instrucciones dirigidas
- **THEN** el turno en curso SHALL continuar intacto
- **AND** la cancelación SHALL seguir siendo un verbo distinto y explícito

### Requirement: Sesión bloqueada por una decisión humana
Mientras una petición de permiso de una sesión espera la decisión humana, el
daemon SHALL declarar esa sesión en estado `waiting_permission` en `status` y
en el listado de sesiones, y SHALL restituirla a `active` cuando la petición
se resuelva por cualquier vía (decisión, vencimiento de la cota o denegación
sin clientes). Con varias peticiones simultáneas de la misma sesión, la
restitución SHALL ocurrir solo cuando ninguna quede pendiente.

#### Scenario: Sesión esperando se declara esperando
- **WHEN** una petición de permiso de una sesión activa escala al humano
- **THEN** el listado de sesiones SHALL mostrar esa sesión en `waiting_permission`

#### Scenario: Resuelta la petición, la sesión vuelve a activa
- **WHEN** la petición pendiente se resuelve
- **THEN** el listado SHALL volver a mostrar la sesión en `active`

#### Scenario: Varias esperas simultáneas
- **WHEN** una sesión tiene más de una petición esperando y se resuelve una
- **THEN** la sesión SHALL seguir declarándose `waiting_permission` mientras quede alguna pendiente
