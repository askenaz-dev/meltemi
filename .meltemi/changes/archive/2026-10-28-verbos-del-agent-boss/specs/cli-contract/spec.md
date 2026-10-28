# cli-contract — delta

## ADDED Requirements

### Requirement: Verbos para decidir las peticiones de permiso
La gramática SHALL ofrecer `permissions`, `approve <petición> [--option <id>]`
y `deny <petición> [--option <id>]`, mapeados a `permission/pending` y
`permission/decide`. `permissions` SHALL abrir con el total de peticiones que
esperan una decisión y SHALL mostrar de cada una su id, su sesión, su
herramienta, su título, cuánto lleva esperando, su vencimiento y las opciones
que el agente ofreció. `approve` y `deny` SHALL leer la petición en la cola
antes de decidir y SHALL decidir únicamente con una opción que el agente ofreció
para esa petición: por omisión, `approve` SHALL elegir la aprobación de una sola
vez cuando el agente ofreció exactamente una, y `deny` SHALL elegir el rechazo
del agente o, si no ofreció ninguno, cancelar; `--option` SHALL nombrar una
opción del agente de la clase del verbo. Una aprobación permanente MUST NOT
elegirse por omisión. Una petición que ya no espera MUST NOT decidirse, y el
binario SHALL terminar con el código de operación rechazada por política
(`12`) diciendo por qué.

#### Scenario: permissions muestra lo que espera y desde cuándo
- **WHEN** una petición espera una decisión y se invoca `permissions`
- **THEN** la salida SHALL abrir con el total de peticiones que esperan
- **AND** SHALL mostrar su id, su sesión, su herramienta, su título, cuánto lleva esperando y las opciones del agente

#### Scenario: approve elige la aprobación simple del agente
- **WHEN** se invoca `approve <petición>` y el agente ofreció una sola aprobación de una vez
- **THEN** el daemon SHALL recibir esa opción como decisión del cliente
- **AND** la salida SHALL nombrar la opción elegida

#### Scenario: deny elige el rechazo del agente o cancela
- **WHEN** se invoca `deny <petición>`
- **THEN** la decisión SHALL ser la opción de rechazo que el agente ofreció
- **AND** SHALL ser la cancelación cuando el agente no ofreció ninguna

#### Scenario: approve rehúsa una opción que el agente no ofreció
- **WHEN** se invoca `approve <petición> --option <id>` con una opción que el agente no ofreció para esa petición, o que no es una aprobación
- **THEN** el binario SHALL rehusar con un error de uso que lista las opciones del agente
- **AND** NO SHALL decidirse nada

#### Scenario: Varias aprobaciones exigen nombrar una
- **WHEN** el agente ofreció más de una aprobación y se invoca `approve <petición>` sin `--option`
- **THEN** el binario SHALL rehusar listando las opciones del agente
- **AND** NO SHALL elegir una por su cuenta

#### Scenario: Una petición que ya no espera no se decide
- **WHEN** se invoca `approve` o `deny` sobre una petición resuelta en otra superficie, vencida o que la cola no tiene
- **THEN** NO SHALL decidirse nada
- **AND** el binario SHALL terminar con el código `12` diciendo que la petición ya no espera

### Requirement: Una decisión por petición, nunca en bloque
`approve` y `deny` SHALL aceptar exactamente un id de petición. La gramática
MUST NOT ofrecer decisión en bloque de ninguna forma —varios ids, un flag para
todas o un comodín—: quien quiera conceder sin que le pregunten lo declara en
las reglas de permisos o en el modo de la sesión, y el mensaje de rehúso SHALL
decirlo.

#### Scenario: Dos ids se rehúsan sin decidir ninguno
- **WHEN** se invoca `approve` o `deny` con más de un id
- **THEN** el binario SHALL rehusar con un error de uso que remite a las reglas de permisos
- **AND** NO SHALL decidirse ninguna petición

