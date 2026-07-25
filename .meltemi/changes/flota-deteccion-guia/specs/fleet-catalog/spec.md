## ADDED Requirements

### Requirement: Detección en dos capas de las entradas con adaptador
WHERE una entrada del catálogo se pilota a través de un adaptador ACP, el
registro SHALL declarar por separado la capa del CLI oficial del proveedor y la
capa del adaptador, y la detección SHALL resolver cada capa de forma
independiente, componiendo un estado único y honesto por entrada: punto de
pilotaje presente (con el CLI oficial cuando la entrada declara ambas capas),
falta el adaptador, falta el CLI oficial, ninguna capa presente, o instalado sin
objetivo ejecutable. El estado de detección de la entrada MUST seguir
significando "el daemon puede pilotar este punto de entrada": una entrada cuyo
CLI oficial existe pero cuyo adaptador falta MUST reportarse como no detectada
con su estado compuesto, y MUST NOT presentarse como pilotable. Las entradas de
una sola capa SHALL conservar su estado y su semántica actuales.

#### Scenario: CLI oficial presente sin adaptador
- **WHEN** una entrada de dos capas tiene su CLI oficial instalado y su adaptador ausente
- **THEN** el catálogo SHALL reportar la capa del CLI como detectada y la del adaptador como ausente
- **AND** el estado compuesto SHALL declarar que falta el adaptador
- **AND** la entrada SHALL seguir reportándose como no detectada para el pilotaje

#### Scenario: Adaptador presente sin CLI oficial
- **WHEN** una entrada de dos capas tiene su adaptador instalado y su CLI oficial ausente
- **THEN** la entrada SHALL reportarse como detectada para el pilotaje
- **AND** el estado compuesto SHALL declarar que el CLI oficial no fue encontrado

#### Scenario: Ambas capas presentes
- **WHEN** una entrada de dos capas tiene su CLI oficial y su adaptador instalados
- **THEN** ambas capas SHALL reportarse detectadas con su ruta absoluta
- **AND** el estado compuesto SHALL declarar la entrada lista para pilotar

#### Scenario: Ninguna capa presente
- **WHEN** ninguna de las capas declaradas por una entrada se encuentra en el sistema
- **THEN** el estado compuesto SHALL declarar que no hay instalación detectada
- **AND** la consulta SHALL completarse con éxito para el resto del catálogo

#### Scenario: Entrada de una sola capa conserva su estado
- **WHEN** una entrada declara un único punto de entrada sin adaptador
- **THEN** el catálogo SHALL reportar esa única capa
- **AND** su estado de detección SHALL coincidir con el comportamiento previo a las dos capas

### Requirement: Remedio por capa accionable en todas las superficies
El catálogo SHALL acompañar todo estado incompleto con un remedio accionable que
nombre la capa que falta y el comando exacto de instalación declarado en el
registro, y toda superficie —CLI en modo humano y `--json`, vista de la TUI y
detalle de la superficie de escritorio— SHALL presentarlo por igual. Meltemi
MUST NOT ejecutar ese comando ni ningún instalador: el remedio es información,
no un efecto externo. WHEN el lanzamiento de un id se rehúsa porque su capa de
pilotaje no está detectada, el diagnóstico MUST nombrar la capa ausente y su
comando en el remedio del error.

#### Scenario: Remedio con el comando exacto por capa
- **WHEN** una entrada reporta un estado con una capa faltante
- **THEN** el catálogo SHALL incluir el remedio con la capa que falta y su comando de instalación
- **AND** cada superficie SHALL mostrarlo junto al estado de esa entrada

#### Scenario: Meltemi no ejecuta el remedio
- **WHILE** una superficie muestra el comando de instalación de una capa faltante
- **THEN** el daemon SHALL NOT lanzar ningún proceso de instalación
- **AND** la acción ofrecida al usuario SHALL limitarse a copiar o leer el comando

#### Scenario: Rehúso de lanzamiento nombra la capa que falta
- **IF** se solicita una sesión con un id cuya capa de pilotaje no está detectada
- **THEN** el rehúso SHALL nombrar la capa ausente y su comando de instalación en el remedio
- **AND** SHALL NOT lanzar ningún subproceso

### Requirement: Estatus legal de la ruta de integración sin maquillaje
WHERE el registro declara para una entrada el estatus legal de su ruta de
integración y una nota del proveedor, el catálogo SHALL exponerlos y las
superficies SHALL mostrarlos tal cual junto al remedio, sin suavizarlos ni
ocultarlos. IF la nota advierte que la ruta ofrecida está en zona gris para las
suscripciones de consumo, la superficie MUST mostrar también el camino seguro
declarado, de modo que el usuario decida informado antes de instalar nada.

