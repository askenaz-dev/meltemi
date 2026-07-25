

### Requirement: Cliente fino sobre el socket local
El cliente de escritorio SHALL consumir el daemon únicamente vía JSON-RPC sobre
el socket local, con la conexión en su backend Rust; la webview MUST NOT abrir
sockets ni realizar peticiones de red, el empaquetado MUST NOT cargar contenido
remoto y las capacidades del runtime SHALL declararse mínimas y denegadas por
defecto. Si el daemon no corre, el cliente MUST intentar el arranque bajo
demanda distinguiendo el estado transitorio "arrancando/conectando…" del fallo
"inalcanzable" con diagnóstico accionable.

#### Scenario: Webview sin acceso a red
- **WHEN** se audita el empaquetado del cliente de escritorio
- **THEN** las capacidades declaradas SHALL NOT incluir acceso de red para la webview
- **AND** la CSP SHALL rehusar todo origen remoto

#### Scenario: Shell inmediato con conexión asíncrona
- **WHEN** se abre la GUI sin daemon en ejecución
- **THEN** el shell SHALL dibujarse de inmediato con estado "conectando…"
- **AND** SHALL intentar el arranque bajo demanda sin bloquear la interfaz

#### Scenario: Fallo de arranque diagnosticado
- **IF** el daemon no puede arrancarse ni alcanzarse dentro del presupuesto
- **THEN** la GUI SHALL mostrar la causa, el socket path y el remedy del ErrorData
- **AND** SHALL ofrecer reintentar y la pista del túnel SSH si el daemon corre en otro host

### Requirement: Paridad de vistas y modelo de navegación
La GUI SHALL presentar las cuatro vistas de primer nivel (Sesiones, Proyecto,
Permisos, Flota) con drill-in de detalle y breadcrumb, con la misma semántica
de estados que la TUI: una sesión que pasa a `ended` durante la ejecución MUST
permanecer temporalmente accesible en la lista, y los estados vacíos (sin
daemon, sin sesiones, sin proyecto, sin agentes) MUST enseñar el siguiente paso
en vez de una pantalla muda, manteniendo Sesiones, Permisos y Flota usables sin
proyecto `.meltemi/`.

#### Scenario: Aterrizaje con daemon y sesiones
- **WHEN** se abre la GUI con el daemon accesible y al menos una sesión
- **THEN** la vista Sesiones SHALL ser la vista de aterrizaje
- **AND** el chrome SHALL mostrar estado del daemon, ámbito de proyecto y el indicador de permisos

#### Scenario: Sesión finalizada sigue accesible
- **WHEN** una sesión pasa a `ended` mientras el usuario está en otra vista
- **THEN** la lista de Sesiones SHALL retenerla temporalmente marcada como finalizada

#### Scenario: Solo Proyecto queda vacío sin `.meltemi/`
- **WHEN** el directorio de trabajo no es un proyecto `.meltemi/`
- **THEN** la vista Proyecto SHALL mostrar el vacío con la acción de inicializar
- **AND** Sesiones, Permisos y Flota SHALL seguir plenamente operativas

### Requirement: Paleta de comandos y registro obligatorio de métodos
La GUI SHALL ofrecer una paleta de comandos que exponga toda capacidad del
daemon, incluso antes de que exista una vista dedicada, respaldada por un
registro tipado de métodos RPC. Todo método nuevo del contrato MUST registrarse
en ese registro para que ninguna capacidad quede sin casa en la superficie de
escritorio.

#### Scenario: Capacidad sin vista dedicada alcanzable
- **WHEN** el usuario filtra la paleta por una capacidad del daemon sin vista propia
- **THEN** la paleta SHALL ofrecer invocarla igualmente

#### Scenario: Registro obligatorio de método nuevo
- **WHEN** el daemon gana un método RPC nuevo
- **THEN** el registro tipado de la paleta GUI SHALL incorporarlo

### Requirement: Matriz de paridad verificada en CI
El proyecto SHALL mantener `docs/paridad-nucleo.md` como matriz viva capacidad
→ RPC → CLI/TUI → GUI, y la CI MUST fallar si un método del contrato
(`proto/schemas/v1/`) carece de entrada en el registro de paleta de la TUI o en
el de la GUI, de modo que la paridad de núcleo (constitución §4) sea un gate
verificado y no una promesa.