#### Scenario: Un comodín no es un id
- **WHEN** se invoca `approve all` o `approve '*'`
- **THEN** NO SHALL decidirse ninguna petición
- **AND** el binario SHALL terminar con el código `12`

### Requirement: Verbos para seguir, leer y cancelar una sesión
La gramática SHALL ofrecer `watch <sesión>`, `log <sesión>` y `cancel
<sesión>`, mapeados a `session/watch`, `session/log` y `session/cancel`; ninguno
SHALL exigir la raíz del proyecto, porque la sesión se localiza por su id en el
listado del daemon. `watch` SHALL declarar su interés antes de comprobar si la
sesión corre, SHALL emitir cada registro de la sesión a medida que ocurre y
SHALL terminar: tras el registro de cierre de la sesión; cuando la sesión deja
de estar viva aunque su cierre no haya llegado por el flujo; o, con el código
`10`, cuando el daemon cierra la conexión. Dejar de seguir MUST NOT cancelar la
sesión. `log` SHALL emitir el registro apend-only completo, tal cual. `cancel`
SHALL pedir la cancelación de una sesión viva con el mismo efecto que el gesto
de cancelar de la TUI y SHALL decir si la sesión seguía viva al responder. Una
sesión que el daemon no conoce SHALL terminar con el código de error de
contrato, como `direct`; una sesión que ya no corre MUST NOT tratarse como error
por `watch` ni por `cancel`.

#### Scenario: watch termina con la sesión
- **WHEN** se sigue una sesión viva con `watch` y la sesión termina
- **THEN** la última línea SHALL ser su registro de cierre
- **AND** el binario SHALL terminar con el código `0`

#### Scenario: watch no sobrevive a su daemon
- **WHEN** el daemon cierra la conexión mientras `watch` sigue una sesión
- **THEN** el binario SHALL terminar de inmediato con el código `10`
- **AND** NO SHALL quedarse esperando

#### Scenario: watch no sobrevive a la sesión aunque el cierre no llegue
- **WHEN** la sesión seguida deja de estar viva sin que su registro de cierre haya llegado por el flujo
- **THEN** `watch` SHALL terminar con el código `0`
- **AND** SHALL remitir a `log` para leer el cierre

#### Scenario: Dejar de seguir no cancela la sesión
- **WHEN** el proceso de `watch` termina mientras la sesión corre
- **THEN** la sesión SHALL seguir viva

#### Scenario: Lo que watch emite es lo que log guarda
- **WHEN** una sesión seguida con `watch` termina y se lee con `log`
- **THEN** cada registro que `watch` emitió SHALL estar en `log` con el mismo contenido
- **AND** SHALL estar en el mismo orden, al final del registro

#### Scenario: Seguir o cancelar una sesión que ya terminó no es un error
- **WHEN** se invoca `watch` o `cancel` sobre una sesión que ya no corre
- **THEN** el binario SHALL terminar con el código `0` diciendo que no corre
- **AND** NO SHALL quedarse esperando

#### Scenario: Una sesión desconocida es error de contrato
- **WHEN** se invoca `watch`, `log` o `cancel` con un id que el daemon no conoce
- **THEN** el binario SHALL terminar con el código `11`

#### Scenario: cancel detiene la sesión como la TUI
- **WHEN** se invoca `cancel` sobre una sesión viva
- **THEN** el daemon SHALL recibir la cancelación de esa sesión
- **AND** la sesión SHALL terminar con su registro de cierre

### Requirement: Lo que el agente escribió se imprime sin mandar en el terminal
El texto que viene del agente —títulos, herramientas, nombres de opción,
contenido de un registro— SHALL imprimirse de modo que un terminal no pueda
obedecerlo. En el listado de `permissions`, los caracteres de control, de
dirección y de formato invisible SHALL mostrarse escapados. En las líneas de
`watch` y `log` SHALL escribirse como escapes de JSON, de modo que cada línea
siga siendo JSON válido y, leída como JSON, conserve el mismo valor.

