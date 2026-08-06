# fleet-catalog Specification

## Purpose
TBD - created by archiving change catalogo-flota. Update Purpose after archive.
## Requirements

### Requirement: Catálogo desde instantánea empaquetada del registro
El daemon SHALL poblar el catálogo de flota desde una instantánea versionada del
registro público ACP embebida en el binario, y MUST NOT realizar peticiones de
red para poblarlo ni refrescarlo. La instantánea MUST ser sustituible por un
archivo local mediante override explícito (`MELTEMI_FLEET_REGISTRY` o clave de
configuración) para pruebas y usuarios avanzados.

#### Scenario: Listado sin red
- **WHEN** un cliente consulta el catálogo
- **THEN** el daemon SHALL responder desde la instantánea embebida y la detección local
- **AND** SHALL NOT abrir conexión de red alguna

#### Scenario: Registro sustituido para pruebas
- **WHERE** el override de registro apunta a un archivo local válido
- **THEN** el catálogo SHALL poblarse desde ese archivo en lugar de la instantánea embebida
- **AND** la versión reportada SHALL reflejar el registro sustituido

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

### Requirement: Agentes personalizados del usuario
La configuración SHALL permitir declarar agentes fuera del registro (id propio,
nombre y comando de lanzamiento), que MUST aparecer en el catálogo con origen
`custom` y participar de la detección y la selección como cualquier entrada.

#### Scenario: Agente custom listado
- **WHEN** la config del proyecto o del usuario declara un agente personalizado
- **THEN** el catálogo SHALL incluirlo con origen `custom`
- **AND** su binario SHALL someterse a la misma detección pasiva

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

### Requirement: Selección de agente por id de catálogo
La configuración SHALL aceptar `[agent] id` como alternativa a `[agent] command`
para seleccionar un agente del catálogo; al abrir sesión, el daemon resuelve el
id a su binario detectado más los argumentos ACP de la entrada. La precedencia
MUST ser: override de entorno, luego `command` literal, luego `id`. Si el id no
existe en el catálogo o su binario no está detectado, el daemon MUST responder el
error de aplicación 2001 `agent_not_detected` con un remedio accionable y MUST
NOT lanzar ningún proceso.

#### Scenario: Sesión por id detectado
- **WHEN** la config del proyecto selecciona por id un agente detectado y se solicita una sesión
- **THEN** el daemon SHALL lanzar el binario detectado con los argumentos ACP de su entrada

#### Scenario: Id no detectado
- **IF** el id configurado no está en el catálogo o su binario no fue detectado
- **THEN** el daemon SHALL responder 2001 `agent_not_detected` con remedio
- **AND** SHALL NOT lanzar ningún subproceso

#### Scenario: Compatibilidad del comando literal
- **WHERE** la config declara `command` literal (con o sin `id`)
- **THEN** el daemon SHALL usar el `command` literal, preservando el comportamiento previo

### Requirement: Vista Flota poblada
La vista Flota de la TUI SHALL materializar el catálogo con la línea base de
accesibilidad: estado de detección como glifo más palabra, nivel de integración
como etiqueta textual, y marcador del agente configurado del proyecto. Con cero
agentes detectados, la vista MUST mostrar igualmente las entradas del registro
con su estado y conservar la pista de remediación BYO-agent.

#### Scenario: Tabla con detectados y no detectados
- **WHEN** el usuario abre la vista Flota con el daemon accesible
- **THEN** la vista SHALL listar cada entrada con glifo+palabra de detección y su nivel
- **AND** SHALL marcar el agente configurado del proyecto cuando exista

#### Scenario: Cero detectados sigue enseñando el camino
- **WHEN** ninguna entrada del catálogo está detectada
- **THEN** la vista SHALL mostrar las entradas como no detectadas junto a la pista BYO-agent
- **AND** SHALL NOT presentar una pantalla muda

### Requirement: Subcomando fleet de la CLI
El subcomando `fleet` SHALL listar el catálogo en modo scriptable: presentación
humana legible y, bajo `--json`, exactamente un objeto JSON con la respuesta de
`fleet/list`. Como subcomando respaldado por RPC, MUST enviar `initialize`
primero y mapear sus desenlaces a la taxonomía de códigos de salida vigente.

#### Scenario: Listado humano
- **WHEN** se invoca `meltemi fleet` con el daemon accesible
- **THEN** el binario SHALL presentar el catálogo con detección y nivel por agente
- **AND** SHALL terminar con el código `0`

#### Scenario: Listado para máquinas
- **WHEN** se invoca `meltemi fleet --json`
- **THEN** el binario SHALL emitir exactamente un objeto JSON con el catálogo en stdout

### Requirement: Resolución de agente por sesión desde la flota
El daemon SHALL resolver el agente de cada sesión que nombra uno a partir de la
flota, en este orden: perfil de lanzamiento, id del catálogo (registro o
declarado por el usuario) y, en su defecto, el agente configurado del proyecto;
el binario efectivo y la fuente de la resolución MUST registrarse en el log de
sesión, de modo que jamás sea ambiguo qué binario corrió. Un id resuelto cuyo
binario no está detectado MUST rehusarse con diagnóstico y remedio, nunca
degradar en silencio a otro proveedor.

