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
`meltemid` SHALL enviar prompts a la sesión ACP y reenviar, en orden y en
tiempo real, todas las actualizaciones de sesión emitidas por el agente hasta
la finalización del turno. El reenvío SHALL alcanzar a la conexión que inició
la sesión sin que ésta pida nada, y a cualquier otra conexión que declare
mirar esa sesión; una conexión que no la mire NO SHALL recibir sus
actualizaciones. Cada actualización SHALL entregarse una sola vez por
conexión. El stream entrega lo emitido desde la suscripción: el histórico se
lee por el registro de sesión, y el daemon NO SHALL encolar eventos para
conexiones cerradas.

#### Scenario: Turno con actualizaciones
- **WHEN** el cliente envía un prompt y el agente emite actualizaciones de progreso antes de completar el turno
- **THEN** el cliente recibe cada actualización en el orden emitido, seguida de la señal de fin de turno

#### Scenario: Cliente que llega a mitad de sesión
- **WHEN** un cliente conecta con una sesión ya en curso que él no inició y declara mirarla
- **THEN** SHALL recibir las actualizaciones emitidas a partir de ese momento
- **AND** el cliente que inició la sesión SHALL seguir recibiéndolas sin declarar nada

#### Scenario: Sin declarar interés no llega el stream
- **WHEN** un cliente conectado no declara mirar una sesión que no inició
- **THEN** NO SHALL recibir las actualizaciones de esa sesión

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
`meltemid` SHALL garantizar que el subproceso del agente termina cuando la sesión se cancela, finaliza o el daemon se apaga. Terminar al agente SHALL terminar también a todo proceso que ese subproceso haya necesitado para existir: WHERE la plataforma lance el agente a través de un intermediario —un script o un shell que a su vez crea el proceso real—, el daemon SHALL terminar al intermediario y a cuanto creó, y NO SHALL dar por terminado un agente cuyo proceso real siga vivo. WHERE la plataforma permita atar la vida de esos procesos a la del daemon, el daemon SHALL atarla al lanzarlos, de modo que un fin abrupto del daemon tampoco deje huérfanos. El identificador de proceso y el programa efectivo del agente SHALL constar en el log de sesión.

#### Scenario: Cancelación de sesión
- **WHEN** el cliente cancela una sesión activa
- **THEN** el subproceso del agente recibe la cancelación ACP y, de no terminar en un plazo razonable, es terminado por el daemon; no queda ningún proceso huérfano

#### Scenario: Un lanzador intermedio no deja huérfanos
- **WHERE** la plataforma resuelve el agente a un intermediario que crea el proceso real
- **WHEN** el daemon termina al agente
- **THEN** el intermediario y todo proceso que creó SHALL haber terminado
- **AND** el daemon NO SHALL reportar la sesión como terminada mientras alguno siga vivo

#### Scenario: El daemon termina de golpe y nada le sobrevive
- **WHERE** la plataforma permite atar la vida del agente a la del daemon
- **WHEN** el proceso del daemon termina sin ejecutar su apagado
- **THEN** los procesos de los agentes que lanzó SHALL terminar igualmente

### Requirement: Dirección de una sesión existente
El daemon SHALL aceptar instrucciones dirigidas a una sesión existente
(`session/direct`): sobre una sesión que ejecuta un turno la instrucción SHALL
encolarse y despacharse como el siguiente turno de la misma sesión del agente al
concluir el turno en curso, sin interrumpirlo; sobre una sesión viva que espera
instrucciones SHALL despacharse de inmediato como su siguiente turno; sobre una
sesión terminada y reanudable SHALL reanudarse con la instrucción como prompt;
sobre una sesión inexistente o no reanudable MUST rehusarse con diagnóstico y
remedio. Cada instrucción MUST registrarse en el log de sesión al encolarse y al
despacharse, y el verbo SHALL ser consumible desde todas las superficies por
igual.

