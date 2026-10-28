

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
El instalador de la GUI SHALL mantenerse por debajo de 15 MB en toda plataforma
con verificación bloqueante en el pipeline, sostenido por no embeber motor de
navegador en artefacto alguno; el arranque hasta shell interactivo SHALL quedar
por debajo de 1 segundo y la memoria en reposo por debajo de 80 MB en el hardware
de referencia, medidos y publicados por release en la documentación de QA. La
medición publicada SHALL cubrir el instalador de cada plataforma que la release
publique.

#### Scenario: Gate de tamaño del instalador
- **WHEN** un build de release produce un instalador que excede el presupuesto
- **THEN** el pipeline SHALL fallar el gate de tamaño

#### Scenario: Medición publicada por release
- **WHEN** se publica una release con GUI
- **THEN** las notas de QA SHALL incluir arranque y memoria en reposo medidos por plataforma

#### Scenario: Tamaño de instalador medido por plataforma publicada
- **WHEN** se publica una release con GUI
- **THEN** las notas de QA SHALL registrar el tamaño medido del instalador de cada plataforma publicada
- **AND** SHALL NOT declarar como medido un tamaño que nadie midió

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

### Requirement: Lienzo completo del shell
La columna central del shell SHALL repartir el alto de la ventana entre sus
barras (banner de daemon, barra superior, avisos) a su altura natural y la
vista enrutada, que SHALL ocupar todo el resto hasta la barra de estado
anclada al borde inferior, con independencia de cuántas barras condicionales
estén presentes. Ninguna fila de los árboles o listas del shell SHALL
renderizarse por debajo de su altura de línea.

#### Scenario: La vista ocupa el alto disponible
- **WHEN** el shell se renderiza con el daemon conectado y sin avisos activos
- **THEN** la vista enrutada SHALL extenderse hasta la barra de estado
- **AND** la barra de estado SHALL quedar anclada al borde inferior de la ventana

#### Scenario: Filas del árbol sin recorte
- **WHEN** el árbol del proyecto renderiza más filas de las que caben en su panel
- **THEN** cada fila SHALL conservar al menos su altura de línea
- **AND** el excedente SHALL desplazarse con scroll, nunca comprimirse

### Requirement: Alineación global de los controles con icono
El skin compartido de botones SHALL disponer icono y etiqueta en una sola
línea, con el icono centrado verticalmente respecto del texto y separación
uniforme, provisto por la regla global del skin: un componente MUST NOT
necesitar re-declarar la alineación en su hoja local para que un botón con
icono y etiqueta se renderice correcto. Las acciones de un estado vacío SHALL
renderizarse cada una a su altura natural y MUST NOT estirarse para igualar
la altura de otra, tampoco cuando la fila envuelve.

#### Scenario: Icono y etiqueta en una línea
- **WHEN** un botón compone un icono y una etiqueta sin reglas de layout locales
- **THEN** ambos SHALL renderizarse en una sola línea
- **AND** el icono SHALL quedar centrado verticalmente respecto del texto

#### Scenario: Par de acciones del estado vacío a altura pareja
- **WHEN** un estado vacío ofrece dos acciones y la fila las envuelve
- **THEN** cada acción SHALL conservar su altura natural
- **AND** ninguna SHALL estirarse para igualar la altura de la otra

### Requirement: Etiquetas de acción sin atajo incrustado
Las cadenas del catálogo de mensajes MUST NOT incrustar la pista de un atajo
de teclado como texto plano dentro de una etiqueta de acción — un número
entre paréntesis se lee como contador vivo que no es; el atajo SHALL
mostrarse únicamente en su afordancia dedicada del chrome (`kbd`).

#### Scenario: La acción de flota sin falso contador
- **WHEN** el estado vacío de Sesiones ofrece la acción de ir a la Flota
- **THEN** su etiqueta SHALL leerse sin número entre paréntesis
- **AND** SHALL NOT sugerir un recuento de agentes

#### Scenario: El atajo conserva su afordancia
- **WHEN** el usuario observa el sidebar con el estado vacío de Sesiones en pantalla
- **THEN** el ítem Flota SHALL seguir mostrando su atajo como `kbd`

### Requirement: Tablero de carrera

La superficie de escritorio SHALL presentar, por cambio y tarea, el tablero
de la carrera: las calles de los competidores lado a lado, cada una con su
procedencia visible (agente y perfil/suscripción cuando aplique), su diff
contra la base, su estado de turno, commit y checkpoint declarado con
señal más palabra, y las acciones de la carrera (despachar un turno,
revertir al checkpoint, commit de la tarea, merge asistido por archivo).
Toda acción destructiva MUST exigir confirmación explícita antes de
ejecutarse. El tablero SHALL reflejar únicamente estado persistido o
derivado del daemon — nunca estado inventado: una calle sin procedencia
registrada se muestra sin procedencia. Al concluir un turno despachado
desde la propia superficie, el tablero SHALL actualizarse sin recargar la
aplicación.

#### Scenario: Calles lado a lado con procedencia visible

- **WHEN** el usuario abre el tablero de una tarea con competidores
- **THEN** cada calle SHALL mostrar su agente, su perfil cuando lo hubo,
  su estado y su diff contra la base
- **AND** una calle sin procedencia registrada SHALL mostrarse sin
  procedencia, con ausencia visible

#### Scenario: Acción destructiva solo con confirmación explícita

- **WHEN** el usuario invoca revertir una calle desde el tablero
- **THEN** la superficie SHALL exigir confirmación explícita antes de
  enviar la operación
- **AND** cancelar la confirmación MUST NOT enviar nada al daemon

#### Scenario: El tablero refleja el turno concluido

- **WHILE** un despacho lanzado desde la propia superficie corre su turno
- **WHEN** el turno concluye
- **THEN** el tablero SHALL actualizar la calle afectada sin recargar la
  aplicación

#### Scenario: Carrera sin competidores, estado vacío honesto

- **IF** la tarea no tiene worktrees de competidores
- **THEN** el tablero SHALL mostrar un estado vacío que lo diga
- **AND** SHALL ofrecer el camino para asignar la carrera, no un tablero en
  blanco

### Requirement: Vincular suscripciones desde la Flota

La ficha del agente en la vista de Flota SHALL ofrecer vincular una
suscripción cuando la entrada declara su variable de contexto, pidiendo
únicamente el nombre del vínculo; al crearse, el gesto de login compuesto
SHALL quedar visible con su acción de copiar. Los vínculos SHALL poder
desvincularse desde la misma ficha, y la superficie SHALL decir que
desvincular no borra el contexto de autenticación. Una entrada sin variable
declarada MUST NOT ofrecer el flujo y SHALL señalar la vía manual.

#### Scenario: Vincular desde la ficha del agente

- **WHEN** el usuario vincula una suscripción desde la ficha de un agente con
  variable declarada
- **THEN** la Flota SHALL listar la fila nueva del perfil sin recargar la
  aplicación
- **AND** el formulario SHALL haber pedido solo el nombre

#### Scenario: El gesto de login queda a un clic de copiar

- **WHEN** el vínculo se crea
- **THEN** la ficha SHALL mostrar el gesto de autenticación compuesto
- **AND** SHALL ofrecer copiarlo con la acción de copia existente

#### Scenario: La entrada sin variable señala la vía manual

- **WHEN** la ficha muestra una entrada sin variable de contexto declarada
- **THEN** la superficie MUST NOT ofrecer el flujo de vincular
- **AND** SHALL señalar la vía manual documentada

#### Scenario: Desvincular dice lo que no borra

- **WHEN** el usuario desvincula desde la ficha
- **THEN** la superficie SHALL declarar que el directorio de contexto queda
  intacto
- **AND** la fila del perfil SHALL desaparecer de la Flota

### Requirement: Superficies flotantes con fondo propio

Toda superficie flotante de la GUI —panel, diálogo, cajón o menú que cubra
contenido— SHALL pintar un fondo opaco, de modo que lo que queda debajo no se
lea a través de ella. Los valores de estilo SHALL provenir de variables
definidas: la suite de la superficie de escritorio MUST fallar, nombrando
archivo y línea, cuando una variable de estilo se use sin estar definida.

#### Scenario: Ninguna variable de estilo se usa sin existir

- **WHEN** la suite de la superficie de escritorio se ejecuta
- **THEN** SHALL reunir cada variable de estilo usada y cada una definida
- **AND** SHALL fallar nombrando archivo y línea de cualquiera usada sin
  definición

#### Scenario: El conmutador de proyectos cubre lo que tapa

- **WHEN** el conmutador de proyectos se abre sobre la navegación
- **THEN** su panel SHALL declarar un fondo desde una variable definida
- **AND** el contenido de debajo MUST NOT leerse a través del panel

