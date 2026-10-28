

### Requirement: Helper de túnel auditable
El cliente SHALL ofrecer un helper de túnel (`tunnel`) que compone la invocación
del `ssh` del propio usuario para llevar el endpoint local del daemon al otro
extremo: SHALL imprimir el comando exacto por plataforma, el snippet de
configuración SSH equivalente y el valor de endpoint que el extremo remoto debe
usar, y MAY ejecutar ese `ssh` de forma visible cuando el usuario lo pida
explícitamente. El helper MUST NOT abrir sockets propios, MUST NOT introducir
transporte de red en el daemon y MUST NOT generar, leer ni almacenar material de
claves del usuario.

#### Scenario: Composición del túnel por plataforma
- **WHEN** el usuario invoca el helper de túnel hacia un host remoto
- **THEN** el helper SHALL imprimir el comando `ssh` exacto de reenvío del endpoint local de esta plataforma
- **AND** SHALL incluir el snippet de configuración y el endpoint que el extremo remoto debe usar

#### Scenario: Ejecución visible solo a pedido
- **WHEN** el usuario pide ejecutar el túnel
- **THEN** el helper SHALL lanzar el `ssh` del usuario como proceso visible
- **AND** SHALL NOT dejar túneles en segundo plano sin que el usuario lo haya pedido

#### Scenario: Plataforma sin reenvío posible rehúsa honesta
- **IF** la plataforma del daemon no admite el reenvío estándar de su endpoint
- **THEN** el helper SHALL rehusarse con diagnóstico y remedio
- **AND** SHALL NOT imprimir un comando que no puede funcionar

### Requirement: Frontera honesta del acceso remoto
La documentación de acceso remoto SHALL declarar la frontera vigente: todas
las capacidades del daemon operan en remoto únicamente a través de un túnel
vivo del usuario hacia el socket local, y el daemon NO SHALL emitir tráfico de
red jamás. Sin conexión establecida no hay control alguno. Como única
apertura, el usuario MAY configurar un **aviso de espera** opt-in: cuando algo
queda esperando su decisión, un cliente conectado o un proceso del propio
usuario — nunca el daemon — MAY emitir un aviso mínimo a un endpoint que el
usuario opera. El aviso SHALL estar desactivado por defecto, su contenido
exacto SHALL estar especificado públicamente antes de existir (constitución
§9) y SHALL NOT transportar contenido del proyecto ni de la petición: solo el
hecho de que una decisión espera. Toda propuesta de relé en la nube de
terceros, puerto de red del daemon o push fuera de esta puerta MUST rechazarse
salvo enmienda fundacional previa.

#### Scenario: La frontera está documentada como postura
- **WHEN** un usuario consulta la documentación de acceso remoto
- **THEN** SHALL encontrar declarado que el control remoto exige túnel vivo
- **AND** la política del aviso de espera SHALL estar explicada como postura de privacidad, con su porqué

#### Scenario: Aviso de espera desactivado por defecto
- **WHEN** el usuario no ha configurado endpoint alguno de aviso
- **THEN** ningún componente de Meltemi emite nada hacia red alguna

#### Scenario: El aviso mínimo no transporta el proyecto
- **WHEN** el aviso de espera configurado se emite
- **THEN** su contenido se limita al hecho de que una decisión espera
- **AND** SHALL NOT incluir contenido del proyecto, de la petición ni código

#### Scenario: El daemon jamás emite el aviso
- **WHEN** una propuesta de cambio sitúa la emisión del aviso en el daemon
- **THEN** la propuesta se rechaza (constitución §3: el daemon no abre transporte de red)

#### Scenario: Propuesta de push fuera de la puerta se rechaza
- **WHEN** una propuesta de cambio introduce notificaciones por relé de terceros o control sin túnel establecido
- **THEN** la propuesta SHALL rechazarse salvo enmienda fundacional aprobada