#### Scenario: Nota legal declarada mostrada tal cual
- **WHEN** una entrada declara estatus legal y nota en el registro
- **THEN** el catálogo SHALL incluir ambos campos
- **AND** las superficies SHALL mostrarlos sin reescribirlos ni abreviarlos

#### Scenario: Camino seguro señalado junto a la zona gris
- **WHERE** la nota de una entrada advierte una zona gris de su ruta de integración
- **THEN** la superficie SHALL mostrar la advertencia junto al remedio de esa capa
- **AND** SHALL indicar el camino seguro declarado

## MODIFIED Requirements

### Requirement: Detección local pasiva de binarios
El daemon SHALL detectar cada entrada del catálogo resolviendo los binarios de
sus capas declaradas en el `PATH` del usuario y en las rutas candidatas
declaradas por la entrada, devolviendo la ruta absoluta cuando exista. En
Windows la resolución MUST considerar las extensiones ejecutables (`.exe`,
`.cmd`, `.bat`) como objetivos de lanzamiento y MUST considerar además los shims
de script (`.ps1`) como evidencia de instalación; un hallazgo que solo exista
como evidencia MUST marcarse como tal y MUST NOT devolverse como objetivo de
lanzamiento. La detección MUST NOT ejecutar el binario detectado ni ningún otro
proceso.

#### Scenario: Agente presente
- **WHEN** el binario de una entrada existe en el PATH
- **THEN** la entrada SHALL reportarse como detectada con su ruta absoluta

#### Scenario: Agente ausente sin error
- **WHEN** el binario de una entrada no se encuentra
- **THEN** la entrada SHALL reportarse como no detectada
- **AND** la consulta SHALL completarse con éxito para el resto del catálogo

#### Scenario: Detección sin efectos laterales
- **WHILE** se resuelve la detección de todo el catálogo
- **THEN** el daemon SHALL NOT lanzar ningún subproceso

#### Scenario: Shim de script cuenta como evidencia sin ser objetivo de lanzamiento
- **WHEN** en Windows la única presencia de un binario es un shim de script
- **THEN** la capa SHALL reportarse instalada y marcada como solo evidencia
- **AND** el estado compuesto SHALL declarar que no hay objetivo ejecutable con su remedio
- **AND** esa ruta SHALL NOT ofrecerse como binario a lanzar

### Requirement: Consulta fleet/list
El daemon SHALL exponer el método `fleet/list` que devuelve la versión del
registro y, por agente: id, nombre, origen, nivel de integración declarado,
estado de detección, ruta si fue detectado y — cuando la petición incluye
`projectRoot` — si es el agente configurado de ese proyecto. La respuesta SHALL
incluir además, de forma aditiva, las capas declaradas de la entrada con su
detección y su comando de instalación, el estado compuesto, el remedio con su
comando cuando el estado sea incompleto, y el estatus y la nota legal cuando la
entrada los declare. Los campos vigentes MUST conservar su significado, de modo
que un cliente previo siga interpretando la respuesta sin cambios. Cada consulta
MUST reflejar el estado presente de la detección.

#### Scenario: Catálogo con configurado marcado
- **WHEN** un cliente invoca `fleet/list` con `projectRoot` de un proyecto cuya config selecciona un agente del catálogo
- **THEN** la respuesta SHALL marcar esa entrada como configurada
- **AND** SHALL incluir la versión del registro y el estado de detección de cada entrada

#### Scenario: La detección se refresca por consulta
- **WHEN** un binario aparece en el PATH después de una consulta previa
- **THEN** una nueva invocación de `fleet/list` SHALL reportarlo como detectado

#### Scenario: Capas de detección reportadas por entrada
- **WHEN** un cliente consulta el catálogo
- **THEN** cada entrada SHALL enumerar sus capas declaradas con su binario, su detección y su ruta cuando exista
- **AND** SHALL declarar su estado compuesto

#### Scenario: Campos aditivos sin romper el contrato vigente
- **WHEN** un cliente que ignora los campos nuevos consume la respuesta
- **THEN** los campos vigentes SHALL conservar su significado y su presencia
- **AND** las entradas sin estatus legal declarado SHALL omitir esos campos en lugar de inventarlos