#### Scenario: Método sin casa rompe la CI
- **WHEN** el contrato incorpora un método que alguna superficie no registra
- **THEN** el check de paridad de la CI SHALL fallar señalando método y superficie ausente

### Requirement: Editor de specs enriquecido
La GUI SHALL ofrecer edición estructurada de los artefactos del método
(constitución, rumbo, proposal/design/tasks y spec deltas) con los findings del
validador del motor visibles en vivo durante la edición, y todo guardado SHALL
materializarse vía el daemon con su traza.

#### Scenario: Findings de validación en vivo
- **WHILE** el usuario edita un spec delta en el editor enriquecido
- **THEN** la GUI SHALL mostrar los findings de `validate` sobre el contenido editado
- **AND** SHALL distinguir hallazgos bloqueantes de advertencias

#### Scenario: Guardado trazable de un artefacto
- **WHEN** el usuario guarda un artefacto del método desde el editor
- **THEN** la escritura SHALL aplicarse vía el daemon y quedar registrada en el log correspondiente

### Requirement: Revisión de diffs línea a línea
La GUI SHALL presentar los diffs de asignaciones y carreras línea a línea, por
archivo y por hunk, permitiendo comparar competidores contra la base común; la
edición de hunks (dentro de la cerca `edit-surface`) SHALL aplicarse vía el
daemon como edición humana trazable, y cada línea SHALL ofrecer "Abrir con…"
hacia el editor del usuario con archivo y línea exactos.

#### Scenario: Comparar competidores de una carrera
- **WHEN** el usuario abre la revisión de una tarea con dos competidores despachados
- **THEN** la GUI SHALL mostrar el diff de cada worktree contra la base común, por archivo y hunk

#### Scenario: Edición de un hunk pasa por el daemon
- **WHEN** el usuario edita un hunk en la vista de diff y lo aplica
- **THEN** la escritura SHALL realizarse vía el daemon y registrarse como `human_edit`

#### Scenario: Abrir con el editor del usuario
- **WHEN** el usuario invoca "Abrir con…" sobre una línea del diff
- **THEN** el editor configurado SHALL abrirse en ese archivo y línea sin cerrar la sesión de Meltemi

### Requirement: Edición utilitaria in situ con LSP BYO
La GUI SHALL ofrecer, dentro de la cerca `edit-surface`, árbol del proyecto,
pestañas multi-archivo, búsqueda en el proyecto y resaltado de sintaxis; WHERE
el usuario tenga un servidor LSP instalado o configurado para el lenguaje, la
superficie SHALL sumar autocompletado, diagnósticos, ir-a-definición, renombrar,
formatear y referencias; sin servidor disponible MUST degradar a resaltado sin
bloquear la edición. El cliente MUST NOT empaquetar servidores LSP y todo
guardado SHALL pasar por el daemon.

#### Scenario: Inteligencia con servidor del usuario
- **WHERE** existe un servidor LSP detectado para el lenguaje del archivo
- **THEN** la superficie SHALL ofrecer autocompletado, diagnósticos y navegación vía ese servidor

#### Scenario: Degradación honesta sin servidor
- **WHERE** no hay servidor LSP disponible para el lenguaje
- **THEN** la edición SHALL seguir disponible con resaltado sintáctico
- **AND** la superficie SHALL indicar que la inteligencia LSP no está activa y cómo habilitarla

### Requirement: Bandeja de permisos y prioridad de señales
La GUI SHALL mostrar un indicador de la bandeja de permisos siempre visible con
símbolo, contador y palabra, y una bandeja donde atender las peticiones. El
orden de prioridad de señales MUST ser: daemon caído por encima de permiso
pendiente, y ambos por encima de error o fin inesperado de sesión y del
streaming silencioso. Toda auto-denegación por vencimiento (`permission/timeout`)
MUST superficializarse con un aviso persistente que no se descarta en silencio.

#### Scenario: Atender un permiso pendiente
- **WHEN** el usuario activa el indicador de permisos
- **THEN** la bandeja SHALL abrirse con una petición pendiente enfocada