#### Scenario: Instrucción a una sesión activa se despacha como siguiente turno
- **WHEN** un cliente dirige una instrucción a una sesión activa
- **THEN** la instrucción SHALL encolarse sin interrumpir el turno en curso
- **AND** al concluir ese turno SHALL despacharse como el siguiente prompt de la misma sesión del agente
- **AND** el encolado y el despacho SHALL constar en el JSONL

#### Scenario: Instrucción a una sesión que espera se despacha de inmediato
- **WHEN** un cliente dirige una instrucción a una sesión en espera
- **THEN** SHALL despacharse como su siguiente turno sin aguardar a que concluya
  nada
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

### Requirement: Modelo y esfuerzo como transporte opaco por sesión

Una sesión SHALL poder declarar el modelo y el nivel de esfuerzo con los que
corre, y cada palanca SHALL llegar al agente por la vía que el agente tiene: la
bandera de lanzamiento que su entrada de la flota declara o, si la entrada no
declara palancas, la opción que el agente anuncia en la categoría de la palanca
(`model` para el modelo, `thought_level` para el esfuerzo). Por la bandera, el
núcleo SHALL transportar el valor como cadena opaca y NO SHALL interpretarlo ni
validarlo contra catálogo alguno. Por la opción anunciada, el valor SHALL
validarse únicamente contra lo que el agente anunció; el núcleo NO SHALL tener
catálogo propio.

Lo que efectivamente rigió SHALL quedar registrado de modo que el histórico
pueda decir con qué modelo corrió cada sesión, y un valor rehusado NO SHALL
figurar como lo que rigió.

WHERE la entrada del agente declara sus palancas y no nombra la pedida, la
petición SHALL rehusarse antes de crear nada, con un diagnóstico que nombre al
agente y la palanca. NO SHALL ignorarse en silencio.

#### Scenario: El modelo pedido viaja sin interpretarse

- **WHERE** la entrada del agente declara el modelo como bandera de lanzamiento
- **WHEN** una sesión declara un modelo
- **THEN** el agente SHALL recibirlo tal cual
- **AND** el núcleo NO SHALL rechazarlo por su contenido

#### Scenario: Lo que rigió queda en el registro

- **WHEN** una sesión con modelo declarado arranca
- **THEN** el registro SHALL anotar el modelo y el esfuerzo efectivos

#### Scenario: Una palanca que el agente no admite se rehúsa

- **WHERE** la entrada del agente declara sus palancas y no nombra el esfuerzo
- **WHEN** se declara esfuerzo
- **THEN** SHALL rehusarse nombrando al agente y la palanca
- **AND** NO SHALL crearse sesión alguna

#### Scenario: Lo rehusado no figura como lo que rigió

- **WHEN** una sesión termina rehusando el modelo pedido
- **THEN** el histórico de sesiones NO SHALL listar ese modelo como el que rigió

### Requirement: Cambio a mitad de sesión por la vía que el agente anuncia

WHERE el agente anuncie opciones de configuración de sesión, el daemon SHALL
fijarlas por esa vía estándar y NO SHALL relanzar la sesión para cambiarlas.
WHERE no las anuncie, la superficie NO SHALL ofrecer el cambio en vivo.

#### Scenario: Se cambia por la vía estándar cuando el agente la anuncia

- **WHERE** el agente anunció una opción de modelo
- **WHEN** se cambia el modelo en vivo
- **THEN** SHALL fijarse por esa opción
- **AND** la sesión NO SHALL relanzarse

#### Scenario: Sin opción anunciada no se ofrece el cambio en vivo

- **WHERE** el agente no anunció opción alguna
- **THEN** la superficie NO SHALL ofrecer cambiarlo en vivo

### Requirement: El modo de la sesión queda escrito

Una sesión que declara modo SHALL registrarlo en su log al arrancar, y cada
decisión de permiso SHALL registrar bajo qué modo se tomó. Un histórico con
modos que no los registre no puede explicar sus propias decisiones.

