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
`14` validación con hallazgos, `15` turno del agente terminado sin decir ni
hacer nada. Todo subcomando MUST terminar con el código que corresponde a su
desenlace.

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

#### Scenario: Turno vacío distinguible
- **WHEN** un subcomando recibe una respuesta que declara vacío el turno que informa
- **THEN** el binario SHALL terminar con el código `15`
- **AND** el mismo subcomando con un turno que llevó trabajo SHALL terminar con el código `0`

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

### Requirement: Modelo y esfuerzo se declaran al arrancar desde la CLI

Los verbos que arrancan una sesión SHALL admitir declarar modelo y esfuerzo, y
su ayuda SHALL decir que son cadenas del proveedor que el núcleo no interpreta.
Un valor vacío SHALL rehusarse; NO SHALL enviarse una cadena vacía como si
fuera una elección.

#### Scenario: Un valor vacío se rehúsa en vez de viajar

- **WHEN** se arranca una sesión con un modelo vacío
- **THEN** SHALL rehusarse
- **AND** NO SHALL arrancar con una cadena vacía

### Requirement: Verbos de identidad

El CLI SHALL ofrecer `identity`, que consulta la identidad y los equipos por
el método de identidad del contrato y los imprime como listado legible, y
`identity-link [--hostname <nombre>] [--exec]`, que imprime el gesto de
vínculo compuesto por el daemon y, solo con `--exec`, ejecuta el binario del
propio usuario con la entrada y la salida heredadas. Ambos SHALL honrar
`--json` y `--yaml` como el resto de los listados. Sin `--exec`,
`identity-link` NO SHALL lanzar ningún proceso del cliente de malla (el
arranque del daemon bajo demanda sigue rigiendo como en todo verbo respaldado
por RPC).

#### Scenario: identity imprime quién y qué equipos

- **WHEN** se ejecuta `identity` con la malla enlazada
- **THEN** la salida SHALL decir el nombre de login, el proveedor, este equipo
  y cada equipo con su presencia en palabra
- **AND** con `--json` SHALL imprimir la respuesta del contrato tal cual

#### Scenario: identity-link imprime el gesto y solo ejecuta a pedido

- **WHEN** se ejecuta `identity-link` sin `--exec`
- **THEN** SHALL imprimir el gesto en la forma de la plataforma y la pista de
  que abrirá el navegador
- **AND** NO SHALL lanzar ningún proceso del cliente de malla; con `--exec`
  SHALL correr el binario del usuario con entrada y salida heredadas

### Requirement: El modo se declara al arrancar desde la CLI

Los verbos que arrancan una sesión SHALL admitir declarar el modo, y su ayuda
SHALL nombrar los modos admitidos. Un nombre que no sea uno de ellos SHALL
rehusarse con los válidos, jamás degradarse a uno.

#### Scenario: Un modo desconocido se rehúsa con los válidos

- **WHEN** se arranca una sesión con un modo que no existe
- **THEN** SHALL rehusarse nombrando los modos admitidos
- **AND** NO SHALL arrancar con otro modo

### Requirement: El arranque scriptable sigue esperando por defecto

El verbo de arranque de la CLI SHALL seguir esperando el desenlace de la sesión
por defecto. El desacople SHALL ser opt-in explícito, y mientras la CLI no tenga
superficie para el stream de eventos ni para el registro de sesión, la ayuda del
verbo SHALL decir qué NO se verá al desacoplar.

#### Scenario: Arrancar desde la CLI sigue mostrando el desenlace

- **WHEN** se arranca una sesión desde la CLI sin pedir desacople
- **THEN** el comando SHALL esperar y SHALL mostrar el desenlace del turno

### Requirement: El harness efectivo se consulta desde la línea de comandos

La CLI SHALL ofrecer un verbo que muestre el harness efectivo, opcionalmente
acotado a un agente, con la capa de origen de cada pieza y con lo pisado y lo
inválido incluidos. SHALL respetar la disciplina de salida vigente: legible por
defecto, estructurada bajo `--json`, y los mismos códigos de salida que el
resto de los verbos de lectura.

#### Scenario: El verbo muestra de qué capa viene cada pieza

- **WHEN** se consulta el harness desde la CLI
- **THEN** cada pieza SHALL mostrarse con su capa de origen
- **AND** lo pisado y lo inválido SHALL aparecer con su motivo

### Requirement: Un turno vacío se advierte con remedio
WHEN la respuesta de un subcomando declara vacío el turno que informa, el
binario SHALL escribir en stderr, en modo humano, una advertencia que diga que
el agente terminó sin decir ni hacer nada y un remedio en condicional: probar el
agente por su cuenta y revisar su proveedor y sus credenciales. En modo `--json`
la respuesta SHALL viajar íntegra por stdout, con el campo que declara el turno
vacío. La regla SHALL depender del campo de la respuesta y no de una lista de
subcomandos.

#### Scenario: La advertencia va a stderr con su remedio
- **WHEN** `meltemi session` termina con un turno vacío en modo humano
- **THEN** stderr SHALL contener la advertencia y el remedio
- **AND** stdout SHALL contener solo la salida útil del comando

#### Scenario: La salida de máquina lleva el veredicto
- **WHEN** un subcomando que arranca sesión termina con un turno vacío en modo `--json`
- **THEN** stdout SHALL contener la respuesta con `emptyTurn` en `true`

#### Scenario: Un verbo de autoría sigue la misma regla
- **WHEN** `meltemi plan` recibe una respuesta con `emptyTurn` en `true`
- **THEN** el binario SHALL advertir en stderr y terminar con el código `15`