#### Scenario: Vencimiento anunciado
- **WHEN** una petición vence y el daemon la deniega por plazo
- **THEN** la GUI SHALL registrar un aviso persistente etiquetado con la sesión y la operación
- **AND** SHALL NOT descartarlo en silencio

#### Scenario: Orden de prioridad de señales
- **WHEN** coinciden daemon caído, permiso pendiente y streaming de sesión
- **THEN** la GUI SHALL superficializar primero la caída del daemon y después el permiso pendiente

### Requirement: Desconexión ruidosa y streaming honesto
La GUI SHALL señalar de forma ruidosa la pérdida de conexión con el daemon,
advirtiendo que los permisos pendientes se denegarán mientras no haya cliente,
y SHALL reintentar con backoff. Si la caída ocurre durante un turno en
streaming, el transcript MUST congelarse con una marca de corte preservando el
scrollback; al reconectar, si la sesión ya no existe, la GUI MUST informar que
terminó y no presentar el transcript como reanudable.

#### Scenario: Caída durante el streaming
- **WHEN** el daemon se vuelve inalcanzable mientras una sesión emite eventos
- **THEN** el transcript SHALL congelarse con una marca de corte etiquetada
- **AND** la señal de daemon caído SHALL elevarse con prioridad máxima

#### Scenario: Reconexión a un daemon sin la sesión
- **WHEN** la reconexión llega a un daemon donde la sesión observada ya no existe
- **THEN** la GUI SHALL informar que la sesión terminó con la caída
- **AND** SHALL NOT presentar el transcript como si fuera a reanudarse

### Requirement: Accesibilidad de la superficie de escritorio
La GUI SHALL ser operable al 100% por teclado con indicador de foco visible, y
su árbol de accesibilidad SHALL exponer roles y etiquetas correctos en toda
vista, overlay y diálogo para lectores de pantalla. Todo estado SHALL
codificarse con símbolo o forma más etiqueta textual — el color MUST NOT ser el
único portador de significado — y la GUI SHALL honrar las preferencias del
sistema de alto contraste y de movimiento reducido.

#### Scenario: Flujo completo por teclado
- **WHEN** el usuario recorre vistas, paleta, bandeja y diálogos solo con teclado
- **THEN** toda acción SHALL ser alcanzable y el foco SHALL ser visible en cada paso

#### Scenario: Estados legibles sin color
- **WHEN** se muestra el estado de una sesión o de un permiso
- **THEN** la GUI SHALL renderizar símbolo + palabra además del color

#### Scenario: Movimiento reducido honrado
- **WHERE** el sistema declara preferencia de movimiento reducido
- **THEN** la GUI SHALL suprimir animaciones no esenciales

### Requirement: Internacionalización ES/EN de la superficie de escritorio
La GUI SHALL enrutar toda cadena visible por el catálogo de mensajes ES/EN
(constitución §11), siguiendo el idioma del sistema con override en la
configuración, y un lint MUST rehusar cadenas hardcodeadas en la webview.

#### Scenario: Sin hardcodeo de idioma
- **WHEN** se añade una cadena visible fuera del catálogo de mensajes
- **THEN** el lint SHALL marcarla como fallo

#### Scenario: Override de idioma
- **WHEN** el usuario fija un idioma distinto del sistema en la configuración
- **THEN** la GUI SHALL renderizar sus textos en el idioma elegido

### Requirement: Presupuestos de huella de la GUI
El instalador de la GUI SHALL mantenerse por debajo de 15 MB por plataforma con
verificación bloqueante en el pipeline; el arranque hasta shell interactivo
SHALL quedar por debajo de 1 segundo y la memoria en reposo por debajo de 80 MB
en el hardware de referencia, medidos y publicados por release en la
documentación de QA.

#### Scenario: Gate de tamaño del instalador
- **WHEN** un build de release produce un instalador que excede el presupuesto
- **THEN** el pipeline SHALL fallar el gate de tamaño

#### Scenario: Medición publicada por release
- **WHEN** se publica una release con GUI
- **THEN** las notas de QA SHALL incluir arranque y memoria en reposo medidos por plataforma