#### Scenario: El arranque registra el modo

- **WHEN** una sesión arranca declarando modo
- **THEN** su log SHALL registrarlo

#### Scenario: Cada decisión dice bajo qué modo se tomó

- **WHEN** se resuelve una petición de permiso en una sesión con modo
- **THEN** el registro de la decisión SHALL nombrar el modo vigente

### Requirement: Interrupción con relevo

La dirección de una sesión SHALL admitir pedir que el turno en vuelo se
interrumpa y que la instrucción la releve. La operación SHALL ser atómica: la
instrucción SHALL quedar encolada **antes** de que la interrupción se señale,
de modo que el borde del turno nunca observe la cola vacía entre ambas mitades.

Al drenar el turno interrumpido, la sesión SHALL continuar con la instrucción
que lo relevó, y NO SHALL terminar. Una cancelación de sesión SHALL seguir
terminándola, y un turno que el agente cancela por su cuenta —sin interrupción
pedida— SHALL seguir terminando la sesión como hasta ahora.

Una petición de permiso en vuelo al interrumpir SHALL resolverse como cancelada
y quedar registrada como tal: interrumpir NO SHALL dejar decisiones sin
desenlace en el registro. El registro SHALL permitir distinguir un turno que el
agente detuvo de uno que el usuario interrumpió.

Cuando una interrupción y una cancelación de sesión concurren, SHALL prevalecer
la cancelación.

#### Scenario: La instrucción releva al turno interrumpido

- **WHEN** se dirige una sesión activa pidiendo interrumpir
- **THEN** la instrucción SHALL quedar encolada antes de señalarse la
  interrupción
- **AND** al drenar el turno la sesión SHALL continuar con esa instrucción
- **AND** NO SHALL terminar

#### Scenario: Una cancelación sigue terminando la sesión

- **WHEN** se cancela una sesión
- **THEN** SHALL terminar, y ninguna instrucción encolada SHALL despacharse

#### Scenario: Un turno cancelado por el agente no continúa

- **WHEN** un turno termina cancelado sin que se haya pedido interrumpir
- **THEN** la sesión SHALL terminar como hasta ahora

#### Scenario: El permiso en vuelo se resuelve al interrumpir

- **WHILE** una petición de permiso espera una decisión
- **WHEN** el turno se interrumpe
- **THEN** la petición SHALL resolverse como cancelada
- **AND** SHALL quedar registrada con ese desenlace

#### Scenario: El registro dice quién detuvo el turno

- **WHEN** se lee el registro de una sesión interrumpida
- **THEN** SHALL poder distinguirse de un turno que el agente detuvo por su
  cuenta

#### Scenario: Cancelar gana a interrumpir

- **WHEN** una interrupción y una cancelación de sesión concurren
- **THEN** la sesión SHALL terminar

### Requirement: Sesión que espera la siguiente instrucción

Al concluir un turno sin instrucciones pendientes, el daemon SHALL mantener la
sesión viva y en espera —con su conexión ACP y su subproceso intactos— en vez de
terminarla, y SHALL declararla en estado `waiting_instruction` en `status` y en
el listado de sesiones. Una instrucción dirigida a una sesión en espera SHALL
despacharse como su siguiente turno sin reanudar ni relanzar nada.

La espera SHALL ser por señal, sin sondeo. Encolar una instrucción SHALL mutar la
cola y despertar la espera **bajo el mismo lock y en ese orden**, de modo que no
exista ventana en la que la espera se registre después de que la instrucción
llegó.

Esperar NO SHALL contar como trabajar: la sesión en espera SHALL distinguirse de
una sesión que ejecuta un turno en toda superficie que declare estado.

#### Scenario: La sesión sobrevive al turno y espera

- **WHEN** un turno concluye y no hay instrucciones pendientes
- **THEN** la sesión SHALL permanecer viva en `waiting_instruction`
- **AND** su subproceso de agente SHALL seguir vivo
- **AND** NO SHALL emitirse el fin de sesión

