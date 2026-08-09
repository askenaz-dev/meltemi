# acp-session — delta

## ADDED Requirements

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