### Requirement: Puente stdio del último metro

El cliente SHALL ofrecer un verbo de puente (`bridge`) que conecta el endpoint
local del daemon de esta máquina con su propia entrada y salida estándar, en
ambas direcciones, hasta que un extremo cierre. El puente SHALL operar sin
terminal interactiva, MUST NOT abrir sockets propios, MUST NOT introducir
transporte de red en el daemon y MUST NOT generar, leer ni almacenar material
de claves. WHERE el daemon no esté accesible, el puente SHALL rehusarse de
inmediato con diagnóstico y remedio, y NO SHALL quedarse esperando. El puente
SHALL funcionar en las tres plataformas, incluido el endpoint de named pipe
que el reenvío estándar no alcanza.

#### Scenario: Un canal remoto completo sobre el puente

- **WHEN** un proceso habla JSON-RPC por el stdio del puente
- **THEN** cada petición SHALL llegar al daemon local y cada respuesta SHALL
  volver por el mismo stdio
- **AND** el daemon NO SHALL distinguir esa conexión de una local

#### Scenario: El puente en la plataforma sin reenvío estándar

- **WHEN** el puente corre en la plataforma cuyo endpoint es un named pipe
- **THEN** SHALL conectar con ese endpoint igual que en las demás plataformas
- **AND** el remedio del rehúso del helper de túnel SHALL nombrar el puente
  como el camino que sí funciona

#### Scenario: Sin daemon, el puente rehúsa sin colgarse

- **IF** el endpoint local no está accesible
- **THEN** el puente SHALL terminar de inmediato con diagnóstico y remedio
- **AND** NO SHALL quedarse esperando a que aparezca

#### Scenario: El cierre de un extremo cierra el puente

- **WHEN** la entrada estándar del puente se cierra, o el endpoint cierra
- **THEN** el puente SHALL terminar ordenadamente
- **AND** NO SHALL dejar procesos ni sockets residuales

### Requirement: El punto de encuentro en dos vías está documentado

La documentación de acceso remoto SHALL declarar el patrón del punto de
encuentro: ambos extremos marcan conexiones salientes hacia infraestructura
del propio usuario, de modo que el acceso funciona con cualquiera de los dos
extremos fuera de su red habitual. SHALL documentar la variante de bastión SSH
(con el túnel inverso permanente y sus precauciones de cuenta dedicada) y la
variante de red privada del usuario (malla con plano de control
autohospedado), incluyendo el estado de licencias de las piezas nombradas. La
documentación SHALL declarar la frontera: esa infraestructura es del usuario —
Meltemi MUST NOT empaquetarla, MUST NOT depender de ella para compilar o
testear, y el transporte final hacia el daemon sigue siendo el SSH del propio
usuario hacia el socket local. En la variante de malla, la identidad del
usuario y la lista de sus equipos SHALL documentarse como capacidad presente
que Meltemi lee del cliente oficial de la malla, sin cuentas propias. El
selector de máquinas SHALL documentarse como capacidad presente del cliente,
sin que el daemon gane nada. La única decisión que sigue perteneciendo a la
superficie móvil de fase 3 —la identidad por certificados SSH de la variante de
bastión— SHALL quedar anotada como nota de design para esa change, no como
capacidad presente.

#### Scenario: Los cuatro cuadrantes usan el mismo camino

- **WHEN** un usuario consulta la documentación del patrón
- **THEN** SHALL encontrar que el PC y el cliente remoto marcan hacia afuera
- **AND** que el camino es el mismo con cualquiera de los dos fuera de casa

#### Scenario: La malla del usuario no es una dependencia de Meltemi

- **WHEN** una propuesta de cambio añade al workspace una dependencia de la
  malla, del bastión, de un plano de control o de un IdP
- **THEN** SHALL rechazarse
- **AND** la documentación SHALL seguir declarando que esa infraestructura es
  del usuario y que compilar y testear Meltemi no exige cuenta ni red

