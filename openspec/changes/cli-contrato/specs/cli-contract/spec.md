## ADDED Requirements

### Requirement: Gramática de subcomandos y reserva
El binario `meltemi` SHALL exponer una gramática de subcomandos estable: los
subcomandos operativos (`status`, `propose`, `stop`, `version`, `help`) y los
subcomandos del ciclo SDD reservados (`explore`, `review`, `plan`, `implement`,
`verify`, `archive`). Un subcomando reservado MUST reconocerse como parte de la
gramática y no como error de uso.

#### Scenario: Subcomando operativo reconocido
- **WHEN** se invoca `meltemi status`
- **THEN** el binario SHALL despachar el subcomando `status` en modo scriptable

#### Scenario: Subcomando reservado no es error de uso
- **WHEN** se invoca un subcomando reservado aún no implementado (p. ej. `meltemi review`)
- **THEN** el binario SHALL informar por stderr que el subcomando está reservado y aún no implementado
- **AND** SHALL terminar con un código distinto del de subcomando desconocido

#### Scenario: Subcomando desconocido
- **WHEN** se invoca `meltemi` con un token que no pertenece a la gramática
- **THEN** el binario SHALL emitir un error de uso por stderr y terminar con el código de error de uso

### Requirement: Regla de despacho CLI↔TUI
El binario SHALL decidir su modo de forma determinista a partir de la presencia de
subcomando y de si stdout está conectado a un TTY. Una invocación con subcomando
MUST entrar en modo scriptable de un disparo con independencia del TTY.

#### Scenario: Con subcomando siempre scriptable
- **WHEN** se invoca `meltemi <subcomando>` con stdout redirigido a un archivo o *pipe*
- **THEN** el binario SHALL ejecutar el subcomando en modo scriptable de un disparo
- **AND** SHALL NOT entrar en modo interactivo

#### Scenario: Invocación desnuda con TTY entra al modo interactivo
- **WHEN** se invoca `meltemi` sin subcomando y stdout está conectado a un TTY
- **THEN** el binario SHALL entrar en el modo interactivo
- **AND** en esta entrega SHALL emitir por stderr un aviso de que la interfaz interactiva llega en una entrega posterior y terminar con éxito

#### Scenario: Invocación desnuda sin TTY es error de uso
- **WHEN** se invoca `meltemi` sin subcomando y stdout no está conectado a un TTY
- **THEN** el binario SHALL emitir un error de uso que remite a `meltemi help`
- **AND** SHALL NOT quedar a la espera de entrada interactiva

### Requirement: Taxonomía de códigos de salida
El binario SHALL usar una taxonomía de códigos de salida estable: `0` éxito, `1`
error interno inesperado, `2` error de uso, `10` daemon inalcanzable, `11` error
de contrato, `12` operación rechazada por política, `13` operación cancelada.
Todo subcomando MUST terminar con el código que corresponde a su desenlace.

#### Scenario: Éxito termina en cero
- **WHEN** un subcomando operativo completa su cometido sin error
- **THEN** el binario SHALL terminar con el código `0`

#### Scenario: Daemon inalcanzable
- **WHEN** un subcomando respaldado por RPC no logra conectar ni arrancar el daemon
- **THEN** el binario SHALL terminar con el código `10`

#### Scenario: Respuesta de error del contrato
- **WHERE** el daemon responde con un error JSON-RPC o rechaza la versión de protocolo
- **THEN** el binario SHALL terminar con el código `11`

### Requirement: Disciplina de flujos stdout/stderr
El binario SHALL reservar stdout exclusivamente para la salida útil del comando y
SHALL dirigir todo progreso, aviso y diagnóstico a stderr. En modo humano, los
mensajes de error MUST emitirse por stderr, nunca por stdout.

#### Scenario: La salida útil va a stdout y el progreso a stderr
- **WHEN** un subcomando produce salida útil y además emite progreso
- **THEN** el binario SHALL escribir la salida útil solo en stdout
- **AND** SHALL escribir el progreso y los diagnósticos solo en stderr

#### Scenario: Error en modo humano va a stderr
- **IF** un subcomando falla en modo humano (sin `--json`)
- **THEN** el binario SHALL escribir el mensaje de error en stderr
- **AND** stdout SHALL permanecer sin contenido de error

### Requirement: Salida legible por máquina con --json
El binario SHALL aceptar el flag global `--json`. Bajo `--json`, cada subcomando
scriptable MUST emitir exactamente un objeto JSON en stdout —tanto en éxito como
en error— y MUST NOT mezclar texto humano en stdout.

#### Scenario: Éxito en JSON emite un objeto
- **WHEN** se invoca un subcomando scriptable con `--json` y completa con éxito
- **THEN** el binario SHALL emitir exactamente un objeto JSON en stdout con el resultado

#### Scenario: Error en JSON emite un objeto de error
- **WHEN** se invoca un subcomando scriptable con `--json` y la operación falla
- **THEN** el binario SHALL emitir en stdout exactamente un objeto JSON de error que incluye el código de la taxonomía de salida
- **AND** stderr SHALL permanecer libre de JSON

### Requirement: Mapeo comando↔método RPC
Cada subcomando respaldado por RPC SHALL enviar `initialize` como primer mensaje y
luego el método correspondiente: `status`→`status`, `propose`→`propose`,
`stop`→`shutdown`. Los subcomandos locales (`version`, `help`) MUST NOT abrir
conexión con el daemon.

#### Scenario: initialize precede a todo método RPC
- **WHEN** un subcomando respaldado por RPC abre una conexión con el daemon
- **THEN** el binario SHALL enviar `initialize` antes de cualquier otro método

#### Scenario: status consulta el estado del daemon
- **WHEN** se invoca `meltemi status` con el daemon accesible
- **THEN** el binario SHALL invocar el método `status` y presentar la versión, el tiempo activo y las sesiones del daemon

#### Scenario: Los subcomandos locales no tocan el daemon
- **WHEN** se invoca `meltemi version` o `meltemi help`
- **THEN** el binario SHALL responder localmente
- **AND** SHALL NOT abrir conexión con el daemon

### Requirement: Arranque del daemon bajo demanda
Un subcomando respaldado por RPC SHALL reutilizar el arranque bajo demanda del
daemon: si no hay daemon en ejecución, el binario MUST intentar arrancarlo y
conectarse antes de fallar por inalcanzable.

#### Scenario: Arranque bajo demanda al no haber daemon
- **WHEN** se invoca un subcomando respaldado por RPC y no hay daemon en ejecución
- **THEN** el binario SHALL intentar arrancar el daemon y conectarse
- **AND** SHALL proceder con el método solicitado si la conexión se establece

#### Scenario: Fallo de arranque se reporta como inalcanzable
- **IF** el daemon no puede arrancarse ni alcanzarse dentro del presupuesto de conexión
- **THEN** el binario SHALL terminar con el código de daemon inalcanzable