#### Scenario: La instrucción despierta la espera sin reanudar

- **WHEN** se dirige una instrucción a una sesión en espera
- **THEN** SHALL despacharse como el siguiente turno de la misma sesión del
  agente
- **AND** NO SHALL crearse una sesión nueva ni relanzarse el subproceso

#### Scenario: Encolar y despertar no dejan ventana

- **WHEN** una instrucción llega justo mientras el borde del turno entra en
  espera
- **THEN** SHALL despacharse igualmente
- **AND** la sesión NO SHALL quedar dormida con trabajo en la cola

#### Scenario: La sesión en espera se cancela como cualquier otra

- **WHEN** se cancela una sesión que espera instrucciones
- **THEN** SHALL terminar y su subproceso SHALL terminar con ella
- **AND** el apagado del daemon SHALL alcanzarla igual

### Requirement: Cotas explícitas de la espera ociosa

La espera ociosa SHALL estar gobernada por cotas configurables con defaults
conservadores: un tiempo máximo de espera sin instrucción y un número máximo de
sesiones esperando a la vez. Alcanzado el tope de sesiones, la espera más
antigua SHALL cerrarse y SHALL decirse; el arranque de sesiones nuevas NO SHALL
rehusarse por esta causa.

Una sesión que espera sin ningún cliente conectado de forma sostenida SHALL
terminar: no hay a quién esperar.

Al vencer cualquiera de las cotas, la sesión SHALL finalizar honestamente con un
motivo que diga que venció la espera, y NUNCA como completada. Desde ahí SHALL
aplicar la reanudación de siempre.

#### Scenario: La espera vencida termina con su motivo

- **WHEN** una sesión espera más que el tiempo configurado
- **THEN** SHALL finalizar con motivo de espera vencida
- **AND** NO SHALL registrarse como completada
- **AND** SHALL quedar reanudable como cualquier sesión terminada

#### Scenario: El tope de esperas cierra la más antigua

- **WHEN** una sesión entra en espera con el tope ya alcanzado
- **THEN** la espera más antigua SHALL cerrarse con su motivo
- **AND** la sesión nueva NO SHALL rehusarse

#### Scenario: Sin clientes sostenidamente, la espera termina

- **WHILE** una sesión espera instrucciones
- **WHEN** no queda ningún cliente conectado durante la gracia configurada
- **THEN** la sesión SHALL terminar con su motivo

### Requirement: Arranque desacoplado de la vida de la sesión

`session/start` SHALL admitir un parámetro aditivo que pida responder en cuanto
la sesión exista, con su identificador, dejando el desenlace al stream de
eventos. Omitido, el verbo SHALL comportarse exactamente como hoy: responder al
terminar la sesión, con el mismo resultado.

#### Scenario: Arranque desacoplado responde con el identificador

- **WHEN** se arranca una sesión pidiendo desacople
- **THEN** la respuesta SHALL llegar en cuanto la sesión exista
- **AND** SHALL llevar su identificador
- **AND** el turno SHALL seguir corriendo y publicándose en el stream

#### Scenario: Sin pedirlo, el arranque responde como siempre

- **WHEN** se arranca una sesión sin pedir desacople
- **THEN** la respuesta SHALL llegar al terminar la sesión
- **AND** SHALL llevar el mismo resultado que hoy

### Requirement: Título derivado de la sesión

El daemon SHALL derivar un título de la instrucción que inicia una sesión, en
local y sin modelo alguno: la primera línea no vacía del texto **tal como el
usuario lo escribió** —antes de expandir referencias—, con los espacios
colapsados y truncada con elipsis a un tope fijo, contando caracteres y nunca
bytes. La derivación SHALL ser determinista: la misma instrucción SHALL
producir el mismo título. Una sesión que no nace de una instrucción de usuario
NO SHALL recibir título; el campo SHALL quedar ausente en vez de componer uno.