#### Scenario: Lo de fase 3 está anotado y no prometido

- **WHEN** se lee la nota de fase 3
- **THEN** SHALL decir que lo anotado no existe todavía
- **AND** NO SHALL presentar como presente lo que es una decisión de design
- **AND** la identidad y la lista de equipos de la variante de malla SHALL
  estar descritas como capacidad presente y leída, no creada, por Meltemi

### Requirement: La identidad es la que la malla del usuario ya conoce

El daemon SHALL poder responder la identidad que el cliente oficial de la
malla del usuario conoce —nombre de login, servidor de login o nombre de la
malla, y el nombre de este equipo— invocando ese cliente en modo de solo
lectura y parseando su salida. El daemon MUST NOT ejecutar ningún login ni
ningún comando que cambie el estado de la malla, MUST NOT abrir conexiones de
red por sí mismo, y MUST NOT leer archivos de estado ni claves privadas del
cliente; de la salida del cliente NO SHALL conservar ni exponer más que los
campos del modelo —sin direcciones ni claves—. WHERE no hay cliente de malla
configurado, no se encuentra, o el equipo no ha iniciado sesión en la malla, la
consulta SHALL rehusarse con un diagnóstico que distinga los tres casos y con
su remedio, sin degradar nada más. La identidad NO SHALL ser requisito para
ningún uso local, y el daemon NO SHALL etiquetar ni distinguir conexiones por
identidad.

#### Scenario: Quién soy, según la malla

- **WHEN** hay un cliente de malla configurado y este equipo está enlazado
- **THEN** la consulta SHALL responder el nombre de login, el servidor o
  nombre de la malla y el nombre de este equipo
- **AND** todo ello SHALL provenir de la salida del cliente oficial, no de
  ningún dato guardado por Meltemi

#### Scenario: Sin cliente de malla, rehúso con remedio

- **WHERE** no hay tabla `[identity]` o el binario no se encuentra
- **THEN** la consulta SHALL rehusarse con diagnóstico y el remedio de
  configurar o instalar el cliente
- **AND** ninguna otra capacidad del daemon SHALL verse afectada

#### Scenario: La identidad nunca es requisito

- **WHEN** un cliente inicia una conexión con el daemon
- **THEN** el saludo NO SHALL exigir ni transportar identidad
- **AND** el daemon NO SHALL distinguir esa conexión de otra por identidad

#### Scenario: Ningún token entra ni sale del daemon

- **WHEN** se lee la identidad de la malla
- **THEN** el daemon SHALL invocar solo un comando de lectura del cliente
- **AND** NO SHALL leer archivos de estado ni claves privadas del cliente, ni
  conservar de su salida nada fuera de los campos del modelo

#### Scenario: Cliente presente, equipo sin enlazar

- **WHEN** el cliente responde que este equipo no tiene sesión en la malla
- **THEN** la consulta SHALL responder sin identidad, con una razón que la
  distinga del cliente ausente
- **AND** SHALL ofrecer el gesto de vínculo como remedio

### Requirement: El vínculo de este equipo se compone y nunca se ejecuta

El daemon SHALL componer, como texto, el gesto que enlaza este equipo a la
malla del usuario: el comando del cliente oficial con el servidor de login
configurado y el nombre del equipo, en forma POSIX y PowerShell, con la pista
de que abrirá el navegador del usuario para iniciar sesión en su propio
proveedor. El daemon MUST NOT ejecutarlo. El gesto MUST NOT contener secretos.
Un cliente MAY ejecutar el gesto únicamente a petición explícita del usuario,
con la entrada y la salida heredadas para que el usuario vea lo que su
cliente imprime.

#### Scenario: El gesto de vínculo se imprime, no se ejecuta

- **WHEN** se pide el vínculo de este equipo
- **THEN** la respuesta SHALL traer el comando en forma POSIX y PowerShell con
  el servidor de login y el nombre del equipo