### Requirement: Aprobador interactivo en terminal
WHERE stdin y stderr del binario son terminales y el terminal admite leer
teclas en modo crudo, cada petición de permiso que el daemon empuje a un
subcomando SHALL presentarse en stderr con su sesión, su herramienta, su
título, el comando, la ruta y el texto que el agente haya enviado —en la
petición o en la llamada a herramienta que la sesión transmitió con el mismo
id— y las opciones que el agente ofreció, numeradas y con su nombre y su clase;
el binario SHALL leer una tecla, sin opción por omisión, y SHALL responder con
la opción del agente que el humano eligió. Ningún texto del agente SHALL llegar
al terminal con caracteres de control o de reordenación sin escapar. Una tecla
llegada antes de que la pregunta pudiera leerse NO SHALL contestarla. Elegir
una opción con la que el agente deja de preguntar SHALL exigir una tecla de
confirmación. La tecla de denegar SHALL elegir la opción de rechazo del agente,
o cancelar si no ofreció ninguna. El binario NO SHALL inventar opciones que el
agente no ofreció, y stdout NO SHALL recibir nada del aprobador. Ctrl+C durante
una pregunta SHALL cancelar la sesión, retirar la petición y terminar el
comando con el código `13`. Una pregunta que vence por la política de espera
del daemon, o que se resuelve por otra vía, SHALL retirarse del terminal
diciendo por qué, y el terminal SHALL volver a su estado antes de que el
binario termine. WHERE no hay terminal donde preguntar, toda petición SHALL
denegarse con la opción de rechazo del agente, o cancelando si no ofreció
ninguna, y con un aviso en stderr que nombre las reglas de permisos como la
manera de conceder sin humano. Esta change NO SHALL añadir bandera alguna que
conceda permisos.

#### Scenario: Una tecla elige una opción del agente
- **WHEN** el agente pide un permiso y el humano pulsa el número de una opción de permiso
- **THEN** el agente SHALL recibir esa opción
- **AND** el registro SHALL anotar la decisión como tomada por el cliente

#### Scenario: Denegar elige el rechazo del agente
- **WHEN** el humano pulsa la tecla de denegar ante una petición que ofrece una opción de rechazo
- **THEN** el agente SHALL recibir su propia opción de rechazo y no una cancelación

#### Scenario: Enter y las teclas sin asignar no deciden
- **WHEN** el humano pulsa Enter, un dígito sin opción o una tecla sin asignar
- **THEN** la pregunta SHALL seguir abierta y el agente no SHALL recibir respuesta

#### Scenario: Lo tecleado antes de poder leer la pregunta no la contesta
- **WHEN** llegan teclas antes de que la pregunta pueda leerse
- **THEN** SHALL descartarse sin elegir nada
- **AND** si siguen llegando así, la petición SHALL denegarse como desatendida

#### Scenario: El texto del agente no controla el terminal
- **WHEN** el título, el comando o una opción del agente traen caracteres de control o de reordenación
- **THEN** la pregunta SHALL mostrarlos escapados

#### Scenario: La pregunta muestra el comando que la sesión transmitió
- **WHEN** la petición no trae el comando pero la llamada a herramienta con su id sí lo trajo
- **THEN** la pregunta SHALL mostrar ese comando

#### Scenario: Una opción con la que el agente deja de preguntar pide confirmación
- **WHEN** el humano elige una opción de clase «permitir siempre»
- **THEN** el binario SHALL pedir una tecla de confirmación antes de responder
- **AND** SHALL decir que el agente dejará de preguntar por peticiones parecidas

#### Scenario: Sin terminal se deniega con el rechazo del agente
- **WHEN** un subcomando recibe una petición de permiso sin terminal donde preguntar
- **THEN** el agente SHALL recibir su opción de rechazo, o una cancelación si no ofreció ninguna
- **AND** el aviso SHALL nombrar las reglas de permisos como la manera de conceder sin humano

#### Scenario: La salida de máquina queda intacta con el aprobador activo
- **WHEN** un subcomando con `--json` pregunta y recibe una respuesta del humano
- **THEN** stdout SHALL contener exactamente un objeto JSON
- **AND** la pregunta SHALL haber ido a stderr

#### Scenario: Ctrl+C deniega y cancela el turno
- **WHEN** el humano pulsa Ctrl+C ante una pregunta
- **THEN** la sesión SHALL cancelarse y el registro SHALL anotar la petición como interrumpida
- **AND** el binario SHALL terminar con el código `13`

#### Scenario: Una pregunta que vence se retira del terminal
- **WHEN** la política de espera del daemon vence mientras la pregunta está abierta
- **THEN** el terminal SHALL retirar la pregunta diciendo que venció

#### Scenario: Una pregunta resuelta por otra vía se retira del terminal
- **WHEN** otra superficie decide la petición mientras la pregunta está abierta
- **THEN** el terminal SHALL retirar la pregunta diciendo que se resolvió en otra parte

#### Scenario: Una instantánea anterior a la petición no la retira
- **WHEN** llega una instantánea de la cola sin la petición antes de que su presencia se haya visto
- **THEN** la pregunta SHALL seguir abierta

#### Scenario: El terminal vuelve a su estado al terminar
- **WHEN** el comando termina con una pregunta abierta o después de retirarla
- **THEN** el modo crudo SHALL haberse abandonado tantas veces como se entró antes de que el binario termine

#### Scenario: Ninguna bandera nueva concede permisos
- **WHEN** se invoca un subcomando con una bandera que pretenda conceder permisos sin preguntar
- **THEN** el binario SHALL rechazarla como error de uso
- **AND** el conjunto de banderas globales que acepta SHALL ser exactamente el declarado
