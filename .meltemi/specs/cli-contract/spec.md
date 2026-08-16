# cli-contract Specification

## Purpose
TBD - created by archiving change cli-contrato. Update Purpose after archive.
## Requirements

### Requirement: Gramática de subcomandos y reserva
El binario `meltemi` SHALL exponer una gramática de subcomandos estable en la que
todo subcomando reconocido es operativo y se despacha en modo scriptable; un
token fuera de la gramática MUST tratarse como error de uso. La **enumeración
autoritativa** de los subcomandos, sus argumentos y la taxonomía de códigos de
salida es la **referencia CLI generada** desde la fuente única (`cli::reference`,
publicada en `docs/referencia-cli.md`); una referencia desactualizada MUST
detectarse en el pipeline. El ciclo SDD completo es operativo: ningún subcomando
del ciclo permanece reservado.

#### Scenario: Subcomando operativo reconocido
- **WHEN** se invoca un subcomando de la gramática (p. ej. `meltemi changes`)
- **THEN** el binario SHALL despacharlo en modo scriptable

#### Scenario: El ciclo completo es operativo
- **WHEN** se invoca `meltemi implement` con el daemon accesible
- **THEN** el binario SHALL despacharlo como subcomando operativo
- **AND** ningún subcomando del ciclo SDD SHALL permanecer reservado

#### Scenario: Subcomando desconocido
- **WHEN** se invoca `meltemi` con un token que no pertenece a la gramática
- **THEN** el binario SHALL emitir un error de uso por stderr y terminar con el código de error de uso

#### Scenario: La referencia es la enumeración autoritativa
- **WHEN** la gramática gana o cambia un subcomando sin regenerar la referencia
- **THEN** la verificación de frescura del pipeline SHALL fallar señalándolo

### Requirement: Regla de despacho CLI↔TUI
El binario SHALL decidir su modo de forma determinista a partir de la presencia de
subcomando y de si stdout está conectado a un TTY. Una invocación con subcomando
MUST entrar en modo scriptable de un disparo con independencia del TTY.

#### Scenario: Con subcomando siempre scriptable
- **WHEN** se invoca `meltemi <subcomando>` con stdout redirigido a un archivo o *pipe*
- **THEN** el binario SHALL ejecutar el subcomando en modo scriptable de un disparo
- **AND** SHALL NOT entrar en modo interactivo

#### Scenario: Invocación desnuda con TTY lanza el shell interactivo
- **WHEN** se invoca `meltemi` sin subcomando y stdout está conectado a un TTY
- **THEN** el binario SHALL entrar en el modo interactivo y lanzar el shell de la TUI (capacidad `tui-shell`)
- **AND** SHALL dibujar el chrome de inmediato y conectar con el daemon de forma asíncrona

#### Scenario: Invocación desnuda sin TTY es error de uso
- **WHEN** se invoca `meltemi` sin subcomando y stdout no está conectado a un TTY
- **THEN** el binario SHALL emitir un error de uso que remite a `meltemi help`
- **AND** SHALL NOT quedar a la espera de entrada interactiva

### Requirement: Taxonomía de códigos de salida
El binario SHALL usar una taxonomía de códigos de salida estable: `0` éxito, `1`
error interno inesperado, `2` error de uso, `10` daemon inalcanzable, `11` error
de contrato, `12` operación rechazada por política, `13` operación cancelada,
`14` validación con hallazgos. Todo subcomando MUST terminar con el código que
corresponde a su desenlace.

#### Scenario: Éxito termina en cero
- **WHEN** un subcomando operativo completa su cometido sin error
- **THEN** el binario SHALL terminar con el código `0`

#### Scenario: Daemon inalcanzable
- **WHEN** un subcomando respaldado por RPC no logra conectar ni arrancar el daemon
- **THEN** el binario SHALL terminar con el código `10`

#### Scenario: Respuesta de error del contrato
- **WHERE** el daemon responde con un error JSON-RPC o rechaza la versión de protocolo
- **THEN** el binario SHALL terminar con el código `11`