#### Scenario: Título derivado de la primera instrucción

- **WHEN** una sesión se inicia con una instrucción de usuario
- **THEN** su título SHALL ser la primera línea no vacía de esa instrucción,
  con los espacios colapsados
- **AND** SHALL truncarse con elipsis al superar el tope, sin partir un carácter

#### Scenario: El título sale del texto que se escribió

- **WHEN** la instrucción contiene referencias que el daemon expande
- **THEN** el título SHALL derivarse del texto previo a la expansión

#### Scenario: Sin instrucción de usuario no hay título inventado

- **WHEN** una sesión se abre sin instrucción de usuario
- **THEN** SHALL quedar sin título
- **AND** las superficies SHALL nombrarla como lo hacían antes

### Requirement: El título acompaña a la sesión mientras exista

El título SHALL viajar en el listado de sesiones y en el evento de inicio, de
modo que un cliente pueda nombrar una sesión recién abierta sin esperar al
siguiente listado. El título SHALL sobrevivir al cierre de la sesión: el
plegado de los registros NO SHALL perderlo por que el registro final no lo
repita. Cuando el índice se reconstruya desde el registro de la sesión, el
título SHALL recuperarse de él. Una sesión reanudada SHALL conservar el título
de la sesión que continúa. Las sesiones anteriores a esta capacidad SHALL
carecer de título sin que ello altere su registro ni exija migración alguna.

#### Scenario: El título sobrevive al cierre de la sesión

- **WHEN** una sesión con título termina
- **THEN** el listado SHALL seguir mostrando su título

#### Scenario: El título se recupera del registro

- **WHEN** el índice se reconstruye desde el registro de una sesión con título
- **THEN** el registro reconstruido SHALL conservar el título

#### Scenario: Una sesión reanudada conserva el título

- **WHEN** una sesión terminada se reanuda con una instrucción nueva
- **THEN** la sesión resultante SHALL conservar el título de la que continúa
- **AND** NO SHALL derivar uno nuevo de la instrucción de continuación

### Requirement: El modo anunciado se lee en cualquiera de sus dos formas

WHERE el agente anuncie sus modos de sesión por el campo de modos del
protocolo y ninguna opción de configuración de categoría modo, el daemon
SHALL exponer esos modos como una opción de configuración de selección con
la categoría de modo, con el modo actual como valor actual, y SHALL fijar el
modo elegido por el verbo de modos del protocolo. WHERE el agente anuncie las
dos formas, la opción de configuración SHALL prevalecer entera y el campo de
modos NO SHALL fusionarse con ella. El contrato hacia las superficies NO
SHALL cambiar: la opción SHALL viajar por la misma forma que cualquier otra
opción anunciada.

#### Scenario: Modos sin opción de configuración se ofrecen como opción

- **WHEN** el agente responde a la apertura de sesión con modos y sin opciones de configuración
- **THEN** la sesión SHALL anunciar una opción de selección de categoría modo
- **AND** sus valores SHALL ser exactamente los modos disponibles, con el modo actual como valor actual

#### Scenario: Con las dos formas gana la opción de configuración

- **WHEN** el agente anuncia una opción de configuración de categoría modo y además el campo de modos
- **THEN** la sesión SHALL anunciar la opción de configuración tal cual
- **AND** ningún valor del campo de modos SHALL añadirse a ella

#### Scenario: El modo elegido viaja por el verbo de modos

- **WHERE** la opción de modo procede del campo de modos
- **WHEN** se fija esa opción por la vía de opciones de sesión
- **THEN** el daemon SHALL enviar el verbo de modos del protocolo con el modo elegido
- **AND** SHALL registrar el modo aceptado como valor actual sin relanzar la sesión

### Requirement: Lo que el agente cambia por su cuenta queda reflejado