### Requirement: Onboarding de primer uso de la GUI
La GUI SHALL mostrar en el primer uso un onboarding ligero, saltable y
re-invocable desde la ayuda, que enseña las vistas, la paleta y la bandeja de
permisos; MUST persistir un flag en el directorio de datos del usuario para no
repetirse y MUST NOT requerir cuenta, red ni telemetría.

#### Scenario: Primer uso enseña el modelo
- **WHEN** se abre la GUI por primera vez
- **THEN** el onboarding SHALL presentar vistas, paleta y bandeja con la opción de saltarlo

#### Scenario: Saltable y persistente
- **WHEN** el usuario descarta el onboarding
- **THEN** el flag SHALL persistir y el onboarding SHALL NOT reaparecer salvo re-invocación desde la ayuda

### Requirement: Arquitectura visual de aplicación de escritorio
El shell SHALL organizarse en tres zonas persistentes: un **sidebar** de
navegación con el proyecto activo arriba (conmutable), las vistas con icono,
etiqueta y contador vivo (sesiones activas, permisos pendientes), y Ajustes
abajo; una **barra superior** con el contexto de la vista, el buscador de la
paleta visible con su atajo, y la acción primaria; y una **barra de estado**
inferior con conexión, versión del daemon, endpoint y resumen de
sesiones/permisos. El keymap vigente (1–4, `:`/Ctrl+K, `a`, `?`, Esc) MUST
conservarse; el sidebar es su representación visible, no su reemplazo.

#### Scenario: Tres zonas presentes
- **WHEN** se abre la superficie de escritorio
- **THEN** el sidebar, la barra superior y la barra de estado SHALL estar presentes en toda vista

#### Scenario: Contadores vivos en el sidebar
- **WHILE** hay sesiones activas o permisos pendientes
- **THEN** los ítems Sesiones y Permisos del sidebar SHALL mostrar sus contadores actualizados

#### Scenario: La barra de estado dice la verdad de conexión
- **WHEN** el daemon está conectado
- **THEN** la barra de estado SHALL mostrar estado, versión y endpoint
- **AND** al perderse la conexión SHALL reflejarlo con la misma prioridad de señales vigente

### Requirement: Densidad y profundidad del design system
Las vistas de datos SHALL usar la escala de elevación del design system
normativo (`design-system/`; página, superficie, flotante — hairlines de 1 px
y un único nivel de sombra reservado a overlays) y tablas densas: filas
compactas (32 px) con jerarquía tipográfica (principal + secundario), hover y
selección distinguibles sin depender solo del color. Los valores categóricos
repetidos (nivel, origen, detección) SHALL presentarse como pills, badges o
dots con palabra — MUST NOT repetirse como texto plano columna abajo. La
bandeja de permisos y los banners de señal MUST NOT animar su layout: nada se
mueve bajo el cursor mientras se decide un permiso.

#### Scenario: Fila con jerarquía y selección visible
- **WHEN** el usuario recorre una tabla con teclado o puntero
- **THEN** hover y selección SHALL distinguirse por superficie y marcador, no solo por color

#### Scenario: Categorías como pills
- **WHEN** la Flota lista el nivel de integración y la detección
- **THEN** el nivel SHALL renderizarse como pill (con su verificación) y la detección como dot + palabra

#### Scenario: La bandeja no se mueve bajo el cursor
- **WHILE** el usuario tiene el puntero sobre una petición de permiso
- **WHEN** llega o vence otra petición
- **THEN** los controles bajo el puntero SHALL NOT desplazarse por animación de layout

### Requirement: Identidad visual de entidades
Cada agente SHALL mostrarse con un avatar de iniciales con color estable
derivado de su id (mismo agente, mismo color, en toda vista y sesión), y los
estados SHALL usar los mismos badges en todas las vistas — una entidad se
reconoce de un vistazo dondequiera que aparezca.

#### Scenario: Avatar estable por id
- **WHEN** el mismo agente aparece en la Flota, en una sesión y en el drawer
- **THEN** su avatar SHALL mostrar las mismas iniciales y el mismo color en los tres lugares