- **AND** ningún proceso del cliente de malla SHALL haberse lanzado

#### Scenario: Ejecutar es explícito y visible

- **WHEN** el usuario pide ejecutar el gesto desde el CLI
- **THEN** SHALL correr el binario del propio usuario con la entrada y la
  salida heredadas
- **AND** el URL o la instrucción que el cliente imprima SHALL llegar al
  usuario sin filtro

#### Scenario: El gesto no lleva secretos

- **WHEN** se compone el gesto
- **THEN** NO SHALL contener claves, tokens ni claves de preautorización
- **AND** el lint de secretos en claro SHALL aplicarse a la configuración que
  lo alimenta

### Requirement: Las máquinas del usuario se leen, no se registran

El daemon SHALL responder la lista de equipos del usuario tal como la malla la
conoce ahora —nombre, presencia, sistema y última vez visto— marcando cuál es
este equipo, y NO SHALL mantener registro propio de equipos. La presencia
SHALL ser de solo lectura y NO SHALL constituir control: el transporte hacia
un daemon remoto sigue siendo únicamente el túnel SSH del usuario, y si en un
equipo corre un daemon se sabe al conectar, nunca se supone. La lista SHALL
poder consumirse desde el CLI, la TUI y la GUI (constitución §4; la regla de
subconjunto vive en `mobile-companion` y no se duplica aquí).

#### Scenario: Las máquinas son los pares de la malla

- **WHEN** la malla conoce tres equipos del usuario y uno está fuera de línea
- **THEN** la lista SHALL traer los tres con su presencia y su sistema
- **AND** SHALL marcar cuál es este equipo

#### Scenario: Presencia no es control

- **WHEN** un equipo aparece en línea en la lista
- **THEN** ninguna superficie SHALL abrir una conexión hacia él por ese hecho
- **AND** el camino hacia su daemon SHALL seguir siendo el túnel del usuario

### Requirement: Decidir y seguir por SSH sin terminal interactiva
Los verbos de la CLI que listan y deciden peticiones de permiso y los que siguen
una sesión SHALL funcionar bajo una ejecución remota de `ssh` sin terminal
interactiva: MUST NOT leer el teclado, preguntar ni exigir un tamaño de
pantalla, y la decisión SHALL ser exactamente la que dice la línea de órdenes.
Una petición decidida así SHALL resolverse con la regla de que la primera
decisión gana, como desde cualquier superficie. La documentación de acceso
remoto SHALL dar la receta de aprobar desde otro equipo sin la TUI —listar,
decidir una petición por su id y seguir la sesión— y SHALL declarar que esa
ruta no necesita túnel: la orden corre en la máquina del daemon por el SSH del
propio usuario, y ningún verbo decide en bloque.

#### Scenario: Aprobar sin terminal desde otra conexión
- **WHEN** una sesión iniciada por otra superficie espera un permiso y se invocan `permissions` y luego `approve <petición>` sin terminal interactiva
- **THEN** la petición SHALL resolverse con la opción elegida, decidida por el cliente
- **AND** la sesión SHALL continuar su turno

#### Scenario: Seguir sin terminal
- **WHEN** se invoca `watch` sin terminal interactiva sobre una sesión viva
- **THEN** SHALL emitir sus registros a medida que ocurren
- **AND** SHALL terminar cuando la sesión termina

#### Scenario: La receta de aprobar desde el teléfono está documentada
- **WHEN** un usuario consulta la documentación de acceso remoto
- **THEN** SHALL encontrar la receta `ssh <pc> meltemi permissions`, luego `ssh <pc> meltemi approve <petición>`, y `watch` para seguir
- **AND** SHALL encontrar declarado que no hace falta túnel y que ningún verbo decide en bloque

