# cli-contract — delta

## ADDED Requirements

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