#### Scenario: Validación con hallazgos distinguible
- **WHEN** una validación concluye con diagnósticos
- **THEN** el binario SHALL terminar con el código `14`
- **AND** una validación limpia SHALL terminar con el código `0`

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
`stop`→`shutdown`, `fleet`→`fleet/list`, `project`→`context/project`,
`sessions`→`session/list`, `explore`→`sdd/explore`, `plan`→`sdd/plan`,
`constitution`→`sdd/constitution`, `review`→`sdd/review`, `verify`→`sdd/verify`,
`archive`→`sdd/archive`, `implement`→`sdd/implement`. Los subcomandos locales
(`version`, `help`) MUST NOT abrir conexión con el daemon.

#### Scenario: initialize precede a todo método RPC
- **WHEN** un subcomando respaldado por RPC abre una conexión con el daemon
- **THEN** el binario SHALL enviar `initialize` antes de cualquier otro método

#### Scenario: implement despliega el plan
- **WHEN** se invoca `meltemi implement` con el daemon accesible
- **THEN** el binario SHALL invocar el método `sdd/implement` y presentar el progreso por tarea

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

### Requirement: Verbos de vínculo de suscripción

La gramática SHALL ofrecer `link <agente> <nombre>` y `unlink <nombre>`,
mapeados a los métodos de vínculo de suscripción del contrato; el nombre MUST
viajar al daemon tal cual se escribió, y la referencia CLI generada SHALL
enumerarlos.

#### Scenario: link crea y responde con el gesto de login

- **WHEN** se invoca `link` con un id del catálogo con variable declarada y
  un nombre válido
- **THEN** la salida SHALL confirmar el vínculo
- **AND** SHALL imprimir el gesto de autenticación compuesto

#### Scenario: unlink de un vínculo manual rehúsa con remedio

- **WHEN** se invoca `unlink` con el nombre de un perfil escrito a mano
- **THEN** el binario SHALL terminar con el código de error de contrato
- **AND** el mensaje SHALL traer el remedio del daemon

### Requirement: El formato de salida es una elección declarada

El binario SHALL tratar el formato de salida como una elección única entre
humano, JSON y YAML, y NO SHALL admitir dos formatos legibles por máquina a la
vez. El flag global `--yaml` SHALL emitir exactamente un documento YAML en
stdout —tanto en éxito como en error— con el mismo contenido que `--json`
emitiría, y MUST NOT mezclar texto humano en stdout. Bajo cualquier formato
legible por máquina la salida NO SHALL llevar color.

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

### Requirement: Los listados se leen de un vistazo

Todo listado del binario SHALL encabezarse con un resumen de los totales que de
otro modo habría que sumar a mano, y SHALL alinear sus columnas con anchos
derivados del contenido, nunca fijos. El resumen de los cambios SHALL declarar
cuántos esperan una decisión.

#### Scenario: El listado abre con su resumen

- **WHEN** se listan las capacidades de la verdad viva o los cambios
- **THEN** la salida SHALL abrir con los totales del listado
- **AND** el resumen de los cambios SHALL decir cuántos esperan una decisión

#### Scenario: Las columnas se alinean con el contenido

- **WHEN** un listado incluye un elemento de nombre más largo que los demás
- **THEN** las columnas SHALL seguir alineadas
- **AND** el ancho NO SHALL provenir de un número fijo en el código

### Requirement: Color redundante y apagable en la salida del cliente

La salida humana del binario MAY usar color para señalar estado y tipo, y el
color MUST ser solo decorativo: toda distinción que el color marque SHALL
marcarse además con símbolo, palabra o cifra, de modo que retirar todo el color
NO SHALL retirar información alguna. El binario SHALL renderizar sin color
alguno cuando se invoque `--no-color`, cuando `NO_COLOR` esté definida con valor
no vacío, cuando `TERM` sea `dumb`, o cuando stdout no esté conectado a un TTY.

#### Scenario: Sin color no se pierde información

- **WHEN** se compara la salida humana coloreada con la misma salida sin color
- **THEN** ambas SHALL contener el mismo texto
- **AND** cada estado y cada tipo SHALL seguir distinguiéndose por símbolo,
  palabra o cifra

#### Scenario: La salida redirigida no lleva color

- **WHEN** stdout no está conectado a un TTY
- **THEN** la salida NO SHALL contener secuencia de color alguna

#### Scenario: El usuario apaga el color

- **WHERE** se invoca `--no-color`, o `NO_COLOR` tiene valor no vacío, o `TERM`
  es `dumb`
- **THEN** la salida NO SHALL contener secuencia de color alguna