### Requirement: Túnel con latido y fallo visible
El helper de túnel SHALL componer el `ssh` del usuario con latido
(`ServerAliveInterval`, `ServerAliveCountMax`) y con `ExitOnForwardFailure`,
en el comando y en el snippet de configuración, diciendo qué hace cada uno.
Con `--exec` SHALL ejecutar exactamente el comando que imprime y, cuando ese
`ssh` termina, SHALL decir con qué estado terminó y qué significa; un túnel
que terminó con error MUST NOT presentarse como éxito.

#### Scenario: El túnel lleva latido y cae si no puede reenviar
- **WHEN** el helper compone el túnel de un daemon macOS/Linux
- **THEN** el comando y el snippet SHALL llevar el latido y `ExitOnForwardFailure`
- **AND** lo que `--exec` ejecuta SHALL ser exactamente el comando impreso

#### Scenario: Un túnel que muere se reporta
- **WHEN** el `ssh` que `--exec` lanzó termina con un estado de error
- **THEN** el binario SHALL terminar con el código de daemon inalcanzable
- **AND** SHALL decir el estado y que el otro extremo ya no alcanza el daemon

### Requirement: El acceso de hoy está documentado con sus límites
La documentación de acceso remoto SHALL cubrir el PC Windows servido por
OpenSSH Server —con `meltemi bridge`, los verbos de la CLI y la TUI por SSH, y
lo que está medido y lo que no—; el teléfono de hoy —los verbos sin TUI, la TUI
por `ssh -t` con su tamaño mínimo, la campana como aviso local— y lo que no hay
(push sin conexión); y cuánto se espera al humano, con cada ajuste, su valor
por omisión y lo que pasa al vencer. La documentación de plataformas MUST NOT
contradecir al helper de túnel.

#### Scenario: El PC Windows servido está documentado con lo no medido
- **WHEN** un usuario consulta cómo servir su PC Windows
- **THEN** SHALL encontrar cómo instalar el servidor SSH y qué orden correr desde el otro extremo
- **AND** SHALL encontrar declarado qué tramos están medidos y cuáles no

#### Scenario: El teléfono de hoy está documentado con sus límites
- **WHEN** un usuario consulta cómo usar Meltemi desde un teléfono
- **THEN** SHALL encontrar los verbos sin TUI, la TUI por `ssh -t` con su tamaño mínimo y la campana
- **AND** SHALL encontrar declarado que no existe push sin conexión

#### Scenario: Cada espera tiene su ajuste y su valor
- **WHEN** un usuario consulta cuánto espera una petición o una sesión
- **THEN** SHALL encontrar cada ajuste de `[permissions]` y `[sessions]` con su valor por omisión

#### Scenario: Plataformas y el helper dicen lo mismo
- **WHEN** un usuario consulta el acceso por SSH en la documentación de plataformas
- **THEN** SHALL encontrar qué extremo corre `-R` y cuál `-L`, y el puente para un daemon en Windows
- **AND** NO SHALL encontrar una recomendación que contradiga al helper

### Requirement: El acceso por túnel está medido o declarado
El repositorio SHALL guardar un smoke del acceso remoto que registre, con fecha
y versiones, cada tramo medido y declare pendiente, con lo que falta para
medirlo, cada tramo que no se pudo medir —GUI y TUI contra un daemon reenviado,
el puente por un `sshd` real en Windows—. La documentación MUST NOT presentar
como verificado lo que el smoke declara pendiente.

#### Scenario: El smoke dice qué se midió y qué no
- **WHEN** se consulta el smoke del acceso remoto
- **THEN** cada tramo SHALL estar medido con fecha y versiones o declarado pendiente con lo que falta

#### Scenario: La documentación no promete lo no medido
- **WHEN** la documentación de acceso remoto habla de GUI y TUI sobre un túnel
- **THEN** SHALL presentarlo como lo esperado y remitir al smoke para lo medido

### Requirement: Transporte por comando del usuario en el cliente