#### Scenario: El listado escapa el texto del agente
- **WHEN** el título de una petición contiene un carácter de control o de dirección
- **THEN** `permissions` SHALL mostrarlo escapado

#### Scenario: Una línea del flujo lleva el texto del agente como escape JSON
- **WHEN** un registro contiene un carácter de control o de dirección enviado por el agente
- **THEN** la línea emitida SHALL llevarlo como escape de JSON
- **AND** leída como JSON SHALL conservar el mismo valor

## MODIFIED Requirements

### Requirement: Salida legible por máquina con --json
El binario SHALL aceptar el flag global `--json`. Bajo `--json`, cada subcomando
scriptable MUST emitir exactamente un objeto JSON en stdout —tanto en éxito como
en error— y MUST NOT mezclar texto humano en stdout. Los subcomandos cuya salida
útil es el registro de una sesión (`watch`, `log`) SHALL ser la excepción
declarada: emiten un objeto JSON por línea, uno por registro (JSON Lines), y un
error durante el flujo SHALL cerrarlo con un objeto de error que incluye el
código de la taxonomía de salida.

#### Scenario: Éxito en JSON emite un objeto
- **WHEN** se invoca un subcomando scriptable con `--json` y completa con éxito
- **THEN** el binario SHALL emitir exactamente un objeto JSON en stdout con el resultado

#### Scenario: Error en JSON emite un objeto de error
- **WHEN** se invoca un subcomando scriptable con `--json` y la operación falla
- **THEN** el binario SHALL emitir en stdout exactamente un objeto JSON de error que incluye el código de la taxonomía de salida
- **AND** stderr SHALL permanecer libre de JSON

#### Scenario: Un flujo emite un objeto por línea
- **WHEN** se invoca `watch` o `log` con `--json`
- **THEN** cada línea de stdout SHALL ser exactamente un objeto JSON, un registro de la sesión
- **AND** stdout NO SHALL contener texto humano

#### Scenario: Un error a mitad del flujo cierra con un objeto
- **WHEN** el daemon cierra la conexión mientras `watch --json` emite registros
- **THEN** la última línea de stdout SHALL ser un objeto de error con el código de salida

### Requirement: El formato de salida es una elección declarada

El binario SHALL tratar el formato de salida como una elección única entre
humano, JSON y YAML, y NO SHALL admitir dos formatos legibles por máquina a la
vez. El flag global `--yaml` SHALL emitir exactamente un documento YAML en
stdout —tanto en éxito como en error— con el mismo contenido que `--json`
emitiría, y MUST NOT mezclar texto humano en stdout. Los subcomandos cuya salida
útil es el registro de una sesión (`watch`, `log`) SHALL emitir bajo `--yaml` un
documento por registro, como un flujo YAML. Bajo cualquier formato legible por
máquina la salida NO SHALL llevar color.

#### Scenario: YAML emite un documento y nada más

- **WHEN** se invoca un subcomando scriptable con `--yaml`
- **THEN** stdout SHALL contener exactamente un documento YAML
- **AND** NO SHALL contener texto humano ni secuencias de color

#### Scenario: El error en YAML también es un documento

- **WHEN** un subcomando invocado con `--yaml` falla
- **THEN** stdout SHALL contener exactamente un documento YAML de error con el
  código de la taxonomía de salida
- **AND** stderr SHALL permanecer libre de ese documento

#### Scenario: Dos formatos de máquina a la vez se rehúsan

- **WHEN** se invocan `--json` y `--yaml` en la misma orden
- **THEN** el binario SHALL rehusar con un error de uso
- **AND** NO SHALL elegir uno por su cuenta

#### Scenario: Un flujo en YAML es un documento por registro

- **WHEN** se invoca `log` con `--yaml`
- **THEN** stdout SHALL contener un documento YAML por registro de la sesión
- **AND** NO SHALL contener texto humano