### Requirement: Barra lateral plegable

La barra de navegación SHALL poder plegarse a un riel angosto y desplegarse
desde un control visible en su cabecera, sin perder entrada alguna: plegada,
cada entrada SHALL conservar su etiqueta accesible y su acceso por teclado, y
el indicador de permisos pendientes SHALL permanecer visible. El estado
plegado SHALL recordarse entre arranques junto a las demás preferencias de
ventana, y un perfil nuevo SHALL arrancar desplegado.

#### Scenario: Plegar y desplegar desde la cabecera

- **WHEN** el usuario acciona el control de pliegue
- **THEN** la barra SHALL pasar a su riel angosto
- **AND** accionarlo de nuevo SHALL devolverla a su ancho completo

#### Scenario: Plegada no pierde alcance

- **WHILE** la barra está plegada
- **THEN** cada entrada de navegación SHALL conservar su etiqueta accesible
- **AND** el indicador de permisos pendientes SHALL seguir visible

#### Scenario: El pliegue se recuerda, el primer arranque no

- **WHEN** el usuario pliega la barra y vuelve a abrir la aplicación
- **THEN** la barra SHALL aparecer plegada
- **AND** un perfil sin preferencia guardada SHALL arrancar desplegado

### Requirement: Reparto ajustable entre la navegación y los proyectos

La barra lateral SHALL repartir su alto entre las entradas de navegación y el
árbol de proyectos por un separador que el usuario puede mover, operable con
puntero y con teclado, con nombre accesible y con su valor y sus límites
declarados. El reparto SHALL respetar un mínimo por cada zona; cuando la barra
no dé para ambos mínimos, las entradas SHALL conservar el suyo y el árbol
SHALL desplazarse con scroll. El reparto SHALL recordarse entre arranques junto
a las demás preferencias de ventana, y un perfil nuevo SHALL arrancar en el
reparto por defecto. Plegada la barra a su riel, el separador NO SHALL
renderizarse ni el reparto guardado SHALL aplicarse.

#### Scenario: Arrastrar la línea reparte el alto

- **WHEN** el puntero arrastra el separador entre las entradas y el árbol
- **THEN** las entradas SHALL tomar el alto arrastrado
- **AND** el árbol SHALL tomar el resto
- **AND** ningún control SHALL desplazarse por animación de layout

#### Scenario: El reparto tiene suelo por los dos lados

- **WHEN** se pide un reparto que dejaría a una de las dos zonas por debajo de
  su mínimo
- **THEN** el reparto SHALL acotarse a ese mínimo
- **AND** en una barra demasiado corta para ambos mínimos las entradas SHALL
  conservar el suyo y el árbol SHALL desplazarse con scroll

#### Scenario: El reparto se ajusta con el teclado

- **WHEN** el separador tiene el foco
- **THEN** las flechas SHALL moverlo un paso y Home y End SHALL llevarlo a sus
  extremos
- **AND** el separador SHALL exponer su rol, su nombre accesible y su valor
  actual con sus límites

#### Scenario: El reparto se recuerda, el primer arranque no

- **WHEN** la superficie se reabre tras haberse movido el separador
- **THEN** SHALL restaurar el reparto guardado junto a las demás preferencias
  de ventana
- **AND** un perfil sin preferencia guardada SHALL arrancar en el reparto por
  defecto

#### Scenario: Una ventana más pequeña no deja el reparto inservible

- **WHEN** la barra se reduce por debajo del reparto recordado
- **THEN** el reparto SHALL reajustarse a lo que la barra puede dar
- **AND** ninguna de las dos zonas SHALL quedar por debajo de su mínimo

#### Scenario: Plegada la barra, no hay reparto que hacer

- **WHILE** la barra lateral está plegada a su riel
- **THEN** el separador NO SHALL renderizarse
- **AND** el reparto guardado NO SHALL aplicarse, y SHALL volver intacto al
  desplegar

#### Scenario: Ninguna entrada se pierde al encoger la navegación

- **WHEN** el reparto deja a las entradas menos alto del que ocupan
- **THEN** el excedente SHALL desplazarse con scroll dentro de la navegación
- **AND** cada entrada SHALL conservar su etiqueta accesible, su foco y su
  dígito

### Requirement: Barras de desplazamiento de la superficie

Toda región desplazable de la GUI SHALL presentar una barra de desplazamiento
angosta y sin botones de paso, con su color tomado de los mismos tokens que el
resto del cromo, de modo que siga al tema sin una declaración por tema ni por
región. El ancho y el color SHALL declararse con propiedades estándar de CSS y
esas propiedades SHALL bastar por sí solas para la barra angosta; los
selectores específicos de motor MAY usarse **únicamente** para retirar cromo
que ninguna propiedad estándar retira, y NO SHALL ser el único portador del
ancho ni del color. Estrechar la barra MUST NOT comprimir ninguna fila: el
excedente SHALL seguir desplazándose.

#### Scenario: El árbol de proyectos desplaza sin comerse la columna

- **WHEN** el árbol de proyectos renderiza más filas de las que caben
- **THEN** su barra de desplazamiento SHALL ser angosta y sin botones de paso
- **AND** cada fila SHALL conservar al menos su altura de línea
- **AND** el excedente SHALL desplazarse, nunca comprimirse

#### Scenario: La barra sigue el tema sin una segunda declaración

- **WHEN** el usuario conmuta entre tema claro y tema oscuro
- **THEN** la barra SHALL tomar su color del mismo token que el resto del cromo
- **AND** NO SHALL existir una regla de barra por tema ni por región

### Requirement: Varias sesiones abiertas a la vez en pestañas

La vista de sesiones SHALL permitir varias sesiones abiertas simultáneamente en
una tira de pestañas que contiene únicamente sesiones abiertas y, cuando se
pide una sesión nueva, su compositor: la lista es la vista Sesiones, no una
pestaña, y SHALL estar en pantalla cuando ninguna pestaña está al frente. Abrir una sesión ya abierta SHALL enfocar su pestaña
sin crear otra. Una pestaña que no está en pantalla SHALL conservar su
transcript, su lectura y su borrador sin enviar, y SHALL declarar cuántos
eventos llegaron sin leer. Cada pestaña SHALL declarar el estado de su sesión
con símbolo y palabra, nunca con color solo. La tira SHALL recorrerse entera
con el teclado. Alcanzado el tope de pestañas, la superficie SHALL rehusar
abrir otra sin cerrar ninguna y SHALL nombrar el remedio. Cada sesión abierta
SHALL declarar su interés por su propia sesión y NO SHALL mostrar los eventos
de otra.

#### Scenario: Abrir una segunda sesión no reemplaza la primera

- **WHEN** se abre una sesión estando otra abierta
- **THEN** ambas SHALL quedar abiertas como pestañas
- **AND** la recién abierta SHALL quedar enfocada
- **AND** la anterior SHALL conservar su lectura

#### Scenario: Abrir dos veces la misma sesión enfoca, no duplica

- **WHEN** se pide abrir una sesión que ya tiene pestaña
- **THEN** el foco SHALL moverse a esa pestaña
- **AND** NO SHALL crearse una segunda
- **AND** su contador de eventos sin leer SHALL volver a cero

#### Scenario: La lista es la vista, no una pestaña

- **WHEN** hay al menos una sesión abierta
- **THEN** la tira SHALL contener solo pestañas de sesión, sin una pestaña de
  lista
- **AND** la entrada Sesiones de la barra SHALL devolver la tabla completa con
  su filtro dejando las pestañas abiertas

#### Scenario: Una pestaña de fondo conserva su lectura y su borrador

- **WHEN** se conmuta a otra pestaña y se vuelve
- **THEN** el transcript, la lectura elegida y el borrador sin enviar SHALL
  estar como se dejaron
- **AND** si se estaba al pie SHALL volver al pie

#### Scenario: La pestaña de fondo dice que llegó algo

- **WHEN** una sesión que no está en pantalla recibe eventos
- **THEN** su pestaña SHALL declarar cuántos llegaron sin leer
- **AND** enfocarla SHALL poner ese contador a cero

#### Scenario: El estado de cada pestaña se lee sin color

- **WHEN** una pestaña representa una sesión activa, una esperando permiso o una
  finalizada
- **THEN** SHALL declarar ese estado con símbolo y palabra
- **AND** el color NO SHALL ser el único portador

#### Scenario: La tira se recorre entera con el teclado

- **WHEN** la tira de pestañas tiene el foco
- **THEN** las flechas SHALL moverse entre pestañas con ciclo y Home y End SHALL
  ir a los extremos
- **AND** cada pestaña SHALL nombrar el panel que controla

#### Scenario: Cerrar la pestaña activa cae en la vecina

