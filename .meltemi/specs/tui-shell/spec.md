# tui-shell Specification

## Purpose
TBD - created by archiving change tui-nucleo-ux. Update Purpose after archive.
## Requirements

### Requirement: Modelo de vistas y chrome persistente
El shell interactivo SHALL presentar un chrome persistente no enfocable que
enmarca cuatro vistas de primer nivel (Sesiones, Proyecto, Permisos, Flota), un
único nivel de drill-in y una capa de overlays (paleta `:`, ayuda `?`). El header
y el footer del chrome MUST permanecer visibles en toda vista y overlay. Una
sesión que pasa a estado `ended` durante la ejecución actual MUST permanecer
temporalmente accesible en la tabla de Sesiones, no desaparecer de inmediato; la
inspección de su registro persistente es interior de #8.

#### Scenario: Aterrizaje con daemon y sesiones
- **WHEN** se entra al modo interactivo con el daemon accesible y al menos una sesión
- **THEN** el shell SHALL mostrar la vista Sesiones como vista de aterrizaje
- **AND** el chrome SHALL mostrar estado del daemon, ámbito de proyecto y el indicador de la bandeja de permisos

#### Scenario: Drill-in con breadcrumb
- **WHEN** el usuario pulsa Enter sobre una fila de la vista Sesiones
- **THEN** el shell SHALL abrir la vista de detalle Sesión como drill-in
- **AND** el header SHALL reflejar la ruta y la profundidad mediante un breadcrumb

#### Scenario: Sesión finalizada sigue accesible
- **WHEN** una sesión pasa al estado `ended` mientras el usuario está en otra vista
- **THEN** el shell SHALL retenerla temporalmente en la tabla de Sesiones marcada como finalizada
- **AND** SHALL NOT hacerla desaparecer de inmediato sin recurso

### Requirement: Navegación por teclado y contrato de consistencia
El shell SHALL exponer un único keymap honrado por toda vista donde cada tecla
ejecuta la misma categoría de acción: Enter drill-in, Esc atrás/cerrar overlay,
Tab/Shift-Tab foco de panel, `/` filtro, `:` paleta, `?` ayuda. Los dígitos 1–4
MUST conmutar las vistas de primer nivel desde cualquier contexto de navegación, y
las letras MUST actuar dentro de la vista enfocada.

#### Scenario: Conmutación global por dígito
- **WHEN** el usuario pulsa un dígito 1–4 fuera de un contexto de entrada de texto
- **THEN** el shell SHALL conmutar de inmediato a la vista de primer nivel correspondiente

#### Scenario: Letra actúa local sin conmutar de vista
- **WHEN** el usuario pulsa una letra de acción local en la vista enfocada fuera de un contexto de entrada de texto
- **THEN** el shell SHALL ejecutar la acción local de esa vista
- **AND** SHALL NOT conmutar de vista de primer nivel

#### Scenario: Ayuda muestra el mapa de teclas
- **WHEN** el usuario pulsa `?` en cualquier vista
- **THEN** el shell SHALL abrir la ayuda con el mapa de teclas completo del modelo de navegación

### Requirement: Conjunto de teclas robusto y desambiguación de Esc
El keymap SHALL limitarse a letras, dígitos, Esc, Enter, Tab y flechas, y MUST NOT
depender de Alt/Meta, de teclas de función F1–F12 ni de combinaciones Ctrl
capturadas por el TTY. Esc SHALL desambiguarse de secuencias CSI mediante un
timeout corto y MUST ofrecer siempre una alternativa `q`/Backspace.

#### Scenario: Esc en enlace lento
- **WHEN** el usuario pulsa Esc sobre un enlace SSH lento
- **THEN** el shell SHALL esperar un timeout corto para distinguir Esc de una secuencia CSI
- **AND** SHALL ejecutar la acción atrás/cerrar solo si no llega una secuencia

#### Scenario: Alternativa siempre disponible
- **WHERE** el terminal no entrega Esc de forma fiable
- **THEN** el shell SHALL aceptar `q` o Backspace como equivalente de atrás/cerrar

### Requirement: Captura de teclado en contextos de entrada de texto
El shell SHALL capturar todo el teclado cuando el foco está en un contexto de
entrada de texto (compositor de prompt, paleta o filtro `/`), de modo que los
dígitos y letras no se filtren como comandos globales, y MUST anunciar la vía de
salida "Esc para salir".

