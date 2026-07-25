## ADDED Requirements

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
(página, superficie, flotante) con sus sombras, y tablas densas: filas
compactas con jerarquía tipográfica (principal + secundario), hover y
selección distinguibles sin depender solo del color. Los valores categóricos
repetidos (nivel, origen, detección) SHALL presentarse como pills, badges o
dots con palabra — MUST NOT repetirse como texto plano columna abajo.

#### Scenario: Fila con jerarquía y selección visible
- **WHEN** el usuario recorre una tabla con teclado o puntero
- **THEN** hover y selección SHALL distinguirse por superficie y marcador, no solo por color

#### Scenario: Categorías como pills
- **WHEN** la Flota lista el nivel de integración y la detección
- **THEN** el nivel SHALL renderizarse como pill (con su verificación) y la detección como dot + palabra

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

### Requirement: Acciones primarias y estados vacíos accionables
La superficie SHALL exponer "proponer un cambio" como acción primaria del
chrome — a un clic y con atajo — abriendo el flujo `propose` con su formulario;
y todo estado vacío SHALL ofrecer su siguiente paso como control ejecutable y
enfocable, nunca solo como texto (Proyecto sin `.meltemi/` SHALL ofrecer
inicializar la constitución; Flota sin agentes SHALL ofrecer refrescar la
detección).

#### Scenario: Proponer desde el chrome
- **WHEN** el usuario activa la acción primaria del chrome
- **THEN** la superficie SHALL abrir el flujo `propose` listo para escribir la idea

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