- **WHEN** se cierra la pestaña que está en pantalla
- **THEN** el foco SHALL pasar a la vecina de la izquierda, o a la última si no
  la hay
- **AND** si no queda ninguna SHALL quedar seleccionada la lista

#### Scenario: El tope se rehúsa nombrando el remedio

- **WHEN** se pide abrir una sesión más habiendo alcanzado el tope
- **THEN** la superficie SHALL rehusar sin cerrar ninguna pestaña
- **AND** SHALL decir cuántas caben y qué hacer para abrir otra

#### Scenario: Cambiar de vista no cierra las pestañas; reiniciar sí las olvida

- **WHEN** se navega a otra vista de primer nivel y se vuelve a Sesiones
- **THEN** las pestañas SHALL seguir abiertas
- **AND** tras reiniciar la superficie SHALL arrancar sin pestañas de sesión,
  con la lista en pantalla si la vista recordada era Sesiones

#### Scenario: Cada sesión abierta lee su propio registro y su propio flujo

- **WHEN** varias sesiones están abiertas a la vez
- **THEN** cada una SHALL declarar su interés por su propia sesión y SHALL
  sembrar su transcript desde el registro persistido
- **AND** ninguna SHALL mostrar los eventos de otra

### Requirement: Avisos transitorios y avisos que se quedan

Los avisos SHALL distinguirse por consecuencia. Un aviso informativo —el que
confirma algo que el usuario acaba de hacer— SHALL retirarse solo tras un plazo
breve, y SHALL poder retirarse antes con su control. Un aviso de advertencia o
de error SHALL permanecer hasta que el usuario lo retire, y NO SHALL existir
plazo alguno capaz de retirarlo: la obligación de que un vencimiento o un error
no se descarte en silencio se conserva intacta. Mientras el puntero o el foco
estén sobre un aviso transitorio, su plazo SHALL detenerse, y al salir SHALL
reiniciarse.

#### Scenario: La confirmación se retira sola

- **WHEN** una operación informa de su éxito
- **THEN** su aviso SHALL retirarse solo tras un plazo breve
- **AND** SHALL poder retirarse antes desde su control

#### Scenario: El error se queda hasta que alguien lo retira

- **WHEN** el aviso es de advertencia o de error
- **THEN** NO SHALL existir plazo capaz de retirarlo
- **AND** SHALL permanecer hasta que el usuario lo retire

#### Scenario: Nada desaparece bajo la mano que iba a leerlo

- **WHILE** el puntero o el foco están sobre un aviso transitorio
- **THEN** su plazo SHALL detenerse
- **AND** al salir SHALL reiniciarse

### Requirement: Ninguna superficie flotante se desplaza de lado

Los paneles, cajones y diálogos de la GUI SHALL desplazarse solo en vertical: su
contenido SHALL partirse para caber en el ancho disponible en vez de producir
una barra de desplazamiento horizontal.

#### Scenario: El cajón parte la ruta larga en vez de desplazarla

- **WHEN** un cajón muestra un contenido más ancho que su panel
- **THEN** el contenido SHALL partirse para caber
- **AND** NO SHALL aparecer una barra de desplazamiento horizontal

### Requirement: Todo velo cierra lo que cubre

Toda superficie que se presente sobre un velo SHALL cerrarse al activar el velo,
además de con la tecla de escape. La suite de la superficie de escritorio MUST
fallar, nombrando el componente, cuando exista un velo sin manejador de cierre.

#### Scenario: Hacer clic fuera cierra la paleta

- **WHEN** el usuario hace clic fuera del área de la paleta de comandos
- **THEN** la paleta SHALL cerrarse

#### Scenario: Ningún velo queda sin cierre

- **WHEN** la suite de la superficie de escritorio se ejecuta
- **THEN** SHALL reunir todo velo de la superficie
- **AND** SHALL fallar nombrando el componente de cualquiera sin manejador de
  cierre

### Requirement: La flota se lee por agente y por suscripción

La vista Flota SHALL presentar cada agente del catálogo seguido de las
suscripciones enlazadas a él, y cada fila de suscripción SHALL declarar **como
texto** de qué agente lo es. Cada agente con suscripciones SHALL declarar
cuántas tiene. Una suscripción cuyo agente subyacente no esté en el catálogo NO
SHALL desaparecer del listado: SHALL mostrarse marcada, con el identificador que
declara.

#### Scenario: Varias suscripciones del mismo agente se leen juntas

- **WHEN** un agente del catálogo tiene varias suscripciones enlazadas
- **THEN** SHALL presentarse seguido de todas ellas
- **AND** cada una SHALL declarar como texto de qué agente lo es
- **AND** el agente SHALL declarar cuántas tiene

#### Scenario: La suscripción sin agente conocido no desaparece

- **WHEN** una suscripción declara un agente que no está en el catálogo
- **THEN** SHALL listarse igualmente, marcada
- **AND** SHALL mostrar el identificador que declara

#### Scenario: La relación no depende de la sangría

- **WHEN** una fila de suscripción se lee como texto
- **THEN** el agente al que pertenece SHALL estar en su contenido
- **AND** NO SHALL depender de su posición ni de su sangría

### Requirement: La tira de pestañas es una sola fila que se desplaza

La tira de pestañas SHALL presentarse en una sola fila y NO SHALL envolver a un
segundo renglón. Cuando las pestañas no quepan, SHALL encogerse hasta un ancho
mínimo legible y, pasado ese punto, la tira SHALL desplazarse en horizontal.
Los controles de desplazamiento SHALL existir únicamente mientras haya
desbordamiento, y cada uno SHALL deshabilitarse en su extremo en vez de
desaparecer.

#### Scenario: Muchas pestañas no producen un segundo renglón

- **WHEN** hay más pestañas de las que caben en el ancho disponible
- **THEN** la tira SHALL seguir siendo una sola fila
- **AND** las pestañas SHALL encogerse hasta su ancho mínimo antes de que la
  tira se desplace

#### Scenario: Los controles aparecen solo cuando sobran pestañas

- **WHEN** las pestañas caben en el ancho disponible
- **THEN** NO SHALL renderizarse control de desplazamiento alguno
- **WHEN** dejan de caber
- **THEN** SHALL renderizarse los dos controles, cada uno deshabilitado en su
  extremo

#### Scenario: La pestaña activa nunca queda fuera de vista

- **WHEN** la pestaña activa cambia y queda fuera del área visible de la tira
- **THEN** la tira SHALL desplazarse lo mínimo para mostrarla
- **AND** una pestaña ya visible NO SHALL provocar desplazamiento

### Requirement: Grupos de pestañas

Las pestañas SHALL poder agruparse bajo un nombre y un color. Una pestaña SHALL
pertenecer a lo sumo a un grupo, y un grupo sin pestañas SHALL dejar de existir.
El nombre del grupo SHALL viajar en el nombre accesible de cada pestaña que le
pertenece: el color NO SHALL ser el único portador de la pertenencia. Plegar un
grupo NO SHALL cerrar ninguna pestaña ni descartar su trabajo, y el grupo
plegado SHALL declarar como texto cuántas guarda. WHERE la pestaña activa
pertenezca a un grupo que se pliega, la actividad SHALL pasar a una pestaña
visible.

#### Scenario: Una pestaña pertenece a un grupo y lo dice

- **WHEN** una pestaña se une a un grupo
- **THEN** su nombre accesible SHALL incluir el nombre del grupo
- **AND** la pertenencia NO SHALL depender solo del color

#### Scenario: Salir del grupo y el grupo que se queda vacío

- **WHEN** la última pestaña de un grupo lo abandona o se cierra
- **THEN** el grupo SHALL dejar de existir
- **AND** la pestaña SHALL seguir abierta

#### Scenario: Plegar guarda espacio, no trabajo

- **WHEN** un grupo se pliega
- **THEN** SHALL declarar como texto cuántas pestañas guarda
- **AND** ninguna pestaña SHALL cerrarse ni perder su borrador

#### Scenario: Plegar el grupo de la pestaña activa mueve la actividad

- **WHEN** se pliega el grupo al que pertenece la pestaña activa
- **THEN** la actividad SHALL pasar a una pestaña visible fuera del grupo
- **AND** si no queda ninguna, SHALL pasar a la lista

### Requirement: El modelo y el esfuerzo se eligen y se ven

El lanzador SHALL permitir elegir modelo y esfuerzo antes de arrancar, con
búsqueda y admitiendo entrada libre, y la sesión SHALL mostrar los valores
efectivos y, cuando una palanca se rehusó al abrir, la palanca, lo pedido y el
motivo, en español y en inglés. La ficha de un modelo SHALL mostrar únicamente
lo que Meltemi conoce —lo anunciado por el agente, lo declarado en perfiles y el
consumo medido localmente— y NO SHALL mostrar precios ni créditos.

