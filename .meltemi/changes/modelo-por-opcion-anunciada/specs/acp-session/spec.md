# acp-session — delta

## MODIFIED Requirements

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

## ADDED Requirements

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