#### Scenario: Dígitos no se filtran al redactar
- **WHILE** el foco está en el compositor de prompt
- **WHEN** el usuario teclea un dígito
- **THEN** el shell SHALL insertar el dígito en el texto
- **AND** SHALL NOT conmutar de vista

#### Scenario: Salida anunciada
- **WHEN** se abre un contexto de entrada de texto
- **THEN** el shell SHALL anunciar "Esc para salir" en el footer

### Requirement: Reserva global de teclas de acción transversal
El keymap SHALL reservar globalmente las teclas de acción transversal con
significado definido en esta change —`a` (saltar a la bandeja de permisos) y `x`
(cancelar la sesión activa, con confirmación)— de modo que ninguna vista futura
pueda reasignarlas a otra categoría de acción. Una tecla reservada MUST significar
la misma categoría en toda vista o permanecer inactiva, nunca colisionar.

#### Scenario: Reserva respetada por vistas futuras
- **WHEN** una vista futura define sus teclas locales
- **THEN** el shell SHALL NOT permitir reutilizar una tecla reservada para otra categoría de acción

#### Scenario: Inactiva sin colisión
- **WHERE** una tecla reservada no aplica en la vista actual
- **THEN** SHALL permanecer inactiva
- **AND** su pulsación SHALL NOT producir una acción local divergente

### Requirement: Indicador de bandeja de permisos y prioridad de señales
El shell SHALL mostrar en el chrome un indicador de la bandeja de permisos siempre
visible con símbolo, contador y palabra (nunca solo un punto de color), alcanzable
como vista por el dígito 3 y por la tecla `a`, que salta a la bandeja y enfoca una
petición pendiente. El shell MUST fijar un orden de prioridad de señales —daemon
caído por encima de permiso pendiente, y ambos por encima de error o fin inesperado
de sesión y del streaming silencioso— y hacer visibles esas señales desde cualquier
vista. El criterio de orden de la cola de la bandeja es interior de #9.

#### Scenario: Atender un permiso pendiente
- **WHEN** el usuario pulsa `a` desde cualquier vista
- **THEN** el shell SHALL abrir la bandeja de permisos
- **AND** SHALL enfocar una petición pendiente

#### Scenario: Indicador legible sin color
- **WHILE** hay peticiones de permiso pendientes
- **THEN** el chrome SHALL mostrar símbolo + contador + palabra (p. ej. "! 3 esperando")

#### Scenario: Orden completo de prioridad de señales
- **WHEN** coinciden un daemon caído, una petición de permiso pendiente y streaming de sesión
- **THEN** el chrome SHALL superficializar primero la caída del daemon y luego el permiso pendiente
- **AND** SHALL situar ambas señales por encima de un error o fin inesperado de sesión y por encima del streaming silencioso