### Requirement: Panel de detalle sin perder la lista
Seleccionar una fila (agente de la Flota, sesión) SHALL abrir un panel de
detalle lateral con la información completa y las acciones aplicables (usar
en este proyecto, correr conformidad; cancelar, dirigir), manteniendo la
lista visible y navegable; Esc SHALL cerrar el panel antes de actuar sobre la
vista.

#### Scenario: Drawer de agente con acciones
- **WHEN** el usuario selecciona un agente detectado en la Flota
- **THEN** el panel SHALL mostrar binario, nivel (declarado vs verificado), MCP y sus acciones

#### Scenario: Esc cierra el panel primero
- **WHEN** el usuario pulsa Esc con el panel de detalle abierto
- **THEN** SHALL cerrarse el panel y la vista SHALL permanecer

### Requirement: Superficie de Ajustes
La superficie SHALL ofrecer Ajustes con casa propia, alcanzable desde el
sidebar, con: tema (claro/oscuro/sistema), idioma (ES/EN), plantilla de
"Abrir con…" (persistida y usada por el deep-link), el visor de la
configuración efectiva del proyecto (`.meltemi/config.toml` y permisos) con
salto directo a editarla en el editor trazable, y la declaración explícita de
que no hay cuentas, red ni telemetría. Los ajustes MUST persistir en el
directorio de datos del usuario.

#### Scenario: La plantilla de Abrir con se configura y persiste
- **WHEN** el usuario define la plantilla de "Abrir con…" en Ajustes y reinicia
- **THEN** el deep-link SHALL usar esa plantilla sin depender de variables de entorno

#### Scenario: Ver y editar la configuración efectiva
- **WHEN** el usuario abre la sección de proyecto en Ajustes
- **THEN** SHALL ver la configuración efectiva y poder saltar a editarla en el editor (guardado trazable)

#### Scenario: La promesa de privacidad es visible
- **WHEN** el usuario abre Ajustes
- **THEN** la declaración sin cuentas, sin red y sin telemetría SHALL estar visible

### Requirement: Identidad visible en toda condición
El chrome SHALL mostrar la marca real del proyecto (mark de `brand/` inline)
junto al wordmark, y los estados vacíos SHALL usar iconografía de línea propia
coherente con el design system — MUST NOT depender de emoji de plataforma. El
wordmark MUST NOT volverse ilegible en ningún modo: WHERE el sistema opera en
forced-colors/alto contraste, la marca SHALL renderizarse con el color de
texto del sistema, nunca transparente.

#### Scenario: Marca presente en el chrome
- **WHEN** se abre la superficie de escritorio
- **THEN** el chrome SHALL mostrar el mark del proyecto junto al wordmark

#### Scenario: Alto contraste no borra la marca
- **WHERE** el sistema opera en modo forced-colors
- **THEN** el wordmark SHALL renderizarse con el color de texto del sistema
- **AND** SHALL NOT quedar transparente ni depender del gradiente

#### Scenario: Estados vacíos sin emoji de plataforma
- **WHEN** una vista renderiza su estado vacío
- **THEN** el glifo SHALL provenir del set de iconos de línea del design system

### Requirement: Paleta con difusa, grupos, recientes y formularios tipados
La paleta SHALL encontrar entradas por coincidencia difusa de subsecuencia
(con bonus por prefijo de palabra y segmento del método), SHALL agrupar las
entradas por dominio del contrato, SHALL subir primero las usadas
recientemente (persistidas localmente) y SHALL mostrar los atajos de teclado
aplicables. WHERE el método tiene parámetros tipados en los schemas del
contrato, la paleta SHALL ofrecer un formulario generado en build desde
`proto/schemas/v1` — con los campos obligatorios marcados — y el JSON crudo
como modo avanzado; la generación MUST verificarse fresca en CI, de modo que
un cambio del contrato sin regenerar falle el build.

#### Scenario: Subsecuencia encuentra el método
- **WHEN** el usuario teclea "wapp" en la paleta
- **THEN** `worktree/apply-edit` SHALL aparecer entre los primeros resultados

#### Scenario: Recientes primero
- **WHEN** el usuario invoca un método y reabre la paleta
- **THEN** ese método SHALL aparecer al tope de su listado