WHERE se ofrezca cambiar el modelo con la sesión en marcha, SHALL advertirse
que el cambio reinicia la caché del proveedor y puede aumentar el costo.

#### Scenario: Se elige con búsqueda y se admite entrada libre

- **WHEN** se abre el selector de modelo
- **THEN** SHALL poder buscarse
- **AND** SHALL admitirse un valor escrito a mano

#### Scenario: La ficha no inventa lo que no sabe

- **WHEN** se muestra la ficha de un modelo
- **THEN** NO SHALL mostrar precios ni créditos

#### Scenario: Cambiar en marcha se advierte

- **WHERE** la sesión está en marcha
- **WHEN** se ofrece cambiar el modelo
- **THEN** SHALL advertirse el efecto sobre la caché y el costo

#### Scenario: La conversación dice la palanca rehusada

- **WHEN** una sesión termina rehusando una palanca al abrir
- **THEN** su conversación SHALL decir la palanca, lo pedido y el motivo

### Requirement: El título del schema declara la ligadura del formulario
La generación de formularios tipados de la paleta SHALL ligar cada método al
schema cuyo `title` declara ese método, y el nombre del archivo del schema NO
SHALL ser llave de esa ligadura. El contrato MAY nombrar un archivo de schema
por la familia a la que sirve o por los verbos que declara, y el gate que
vigila la correspondencia método→schema MUST NOT rehusar una ligadura correcta
por la forma del nombre del archivo, de modo que un schema que sirve a varios
verbos de una familia quepa en el contrato sin renombrarse.

#### Scenario: Un schema nombrado por sus verbos pasa el gate
- **WHERE** un archivo de schema lleva el nombre de los verbos que declara en lugar del de su familia
- **WHEN** el gate revisa la ligadura de un método servido por ese schema
- **THEN** SHALL aceptarla

#### Scenario: Los dos métodos del taller se ligan a su schema
- **WHEN** el gate revisa `change/workspace` y `change/land`
- **THEN** ambos SHALL aceptarse contra el schema que los declara en su `title`

### Requirement: La identidad de la malla se ve donde se ven los ajustes

La superficie de escritorio SHALL mostrar en Ajustes una sección de identidad
con lo que el daemon lee de la malla del usuario: quién es, el proveedor o
nombre de la malla, este equipo, y la lista de sus equipos con la presencia de
cada uno declarada con símbolo y palabra. Esa sección SHALL ofrecer el gesto de
vínculo como texto copiable, NO SHALL ejecutarlo ni abrir el navegador, y SHALL
decir de quién es la identidad —de la malla del usuario, leída y no guardada—.

Con la identidad configurada, la barra lateral SHALL mostrar en su zona
inferior —por encima de Ajustes, que SHALL seguir siendo la última entrada— el
nombre de login con la cuenta de equipos, o «Enlazar este equipo» cuando este
equipo aún no la tiene; sin configurar, el pie NO SHALL cambiar. Esa fila NO
SHALL participar del reparto entre la navegación y el árbol, y con la barra
plegada a riel SHALL conservar su etiqueta accesible y su foco.

La declaración de privacidad «sin cuentas» SHALL conservarse tal cual, y el
onboarding NO SHALL mencionar ni exigir la identidad.

#### Scenario: Ajustes muestra quién soy y mis equipos

- **WHEN** hay identidad en la malla y se abre Ajustes
- **THEN** la sección SHALL mostrar el nombre de login, el proveedor, este
  equipo y la lista de equipos con su presencia en símbolo y palabra
- **AND** SHALL ofrecer el gesto de vínculo con un control de copiar

#### Scenario: Sin enlazar, la barra ofrece enlazar y no exige nada

- **WHERE** hay identidad configurada y el daemon responde que este equipo no
  la tiene
- **THEN** el pie de la barra SHALL mostrar «Enlazar este equipo» que lleva a
  Ajustes
- **AND** ninguna otra parte de la superficie SHALL cambiar ni pedir nada

#### Scenario: Sin configurar, el cromo no pide nada

- **WHERE** no hay tabla de identidad en la configuración
- **THEN** el pie de la barra NO SHALL cambiar
- **AND** Ajustes SHALL seguir siendo la única puerta a la identidad

#### Scenario: Plegada, la identidad sigue alcanzable

- **WHEN** la barra está plegada a riel con identidad configurada
- **THEN** la fila SHALL conservar su etiqueta accesible y su foco
- **AND** Ajustes SHALL seguir siendo la última entrada

#### Scenario: La promesa de privacidad sigue siendo cierta

- **WHEN** se lee la sección de identidad junto a la de privacidad
- **THEN** la declaración «sin cuentas» SHALL seguir presente sin cambios
- **AND** la sección de identidad SHALL decir que la identidad es de la malla
  del usuario y que Meltemi la lee, no la guarda

### Requirement: La barra de estado dice sobre qué se trabaja

La barra de estado SHALL nombrar el proyecto activo y, cuando el proyecto lleva
el método, la change que reclama una decisión junto al artefacto que la espera;
si ninguna la reclama, SHALL decir cuántas hay activas. El recuento de sesiones
SHALL distinguir las que trabajan de las que esperan una decisión del usuario,
porque significan cosas distintas para quien lee. La espera de una compuerta
SHALL presentarse como propiedad de la change y NO como un estado de sesión.

#### Scenario: La barra nombra el proyecto y la compuerta que espera

- **WHEN** el proyecto activo lleva el método y una change espera una decisión
- **THEN** la barra SHALL nombrar el proyecto y esa change con su artefacto

#### Scenario: Sin compuerta pendiente, la barra dice cuántas changes hay

- **WHEN** ninguna change reclama una decisión
- **THEN** la barra SHALL decir cuántas están activas

#### Scenario: Las sesiones que trabajan se distinguen de las que esperan

- **WHEN** hay sesiones activas y sesiones esperando una decisión
- **THEN** la barra SHALL contarlas por separado

### Requirement: El consumo se dice medido o no se dice

La barra SHALL mostrar el consumo medido del proyecto en el día. Cuando el
consumo no está medido NO SHALL mostrar un cero: SHALL callar o declarar que no
fue reportado, con el motivo estable que la analítica entrega. Ningún valor
SHALL estimarse.

#### Scenario: El consumo medido se muestra

- **WHEN** la analítica reporta consumo medido para el proyecto en el día
- **THEN** la barra SHALL mostrarlo

#### Scenario: Sin medición no se inventa un cero

- **WHEN** la analítica no reporta consumo para el periodo
- **THEN** la barra NO SHALL mostrar un cero
- **AND** SHALL callar o decir que no fue reportado

### Requirement: Los segmentos llevan a su vista y ceden en orden declarado

Cada segmento que nombra algo con vista propia SHALL llevar a ella al activarse,
con nombre accesible y alcanzable por teclado. Cuando el ancho no alcanza, los
segmentos SHALL cederlo en un orden declarado; el estado de la conexión y las
decisiones pendientes NO SHALL cederlo nunca.

#### Scenario: Un segmento lleva a su vista

- **WHEN** el usuario activa el segmento del proyecto, la change, las sesiones,
  los permisos o el consumo
- **THEN** la superficie SHALL mostrar la vista correspondiente

#### Scenario: Al estrecharse, lo último que se cae

- **WHEN** el ancho disponible no alcanza para todos los segmentos
- **THEN** SHALL cederse primero el endpoint y después la versión
- **AND** la conexión y las decisiones pendientes SHALL permanecer

### Requirement: El compositor dice que se trabaja

Mientras un agente trabaja, el compositor de la superficie SHALL mostrarlo con
un indicador ambiental en su propio marco, de modo que la señal esté donde el
usuario va a escribir lo siguiente. El indicador SHALL animar únicamente
transform u opacidad y NO SHALL animar disposición alguna. SHALL apagarse
cuando la sesión espera una decisión del usuario: esperar no es trabajar, y un
indicador en marcha mientras el agente está detenido mentiría sobre quién debe
actuar. SHALL existir un solo indicador ambiental por vista.

#### Scenario: El compositor se enciende mientras el agente trabaja

- **WHEN** una sesión está iniciándose o activa
- **THEN** el compositor SHALL mostrar su indicador de trabajo
- **AND** el indicador NO SHALL animar la disposición de nada

#### Scenario: La luz se apaga cuando la sesión espera una decisión

- **WHILE** una sesión espera una decisión de permiso
- **THEN** el compositor NO SHALL mostrar el indicador de trabajo

#### Scenario: Sin movimiento, el estado se sigue diciendo

- **WHEN** el usuario ha pedido movimiento reducido
- **THEN** el indicador SHALL retirarse por completo, sin quedar detenido a la
  vista
