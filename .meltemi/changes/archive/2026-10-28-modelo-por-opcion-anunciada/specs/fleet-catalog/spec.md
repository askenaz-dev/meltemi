# fleet-catalog — delta

## ADDED Requirements

### Requirement: El registro declara las palancas de lanzamiento
Una entrada del registro MAY declarar las palancas —modelo, esfuerzo— que su
agente toma por bandera de lanzamiento, y la declaración SHALL ser el conjunto
completo de esa entrada. Una entrada sin declaración SHALL remitir toda palanca
a lo que su agente anuncie. Ninguna decisión sobre palancas SHALL derivarse del
nombre del binario, y un comando literal NO SHALL heredar la declaración de
entrada alguna. Un nombre de palanca desconocido o repetido SHALL invalidar el
documento del registro que lo trae.

#### Scenario: Una entrada declara sus palancas de lanzamiento
- **WHERE** una entrada declara el modelo como palanca de lanzamiento
- **WHEN** una sesión con ese agente declara un modelo
- **THEN** el modelo SHALL viajar por la bandera de lanzamiento

#### Scenario: Sin declaración la palanca es lo que el agente anuncia
- **WHERE** una entrada no declara palancas
- **WHEN** una sesión con ese agente declara un modelo
- **THEN** el modelo SHALL ir por la opción que el agente anuncie

#### Scenario: El nombre del binario no decide
- **WHERE** un comando literal lleva un nombre de binario igual al de un agente con palancas declaradas
- **THEN** NO SHALL heredar esas palancas

## MODIFIED Requirements

### Requirement: El perfil declara su modelo y su esfuerzo

Un perfil de lanzamiento SHALL poder declarar modelo y esfuerzo por defecto, y
lo declarado SHALL aplicarse en todo verbo que arranque un agente con ese perfil
—la sesión libre, `propose`, los verbos de autoría, `dispatch` e `implement`—
por la misma vía, con la misma confirmación y el mismo registro que una palanca
declarada en la sesión. Lo declarado explícitamente en la sesión SHALL prevalecer
sobre el default del perfil, y un perfil que no declare nada NO SHALL imponer
nada.

#### Scenario: La sesión pisa el default del perfil

- **WHERE** un perfil declara un modelo
- **WHEN** la sesión declara otro
- **THEN** SHALL regir el de la sesión

#### Scenario: Un perfil sin declaración no impone nada

- **WHERE** un perfil no declara modelo ni esfuerzo
- **THEN** la sesión SHALL correr como si el perfil no existiera en ese punto

#### Scenario: El modelo del perfil rige en todo verbo que arranca un agente

- **WHERE** un perfil declara un modelo
- **WHEN** `propose`, un verbo de autoría, `dispatch` o `implement` arrancan un agente con ese perfil
- **THEN** el modelo SHALL llegar al agente por su vía
- **AND** SHALL quedar en el registro de la sesión