#### Scenario: Formulario tipado con obligatorios marcados
- **WHEN** el usuario selecciona un método con `Params` en el schema
- **THEN** la paleta SHALL renderizar campos tipados con los `required` marcados
- **AND** SHALL conservar el modo JSON crudo como alternativa

#### Scenario: La frescura del generador es un gate
- **WHEN** el contrato cambia y los formularios generados no se regeneran
- **THEN** la verificación de frescura de CI SHALL fallar

### Requirement: La sesión como acción primaria; proponer como herramienta
La superficie SHALL exponer "Nueva sesión" como acción primaria del chrome —
a un clic y con atajo — abriendo un lanzador sobre los métodos existentes:
elegir agente o perfil de la flota y el modo (explorar, proponer, despachar
una tarea, dirigir una sesión existente), sin introducir métodos nuevos.
`propose` MUST seguir alcanzable a una tecla (paleta y vista Proyecto) como
herramienta, pero MUST NOT ser la única entrada visible al trabajo. Todo
estado vacío SHALL ofrecer su siguiente paso como control ejecutable y
enfocable, nunca solo como texto (Proyecto sin `.meltemi/` SHALL ofrecer
inicializar la constitución; Flota sin agentes SHALL ofrecer refrescar la
detección).

#### Scenario: Nueva sesión desde el chrome
- **WHEN** el usuario activa la acción primaria del chrome
- **THEN** el lanzador SHALL ofrecer agente/perfil y modo (explorar, proponer, despachar, dirigir)
- **AND** SHALL invocar únicamente métodos existentes del contrato

#### Scenario: Proponer sigue a una tecla
- **WHEN** el usuario abre la paleta o la vista Proyecto
- **THEN** `propose` SHALL estar disponible como herramienta directa

#### Scenario: Inicializar desde el vacío de Proyecto
- **WHEN** la vista Proyecto muestra que el directorio no es un proyecto
- **THEN** SHALL ofrecer un control ejecutable que inicia `sdd/constitution`

#### Scenario: Flota vacía ofrece refrescar
- **WHEN** la vista Flota no detecta agentes
- **THEN** SHALL ofrecer un control que reejecuta la detección

### Requirement: Sesiones filtrables con tiempo humano y acciones por fila
La vista Sesiones SHALL ofrecer filtro por tecleo (`/`), orden por columna,
chips de resumen por estado, y en cada fila las acciones aplicables (cancelar
con confirmación; dirigir una instrucción). Los tiempos SHALL mostrarse
relativos y localizados, con el instante absoluto accesible (title/aria).

#### Scenario: Filtrar por agente
- **WHEN** el usuario pulsa `/` y teclea el nombre de un agente
- **THEN** la tabla SHALL reducirse a las sesiones cuyo agente o id coincide

#### Scenario: Tiempo relativo con absoluto accesible
- **WHEN** la tabla muestra el inicio de una sesión
- **THEN** SHALL renderizar el tiempo relativo localizado
- **AND** el instante absoluto SHALL quedar accesible en el mismo elemento

#### Scenario: Cancelar desde la fila
- **WHEN** el usuario activa cancelar en la fila de una sesión activa
- **THEN** la superficie SHALL pedir la confirmación vigente y enviar `session/cancel`

### Requirement: Transcript de primera clase
El transcript SHALL renderizar cada evento con glifo y tono según su tipo
(los tipos desconocidos caen a neutro con su nombre crudo), SHALL permitir
expandir el texto completo de un evento, conmutar timestamps, copiar una línea
o todo lo cargado, y buscar dentro de lo cargado.

#### Scenario: Evento con texto expandible
- **WHEN** un evento con texto excede la línea
- **THEN** el usuario SHALL poder expandirlo completo en su lugar

#### Scenario: Buscar en el transcript
- **WHEN** el usuario busca un término en el transcript
- **THEN** las líneas coincidentes SHALL resaltarse y ser navegables

#### Scenario: Tipo desconocido no rompe
- **WHEN** llega un evento de tipo no catalogado
- **THEN** SHALL renderizarse en tono neutro con su nombre crudo

