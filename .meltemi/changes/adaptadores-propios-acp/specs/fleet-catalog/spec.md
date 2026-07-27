# fleet-catalog — delta

## ADDED Requirements

### Requirement: Detección de capa empaquetada junto al daemon
WHERE una capa de una entrada del registro se declara empaquetada
(`bundled`), la detección SHALL sondear, además del `PATH` y de las rutas
candidatas declaradas, el directorio del ejecutable del daemon en
ejecución, y SHALL reportar la fuente del hallazgo junto a la ruta
absoluta. La precedencia MUST ser: `PATH`, rutas candidatas declaradas,
directorio hermano del daemon; el binario efectivo al lanzar SHALL constar
en el log de sesión como toda resolución vigente. El mecanismo MUST ser
genérico del registro — aplicable a cualquier entrada que declare la capa
empaquetada — y MUST NOT depender del id de ninguna entrada concreta.

#### Scenario: Capa empaquetada detectada junto al daemon
- **WHEN** una capa declarada empaquetada no está en el PATH pero su binario existe junto al daemon en ejecución
- **THEN** la capa SHALL reportarse detectada con esa ruta absoluta
- **AND** la fuente del hallazgo SHALL indicar la procedencia empaquetada

#### Scenario: El PATH conserva la precedencia sobre el empaquetado
- **WHEN** el binario de una capa empaquetada existe en el PATH y también junto al daemon
- **THEN** la detección SHALL reportar la ruta del PATH como binario a lanzar
- **AND** el log de sesión SHALL registrar el binario efectivo al lanzar

#### Scenario: Mecanismo genérico sin casos por id
- **WHEN** un registro sustituido para pruebas declara la capa empaquetada en una entrada cualquiera
- **THEN** la detección SHALL aplicarle el mismo sondeo del directorio hermano
- **AND** ninguna lógica SHALL condicionarse al id de la entrada

### Requirement: Adaptador propio como punto de pilotaje por defecto
WHERE existe un adaptador propio de Meltemi para una entrada pilotada por
adaptador, el registro SHALL declararlo como la capa de pilotaje de esa
entrada, con distribución empaquetada, y MUST NOT declarar comando de
instalación de terceros para esa capa. La vía de un adaptador de terceros
SHALL permanecer disponible por configuración del usuario (entrada
personalizada o comando literal), pilotada con el mismo trato que
cualquier otra entrada; es el usuario quien la declara, no el registro
quien la recomienda.

#### Scenario: Entrada lista con solo Meltemi y el CLI oficial
- **WHEN** el CLI oficial de una entrada con adaptador propio está instalado y Meltemi fue instalado con sus binarios empaquetados
- **THEN** la capa adaptador SHALL reportarse detectada desde el empaquetado
- **AND** el estado compuesto SHALL declarar la entrada lista para pilotar sin instalar nada más

#### Scenario: Adaptador de terceros por configuración sigue pilotable
- **WHEN** la configuración del usuario declara una entrada personalizada cuyo comando es un adaptador de terceros
- **THEN** el daemon SHALL pilotarla como cualquier entrada personalizada
- **AND** ninguna lógica SHALL tratarla distinto por no usar el adaptador propio

#### Scenario: La capa propia no ofrece instalación de terceros
- **WHEN** un cliente consulta el catálogo
- **THEN** la capa adaptador de una entrada con adaptador propio SHALL declararse empaquetada
- **AND** SHALL NOT ofrecer comando de instalación de terceros para esa capa

## MODIFIED Requirements

### Requirement: Remedio por capa accionable en todas las superficies
El catálogo SHALL acompañar todo estado incompleto con un remedio
accionable que nombre la capa que falta y el comando exacto de instalación
declarado en el registro, y toda superficie —CLI en modo humano y `--json`,
vista de la TUI y detalle de la superficie de escritorio— SHALL presentarlo
por igual. WHERE la capa faltante está declarada como empaquetada con
Meltemi, el remedio MUST decir que la capa viaja en los instaladores de
Meltemi y remitir a reinstalar o reparar la instalación, y MUST NOT ofrecer
un comando de instalación de terceros para ella. Meltemi MUST NOT ejecutar
ese comando ni ningún instalador: el remedio es información, no un efecto
externo. WHEN el lanzamiento de un id se rehúsa porque su capa de pilotaje
no está detectada, el diagnóstico MUST nombrar la capa ausente y su remedio
—comando de instalación o reinstalación de Meltemi, según la capa— en el
remedio del error.

#### Scenario: Remedio con el comando exacto por capa
- **WHEN** una entrada reporta un estado con una capa instalable faltante
- **THEN** el catálogo SHALL incluir el remedio con la capa que falta y su comando de instalación
- **AND** cada superficie SHALL mostrarlo junto al estado de esa entrada

#### Scenario: Capa empaquetada ausente remite a la instalación de Meltemi
- **WHEN** una entrada reporta ausente una capa declarada empaquetada
- **THEN** el remedio SHALL decir que esa capa viaja en los instaladores de Meltemi y remitir a reinstalar o reparar
- **AND** SHALL NOT ofrecer un comando de instalación de terceros para esa capa

#### Scenario: Meltemi no ejecuta el remedio
- **WHILE** una superficie muestra el remedio de una capa faltante
- **THEN** el daemon SHALL NOT lanzar ningún proceso de instalación
- **AND** la acción ofrecida al usuario SHALL limitarse a copiar o leer el remedio

#### Scenario: Rehúso de lanzamiento nombra la capa que falta
- **IF** se solicita una sesión con un id cuya capa de pilotaje no está detectada
- **THEN** el rehúso SHALL nombrar la capa ausente y su remedio según el tipo de capa
- **AND** SHALL NOT lanzar ningún subproceso
