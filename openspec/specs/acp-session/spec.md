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