#### Scenario: Sesión lanza el binario de su id de catálogo
- **WHEN** una sesión se lanza nombrando un id del catálogo detectado en el sistema
- **THEN** el daemon SHALL lanzar el binario de ese id
- **AND** el log de sesión SHALL registrar el binario efectivo y la fuente de resolución

#### Scenario: Etiqueta libre cae al agente configurado con registro
- **WHEN** el nombre no corresponde a ningún perfil ni id del catálogo
- **THEN** la sesión SHALL usar el agente configurado del proyecto
- **AND** la resolución con fuente de fallback SHALL constar en el log de sesión

#### Scenario: Id no detectado rehúsa sin degradar
- **IF** el nombre resuelve a un id del catálogo cuyo binario no está detectado
- **THEN** el lanzamiento SHALL rehusarse con diagnóstico y remedio
- **AND** ningún otro proveedor SHALL lanzarse en su lugar

### Requirement: Perfiles de lanzamiento ciegos a credenciales
La configuración SHALL admitir perfiles de lanzamiento (`[[fleet.profile]]`) con
nombre, agente del catálogo y una sobrecapa de entorno que selecciona el
contexto de autenticación del binario oficial; los valores SHALL admitir
referencias `${VAR}` resueltas al lanzar y el lint de higiene MUST rehusar
valores que parezcan secretos en claro. Meltemi MUST NOT leer, almacenar ni
reenviar material secreto de los agentes: el binario se autentica solo dentro
del contexto seleccionado, y un fallo de autenticación se muestra tal cual.

#### Scenario: Perfil lanza el mismo binario con otro contexto de autenticación
- **WHEN** una sesión se lanza nombrando un perfil
- **THEN** el daemon SHALL lanzar el binario del agente subyacente con la sobrecapa de entorno aplicada
- **AND** el material de autenticación SHALL permanecer gestionado únicamente por el binario

#### Scenario: Secreto en claro rehusado por higiene
- **WHEN** un valor de entorno de un perfil parece un secreto en claro
- **THEN** la configuración del perfil SHALL rehusarse con diagnóstico
- **AND** el remedio SHALL indicar la referencia `${VAR}` resuelta al lanzar

### Requirement: Perfiles visibles en el catálogo
El listado de la flota SHALL incluir los perfiles de lanzamiento declarados, con
fuente propia, su agente subyacente y la detección del binario subyacente, en
todas las superficies por igual.

#### Scenario: fleet/list incluye los perfiles
- **WHEN** un cliente consulta el catálogo de la flota
- **THEN** cada perfil declarado SHALL aparecer con su fuente, su agente subyacente y su detección

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

### Requirement: Guía de perfiles multi-suscripción
La guía de agentes SHALL documentar los perfiles de lanzamiento como suscripciones
con nombre, con el ejemplo canónico de dos cuentas del mismo proveedor conviviendo
en un proyecto mediante la redirección del contexto de autenticación del binario
oficial, y SHALL enseñar la referencia `${VAR}` como única vía para valores
sensibles. La guía MUST NOT incluir credencial alguna ni instruir a pegarla en la
configuración de Meltemi (constitución §2).

#### Scenario: Ejemplo canónico de dos cuentas del mismo agente
- **WHEN** el lector busca cómo usar dos suscripciones del mismo proveedor
- **THEN** la guía SHALL mostrar dos perfiles nombrados sobre el mismo agente del catálogo
- **AND** SHALL explicar que cada uno solo selecciona el contexto donde el binario se autentica

#### Scenario: La guía no pide credenciales
- **WHEN** la guía documenta la sobrecapa de entorno de un perfil
- **THEN** SHALL usar referencias `${VAR}` resueltas al lanzar
- **AND** SHALL NOT mostrar ni pedir material secreto en la configuración

### Requirement: Vigencia de las rutas de instalación de la instantánea
Cada comando de instalación que la instantánea del registro declare SHALL
nombrar la distribución canónica vigente del proyecto upstream, verificada
contra su fuente de distribución en la fecha de la revisión — nunca citada de
memoria. WHEN una distribución declarada queda archivada, deprecada o
renombrada por su upstream, la siguiente revisión de la instantánea MUST
reemplazarla por su sucesora, actualizando el campo `version`, y MUST NOT
seguir remitiendo a la ruta muerta; la guía de agentes SHALL actualizarse en
el mismo cambio, forzada por la verificación de coherencia registro↔guía
vigente.

#### Scenario: Comando de instalación verificado contra la distribución vigente
- **WHEN** se revisa la instantánea del registro
- **THEN** cada comando de instalación declarado SHALL corresponder a una distribución publicada y vigente
- **AND** la verificación (fuente y fecha) SHALL quedar documentada en la change que revisa la instantánea

#### Scenario: Distribución archivada reemplazada por su sucesora
- **IF** una distribución declarada fue archivada, deprecada o renombrada por su upstream
- **THEN** la instantánea SHALL apuntar a la distribución sucesora con el campo `version` actualizado
- **AND** la guía de agentes SHALL reflejar el mismo comando en el mismo cambio

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