WHEN el agente notifique un cambio del modo actual o una lista nueva de
opciones de configuración, el daemon SHALL actualizar lo anunciado de esa
sesión y SHALL registrarlo en el log como lo anunciado ahora, de modo que las
superficies lo vean sin relanzar. Una lista nueva de opciones SHALL
reemplazar entera a la anterior; NO SHALL fusionarse.

#### Scenario: Un cambio de modo del agente actualiza la opción

- **WHEN** el agente notifica un cambio del modo actual
- **THEN** la opción de modo de la sesión SHALL pasar a ese valor
- **AND** el log SHALL registrar lo anunciado ahora

#### Scenario: Una lista de opciones nueva reemplaza a la anterior

- **WHEN** el agente notifica una lista nueva de opciones de configuración
- **THEN** lo anunciado de la sesión SHALL ser exactamente esa lista
- **AND** ninguna opción de la lista anterior SHALL sobrevivir por su cuenta

### Requirement: Un turno que no llevó nada se declara vacío
WHEN un turno del agente termina con el motivo de parada que Meltemi traduce
como `completed`, sin que el agente haya enviado durante ese turno un fragmento
de mensaje o de pensamiento con texto que no sea solo espacios o con contenido
no textual, una llamada a herramienta o su actualización, un plan ni una
petición de permiso, y sin que se haya pedido cancelar la sesión o interrumpir
ese turno, el daemon SHALL registrar en el log de la sesión un evento
`turn_empty` que conserve el motivo de parada. El evento SHALL registrarse para
cada turno así terminado, también en los turnos intermedios de una sesión que
corre varios turnos por instrucciones dirigidas, en un despacho y en cada tarea
de una implementación. El motivo de parada que el agente reportó NO SHALL
cambiar.

#### Scenario: Turno sin palabra ni acción declarado vacío
- **WHEN** el agente cierra su turno con `end_turn` sin haber enviado nada que lleve trabajo
- **THEN** el log de la sesión SHALL registrar `turn_empty` con el motivo de parada `completed`
- **AND** el cierre del turno SHALL conservar el motivo `completed` que el agente reportó

#### Scenario: Un turno con cualquier palabra o acción no es vacío
- **WHEN** el agente envía durante el turno al menos un fragmento de mensaje con texto, una llamada a herramienta o una petición de permiso
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Un turno que solo piensa, actualiza una herramienta o su plan no es vacío
- **WHEN** lo único que el agente envía durante el turno es un pensamiento, la actualización de una herramienta, un plan o un fragmento con contenido no textual
- **THEN** el turno NO SHALL declararse vacío

#### Scenario: Un fragmento de solo espacios no cuenta como trabajo
- **WHEN** lo único que el agente envía durante el turno es un fragmento de mensaje cuyo texto es solo espacios
- **THEN** el turno SHALL declararse vacío

#### Scenario: Lo que describe la sesión no cuenta como trabajo
- **WHEN** el agente envía durante el turno solo actualizaciones de la sesión, como sus comandos disponibles o su consumo
- **THEN** el turno SHALL declararse vacío

#### Scenario: Un turno que no terminó no se declara vacío
- **WHEN** un turno termina cancelado, rehusado o por un límite del agente
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno

#### Scenario: Un turno que el humano detuvo no se declara vacío
- **WHEN** el humano cancela la sesión o interrumpe el turno y el agente lo cierra igual con `end_turn` sin haber enviado nada
- **THEN** el log NO SHALL registrar `turn_empty` para ese turno
- **AND** la respuesta NO SHALL declarar el turno vacío

#### Scenario: Cada turno de una sesión de varios turnos se mide
- **WHEN** una sesión corre un turno con trabajo, uno vacío y otro con trabajo por instrucciones dirigidas
- **THEN** el log SHALL registrar exactamente un `turn_empty`, después del segundo prompt y antes del tercero