Un cliente de Meltemi SHALL aceptar un endpoint que nombre un **comando** en
lugar de un socket, lanzarlo como subproceso y hablar el protocolo por su
entrada y salida estándar, con la misma delimitación por líneas que sobre un
socket. El comando SHALL lanzarse directamente, sin intérprete de shell
intermedio.

El comando SHALL leerse únicamente del ámbito de usuario —su entorno o su
configuración personal— y NO SHALL leerse de la configuración de un
repositorio: abrir un proyecto NO SHALL poder hacer que el cliente de quien lo
abre ejecute un comando.

Meltemi NO SHALL añadir opciones al comando del usuario, NO SHALL leer,
almacenar ni reenviar material de claves o contraseñas, y NO SHALL ofrecer al
subproceso un terminal: su entrada y su salida son el canal del protocolo.
WHERE el subproceso termine antes de hablar el protocolo, el cliente SHALL
rehusar con un diagnóstico que incluya lo que ese proceso haya escrito en su
salida de error y el remedio que corresponda.

Al cerrarse el cliente, el subproceso SHALL terminar con él. WHERE la
plataforma permita atar su vida a la del cliente, el cliente SHALL atarla, de
modo que un fin abrupto tampoco deje el proceso vivo.

#### Scenario: El cliente habla por el comando que el usuario nombró

- **WHEN** el endpoint del cliente nombra un comando y ese comando pone un daemon en su stdio
- **THEN** el cliente completa el apretón de manos y opera como si hablara por un socket local
- **AND** el daemon no distingue esa conexión de cualquier otra conexión local

#### Scenario: Un repositorio no decide qué comando corre tu cliente

- **WHEN** la configuración de un repositorio declara un endpoint con forma de comando
- **THEN** el cliente lo ignora y no ejecuta nada
- **AND** el endpoint efectivo sigue siendo el del ámbito de usuario

#### Scenario: El comando se lanza sin intérprete intermedio

- **WHEN** el comando nombrado se lanza
- **THEN** el proceso hijo del cliente es el programa nombrado, no un shell que lo invoque
- **AND** las comillas dobles del comando agrupan un argumento con espacios

#### Scenario: Un comando que no llega a hablar el protocolo se rehúsa con su causa

- **WHEN** el subproceso termina antes de completar el apretón de manos
- **THEN** el cliente rehúsa nombrando el comando y lo que el proceso escribió en su salida de error
- **AND** el mensaje dice el remedio, sin que Meltemi pida ni transporte contraseña alguna

#### Scenario: Al cerrar el cliente no queda el proceso

- **WHEN** el cliente termina, ordenadamente o de golpe
- **THEN** el subproceso que lanzó SHALL haber terminado también

### Requirement: Emisor del aviso de espera en el cliente

El usuario MAY configurar, **solo en su configuración de ámbito usuario**, un
comando que un cliente conectado ejecute cuando algo empiece a esperar su
decisión. La configuración SHALL estar desactivada por defecto y NO SHALL
leerse de la configuración de un repositorio. El daemon NO SHALL leerla,
validarla ni ejecutarla.

El comando SHALL recibir, por variables de entorno y por ningún otro medio,
únicamente: la clase de espera —permiso, instrucción, gate o prueba—, el
identificador de sesión, el nombre de la máquina del cliente, el instante en
RFC 3339 y cuántas cosas esperan. NO SHALL recibir ruta de proyecto, nombre de
repositorio, texto de la petición, nombre de herramienta ni título de sesión.
Toda ampliación de ese contenido exige enmienda previa (§9).

El cliente SHALL lanzar el comando sin esperarlo, sin darle su entrada ni su
salida, y SHALL terminarlo al agotarse un tope de tiempo, de modo que un
comando que no responde NO SHALL bloquear la interfaz. El cliente SHALL emitir
un aviso por **evento de espera** y no uno por elemento en cola, y NO SHALL
emitir otro antes de un piso temporal declarado. Cuando la espera se resuelva,
el cliente NO SHALL emitir aviso alguno.

