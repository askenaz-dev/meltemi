## ADDED Requirements

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
El daemon SHALL detectar cada entrada del catálogo resolviendo su binario en el
`PATH` del usuario y en las rutas candidatas declaradas por la entrada,
devolviendo la ruta absoluta cuando exista. En Windows la resolución MUST
considerar las extensiones ejecutables (`.exe`, `.cmd`, `.bat`). La detección
MUST NOT ejecutar el binario detectado ni ningún otro proceso.

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
`projectRoot` — si es el agente configurado de ese proyecto. Cada consulta MUST
reflejar el estado presente de la detección.

#### Scenario: Catálogo con configurado marcado
- **WHEN** un cliente invoca `fleet/list` con `projectRoot` de un proyecto cuya config selecciona un agente del catálogo
- **THEN** la respuesta SHALL marcar esa entrada como configurada
- **AND** SHALL incluir la versión del registro y el estado de detección de cada entrada

#### Scenario: La detección se refresca por consulta
- **WHEN** un binario aparece en el PATH después de una consulta previa
- **THEN** una nueva invocación de `fleet/list` SHALL reportarlo como detectado

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