### Requirement: Las respuestas de sesión dicen si el turno fue vacío
La respuesta de `session/start` sin `detach` y la de `session/direct` cuando
reanuda SHALL declarar con `emptyTurn` en `true` que el turno cuyo estado
informan se declaró vacío, conservando ese estado, y SHALL omitir el campo en
cualquier otro caso. Un arranque con `detach` y una instrucción encolada o
relevada, que no informan estado, NO SHALL llevar el campo.

#### Scenario: Un arranque cuyo turno fue vacío lo declara
- **WHEN** un `session/start` sin `detach` corre un turno que se declara vacío
- **THEN** la respuesta SHALL traer `status` en `completed` y `emptyTurn` en `true`

#### Scenario: Un turno con trabajo no deja clave
- **WHEN** un `session/start` sin `detach` corre un turno que llevó trabajo
- **THEN** la respuesta NO SHALL contener la clave `emptyTurn`

#### Scenario: Sin estado no hay veredicto en la respuesta
- **WHEN** un `session/start` con `detach` arranca un turno que resulta vacío
- **THEN** la respuesta NO SHALL contener `status` ni `emptyTurn`
- **AND** el log SHALL registrar `turn_empty`

### Requirement: La palanca anunciada se confirma antes del primer prompt

WHEN una palanca va por la opción que el agente anuncia, el daemon SHALL fijarla
al abrir la sesión y antes del primer prompt —el modelo primero y el esfuerzo
contra lo que quede anunciado después—, y SHALL enviar el primer prompt
únicamente si la respuesta del agente deja cada palanca pedida con exactamente el
valor pedido y no cambió el modo. La espera de cada respuesta SHALL estar
acotada y SHALL poder cancelarse. En cualquier otro caso —la categoría no está
anunciada o lo está más de una vez, el valor no está entre los anunciados, un
interruptor recibe algo distinto de `true` o `false`, el agente dejó otro valor,
cambió el modo o no contestó a tiempo— la sesión SHALL terminar sin prompt con
la razón `lever_refused`, y el registro SHALL anotar la palanca, lo pedido, la
razón y lo que el agente anunció o dejó, acotado. El rehúso SHALL llegar a quien
arrancó la sesión, en todo verbo, como el error 2007 con esos datos; una sesión
desacoplada SHALL decirlo por su registro.

#### Scenario: La opción anunciada fija el modelo antes del primer prompt

- **WHERE** el agente anuncia una opción de categoría `model` y su entrada no declara palancas
- **WHEN** la sesión declara un modelo que el agente anunció
- **THEN** SHALL fijarse por esa opción antes del primer prompt
- **AND** el registro SHALL anotar el modelo que rigió

#### Scenario: La opción anunciada fija el esfuerzo antes del primer prompt

- **WHERE** el agente anuncia una opción de categoría `thought_level`
- **WHEN** la sesión declara un esfuerzo que el agente anunció
- **THEN** SHALL fijarse por esa opción antes del primer prompt

#### Scenario: Un valor no anunciado se rehúsa listando lo anunciado

- **WHEN** la sesión declara un modelo que el agente no anunció
- **THEN** SHALL rehusarse con el error 2007 listando los valores anunciados
- **AND** NO SHALL enviarse prompt alguno

#### Scenario: Sin una opción de la categoría la palanca se rehúsa al abrir

- **WHERE** el agente no anuncia opción de la categoría de la palanca, o anuncia más de una
- **WHEN** se declara esa palanca
- **THEN** SHALL rehusarse listando las opciones anunciadas
- **AND** NO SHALL enviarse prompt alguno

#### Scenario: Lo que el agente no confirma se rehúsa sin prompt

- **WHEN** el agente responde dejando un valor distinto del pedido
- **THEN** SHALL rehusarse nombrando lo pedido y lo que el agente dejó
- **AND** NO SHALL enviarse prompt alguno

#### Scenario: Un cambio de modo al fijar la palanca se rehúsa

- **WHEN** fijar la palanca le cambia el modo al agente
- **THEN** SHALL rehusarse sin enviar prompt

