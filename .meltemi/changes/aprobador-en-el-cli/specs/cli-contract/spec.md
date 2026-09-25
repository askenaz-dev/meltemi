# cli-contract — delta

## ADDED Requirements

### Requirement: Aprobador interactivo en terminal
WHERE stdin y stderr del binario son terminales, cada petición de permiso que
el daemon empuje a un subcomando SHALL presentarse en stderr con su sesión, su
herramienta, su título, el comando y la ruta que el agente haya enviado, y las
opciones que el agente ofreció, numeradas y con su nombre y su clase; el binario
SHALL leer una tecla, sin opción por omisión, y SHALL responder con la opción
del agente que el humano eligió. La tecla de denegar SHALL elegir la opción de
rechazo del agente, o cancelar si no ofreció ninguna. El binario NO SHALL
inventar opciones que el agente no ofreció, y stdout NO SHALL recibir nada del
aprobador. Ctrl+C durante una pregunta SHALL denegarla, cancelar la sesión y
terminar con el código `13`. Una pregunta que vence por la política de espera
del daemon, o que se decide en otra superficie, SHALL retirarse del terminal
diciendo por qué. WHERE stdin o stderr no es un terminal, toda petición SHALL
denegarse con un aviso en stderr que nombre cómo conceder sin humano: una regla
o un modo. Ninguna bandera SHALL conceder permisos sin humano ni regla.

#### Scenario: Una tecla elige una opción del agente
- **WHEN** el agente pide un permiso y el humano pulsa el número de una opción de permiso
- **THEN** el agente SHALL recibir esa opción
- **AND** el registro SHALL anotar la decisión como tomada por el cliente

#### Scenario: Denegar elige el rechazo del agente
- **WHEN** el humano pulsa la tecla de denegar ante una petición que ofrece una opción de rechazo
- **THEN** el agente SHALL recibir su propia opción de rechazo y no una cancelación

#### Scenario: Sin terminal se deniega como hasta ahora
- **WHEN** un subcomando recibe una petición de permiso sin terminal donde preguntar
- **THEN** la petición SHALL denegarse
- **AND** el aviso SHALL nombrar la regla y el modo como las maneras de conceder sin humano

#### Scenario: La salida de máquina queda intacta con el aprobador activo
- **WHEN** un subcomando con `--json` pregunta y recibe una respuesta del humano
- **THEN** stdout SHALL contener exactamente un objeto JSON
- **AND** la pregunta SHALL haber ido a stderr

#### Scenario: Ctrl+C deniega y cancela el turno
- **WHEN** el humano pulsa Ctrl+C ante una pregunta
- **THEN** la petición SHALL denegarse y la sesión SHALL cancelarse
- **AND** el binario SHALL terminar con el código `13`

#### Scenario: Una pregunta que vence se retira del terminal
- **WHEN** la política de espera del daemon vence mientras la pregunta está abierta
- **THEN** el terminal SHALL retirar la pregunta diciendo que venció

#### Scenario: Una pregunta decidida en otra superficie se retira del terminal
- **WHEN** otra superficie decide la petición mientras la pregunta está abierta
- **THEN** el terminal SHALL retirar la pregunta diciendo que se decidió en otra parte

#### Scenario: Ninguna bandera concede permisos
- **WHEN** se invoca un subcomando con una bandera que pretenda conceder permisos sin preguntar
- **THEN** el binario SHALL rechazarla como error de uso
