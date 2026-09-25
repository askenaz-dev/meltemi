# acp-session — delta

## MODIFIED Requirements

### Requirement: Modelo y esfuerzo como transporte opaco por sesión

Una sesión SHALL poder declarar el modelo y el nivel de esfuerzo con los que
corre, y cada palanca SHALL llegar al agente por la primera vía que el agente
tenga: la bandera de lanzamiento que su entrada de la flota declara, o la opción
de configuración que el agente anuncia en la categoría de la palanca (`model`
para el modelo, `thought_level` para el esfuerzo). Por la bandera, el núcleo
SHALL transportar el valor como cadena opaca y NO SHALL interpretarlo ni
validarlo contra catálogo alguno. Por la opción anunciada, el valor SHALL
fijarse antes del primer prompt y SHALL validarse únicamente contra lo que el
agente anunció; el núcleo NO SHALL tener catálogo propio.

Lo que efectivamente rigió SHALL quedar registrado —el valor que viajó por
bandera en el evento de resolución del agente, y el que el agente confirmó al
fijar su opción en un evento propio—, de modo que el histórico pueda decir con
qué modelo corrió cada sesión.

WHERE el agente no tenga la palanca por ninguna de las dos vías, o no haya
anunciado el valor pedido, la petición SHALL rehusarse con un diagnóstico que
nombre al agente y la palanca y, si el agente anunció la categoría, los valores
que anunció. El rehúso SHALL ocurrir antes de crear la sesión cuando ya se sabe
—la entrada declara sus palancas sin nombrar la pedida, o el agente no habla
ACP— y, cuando solo el anuncio puede decirlo, al abrir la sesión y antes del
primer prompt. NO SHALL ignorarse en silencio.

#### Scenario: El modelo pedido viaja sin interpretarse

- **WHERE** la entrada del agente declara el modelo como bandera de lanzamiento
- **WHEN** una sesión declara un modelo
- **THEN** el núcleo SHALL transportarlo tal cual al agente
- **AND** NO SHALL rechazarlo por su contenido

#### Scenario: Lo que rigió queda en el registro

- **WHEN** una sesión con modelo declarado arranca
- **THEN** el registro SHALL anotar el modelo y el esfuerzo efectivos

#### Scenario: La opción anunciada fija el modelo antes del primer prompt

- **WHERE** el agente anuncia una opción de categoría `model` y su entrada no declara la bandera
- **WHEN** la sesión declara un modelo que el agente anunció
- **THEN** SHALL fijarse por esa opción antes del primer prompt
- **AND** el registro SHALL anotar el valor que el agente confirmó

#### Scenario: Un valor no anunciado se rehúsa listando lo anunciado

- **WHEN** la sesión declara un modelo que el agente no anunció
- **THEN** SHALL rehusarse listando los valores anunciados
- **AND** NO SHALL enviarse prompt alguno

#### Scenario: Una palanca que el agente no admite se rehúsa

- **WHERE** la entrada del agente declara sus palancas y no nombra el esfuerzo
- **WHEN** se declara esfuerzo
- **THEN** SHALL rehusarse nombrando al agente y la palanca
- **AND** la sesión NO SHALL arrancar como si nada se hubiera pedido

#### Scenario: Sin bandera ni anuncio la palanca se rehúsa al abrir

- **WHERE** el agente no declara la palanca ni anuncia su categoría
- **WHEN** se declara esa palanca
- **THEN** SHALL rehusarse al abrir la sesión, nombrando al agente y la palanca
- **AND** NO SHALL enviarse prompt alguno