- **AND** el estado SHALL seguir dicho por el marco y por el texto de la fila
  del compositor

### Requirement: Detener está al alcance de donde se escribe

Mientras una sesión está viva, el compositor SHALL ofrecer detenerla junto al
envío. La acción SHALL pedir la misma confirmación explícita que el acceso del
encabezado y SHALL ejecutar el mismo verbo; el acceso del encabezado SHALL
conservarse. Detener SHALL ofrecerse también mientras la sesión espera una
decisión.

#### Scenario: Detener desde el compositor

- **WHEN** una sesión está viva
- **THEN** el compositor SHALL ofrecer detenerla junto al envío
- **AND** activarlo SHALL pedir confirmación explícita antes de detenerla

#### Scenario: Un verbo, dos accesos

- **WHEN** se detiene desde el compositor y desde el encabezado
- **THEN** ambos SHALL ejecutar el mismo verbo con la misma confirmación

### Requirement: El modo de la sesión es visible y elegible

El lanzador SHALL permitir elegir el modo antes de arrancar, y la sesión SHALL
mostrar el modo vigente junto al compositor. WHERE la sesión corre sobre el árbol
del usuario sin worktree, el modo semi NO SHALL presentarse como contención: la
superficie SHALL nombrar el ámbito real.

#### Scenario: El modo se elige al lanzar y se ve en la sesión

- **WHEN** se lanza una sesión eligiendo modo
- **THEN** la sesión SHALL mostrar ese modo junto al compositor

#### Scenario: Semi sin worktree dice cuál es su ámbito real

- **WHERE** la sesión corre sobre el árbol del usuario sin worktree
- **THEN** la superficie NO SHALL presentar semi como contención al worktree

### Requirement: El pensamiento del turno se ve mientras ocurre

Mientras un turno está en vuelo, el pensamiento que el agente emite SHALL
mostrarse desplegado, sin exigir un gesto por turno; al cerrarse el turno SHALL
plegarse. El usuario SHALL poder plegarlo o desplegarlo en cualquier momento, y
plegarlo mientras el turno sigue en vuelo NO SHALL deshacerse solo. Cuando un
turno no emite pensamiento NO SHALL mostrarse sección alguna en su lugar.

#### Scenario: El pensamiento se ve mientras el turno corre

- **WHEN** un turno en vuelo emite pensamiento
- **THEN** SHALL mostrarse desplegado
- **AND** al cerrarse el turno SHALL quedar plegado

#### Scenario: Plegarlo a mano no se deshace solo

- **WHILE** un turno sigue en vuelo
- **WHEN** el usuario pliega su pensamiento
- **THEN** SHALL permanecer plegado aunque sigan llegando fragmentos

#### Scenario: Sin pensamiento no hay sección

- **WHEN** un turno no emite pensamiento
- **THEN** NO SHALL mostrarse encabezado ni marcador alguno en su lugar

### Requirement: La tira de pestañas se lee como la de un navegador

La tira de sesiones SHALL presentarse como una capa propia sobre la que
descansan las pestañas: la pestaña activa SHALL tomar la superficie del panel
que gobierna y unirse a él, y las inactivas SHALL recostarse sobre la capa de
la tira sin dibujar cada una su propia caja. La distinción de la pestaña activa
SHALL sostenerse por forma y superficie, no por un color de acento, y SHALL
seguir siendo perceptible cuando el sistema sustituye los colores del fondo. El
anillo de foco de la superficie SHALL conservarse sin cambios, y el estado
seleccionado SHALL seguir anunciándose al lector de pantalla.

#### Scenario: La activa se distingue por su forma

- **WHEN** una pestaña está activa
- **THEN** SHALL tomar la superficie del panel y unirse a él
- **AND** NO SHALL marcarse con un color de acento en su borde

#### Scenario: La selección sobrevive a la sustitución de colores

- **WHILE** el sistema fuerza sus propios colores
- **THEN** la pestaña activa SHALL conservar una marca perceptible que no
  dependa de su fondo

#### Scenario: Las inactivas comparten silueta

- **WHEN** la tira muestra dos o más pestañas inactivas contiguas
- **THEN** SHALL separarlas una línea de un píxel
- **AND** esa línea SHALL ocultarse junto a la pestaña activa y junto a la
  pestaña bajo el puntero
- **AND** ocultarla NO SHALL desplazar ninguna pestaña

### Requirement: La tira responde al puntero y revela el cierre sin perder el teclado

Una pestaña inactiva bajo el puntero SHALL responder con un cambio de
superficie. El control de cierre SHALL revelarse cuando la pestaña está bajo el
puntero o contiene el foco, y SHALL permanecer visible en la pestaña activa;
su espacio SHALL reservarse siempre, de modo que revelarlo no altere el ancho
del rótulo ni la posición de las demás. Ningún gesto de la tira SHALL quedar
disponible únicamente mediante puntero.

#### Scenario: La inactiva responde al puntero

- **WHEN** el puntero se sitúa sobre una pestaña inactiva
- **THEN** su superficie SHALL cambiar de forma perceptible

#### Scenario: El cierre se revela sin perder el camino de teclado

- **WHEN** una pestaña recibe el foco de teclado
- **THEN** su control de cierre SHALL revelarse
- **AND** la tecla de cierre del patrón vigente SHALL seguir cerrándola
- **AND** revelar el control NO SHALL mover las pestañas vecinas

### Requirement: La pestaña gasta su ancho en el rótulo

El estado de una sesión SHALL representarse dentro de la pestaña por su glifo,
y su palabra SHALL viajar en el nombre accesible y en el texto emergente de la
pestaña. La pertenencia a un grupo SHALL verse en cada pestaña miembro
mediante una franja del color del grupo, sin que el color sea el único
portador: el nombre del grupo SHALL seguir viajando en el nombre accesible de
cada pestaña.

#### Scenario: El estado no gasta ancho en repetirse

- **WHEN** una pestaña muestra el estado de su sesión
- **THEN** dentro de la pestaña SHALL mostrarse su glifo
- **AND** la palabra del estado SHALL estar en el nombre accesible de la pestaña

#### Scenario: La pertenencia se ve en la pestaña, no solo en la etiqueta

- **WHEN** una pestaña pertenece a un grupo
- **THEN** SHALL mostrar una franja del color del grupo
- **AND** el nombre del grupo SHALL seguir presente en su nombre accesible

### Requirement: Las medidas de la tira tienen un solo dueño

Las medidas que gobiernan la tira de pestañas —ancho mínimo y máximo de una
pestaña, paso de desplazamiento y las medidas de su piel— SHALL declararse una
sola vez en el módulo que las publica, y la hoja de estilo SHALL consumirlas
desde allí. NO SHALL existir en la hoja de estilo una copia literal de una
medida que el módulo ya declara.

#### Scenario: La hoja de estilo no repite lo que el módulo declara

- **WHEN** se inspecciona la hoja de estilo de la tira
- **THEN** las medidas que el módulo declara SHALL consumirse como propiedades
  personalizadas
- **AND** NO SHALL aparecer su valor literal duplicado

### Requirement: La pregunta se contesta donde se escribe

Mientras una sesión espera una decisión, la zona del compositor SHALL presentar
la petición y sus opciones como un listado recorrible con teclado y activable
con Enter, con el rótulo de cada opción tal como el agente lo envió. SHALL
decidir por el mismo verbo que la bandeja, y NO SHALL constituir una segunda
cola. La tarjeta del transcript SHALL conservarse: el registro es la verdad.

La aparición NO SHALL animar el layout: nada se mueve bajo el cursor mientras se
decide. WHERE las opciones no caben, el listado SHALL desplazarse dentro de sí
mismo y NO SHALL desplazar el panel.

La última opción SHALL ser una salida de texto libre, y su rótulo SHALL decir
qué hará realmente: interrumpir el turno y relevarlo con el texto. NO SHALL
presentarse como una respuesta a la pregunta mientras el protocolo de permisos
no transporte texto.

#### Scenario: La pregunta aparece en el compositor y se contesta con teclado

- **WHILE** una sesión espera una decisión
- **THEN** el compositor SHALL presentar sus opciones recorribles con teclado
- **AND** activar una SHALL decidir por el mismo verbo que la bandeja

#### Scenario: La pregunta aparece sin mover nada

- **WHEN** la petición llega
- **THEN** NO SHALL animarse el layout
- **AND** ningún control SHALL desplazarse bajo el cursor

#### Scenario: Muchas opciones se desplazan dentro del listado

- **WHERE** las opciones exceden el alto disponible
- **THEN** el listado SHALL desplazarse dentro de sí mismo
- **AND** el panel NO SHALL desplazarse

#### Scenario: La salida de texto libre dice lo que hará