### Requirement: Aviso de vencimiento de permisos
El shell SHALL superficializar de forma ruidosa toda auto-denegación por
vencimiento de permiso —el daemon deniega la petición al vencer su plazo y notifica
el vencimiento (`permission/timeout`)— mediante un aviso persistente que MUST NOT
descartarse en silencio. La mecánica de la cola de la bandeja y de las decisiones
aprobar/denegar es interior de la bandeja de permisos (#9), fuera de este alcance.

#### Scenario: Vencimiento anunciado
- **WHEN** una petición pendiente vence y el daemon la deniega por plazo
- **THEN** el shell SHALL registrar un aviso persistente y etiquetado con la sesión y la operación afectadas
- **AND** SHALL NOT descartarla en silencio

#### Scenario: Bandeja vaciada por vencimientos en ausencia
- **WHEN** todas las peticiones pendientes vencen mientras el usuario está en otra vista
- **THEN** el contador del chrome SHALL reflejar cero pendientes
- **AND** SHALL conservar el aviso de los vencimientos ocurridos

### Requirement: Control de ciclo de vida de sesión y daemon
El shell SHALL dar casa y tecla reservada a la cancelación de una sesión activa
—la única primitiva que el daemon soporta hoy (`session/cancel`), que termina el
subproceso del agente y finaliza la sesión— y SHALL superficializar el apagado del
daemon (`shutdown`) al menos desde la paleta, de modo que ninguna capacidad de
control de ciclo de vida quede sin casa en la TUI (paridad de núcleo). Cancelar una
sesión activa o apagar el daemon MUST requerir confirmación explícita por ser
irreversibles.

#### Scenario: Cancelar la sesión en curso con confirmación
- **WHEN** el usuario solicita cancelar la sesión desde el drill-in de Sesión
- **THEN** el shell SHALL pedir confirmación advirtiendo que la cancelación termina el subproceso del agente y finaliza la sesión (no solo el turno)
- **AND** al confirmar SHALL enviar `session/cancel` y reflejar el estado resultante sin dejar el transcript en un estado ambiguo

#### Scenario: Apagado del daemon superficiado
- **WHERE** el daemon expone el método `shutdown`
- **THEN** el shell SHALL ofrecer esa acción al menos desde la paleta de comandos
- **AND** SHALL exigir confirmación por afectar a todas las sesiones activas

### Requirement: Diálogos de confirmación como superficie modal de primera clase
El shell SHALL tratar los diálogos de confirmación (salir con sesiones activas o
permisos pendientes, cancelar sesión, apagar daemon) como una superficie modal de
primera clase: atrapan el foco, Esc/`q`/Backspace SIEMPRE cancelan de forma segura
sin ejecutar la acción, y la opción no destructiva es la predeterminada. El shell
MUST NOT presentar un modal sin salida ni foco atrapado sin cancelación.

#### Scenario: Cancelar es seguro por defecto
- **WHEN** aparece un diálogo de confirmación de una acción irreversible
- **THEN** el foco predeterminado SHALL recaer en la opción no destructiva
- **AND** Esc SHALL cancelar sin efecto alguno

#### Scenario: Apilamiento acotado sobre overlays
- **IF** ya hay un overlay abierto y surge una confirmación
- **THEN** la confirmación SHALL apilarse encima con foco propio
- **AND** al cancelarse SHALL devolver el foco al overlay subyacente sin cerrarlo

### Requirement: Desconexión ruidosa y reconexión con backoff
Dado que el daemon deniega por defecto sin cliente conectado, el shell SHALL
señalar de forma ruidosa en el chrome la pérdida de conexión y MUST advertir que
los permisos pendientes se denegarán mientras no haya cliente. El shell SHALL
reintentar la conexión con backoff sin colgarse a la espera de entrada.

#### Scenario: Aviso de denegación silenciosa evitado
- **IF** la conexión con el daemon se pierde mientras hay permisos pendientes
- **THEN** el shell SHALL mostrar un banner de desconexión en el chrome
- **AND** SHALL advertir que los permisos pendientes se denegarán

#### Scenario: Reconexión con backoff
- **WHILE** la conexión está caída
- **THEN** el shell SHALL reintentar la conexión con backoff mostrando el estado "reconectando…"
- **AND** SHALL preservar el marco sin colgarse a la espera de entrada

### Requirement: Estado vacío sin daemon
El shell SHALL dibujar el chrome de inmediato y conectar de forma asíncrona; si el
daemon no está en ejecución MUST intentar el arranque bajo demanda antes de fallar,
distinguiendo el estado transitorio "arrancando/conectando…" del fallo
"inalcanzable" (código 10) con diagnóstico accionable.

#### Scenario: Shell inmediato con conexión asíncrona
- **WHEN** se entra al modo interactivo sin daemon en ejecución
- **THEN** el shell SHALL dibujar el chrome de inmediato con estado "conectando…"
- **AND** SHALL intentar alcanzar el daemon sin bloquear la interfaz

#### Scenario: Fallo de arranque diagnosticado
- **IF** el daemon no puede arrancarse ni alcanzarse dentro del presupuesto
- **THEN** el shell SHALL mostrar una tarjeta con la causa, el socket path y el remedy del ErrorData
- **AND** SHALL ofrecer reintentar y, si el daemon corre en otro host, la pista de reenvío del socket local por SSH (el daemon nunca abre un puerto de red)

### Requirement: Estado vacío sin sesiones
Cuando el daemon está accesible pero no hay sesiones, la vista Sesiones SHALL
presentar un launchpad que enseña el siguiente paso en vez de una tabla muda, y
MUST enlazar a la vista Flota y a la paleta.

#### Scenario: Launchpad en lugar de tabla vacía
- **WHEN** el daemon está accesible y hay cero sesiones
- **THEN** la vista Sesiones SHALL explicar qué es una sesión y la acción primaria para iniciar trabajo de agente
- **AND** SHALL ofrecer un atajo a la vista Flota (4) y una pista de la paleta `:`

### Requirement: Estado vacío sin proyecto y desacople de ámbito
El shell SHALL mostrar vacía solo la vista Proyecto cuando el directorio actual no
es un proyecto `.meltemi/`, con la acción para inicializar; las vistas Sesiones,
Permisos y Flota MUST permanecer plenamente usables porque pilotar agentes no
exige un proyecto.

#### Scenario: Solo Proyecto queda vacío
- **WHEN** el cwd no contiene `.meltemi/`
- **THEN** la vista Proyecto SHALL mostrar el vacío con la acción de iniciar constitución/andamiaje
- **AND** las vistas Sesiones, Permisos y Flota SHALL seguir operativas

#### Scenario: Verbos reservados no son error
- **WHEN** el usuario invoca un verbo del ciclo SDD aún reservado desde Proyecto o la paleta
- **THEN** el shell SHALL anunciarlo como reservado y próximamente
- **AND** SHALL NOT tratarlo como error

### Requirement: Estado vacío de las casas reservadas
Las vistas que reservan casa a features futuras SHALL presentar un estado vacío
etiquetado en vez de una pantalla muda: la vista Permisos sin peticiones pendientes
y la vista Flota sin agentes detectados MUST mostrar glifo + etiqueta + la siguiente
pista, honrando la línea base de accesibilidad, de modo que navegar a una casa
reservada-pero-vacía nunca produzca un callejón sin salida.

#### Scenario: Bandeja sin peticiones pendientes
- **WHEN** el usuario abre la vista Permisos y no hay peticiones pendientes
- **THEN** la vista SHALL mostrar un estado vacío etiquetado ("sin permisos pendientes") en vez de una pantalla muda

#### Scenario: Flota sin agentes, alcanzada desde el launchpad
- **WHEN** el usuario llega a la vista Flota (p. ej. desde el launchpad de Sesiones) y no hay agentes detectados
- **THEN** la vista SHALL mostrar "sin agentes detectados" con la pista de remediación (BYO-agent)
- **AND** SHALL NOT dejar un callejón sin salida

### Requirement: Pérdida de daemon durante una sesión en vivo
El shell SHALL manejar de forma honesta la caída del daemon mientras el drill-in de
Sesión muestra un turno en streaming: la región append-only MUST congelarse con una
marca textual de corte y el scrollback previo MUST preservarse. Al reconectar, si la
sesión observada ya no existe, el shell MUST informar que terminó con la caída y no
presentar el transcript como si fuera a reanudarse.

#### Scenario: Caída durante el streaming
- **WHEN** el daemon se vuelve inalcanzable mientras el drill-in de Sesión recibe `session/event`
- **THEN** la región append-only SHALL insertar una línea etiquetada de corte y detener el auto-follow
- **AND** el header SHALL elevar la señal de daemon inalcanzable con prioridad máxima

#### Scenario: Reconexión a un daemon sin la sesión
- **WHEN** el bucle de reconexión restablece la conexión y la sesión observada ya no existe
- **THEN** el shell SHALL informar que esa sesión terminó al caer el daemon y ofrecer volver a Sesiones
- **AND** SHALL NOT presentar el transcript como si el turno fuera a reanudarse

### Requirement: Suelo duro de tamaño de terminal
El shell SHALL presentar un estado explícito y etiquetado por debajo del mínimo
utilizable (80x24) —una vez agotado el reflow definido en «Reflow, streaming y
seguimiento de cola sobre SSH»— que indique el tamaño actual y el requerido, y MUST
NOT fallar ni colgarse. Si ni el estado de conexión y el contador de bandeja caben,
el shell MUST degradar a una única línea que los priorice por encima de todo lo
demás.

#### Scenario: Aviso de tamaño insuficiente
- **WHEN** el área disponible cae por debajo del mínimo utilizable
- **THEN** el cuerpo SHALL sustituirse por un mensaje que indique el tamaño actual y el requerido
- **AND** el shell SHALL permanecer receptivo a SIGWINCH para recuperarse al agrandar

#### Scenario: Suelo crítico
- **IF** ni siquiera el header completo cabe
- **THEN** el shell SHALL reducirse a una sola línea que priorice el estado de conexión y el contador de permisos pendientes

### Requirement: Onboarding de primer uso
El shell SHALL mostrar en el primer uso —detectado por ausencia de un flag en el
directorio de datos del usuario— un overlay ligero, saltable y re-invocable que
enseña el modelo de navegación (incluido cómo salir con `q` y cómo abandonar un
contexto de captura con Esc) mediante una checklist contextual. El shell MUST
persistir el flag para no repetirse y MUST NOT bloquear ni requerir cuenta, red ni
telemetría.

#### Scenario: Primer uso enseña navegación y salida
- **WHEN** se entra al modo interactivo por primera vez
- **THEN** el shell SHALL mostrar la bienvenida con las teclas 1–4, `:`, `?`, `a`, `q` para salir y Esc para cerrar overlays
- **AND** SHALL mostrar una checklist contextual daemon → proyecto → agente → propose (este último marcado como próximamente, coherente con los verbos SDD reservados)

#### Scenario: Saltable y persistente
- **WHEN** el usuario descarta el onboarding con Esc o `q`
- **THEN** el shell SHALL persistir el flag de primer uso
- **AND** SHALL NOT volver a mostrarlo salvo re-invocación desde Ayuda

#### Scenario: Honra la accesibilidad desde el primer fotograma
- **WHERE** están activos `NO_COLOR`, ASCII o un terminal 80x24
- **THEN** el onboarding SHALL renderizarse legible sin color, sin Unicode y sin animación temporizada

### Requirement: Accesibilidad — nunca solo color
Toda vista SHALL codificar cada estado de forma redundante con glifo o forma más
etiqueta textual; el color MUST ser solo decorativo y el significado MUST NOT
depender de él ni de caracteres de dibujo de caja. El foco y la selección MUST
distinguirse por marcador y atributos, nunca por color solo, y MUST seguir
distinguibles entre sí en monocromo.

#### Scenario: Estado de sesión legible sin color
- **WHEN** se muestra el estado de una sesión
- **THEN** el shell SHALL renderizar símbolo + palabra para starting/active/waiting_permission/ended

#### Scenario: Foco perceptible sin color
- **WHEN** un panel recibe el foco
- **THEN** el shell SHALL marcarlo con borde/título en vídeo inverso y marcador `▸`
- **AND** SHALL reflejar el foco con eco textual en el footer

### Requirement: Accesibilidad — NO_COLOR
El shell SHALL honrar la variable de entorno `NO_COLOR` (cualquier valor no vacío),
el flag `--no-color` y `TERM=dumb` renderizando sin color alguno, y MUST NOT pintar
fondo: usará los colores fg/bg por defecto del terminal.

#### Scenario: Render monocromo
- **WHERE** `NO_COLOR` está definida con valor no vacío
- **THEN** el shell SHALL renderizar sin ANSI de color
- **AND** toda distinción SHALL recaer en símbolo, etiqueta, disposición y atributos (inverso, negrita, subrayado)

### Requirement: Accesibilidad — fallback ASCII
El shell SHALL detectar la capacidad Unicode del terminal (locale/TERM) con
overrides explícitos (`--ascii`, `MELTEMI_ASCII=1`, config) y defaults
conservadores, y MUST definir para cada glifo un gemelo ASCII: ningún glifo Unicode
SHALL renderizarse sin gemelo definido.

#### Scenario: Conmutación a ASCII
- **WHERE** el terminal no anuncia capacidad Unicode o se fuerza `--ascii`/`MELTEMI_ASCII=1`
- **THEN** el shell SHALL sustituir cajas por `+ - |`, estados por `~ > ! x`, spinner por `-\|/` o "..."
- **AND** SHALL conservar layout y significado

#### Scenario: Invariante verificable
- **WHEN** se añade un símbolo nuevo a cualquier vista
- **THEN** un lint/test SHALL fallar si ese glifo Unicode carece de gemelo ASCII

### Requirement: Reflow, streaming y seguimiento de cola sobre SSH
El shell SHALL reajustar el layout en SIGWINCH sin scroll horizontal del texto
esencial, colapsando a una columna bajo el ancho mínimo y soltando columnas de
tabla de baja prioridad en orden definido en vez de truncar-y-ocultar. El streaming
del agente MUST escribirse en una región append-only con repintado por diff y techo
de FPS, y el auto-follow MUST suspenderse al desplazarse hacia atrás sin perder
líneas nuevas, indicándolo textualmente.

#### Scenario: Streaming sin repintar toda la pantalla
- **WHILE** una sesión emite eventos en streaming
- **THEN** el shell SHALL anexar líneas a una región append-only
- **AND** SHALL NOT repintar la pantalla completa por evento

#### Scenario: Desplazarse suspende el seguimiento
- **WHEN** el usuario se desplaza hacia arriba mientras el agente emite líneas
- **THEN** el auto-follow SHALL suspenderse sin perder líneas nuevas
- **AND** un indicador textual SHALL mostrar el estado desplazado con el recuento de líneas nuevas

#### Scenario: Tabla degrada sin ocultar datos
- **WHEN** el ancho disponible cae por debajo del umbral de una tabla
- **THEN** el shell SHALL soltar columnas de baja prioridad en el orden definido
- **AND** SHALL ofrecer scroll horizontal en vez de truncar y ocultar en silencio

### Requirement: Paridad de núcleo por la paleta de comandos
La paleta `:` SHALL exponer toda capacidad del daemon alcanzable por tecleo, incluso
antes de que exista una vista dedicada. Todo método RPC nuevo del daemon MUST
registrarse en el autocompletado de la paleta para que ninguna capacidad quede sin
casa.

#### Scenario: Capacidad sin vista dedicada alcanzable
- **WHEN** el usuario abre la paleta y filtra por una capacidad del daemon
- **THEN** el shell SHALL ofrecer invocarla aunque no tenga tecla o vista dedicada

#### Scenario: Registro obligatorio de método nuevo
- **WHEN** el daemon gana un método RPC nuevo
- **THEN** el shell SHALL registrarlo en el autocompletado de la paleta

### Requirement: Ruta accesible garantizada e internacionalización
El shell SHALL enrutar toda cadena visible por una tabla de mensajes ES/EN desde el
inicio (constitución §11), y MUST reconocer el modo CLI scriptable (`--json`) como
ruta accesible garantizada cuando el terminal no permita la TUI.

#### Scenario: Sin hardcodeo de idioma
- **WHEN** el shell muestra una cadena visible
- **THEN** SHALL obtenerla de la tabla de mensajes
- **AND** un lint SHALL marcar como fallo el texto hardcodeado

#### Scenario: Escape scriptable
- **WHERE** el terminal es insuficiente para la TUI (`TERM=dumb`, enlace pésimo, lector de pantalla real)
- **THEN** el usuario SHALL disponer del modo CLI scriptable `--json` como ruta accesible lineal

### Requirement: Sesiones agrupadas por proyecto con ámbito conmutable
La vista Sesiones SHALL agrupar las sesiones por proyecto con encabezado de grupo,
y cada fila SHALL mostrar su agente y el nombre de su suscripción cuando la
sesión resolvió por perfil; el filtro `/` vigente SHALL admitir además reducir por
proyecto, y el ámbito de proyecto SHALL ser conmutable desde la paleta sin salir
del shell, con el cwd como ámbito inicial. La agrupación MUST honrar la línea base
de accesibilidad (glifo o forma más palabra, gemelo ASCII, `NO_COLOR`) y el
degradado de columnas vigente, sin ocultar datos en silencio.

#### Scenario: Sesiones agrupadas por proyecto
- **WHEN** el usuario abre la vista Sesiones con sesiones en dos proyectos
- **THEN** la tabla SHALL presentarlas bajo un encabezado por proyecto
- **AND** cada fila SHALL indicar su agente y su suscripción

#### Scenario: Filtro por proyecto reduce a un grupo
- **WHEN** el usuario pulsa `/` y teclea parte de la raíz de un proyecto
- **THEN** la vista SHALL reducirse a las sesiones de ese proyecto

#### Scenario: Suscripción legible sin color ni Unicode
- **WHERE** están activos `NO_COLOR` o el modo ASCII
- **THEN** el proyecto y la suscripción de cada sesión SHALL seguir legibles como texto
- **AND** ninguna distinción SHALL depender del color

#### Scenario: Ámbito de proyecto conmutado desde la paleta
- **WHEN** el usuario conmuta el proyecto de ámbito desde la paleta
- **THEN** las consultas con ámbito de proyecto SHALL usar esa raíz
- **AND** el chrome SHALL reflejar el ámbito vigente