#### Scenario: Un agente que no contesta a tiempo se rehúsa

- **WHEN** el agente no responde a la fijación dentro de la espera
- **THEN** SHALL rehusarse sin enviar prompt

#### Scenario: Cancelar mientras se fija la palanca no envía prompt

- **WHEN** la sesión se cancela mientras se espera la confirmación de la palanca
- **THEN** la sesión SHALL terminar cancelada
- **AND** NO SHALL enviarse prompt alguno

#### Scenario: El rehúso llega a quien lo pidió

- **WHEN** un rehúso al abrir ocurre en `propose`, `dispatch` o `implement`
- **THEN** el verbo SHALL responder el error 2007 con la palanca, lo pedido y la razón
- **AND** NO SHALL informarlo como un agente que se negó a continuar

### Requirement: El estado de cada sesión se notifica a todo cliente conectado

`meltemid` SHALL notificar a **toda** conexión iniciada cada transición de
estado de **cualquier** sesión, sin que el cliente la pida y sin importar qué
conexión abrió esa sesión. La notificación SHALL afirmar el estado completo
—identificador de sesión, raíz del proyecto, estado, el instante desde el que
la sesión está en ese estado, y cuántos permisos suyos esperan decisión— y NO
SHALL expresar una diferencia respecto de un estado anterior ni llevar número
de secuencia: aplicarla dos veces, o aplicar dos de ellas en cualquier orden,
SHALL converger al último estado afirmado.

Al iniciar, una conexión SHALL recibir el estado actual de cada sesión que el
daemon conoce **antes** de recibir cualquier transición posterior, de modo que
una conexión que llega tarde no dependa de haber visto la transición que la
precedió.

La notificación NO SHALL llevar contenido del proyecto, de la instrucción ni
del turno: lo que un cliente no podía leer antes SHALL seguir sin poder
leerlo.

WHERE una conexión no alcance a leer las notificaciones al ritmo en que se
emiten y alguna se pierda, el daemon SHALL volver a afirmar el estado de cada
sesión conocida a esa conexión, de modo que un hueco se repare sin que el
cliente tenga que advertirlo ni pedir nada. El daemon NO SHALL dejar el hueco
registrado solo de su lado.

#### Scenario: Una transición llega a un cliente que no inició la sesión

- **WHEN** un cliente abre una sesión y otro cliente distinto está conectado al mismo daemon
- **THEN** el segundo cliente recibe la notificación de cada transición de esa sesión sin haberla pedido
- **AND** el estado que recibe es el mismo que devolvería la lista de sesiones en ese instante

#### Scenario: Una conexión nueva recibe el estado actual antes de cualquier transición

- **WHEN** una conexión se inicia mientras ya hay sesiones en curso
- **THEN** recibe una afirmación del estado de cada sesión conocida antes de cualquier transición posterior
- **AND** una transición ocurrida durante ese envío llega después y corrige lo afirmado, sin perderse

#### Scenario: La notificación afirma el estado entero y es idempotente

- **WHEN** una conexión aplica dos veces la misma notificación, o aplica dos notificaciones de una sesión en orden invertido
- **THEN** el estado con el que queda es el de la última notificación afirmada
- **AND** ninguna de las dos lecturas exige un número de secuencia ni un estado previo

#### Scenario: Un rezago se repara reafirmando el estado, no con un hueco

- **WHEN** una conexión pierde notificaciones por no leerlas al ritmo en que se emiten
- **THEN** el daemon vuelve a afirmar el estado de cada sesión conocida a esa conexión
- **AND** el cliente queda con estado válido sin haber pedido nada ni haber advertido el hueco

#### Scenario: La notificación no lleva contenido del proyecto

- **WHEN** se examina el contenido de una notificación de estado
- **THEN** lleva identificador, raíz, estado, instante y conteo de permisos en espera, y nada más
- **AND** no lleva instrucción, ni título, ni texto del turno
