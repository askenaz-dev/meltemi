# fleet-catalog — delta

## ADDED Requirements

### Requirement: El registro declara las palancas de lanzamiento
Una entrada del registro MAY declarar las palancas —modelo, esfuerzo— que su
agente toma por bandera de lanzamiento, y la declaración SHALL ser el conjunto
completo de esa entrada. Una entrada sin declaración SHALL remitir toda palanca
a lo que su agente anuncie. Ninguna decisión sobre palancas SHALL derivarse del
nombre del binario, y `fleet/list` SHALL devolver lo declarado por entrada.

#### Scenario: Una entrada declara sus palancas de lanzamiento
- **WHERE** una entrada declara el modelo como palanca de lanzamiento
- **WHEN** se consulta la flota
- **THEN** la entrada SHALL decir que toma el modelo por bandera

#### Scenario: Sin declaración la palanca es lo que el agente anuncia
- **WHERE** una entrada no declara palancas
- **WHEN** una sesión con ese agente declara un modelo
- **THEN** el modelo SHALL ir por la opción que el agente anuncie

#### Scenario: El nombre del binario no decide
- **WHERE** un agente sin entrada en el registro lleva un nombre de binario igual al de un agente con palancas declaradas
- **THEN** NO SHALL heredar esas palancas

### Requirement: La flota recuerda lo último que cada agente anunció
WHEN una sesión con un agente de la flota abre y el agente anuncia opciones de
configuración, el daemon SHALL recordar ese anuncio por agente, con su fecha, y
`fleet/list` SHALL devolverlo. El recuerdo SHALL servir para ofrecer, y NO SHALL
sustituir la validación contra lo que el agente anuncie al abrir.

#### Scenario: El anuncio se recuerda por agente
- **WHEN** un agente de la flota anuncia opciones al abrir una sesión
- **THEN** la flota SHALL devolver ese anuncio y su fecha para ese agente

#### Scenario: El recuerdo no decide
- **WHERE** la flota recuerda un anuncio que no traía el modelo pedido
- **WHEN** el agente lo anuncia al abrir una sesión nueva
- **THEN** SHALL fijarse el modelo pedido

### Requirement: Las palancas de cada agente se ven en toda superficie
El CLI, la TUI y la GUI SHALL mostrar por agente las palancas que toma por
bandera y lo último que anunció, con cuántos valores y cuándo, y NO SHALL
mostrar lo que no se sabe como si se supiera.

#### Scenario: El CLI dice las palancas de cada agente
- **WHEN** se lista la flota desde el CLI
- **THEN** cada agente SHALL decir su palanca por bandera o lo último que anunció

#### Scenario: La TUI dice las palancas de cada agente
- **WHEN** se abre la vista de la flota en la TUI
- **THEN** cada agente SHALL decir su palanca por bandera o lo último que anunció

## MODIFIED Requirements

### Requirement: El perfil declara su modelo y su esfuerzo

Un perfil de lanzamiento SHALL poder declarar modelo y esfuerzo por defecto, y
lo declarado SHALL aplicarse en todo verbo que arranque un agente con ese perfil
—la sesión libre, `propose`, los verbos de autoría, `dispatch` e `implement`—
por la misma vía y con el mismo registro que una palanca declarada en la sesión.
Lo declarado explícitamente en la sesión SHALL prevalecer sobre el default del
perfil, y un perfil que no declare nada NO SHALL imponer nada.

#### Scenario: La sesión pisa el default del perfil

- **WHERE** un perfil declara un modelo
- **WHEN** la sesión declara otro
- **THEN** SHALL regir el de la sesión

#### Scenario: Un perfil sin declaración no impone nada

- **WHERE** un perfil no declara modelo ni esfuerzo
- **THEN** la sesión SHALL correr como si el perfil no existiera en ese punto

#### Scenario: El modelo del perfil rige en todo verbo que arranca un agente

- **WHERE** un perfil declara un modelo
- **WHEN** `propose`, `dispatch` o `implement` arrancan un agente con ese perfil
- **THEN** el modelo SHALL llegar al agente por su vía
- **AND** SHALL quedar en el registro de la sesión
