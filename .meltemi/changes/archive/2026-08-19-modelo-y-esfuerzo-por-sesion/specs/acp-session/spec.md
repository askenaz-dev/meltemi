# acp-session — delta

## ADDED Requirements

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
