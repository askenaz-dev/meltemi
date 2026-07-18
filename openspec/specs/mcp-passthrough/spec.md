# mcp-passthrough Specification

## Purpose
TBD - created by archiving change mcp-passthrough. Update Purpose after archive.
## Requirements
### Requirement: Declaración única de servidores MCP
La configuración SHALL permitir declarar servidores MCP por proyecto y por
usuario (nombre y transporte stdio o HTTP), y todo valor sensible MUST declararse
por referencia a variable de entorno del usuario, nunca como literal exigido. La
configuración de proyecto SHALL complementar a la global; ante nombre repetido,
el proyecto MUST prevalecer.

#### Scenario: Servidor declarado una vez
- **WHEN** el proyecto declara un servidor MCP stdio con su comando
- **THEN** el daemon SHALL incorporarlo al conjunto a inyectar en las sesiones del proyecto

#### Scenario: Nombre repetido resuelto por ámbito
- **WHERE** el usuario y el proyecto declaran un servidor con el mismo nombre
- **THEN** la declaración del proyecto SHALL prevalecer

### Requirement: Inyección negociada por capacidad
El daemon SHALL pasar los servidores declarados al agente en la creación de la
sesión únicamente cuando el agente anuncie soporte MCP en el handshake; sin
soporte anunciado, la sesión SHALL abrirse igualmente y la ausencia de entrega
MUST declararse de forma visible, nunca silenciosa.

#### Scenario: Agente con soporte recibe los servidores
- **WHEN** se abre una sesión con un agente que anuncia soporte MCP
- **THEN** la creación de sesión SHALL incluir los servidores declarados

#### Scenario: Degradación honesta sin soporte
- **WHEN** el agente no anuncia soporte MCP
- **THEN** la sesión SHALL abrirse sin servidores
- **AND** la superficie SHALL declarar que no fueron entregados y por qué

### Requirement: Higiene de secretos
El daemon SHALL analizar la declaración de servidores y marcar como diagnóstico
con remedio todo valor con apariencia de secreto en claro; el registro de sesión
y las superficies MUST NOT exponer valores resueltos de variables de entorno ni
credenciales incrustadas.

#### Scenario: Secreto en claro detectado
- **WHEN** la config declara un valor con apariencia de token en claro
- **THEN** el daemon SHALL emitir el diagnóstico con el remedio (referenciar una variable de entorno)
- **AND** SHALL NOT copiar el valor a ningún registro

### Requirement: Visibilidad y registro de la inyección
El registro JSONL de la sesión SHALL incluir el evento de inyección con los
nombres de los servidores entregados, y el detalle de Sesión SHALL mostrar qué
recibió el agente. El catálogo SHALL exponer el soporte MCP como atributo del
agente.

#### Scenario: Inyección auditada por nombre
- **WHEN** una sesión arranca con servidores inyectados
- **THEN** el log SHALL registrar sus nombres
- **AND** SHALL NOT registrar credenciales ni valores de entorno