#### Scenario: Sin configurar, no se emite nada

- **WHEN** algo empieza a esperar y el usuario no configuró comando alguno
- **THEN** no se ejecuta nada y nada se envía a ningún sitio

#### Scenario: El aviso lleva el hecho y nada del proyecto

- **WHEN** el cliente emite un aviso de espera
- **THEN** el comando recibe la clase de espera, el identificador de sesión, la máquina, el instante y la cuenta
- **AND** no recibe ruta, ni nombre de proyecto, ni texto de la petición

#### Scenario: Un repositorio no configura tu aviso

- **WHEN** la configuración de un repositorio declara un comando de aviso
- **THEN** el cliente lo ignora y no ejecuta nada

#### Scenario: Una ráfaga de esperas produce un aviso con su cuenta

- **WHEN** varias cosas empiezan a esperar en rápida sucesión
- **THEN** se emite un aviso con la cuenta de lo que espera, no uno por elemento
- **AND** no se emite otro antes del piso temporal

#### Scenario: Un comando que no responde no bloquea al cliente

- **WHEN** el comando configurado no termina
- **THEN** el cliente sigue respondiendo y termina ese comando al agotarse el tope

#### Scenario: Resolver la espera no avisa

- **WHEN** lo que esperaba se decide
- **THEN** no se emite ningún aviso por ello

### Requirement: Las máquinas del usuario tienen nombre en el cliente

El cliente SHALL poder leer una lista de máquinas de la configuración **de
usuario** —nombre y endpoint, donde el endpoint es el default de la plataforma
o un comando `exec:`— y NO SHALL leerla de ningún repositorio. Esa lista NO
SHALL contener material de claves —ni ruta de clave, ni passphrase, ni puerto,
ni usuario—: esos datos viven en la configuración `ssh` del propio usuario y en
su agente de claves. El cliente SHALL poder cambiar la máquina activa sin
reiniciarse, reconectando contra el endpoint de la nueva, y SHALL advertir del
trabajo sin guardar con la misma guarda que el cambio de proyecto. Ninguna
superficie SHALL agregar sesiones de más de una máquina: lo que se muestra es
lo de la máquina activa, y el cromo SHALL decir cuál.

#### Scenario: Las máquinas se leen del usuario y nunca de un repositorio

- **WHEN** un repositorio declara máquinas en su `.meltemi/config.toml`
- **THEN** esa lista SHALL ignorarse
- **AND** solo la configuración de usuario SHALL poder nombrar máquinas

#### Scenario: La lista no lleva material de claves

- **WHEN** se lee la lista de máquinas
- **THEN** NO SHALL admitirse ruta de clave, passphrase, puerto ni usuario
- **AND** el comando remoto SHALL nombrar un alias que resuelve la
  configuración `ssh` del usuario

#### Scenario: Cambiar de máquina reconecta y lo dice

- **WHEN** el usuario elige otra máquina
- **THEN** el cliente SHALL reconectar contra el endpoint de esa máquina sin
  reiniciarse
- **AND** el cromo SHALL decir en qué máquina está

#### Scenario: Probar una conexión no arranca nada

- **WHEN** el usuario prueba una máquina remota
- **THEN** SHALL abrirse la conexión, hacerse `initialize` y cerrarse,
  informando versión y latencia
- **AND** NO SHALL arrancarse ningún daemon: una máquina remota apagada SHALL
  fallar con su diagnóstico

#### Scenario: La presencia se muestra a pedido y no es una prueba de conexión

- **WHEN** el usuario abre la lista de máquinas
- **THEN** la presencia SHALL leerse en ese momento y NO SHALL sondearse de
  fondo
- **AND** una máquina en línea en la malla NO SHALL presentarse como una
  máquina con daemon: eso se sabe al probar o al conectar
