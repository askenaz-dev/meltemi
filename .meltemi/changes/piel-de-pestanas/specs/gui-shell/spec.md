# gui-shell — delta

## ADDED Requirements

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