### Requirement: Ninguna pérdida silenciosa de edición
Cerrar una pestaña con cambios sin guardar, desmontar el editor por
navegación o cerrar la ventana con sucios abiertos MUST exigir una decisión
explícita (guardar / descartar / cancelar) con la opción no destructiva como
predeterminada; guardar SHALL pasar por el flujo `apply-edit` vigente con su
política de bloqueo suave. El editor SHALL ofrecer quick-open (Ctrl+P) sobre
el árbol del proyecto y archivos recientes por proyecto.

#### Scenario: Cerrar pestaña sucia pide decisión
- **WHEN** el usuario cierra una pestaña con cambios sin guardar
- **THEN** la superficie SHALL exigir guardar, descartar o cancelar
- **AND** SHALL NOT descartar en silencio

#### Scenario: Salir del editor con sucios pide decisión
- **WHEN** el usuario navega fuera del editor con pestañas sucias
- **THEN** la superficie SHALL exigir la misma decisión antes de desmontar

#### Scenario: Cerrar la ventana con sucios pide decisión
- **WHEN** el usuario cierra la ventana con pestañas sucias
- **THEN** el cierre SHALL retenerse hasta la decisión explícita

#### Scenario: Quick-open por nombre
- **WHEN** el usuario pulsa Ctrl+P en el editor y teclea parte de un nombre
- **THEN** el archivo SHALL poder abrirse sin tocar el árbol

### Requirement: Tema y estado de ventana persistentes
La superficie SHALL ofrecer selector de tema claro/oscuro/sistema y SHALL
persistir tema, geometría y estado de ventana y última vista en el directorio
de datos del usuario, restaurándolos al abrir; IF la geometría guardada ya no
es visible (monitores cambiados), SHALL caer a los valores por defecto. El
chrome SHALL mostrar un indicio visible del atajo de la paleta.

#### Scenario: El tema sobrevive al reinicio
- **WHEN** el usuario fija el tema oscuro y reabre la aplicación
- **THEN** la superficie SHALL abrir en oscuro sin destello del tema contrario

#### Scenario: Ventana restaurada con regla de visibilidad
- **WHEN** la aplicación reabre tras un cambio de monitores que deja la
  geometría guardada fuera de pantalla
- **THEN** la ventana SHALL abrir con los valores por defecto

#### Scenario: El atajo de la paleta es visible
- **WHEN** el usuario observa el chrome
- **THEN** SHALL existir un indicio visible del atajo que abre la paleta

### Requirement: Atención del sistema ante permisos sin foco
WHEN llega o vence un permiso pendiente y la ventana no tiene el foco, la
superficie SHALL solicitar la atención del sistema mediante la API de ventana
(parpadeo/bounce/urgencia según SO — local, sin red ni sonido propio) y el
título de la ventana SHALL reflejar el contador de pendientes; al recuperar el
foco, la atención SHALL limpiarse y el título volver a su forma base.

#### Scenario: Permiso sin foco reclama atención
- **WHEN** llega un permiso pendiente con la ventana desenfocada
- **THEN** la superficie SHALL solicitar la atención del sistema
- **AND** el título SHALL incluir el contador de pendientes

#### Scenario: El foco limpia la señal
- **WHEN** el usuario devuelve el foco a la ventana
- **THEN** la atención del sistema SHALL cesar y el título SHALL normalizarse

### Requirement: Avisos con memoria acotada y banner accionable
Los avisos SHALL llevar marca temporal relativa y un tope visible —los
excedentes colapsan a un contador con historial consultable—, y el banner de
daemon inalcanzable SHALL ofrecer "reintentar ahora" y "copiar diagnóstico"
(estado, endpoint y detalle) además del reintento automático.

#### Scenario: Overflow de avisos colapsa con historial
- **WHEN** los avisos superan el tope visible
- **THEN** los anteriores SHALL colapsar a un contador
- **AND** SHALL poder consultarse completos desde ese control

#### Scenario: Copiar el diagnóstico de conexión
- **WHEN** el usuario activa "copiar diagnóstico" en el banner
- **THEN** el portapapeles SHALL recibir estado, endpoint y detalle del fallo