- **WHEN** se ofrece la salida de texto libre
- **THEN** su rótulo SHALL decir que interrumpe el turno y lo releva con el texto
- **AND** NO SHALL presentarse como una respuesta a la pregunta

### Requirement: Interrumpir y enviar desde el compositor

Con la sesión trabajando y texto escrito, el compositor SHALL ofrecer
interrumpir el turno y enviar la instrucción como su relevo, junto al envío que
la encola. Sin texto NO SHALL ofrecerse: no hay nada con lo que relevar. La
acción SHALL distinguirse de detener la sesión, que conserva su confirmación.

#### Scenario: Interrumpir y enviar se ofrece con texto y sesión trabajando

- **WHEN** la sesión trabaja y hay texto en el compositor
- **THEN** SHALL ofrecerse interrumpir y enviar, junto al envío que encola

#### Scenario: Sin texto no hay nada que relevar

- **WHEN** el compositor está vacío
- **THEN** NO SHALL ofrecerse interrumpir y enviar

### Requirement: La conversación sigue viva tras el turno

Terminado un turno sin que la sesión termine, la superficie SHALL declarar la
sesión en espera con símbolo y palabra —nunca con color solo— y SHALL mantener
el compositor utilizable, sin rótulo de reanudación. El indicador de trabajo NO
SHALL animarse en espera: esperar no es trabajar.

Toda superficie que declare el estado de una sesión SHALL declarar también el
estado de espera; ninguna SHALL representarlo como terminada ni omitirlo.

#### Scenario: El compositor no muere al terminar el turno

- **WHEN** un turno termina y la sesión queda esperando
- **THEN** el compositor SHALL seguir aceptando la siguiente instrucción
- **AND** NO SHALL ofrecer reanudar

#### Scenario: Esperar no enciende el indicador de trabajo

- **WHILE** una sesión espera instrucciones
- **THEN** el indicador de trabajo NO SHALL animarse

#### Scenario: Ninguna superficie omite el estado de espera

- **WHEN** se recorre cada superficie que declara estado de sesión
- **THEN** todas SHALL tener símbolo y palabra para la espera
- **AND** ninguna SHALL pintarla como terminada

### Requirement: Las pestañas y listas nombran el trabajo

Donde la superficie nombra una sesión —pestañas, lista de sesiones, encabezado
del detalle, árbol y recientes— SHALL mostrar su título cuando lo tenga, con la
identidad del agente conservada como marca visual. El identificador de la
sesión NO SHALL desaparecer: SHALL permanecer accesible junto al título o en el
texto emergente, que además SHALL seguir diciendo el proyecto al que pertenece.
Una sesión sin título SHALL nombrarse como antes de esta capacidad, sin
sustituto inventado.

#### Scenario: La pestaña dice de qué trata la sesión

- **WHEN** una sesión con título está abierta en una pestaña
- **THEN** el rótulo de la pestaña SHALL ser su título
- **AND** el identificador completo y el proyecto SHALL seguir en su texto
  emergente

#### Scenario: Una sesión sin título se nombra como antes

- **WHEN** una sesión carece de título
- **THEN** SHALL nombrarse con su agente y su identificador corto

### Requirement: El proyecto se antepone solo cuando hace falta

Cuando las pestañas abiertas pertenecen a más de un proyecto, el rótulo de cada
una SHALL anteponer el nombre de su proyecto. Cuando todas pertenecen al mismo,
NO SHALL anteponerlo.

#### Scenario: El proyecto se antepone ante ambigüedad

- **WHEN** las pestañas abiertas cruzan más de un proyecto
- **THEN** cada rótulo SHALL anteponer el nombre de su proyecto

#### Scenario: Con un solo proyecto el rótulo no lo repite

- **WHEN** todas las pestañas abiertas pertenecen al mismo proyecto
- **THEN** ningún rótulo SHALL anteponer el nombre del proyecto

### Requirement: El harness efectivo se ve desde la Flota

La superficie de escritorio SHALL ofrecer, desde la Flota, la lectura del
harness que aplica a un agente, mostrando por cada pieza su capa de origen y su
ruta, y mostrando también lo pisado y lo inválido con su motivo.

La vista SHALL ser de solo lectura: la autoría de las piezas vive en archivos.

#### Scenario: La ficha de un agente muestra su harness con el origen

- **WHEN** se abre el harness de un agente desde la Flota
- **THEN** SHALL verse cada pieza con la capa de la que viene
- **AND** SHALL verse lo que no aplica, con el motivo por el que no aplica

### Requirement: Las sesiones de cada proyecto se agrupan por lo que piden

Dentro de cada nodo de proyecto del árbol de la barra lateral, las sesiones
SHALL agruparse en cubetas por estado, en orden de señal: las que esperan una
decisión (`waiting_permission`), las que esperan una instrucción
(`waiting_instruction`), las que trabajan (`active`, `starting`) y las
detenidas (`ended`, `interrupted`). Cada cubeta SHALL llevar glifo, palabra y
cuenta; una cubeta vacía NO SHALL dibujarse. Cada fila SHALL mostrar el
avatar del agente y el título de la sesión —o el agente y el identificador
corto cuando no hay título—, SHALL conservar el nombre de su suscripción como
pastilla y la palabra canónica del estado como nombre accesible, y SHALL
marcar con forma y palabra si la sesión está abierta en una pestaña. La cubeta de detenidas SHALL acotarse a las más
recientes y ofrecer «Ver las {n}» hacia la vista Sesiones. Un cambio de estado
SHALL mover la fila a su cubeta sin animar el layout: nada se mueve bajo el
cursor mientras se decide un permiso.

#### Scenario: Cuatro estados, cuatro cubetas en orden de señal

- **WHEN** un proyecto tiene sesiones en `waiting_permission`, `active`,
  `waiting_instruction` y `ended`
- **THEN** su nodo SHALL listarlas en cuatro cubetas en este orden: esperan tu
  decisión, listas para tu instrucción, trabajando, detenidas
- **AND** cada cabecera SHALL decir el estado con glifo y palabra y la cuenta
  de la cubeta

#### Scenario: Una cubeta vacía no ocupa sitio

- **WHEN** ninguna sesión del proyecto está en uno de los cuatro estados
- **THEN** la cabecera de esa cubeta NO SHALL dibujarse

#### Scenario: La fila dice el título y si está abierta

- **WHEN** una sesión con título y suscripción está abierta en una pestaña
- **THEN** su fila SHALL mostrar el avatar, el título y la pastilla de
  suscripción
- **AND** SHALL marcar que está abierta con forma y palabra, no con color solo
- **AND** activarla SHALL enfocar su pestaña sin crear otra

#### Scenario: Las detenidas no desbordan la barra

- **WHEN** un proyecto tiene más sesiones detenidas que el tope de la cubeta
- **THEN** la cubeta SHALL mostrar solo las más recientes
- **AND** SHALL ofrecer «Ver las {n}» que lleva a la vista Sesiones con el
  proyecto conmutado

#### Scenario: Un cambio de estado salta de cubeta sin animarse

- **WHEN** una sesión que trabaja pasa a esperar un permiso
- **THEN** su fila SHALL aparecer en la cubeta de decisión en el siguiente
  refresco
- **AND** ningún elemento de la barra SHALL animar su posición

### Requirement: Las pestañas abiertas se gobiernan desde la barra

La barra lateral SHALL listar las pestañas de sesión abiertas en una sección
propia, con la cuenta, en el mismo orden que la tira. Activar una fila SHALL
traer su pestaña al frente; cada fila SHALL ofrecer un control de cierre con
nombre accesible, y `Delete` sobre una fila enfocada SHALL cerrarla. Cerrar
desde la barra SHALL comportarse como cerrar desde la tira: el foco cae en la
vecina y, sin ninguna, queda la lista. La fila de la pestaña que está al frente
SHALL marcarse por forma y palabra. Cerrar una pestaña NO SHALL terminar la
sesión: sigue en su cubeta. Con la barra plegada a riel la sección NO SHALL
dibujarse y la tira SHALL seguir siendo el camino a cada pestaña.

#### Scenario: Seleccionar en la barra trae la pestaña al frente

- **WHEN** hay dos pestañas abiertas y se activa en la barra la que no está al
  frente
- **THEN** esa pestaña SHALL pasar al frente en la tira
- **AND** su fila SHALL quedar marcada como la actual

#### Scenario: Cerrar desde la barra cae en la vecina

- **WHEN** se cierra desde la barra la pestaña que está al frente
- **THEN** el foco SHALL pasar a la vecina de la izquierda, o a la última si no
  la hay
- **AND** si no queda ninguna SHALL quedar la lista en pantalla

#### Scenario: Cerrar no es olvidar

