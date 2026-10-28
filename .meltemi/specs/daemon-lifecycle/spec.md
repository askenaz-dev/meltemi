# daemon-lifecycle Specification

## Purpose
TBD - created by archiving change fase-0-fundacion. Update Purpose after archive.
## Requirements

### Requirement: Arranque bajo demanda e instancia única
El daemon `meltemid` SHALL arrancar bajo demanda cuando un cliente lo invoque
sobre el endpoint propio del usuario en esta máquina y no exista una instancia
en ejecución, y SHALL garantizar que nunca corren dos instancias para el mismo
usuario. Un cliente MUST NOT arrancar un daemon en un endpoint explícito que no
es el propio —uno reenviado por un túnel, uno de otra herramienta— salvo que
se le pida expresamente arrancar ahí: si ese endpoint no responde, SHALL
rehusar de inmediato con diagnóstico y remedio, sin esperar un arranque que no
va a intentar.

#### Scenario: Primer cliente arranca el daemon
- **WHEN** un cliente intenta conectarse a su endpoint propio y no hay ninguna instancia de `meltemid` en ejecución
- **THEN** el cliente arranca `meltemid` de forma desacoplada y completa su conexión contra la nueva instancia

#### Scenario: Segunda invocación reutiliza la instancia
- **WHEN** un cliente intenta conectarse y ya existe una instancia de `meltemid` en ejecución
- **THEN** la conexión se establece contra la instancia existente y no se crea un segundo proceso

#### Scenario: Un endpoint ajeno que no responde no arranca nada
- **WHEN** un cliente intenta conectarse a un endpoint explícito que no es el propio y nada responde
- **THEN** el cliente SHALL rehusar de inmediato con diagnóstico y remedio
- **AND** NO SHALL arrancar un daemon, aunque tenga el binario a mano

#### Scenario: Arrancar en un endpoint explícito se pide
- **WHEN** se pide expresamente arrancar en un endpoint explícito y nada responde en él
- **THEN** el cliente SHALL arrancar `meltemid` en ese endpoint y completar su conexión

### Requirement: Socket local con permisos exclusivos del usuario
`meltemid` SHALL escuchar únicamente en un socket local (Unix domain socket en macOS/Linux con permisos `0700`; named pipe en Windows con ACL restringida al usuario actual) y SHALL NOT abrir ningún puerto de red.

#### Scenario: Sin superficie de red
- **WHEN** `meltemid` está en ejecución
- **THEN** no existe ningún socket TCP/UDP en escucha perteneciente al proceso

#### Scenario: Otro usuario del sistema no puede conectarse
- **WHEN** un proceso de un usuario del sistema distinto intenta conectarse al socket local de `meltemid`
- **THEN** la conexión es rechazada por los permisos del sistema operativo

### Requirement: Transporte JSON-RPC 2.0 conforme al contrato
El transporte daemon↔cliente SHALL ser JSON-RPC 2.0 con delimitación por líneas, conforme a los esquemas versionados de `proto/`, y SHALL responder a mensajes malformados con los errores estándar del protocolo sin terminar el proceso.

#### Scenario: Mensaje malformado
- **WHEN** un cliente envía una línea que no es JSON-RPC 2.0 válido
- **THEN** `meltemid` responde con el error estándar correspondiente (parse error / invalid request) y la conexión y el daemon siguen operativos

### Requirement: Versión de protocolo negociada
El cliente SHALL declarar la versión del contrato `proto/` al conectar, y `meltemid` SHALL aceptar la conexión solo si soporta esa versión, respondiendo en caso contrario con un error que incluya la versión declarada y las soportadas.

#### Scenario: Versión soportada
- **WHEN** un cliente declara una versión de protocolo soportada por el daemon
- **THEN** la conexión queda establecida y ambas partes operan bajo esa versión

#### Scenario: Versión no soportada
- **WHEN** un cliente declara una versión de protocolo que el daemon no soporta
- **THEN** el cliente recibe un error con ambas versiones y la conexión se cierra ordenadamente

### Requirement: Estado consultable
`meltemid` SHALL exponer un método `status` que devuelva versión, tiempo de actividad y sesiones activas.

#### Scenario: Consulta de estado
- **WHEN** un cliente invoca el método `status`
- **THEN** recibe versión del daemon, uptime y la lista de sesiones activas con su identificador y su agente

### Requirement: Apagado limpio
`meltemid` SHALL exponer un método `shutdown` que termine ordenadamente todas las sesiones de agente (subprocesos incluidos), cierre los registros de sesión y finalice el proceso.

#### Scenario: Apagado con sesiones activas
- **WHEN** un cliente invoca `shutdown` mientras existe una sesión de agente activa
- **THEN** el subproceso del agente recibe terminación ordenada, el registro JSONL de la sesión queda cerrado y completo, y el proceso `meltemid` finaliza sin dejar procesos huérfanos

### Requirement: Un endpoint con forma de comando nunca arranca un daemon

WHERE el endpoint del cliente nombre un comando en lugar de un socket, el
cliente NO SHALL arrancar un daemon local bajo ninguna circunstancia, ni
siquiera cuando ese comando falle o no responda. Arrancar uno serviría una
máquina en el nombre de otra.

#### Scenario: El comando falla y no se arranca nada

- **WHEN** el endpoint nombra un comando y ese comando termina sin hablar el protocolo
- **THEN** el cliente rehúsa con su diagnóstico
- **AND** no queda ningún daemon arrancado por ese intento
