# own-adapters — delta

## ADDED Requirements

### Requirement: Hechos de la herramienta en la petición de permiso
La petición de permiso que un adaptador propio releva SHALL llevar los hechos
de la herramienta que las reglas y el aprobador leen: la entrada que la
herramienta recibió como `rawInput`, sin editar, y la ruta que nombra en
`locations`. Cuando el proveedor no repite esos hechos en su pregunta, el
adaptador SHALL tomarlos de la llamada que la sesión ya transmitió con el mismo
id. Una aprobación que concede algo distinto de la llamada —un acceso de red,
la entrada a un terminal que ya corre, una raíz de escritura para el resto de
la sesión— MUST NOT llevar el comando de la llamada, y un cambio que toca más
de un archivo MUST NOT presentar una sola ruta.

#### Scenario: La petición lleva la entrada y la ruta
- **WHEN** el CLI pilotado pide permiso para escribir un archivo o ejecutar un comando
- **THEN** la petición SHALL llevar su entrada como `rawInput` y la ruta en `locations`
- **AND** una regla por prefijo de comando o de ruta SHALL poder decidirla sin escalar al humano

#### Scenario: La aprobación de un comando lleva el comando del ítem
- **WHEN** el servidor pilotado pide aprobar un comando que anunció antes como ítem
- **THEN** la petición SHALL llevar el título y el comando de ese ítem

#### Scenario: Lo que concede otra cosa no hereda el comando
- **WHEN** la aprobación de un comando es en realidad un acceso de red o la entrada a un terminal que ya corre
- **THEN** la petición SHALL NOT llevar el comando del ítem

#### Scenario: Un cambio de varios archivos no presenta una sola ruta
- **WHEN** el servidor pilotado pide aprobar un cambio que toca más de un archivo
- **THEN** la petición SHALL NOT llevar ruta
- **AND** una regla por prefijo de ruta SHALL NOT poder decidirla