- **WHEN** se cierra desde la barra la pestaña de una sesión que trabaja
- **THEN** la sesión SHALL seguir en su cubeta con su estado
- **AND** activarla SHALL volver a abrir su pestaña con su lectura

#### Scenario: Plegada, la tira sigue siendo el camino

- **WHEN** la barra está plegada a riel
- **THEN** la sección de pestañas abiertas NO SHALL dibujarse
- **AND** cada pestaña SHALL seguir alcanzable desde la tira con el teclado

### Requirement: Nueva sesión nace como pestaña con el cursor dentro

Pedir una sesión nueva —desde la acción primaria, la entrada de la barra, la
acción rápida de un proyecto o su atajo— SHALL abrir en la tira una pestaña
«Nueva sesión» con el compositor, traerla al frente y poner el foco en el
campo. Pedirla de nuevo mientras existe SHALL enfocarla, no duplicarla. Enviar
SHALL convertir esa pestaña en la pestaña de la sesión creada, en el mismo
sitio y sin abrir otra, y su rótulo SHALL pasar a ser el título derivado. La
vista de llegada SHALL ser esa pestaña al frente con el foco en el campo. La
pestaña nueva SHALL contar contra el tope de pestañas. Cerrarla con borrador
escrito SHALL pedir decisión; vacía, NO SHALL preguntar.

#### Scenario: Pedir una sesión nueva abre su pestaña y da el foco

- **WHEN** se pide una sesión nueva desde cualquiera de sus puertas
- **THEN** la tira SHALL mostrar una pestaña «Nueva sesión» al frente
- **AND** el foco SHALL estar en el campo del compositor sin ningún clic más

#### Scenario: Pedirla de nuevo enfoca, no duplica

- **WHEN** ya hay una pestaña «Nueva sesión» abierta y se vuelve a pedir
- **THEN** esa pestaña SHALL pasar al frente con el foco en el campo
- **AND** NO SHALL crearse una segunda

#### Scenario: Enviar convierte la pestaña en la sesión

- **WHEN** se envía la primera instrucción desde la pestaña «Nueva sesión»
- **THEN** la misma pestaña SHALL pasar a representar la sesión creada
- **AND** su rótulo SHALL ser el título derivado de la instrucción
- **AND** NO SHALL abrirse otra pestaña

#### Scenario: Llegar es llegar a la pestaña nueva

- **WHEN** la superficie arranca sin última vista recordada
- **THEN** SHALL mostrar el compositor como pestaña «Nueva sesión» al frente,
  con el foco en el campo

#### Scenario: Cerrar con borrador pide decisión

- **WHEN** se cierra la pestaña «Nueva sesión» con texto escrito
- **THEN** la superficie SHALL pedir decisión antes de descartarlo
- **AND** sin texto SHALL cerrarla sin preguntar

### Requirement: Dos acordes en el compositor: despachar ahora y encolar

Todo compositor SHALL atender `Ctrl+Enter` (`Cmd+Enter` en macOS) como
*despachar ahora* y `Ctrl+Shift+Enter` como *encolar*; `Enter` a secas SHALL
insertar un salto de línea. Cuando la sesión espera una instrucción, despachar
ahora SHALL enviarla como siguiente turno. Cuando un turno está en vuelo,
despachar ahora SHALL **relevar** —interrumpir el turno y enviar la instrucción
como su relevo— y encolar SHALL dejarla detrás del turno sin interrumpir nada;
en ese estado el botón primario SHALL llamarse «Interrumpir y enviar» y
«Encolar» SHALL estar a la vista, cada uno con su atajo en la afordancia
`kbd`, y con el compositor vacío NO SHALL ofrecerse ninguno de los dos. Los
manejadores que atienden `Ctrl+Enter` SHALL comprobar que `Shift` no está
pulsado, de modo que los dos acordes nunca disparen la misma acción.

#### Scenario: Ctrl+Enter despacha cuando la sesión espera

- **WHEN** la sesión espera una instrucción y se pulsa `Ctrl+Enter` con texto
- **THEN** el texto SHALL enviarse como siguiente turno
- **AND** el campo SHALL quedar vacío y con el foco

#### Scenario: Ctrl+Shift+Enter encola y nunca interrumpe

- **WHEN** un turno está en vuelo y se pulsa `Ctrl+Shift+Enter` con texto
- **THEN** la instrucción SHALL quedar encolada detrás del turno con su
  posición declarada
- **AND** el turno en vuelo NO SHALL interrumpirse

#### Scenario: Ctrl+Enter mientras trabaja avisa antes y releva

- **WHEN** un turno está en vuelo y hay texto en el compositor
- **THEN** el botón primario SHALL leerse «Interrumpir y enviar» con su atajo
  y «Encolar» SHALL estar a la vista con el suyo
- **AND** pulsar `Ctrl+Enter` SHALL interrumpir el turno y enviar el texto como
  el siguiente

#### Scenario: Con el compositor vacío no hay nada que relevar

- **WHEN** un turno está en vuelo y el compositor está vacío
- **THEN** NO SHALL ofrecerse interrumpir y enviar
- **AND** ningún acorde SHALL despachar ni encolar nada

#### Scenario: Los acordes viejos no se pisan

- **WHEN** se pulsa `Ctrl+Shift+Enter` en la paleta, en el compositor de
  llegada o en el de una sesión
- **THEN** NO SHALL dispararse la acción de `Ctrl+Enter`

#### Scenario: Enter sigue siendo salto de línea

- **WHEN** se pulsa `Enter` sin modificadores en un compositor
- **THEN** SHALL insertarse un salto de línea
- **AND** nada SHALL enviarse

### Requirement: El turno vacío se dice con glifo, palabra y remedio
WHEN el log de una sesión registra un turno vacío, la conversación de esa
sesión SHALL mostrarlo en su lugar como un estado propio, con glifo y palabra,
nunca solo con color, y con el remedio en condicional: probar el agente por su
cuenta y revisar su proveedor y sus credenciales. El log de operador SHALL darle
glifo y tono propios. El texto SHALL existir en español y en inglés.

#### Scenario: El turno vacío se lee en la conversación
- **WHEN** la conversación pliega un log con un `turn_empty`
- **THEN** SHALL mostrar un aviso de turno vacío con glifo, la palabra y el remedio
- **AND** el aviso NO SHALL presentarse como una línea de sistema neutra

#### Scenario: Un turno con salida no muestra el aviso
- **WHEN** la conversación pliega un turno con mensajes o herramientas
- **THEN** NO SHALL mostrar el aviso de turno vacío

### Requirement: Los indicadores de la GUI se alimentan del estado vivo

Todo indicador de la GUI que afirme que una sesión está trabajando, esperando
o terminada —el contador de la barra de estado, las cubetas de la barra
lateral y del árbol, el glifo de la pestaña y el anillo del detalle— SHALL
derivar del estado que el daemon notifica, y NO SHALL depender de haber vuelto
a consultar la lista de sesiones. WHERE la GUI consulte esa lista, SHALL ser
para reconciliarse al conectar, al cambiar de proyecto o tras haberse
rezagado, nunca como fuente de la que un indicador dependa para ser cierto.

Una sesión abierta desde otro cliente SHALL aparecer y avanzar en la GUI con
la misma fidelidad que una abierta en ella.

#### Scenario: El contador de la barra cuenta una sesión de otro cliente

- **WHEN** otro cliente abre una sesión y su agente empieza a trabajar
- **THEN** el contador de la barra de estado de la GUI la cuenta como trabajando sin que el usuario haga nada
- **AND** la cubeta de la barra lateral y el anillo del detalle afirman lo mismo

#### Scenario: El sondeo solo reconcilia

- **WHEN** la GUI reconecta al daemon o cambia de proyecto
- **THEN** consulta la lista de sesiones una vez para reconciliarse
- **AND** entre reconciliaciones los indicadores se mueven solo con las notificaciones

### Requirement: La GUI dice cuándo no está hablando con su propia máquina

WHERE la GUI hable con el daemon a través de un comando, su cromo SHALL
mostrarlo de forma legible, con el comando abreviado. Una superficie que
callara la diferencia haría que una acción sobre la máquina equivocada
pareciera una acción sobre la propia.

#### Scenario: El cromo nombra la vía

- **WHEN** la GUI está conectada a través de un comando
- **THEN** su barra de estado muestra que habla por esa vía, con el comando abreviado

### Requirement: La GUI configura y prueba el aviso de espera

La GUI SHALL mostrar en sus ajustes si el aviso de espera está configurado y
SHALL ofrecer enviar un aviso de prueba que recorra exactamente el mismo camino
que el aviso real. El comando SHALL ejecutarse desde el proceso nativo de la
aplicación y NO SHALL ejecutarse desde la vista web.

#### Scenario: La prueba usa el mismo camino que el aviso real

