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
`meltemid` SHALL garantizar que el subproceso del agente termina cuando la sesión se cancela, finaliza o el daemon se apaga.

#### Scenario: Cancelación de sesión
- **WHEN** el cliente cancela una sesión activa
- **THEN** el subproceso del agente recibe la cancelación ACP y, de no terminar en un plazo razonable, es terminado por el daemon; no queda ningún proceso huérfano

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
corre. El núcleo SHALL transportarlos como **cadenas opacas** y NO SHALL
interpretarlas, validarlas contra catálogo alguno ni derivar de ellas
comportamiento propio: quien las acepta o las rechaza es el agente.

Lo que efectivamente rigió SHALL quedar registrado en el evento de resolución
del agente —los valores efectivos tras aplicar el default del perfil y lo
declarado en la sesión—, de modo que el histórico pueda decir con qué modelo
corrió cada sesión.

WHERE el agente de la sesión no admita una de las dos palancas, la petición
SHALL rehusarse con diagnóstico que nombre al agente y la palanca. NO SHALL
ignorarse en silencio.

#### Scenario: El modelo pedido viaja sin interpretarse

- **WHEN** una sesión declara modelo y esfuerzo
- **THEN** el núcleo SHALL transportarlos tal cual al agente
- **AND** NO SHALL rechazarlos por su contenido

#### Scenario: Lo que rigió queda en el registro

- **WHEN** una sesión con modelo declarado arranca
- **THEN** el evento de resolución SHALL registrar el modelo y el esfuerzo
  efectivos

#### Scenario: Una palanca que el agente no admite se rehúsa

- **WHERE** el agente no admite el esfuerzo por sesión
- **WHEN** se declara esfuerzo
- **THEN** SHALL rehusarse nombrando al agente y la palanca
- **AND** la sesión NO SHALL arrancar como si nada se hubiera pedido

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
