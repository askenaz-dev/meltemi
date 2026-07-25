

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