- **WHEN** el usuario envía un aviso de prueba desde los ajustes
- **THEN** se ejecuta el mismo comando con la clase de espera de prueba
- **AND** el resultado —enviado o fallido— se muestra donde se pidió

### Requirement: Las máquinas se eligen donde se eligen los ajustes

La superficie de escritorio SHALL mostrar en Ajustes una sección de máquinas
con las que el usuario configuró —nombre, endpoint y, a pedido, la presencia
que la malla conoce—, SHALL ofrecer probar la conexión de cada una informando
versión y latencia, y SHALL permitir hacer activa a otra, reconectando sin
reiniciar la aplicación y advirtiendo del trabajo sin guardar con la misma
guarda del cambio de proyecto. La barra de estado SHALL decir en qué máquina
está. Sin máquinas configuradas, el cromo NO SHALL cambiar.

#### Scenario: Ajustes lista las máquinas con su endpoint

- **WHEN** hay máquinas configuradas y se abre Ajustes
- **THEN** la sección SHALL mostrar cada una con su nombre y su endpoint
- **AND** SHALL marcar cuál es la activa

#### Scenario: Probar una máquina dice si respondió y cuánto tardó

- **WHEN** el usuario prueba una máquina
- **THEN** SHALL informarse la versión del daemon y la latencia, o el
  diagnóstico de por qué no respondió
- **AND** la máquina activa NO SHALL cambiar por probar

#### Scenario: Cambiar de máquina avisa del trabajo sin guardar

- **WHERE** hay trabajo sin guardar y el usuario elige otra máquina
- **THEN** SHALL advertirse con la misma guarda que el cambio de proyecto
- **AND** al confirmar, la aplicación SHALL reconectar contra la nueva sin
  reiniciarse

#### Scenario: Sin máquinas configuradas el cromo no cambia

- **WHERE** no hay máquinas en la configuración de usuario
- **THEN** la barra de estado NO SHALL ganar nada
- **AND** Ajustes SHALL decir cómo se configuran, sin pedir nada

### Requirement: Vocabulario de movimiento acotado
La GUI SHALL animar solo con las clases del vocabulario de movimiento de
`docs/ux/design-system.md`, cada una con su lugar, su token de duración
(`--dur-1`, `--dur-2`, `--dur-pulse`, `--dur-ambient`, `--dur-mark`, con
`--ease-out`) y su
equivalente sin movimiento. Las animaciones con nombre SHALL ser exactamente las
de una lista cerrada —`composer-wind`, `arrive`, `skeleton-pulse` y
`mark-draw`— y SHALL animar solo opacidad o transform, con la única excepción
del trazo de la marca, que dibuja su contorno. Ninguna SHALL animar altura,
posición ni color de fondo; la bandeja de permisos, las tarjetas de permiso,
los avisos y la barra lateral MUST NOT animarse; y todo archivo que anima SHALL
retirar su animación bajo `prefers-reduced-motion`.

#### Scenario: Una animación fuera de la lista rompe la suite
- **WHEN** un archivo de la interfaz declara una animación con nombre que no está en la lista cerrada
- **THEN** la suite SHALL fallar nombrando el archivo y la animación

#### Scenario: Toda animación se retira con movimiento reducido
- **WHEN** un archivo de la interfaz declara una animación
- **THEN** el mismo archivo SHALL declarar la rama de `prefers-reduced-motion` que la retira

#### Scenario: Las duraciones salen de los tokens
- **WHEN** una animación de la interfaz declara su duración
- **THEN** SHALL tomarla de un token del vocabulario y no de un literal

#### Scenario: Las superficies de decisión no se mueven
- **WHEN** se recorre la fuente de la bandeja de permisos, de la tarjeta de permiso del transcript, de los avisos y de la barra lateral
- **THEN** ninguna SHALL declarar animación ni transición

#### Scenario: La marca se dibuja una vez en la bienvenida
- **WHEN** se abre la bienvenida
- **THEN** el contorno de la marca SHALL dibujarse una vez, en 600 ms
- **AND** bajo movimiento reducido SHALL aparecer ya dibujado

### Requirement: Estados de carga con esqueleto y umbral
Una vista que espera datos del daemon SHALL mostrar un esqueleto en lugar de la
palabra de carga: filas de 32 px con barras de anchos fijos, marcadas
`aria-busy="true"` y con el texto de carga del catálogo para lectores de
pantalla. El esqueleto SHALL aparecer solo cuando la espera supera 150 ms,
SHALL latir con un pulso de opacidad y MUST NOT llevar un gradiente en barrido.
Bajo movimiento reducido las barras SHALL quedar quietas y el texto de carga
SHALL verse. Un botón cuya etiqueta cambia mientras espera SHALL conservar su
ancho.

#### Scenario: Una carga rápida no parpadea
- **WHEN** los datos llegan antes de 150 ms
- **THEN** la vista SHALL NOT haber mostrado el esqueleto

#### Scenario: Una carga lenta muestra el esqueleto
- **WHEN** la espera supera 150 ms
- **THEN** la vista SHALL mostrar el esqueleto con `aria-busy` y el texto de carga accesible
- **AND** SHALL retirarlo cuando llegan los datos

#### Scenario: El esqueleto se queda quieto con movimiento reducido
- **WHERE** el sistema declara preferencia de movimiento reducido
- **THEN** las barras SHALL quedar quietas y el texto de carga SHALL verse

#### Scenario: Ninguna vista espera con una palabra suelta
- **WHEN** se recorre la fuente de Project, Review, Fleet, Settings y Usage
- **THEN** ninguna espera de bloque SHALL ser solo la palabra de carga
- **AND** los botones que la usan como etiqueta SHALL conservar su ancho

### Requirement: Señal de vida del turno
Mientras el agente de una sesión trabaja, el transcript SHALL decir que el
turno está en curso: antes del primer fragmento, una línea «trabajando…» con el
tiempo del turno en `m:ss`, que avanza cada segundo; con fragmentos, un cursor
fijo tras el último, que MUST NOT parpadear. La señal SHALL retirarse cuando el
agente deja de trabajar —turno cerrado, sesión que espera una decisión o una
instrucción, sesión terminada— y un turno cerrado sin partes SHALL seguir
diciendo «sin salida». El tiempo de la sesión en la cabecera SHALL avanzar cada
segundo mientras la sesión vive, en cifras tabulares, y quedar fijo en su
duración final cuando termina.

#### Scenario: Antes del primer fragmento se dice que trabaja
- **WHEN** el agente trabaja y el turno aún no tiene fragmentos
- **THEN** el transcript SHALL mostrar «trabajando…» con el tiempo transcurrido desde la instrucción, y no «sin salida»

#### Scenario: El cursor sigue al último fragmento sin parpadear
- **WHEN** el turno en curso tiene fragmentos
- **THEN** un cursor fijo SHALL seguir al último fragmento
- **AND** SHALL desaparecer cuando el turno se cierra

#### Scenario: Esperar una decisión no es trabajar
- **WHEN** la sesión espera una decisión de permiso o una instrucción
- **THEN** el transcript SHALL NOT mostrar la señal de vida

#### Scenario: El tiempo de la sesión corre mientras vive
- **WHEN** la sesión vive
- **THEN** su tiempo en la cabecera SHALL avanzar cada segundo en cifras tabulares
- **AND** cuando termina SHALL quedar fijo en su duración final

### Requirement: Ventana visible tras el primer frame
La ventana principal SHALL crearse oculta, recibir la geometría restaurada
mientras está oculta y mostrarse cuando la interfaz montó su primer árbol, o a
los 500 ms si la interfaz no lo señala, lo que ocurra antes, y una sola vez.
Desde antes de mostrarse SHALL llevar el fondo del tema de la interfaz —el
elegido o, sin elección, el del sistema—, de modo que ningún instante enseñe
el fondo propio del webview.

#### Scenario: La ventana nace oculta y con su geometría
- **WHEN** arranca la aplicación
- **THEN** la ventana SHALL crearse oculta
- **AND** la geometría restaurada SHALL aplicarse antes de mostrarla

#### Scenario: La interfaz lista muestra la ventana
- **WHEN** la interfaz monta su primer árbol
- **THEN** SHALL pedir al anfitrión que muestre la ventana

#### Scenario: Una interfaz muda no deja la ventana invisible
- **WHEN** la interfaz no lo señala en 500 ms
- **THEN** el anfitrión SHALL mostrar la ventana igualmente
- **AND** una señal tardía SHALL NOT volver a mostrarla

#### Scenario: La ventana nunca enseña un fondo ajeno al tema
- **WHEN** la ventana se muestra antes de que la interfaz haya pintado
- **THEN** lo visible SHALL ser el fondo del tema de la interfaz, claro u oscuro según la elección o el sistema