### Requirement: Árbol Proyecto → Sesiones en el sidebar
El sidebar SHALL presentar el árbol de los proyectos conocidos con sus sesiones,
y cada sesión SHALL mostrar el avatar de su agente, el nombre de su suscripción
como pill y su badge de estado, honrando la densidad y la escala de elevación del
design system normativo (`design-system/`). El árbol MUST NOT animar su layout
cuando llegan o terminan sesiones: nada se mueve bajo el cursor. WHERE la raíz de
un proyecto ya no existe en disco, su nodo SHALL mostrarse marcado como ausente
con su remedio y MUST NOT desaparecer sin aviso.

#### Scenario: Árbol con dos proyectos y sus sesiones
- **WHEN** hay sesiones en dos proyectos distintos
- **THEN** el sidebar SHALL mostrar un nodo por proyecto con sus sesiones debajo
- **AND** cada sesión SHALL mostrar agente y suscripción sin abrir el detalle

#### Scenario: Dos suscripciones del mismo agente distinguibles
- **WHEN** un proyecto tiene dos sesiones del mismo agente lanzadas con perfiles distintos
- **THEN** ambas SHALL mostrar el mismo avatar de agente
- **AND** SHALL distinguirse por el nombre de su suscripción

#### Scenario: Proyecto ausente en disco marcado en el árbol
- **WHEN** el árbol incluye un proyecto cuya raíz ya no existe
- **THEN** su nodo SHALL renderizarse marcado como ausente con símbolo y palabra
- **AND** SHALL ofrecer el siguiente paso en vez de desaparecer

#### Scenario: El árbol no salta bajo el cursor
- **WHILE** el usuario tiene el puntero sobre una sesión del árbol
- **WHEN** otra sesión arranca o termina
- **THEN** los controles bajo el puntero SHALL NOT desplazarse por animación de layout

### Requirement: Ámbito de proyecto conmutable y persistente
El conmutador de proyecto del sidebar SHALL fijar el proyecto activo de la
superficie a partir de los proyectos conocidos; el directorio de trabajo del
proceso SHALL ser únicamente el ámbito inicial por omisión y MUST NOT seguir
siendo una jaula. Toda invocación de un método con ámbito de proyecto SHALL usar
el proyecto activo, el ámbito activo MUST persistir en el directorio de datos del
usuario, y sin proyecto activo las vistas Sesiones, Permisos y Flota MUST seguir
plenamente usables.

#### Scenario: Conmutar el proyecto activo reenfoca las vistas
- **WHEN** el usuario elige otro proyecto en el conmutador del sidebar
- **THEN** las vistas con ámbito de proyecto SHALL consultar el nuevo proyecto activo
- **AND** el chrome SHALL reflejar el ámbito vigente

#### Scenario: El ámbito activo sobrevive al reinicio
- **WHEN** el usuario fija un proyecto activo distinto del cwd y reabre la aplicación
- **THEN** la superficie SHALL restaurar ese proyecto como activo

#### Scenario: Sin proyecto activo las vistas globales siguen vivas
- **WHERE** no hay proyecto activo seleccionado
- **THEN** Sesiones, Permisos y Flota SHALL seguir operativas
- **AND** solo lo que exige un proyecto SHALL mostrar su estado vacío con el siguiente paso

### Requirement: Selector de proyecto en el lanzador de sesión
El lanzador de "Nueva sesión" SHALL ofrecer, además de agente o perfil y modo, la
elección del proyecto entre los conocidos, con el proyecto activo preseleccionado,
y MUST NOT introducir métodos nuevos del contrato para lanzar el trabajo.

#### Scenario: Lanzar en otro proyecto sin conmutar antes
- **WHEN** el usuario abre el lanzador y elige un proyecto distinto del activo
- **THEN** la sesión SHALL lanzarse sobre la raíz de ese proyecto
- **AND** SHALL invocar únicamente métodos existentes del contrato

#### Scenario: Suscripción elegible por nombre en el lanzador
- **WHEN** el usuario despliega la selección de agente en el lanzador
- **THEN** los perfiles SHALL ofrecerse por su nombre de suscripción junto a su agente subyacente
