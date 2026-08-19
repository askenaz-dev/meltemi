

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
una tira de pestañas cuya primera pestaña es la lista y NO SHALL ser cerrable.
Abrir una sesión ya abierta SHALL enfocar su pestaña sin crear otra. Una
pestaña que no está en pantalla SHALL conservar su transcript, su lectura y su
borrador sin enviar, y SHALL declarar cuántos eventos llegaron sin leer. Cada
pestaña SHALL declarar el estado de su sesión con símbolo y palabra, nunca con
color solo. La tira SHALL recorrerse entera con el teclado. Alcanzado el tope de
pestañas, la superficie SHALL rehusar abrir otra sin cerrar ninguna y SHALL
nombrar el remedio. Cada sesión abierta SHALL declarar su interés por su propia
sesión y NO SHALL mostrar los eventos de otra.

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

#### Scenario: La lista es la primera pestaña y nunca se cierra

- **WHEN** hay al menos una sesión abierta
- **THEN** la tira SHALL presentar la lista como primera pestaña sin control de
  cierre
- **AND** seleccionarla SHALL devolver la tabla completa con su filtro

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
- **AND** tras reiniciar la superficie SHALL arrancar sin pestañas de sesión y
  con la lista en pantalla

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
efectivos. La ficha de un modelo SHALL mostrar únicamente lo que Meltemi
conoce —lo anunciado por el agente, lo declarado en perfiles y el consumo
medido localmente— y NO SHALL mostrar precios ni créditos.

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
