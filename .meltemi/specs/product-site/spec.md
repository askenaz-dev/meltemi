

### Requirement: Sitio estático sin rastreo ni orígenes de terceros
El sitio SHALL vivir en `site/` como HTML y CSS estáticos —sin framework, sin
generador y sin paso de build— y SHALL publicarse tal como está en el
repositorio. El sitio MUST NOT incluir JavaScript, MUST NOT cargar recursos de
un origen externo (scripts, hojas de estilo, fuentes, imágenes o marcos
incrustados) y MUST NOT fijar cookies, usar almacenamiento del navegador,
ofrecer formularios ni recolectar dato alguno del visitante: la constitución §9 y
§10 aplican también a la web. WHERE una funcionalidad de la página exigiría
ejecutar código, la página SHALL resolverla con HTML y CSS o prescindir de ella.

#### Scenario: Origen externo rehusado
- **WHEN** una página referencia un recurso alojado fuera del propio sitio
- **THEN** la verificación del sitio SHALL fallar señalando página y origen
- **AND** ninguna publicación SHALL ocurrir mientras ese origen esté presente

#### Scenario: Ninguna página ejecuta código
- **WHEN** se audita el contenido publicado
- **THEN** no SHALL existir etiqueta de script ni manejador de eventos en línea
- **AND** la navegación completa SHALL funcionar solo con enlaces y CSS

#### Scenario: Sin cookies ni recolección de datos
- **WHEN** un visitante recorre el sitio entero
- **THEN** el sitio SHALL NOT fijar cookies ni almacenar datos del visitante
- **AND** SHALL NOT ofrecer formularios ni destinos de envío de datos

### Requirement: Historia del producto completa y honesta
La portada SHALL presentar Meltemi como el producto completo que es: el daemon
`meltemid` con dos superficies en paridad de núcleo —cliente de escritorio y
terminal— sobre Windows, macOS y Linux, con el lema "un rumbo, muchas velas", la
promesa BYO (agente, clave y modelo) sin créditos ni tarifas ni lock-in, el
estado real del proyecto y lo que Meltemi no es. El método SHALL presentarse
como herramienta que da poder al usuario, nunca como peaje. El sitio MUST NOT
nombrar productos de terceros fuera de datos factuales de interoperabilidad
—misma regla que el README— y MUST NOT presentar como disponible una capacidad
que el producto todavía no publica.

#### Scenario: Portada nombra producto, superficies y plataformas
- **WHEN** un recién llegado abre la portada
- **THEN** SHALL leer qué es Meltemi, sus dos superficies en paridad y las tres plataformas soportadas
- **AND** SHALL encontrar el lema y la promesa BYO sin créditos ni tarifas

#### Scenario: Estado honesto antes de la v1.0
- **WHEN** el producto está antes de su v1.0
- **THEN** la portada SHALL declarar el estado real y lo que aún no existe
- **AND** SHALL NOT anunciar como disponible una capacidad no publicada

#### Scenario: Sin nombres de terceros como argumento de venta
- **WHEN** la verificación revisa el contenido publicado
- **THEN** SHALL fallar si aparece un producto de terceros fuera de un dato factual de interoperabilidad

### Requirement: Descargas resueltas a la última release firmada
La página de descargas SHALL ofrecer por plataforma el archivo con `meltemi` y
`meltemid`, el instalador del cliente de escritorio y el script instalador, y
SHALL enlazar el `SHA256SUMS` de la release junto a las instrucciones publicadas
de verificación de checksum y firma. Todo enlace de descarga MUST resolverse a
la release firmada más reciente sin literal de versión alguno en el sitio, y el
sitio MUST NOT hospedar copia de un artefacto ni de un script instalador. IF el
sitio nombra un artefacto que el pipeline de release no produce, la verificación
MUST fallar.

#### Scenario: Literal de versión rehusado
- **WHEN** un enlace de descarga incluye un literal de versión
- **THEN** la verificación SHALL fallar exigiendo la URL de última release

#### Scenario: Nombre de artefacto inexistente rehusado
- **IF** el sitio enlaza un nombre de artefacto que el pipeline no emite
- **THEN** la verificación SHALL fallar nombrando el artefacto ausente

#### Scenario: Verificación alcanzable desde la descarga
- **WHEN** el visitante llega a la página de descargas
- **THEN** SHALL encontrar enlazados el `SHA256SUMS` de la release y el procedimiento de verificación de firma
- **AND** el sitio SHALL NOT servir esos artefactos desde su propio origen

### Requirement: Fuente única compartida con la documentación
El sitio MUST NOT duplicar la documentación operativa del repositorio: el
quickstart, la guía de agentes con sus perfiles multi-suscripción, la referencia
CLI, el procedimiento de verificación y las notas de plataforma SHALL enlazarse
a su fuente única acompañados a lo sumo de un resumen breve, y el manifiesto
fundacional y la constitución SHALL enlazarse íntegros. La verificación MUST
rehusar todo bloque de seis o más líneas consecutivas idéntico a un documento de
`docs/`, y todo enlace interno o hacia la documentación SHALL resolverse.

#### Scenario: Bloque copiado de la documentación rehusado
- **WHEN** una página repite seis o más líneas consecutivas de un documento de `docs/`
- **THEN** la verificación SHALL fallar señalando página y documento duplicado

#### Scenario: Enlaces requeridos presentes y resueltos
- **WHEN** corre la verificación del sitio
- **THEN** los enlaces al manifiesto, la constitución, el quickstart, la guía de agentes y el procedimiento de verificación SHALL estar presentes
- **AND** cada enlace interno SHALL resolver a un archivo existente

### Requirement: Identidad del design system aplicada al sitio
El sitio SHALL vestir la identidad normativa de `design-system/`: tokens
semánticos y de marca, la misma pila tipográfica de sistema del cliente de
escritorio, escala de espaciado de 4 px, radios de 4 y 8 px, filetes de 1 px con
un único nivel de sombra reservado a superposiciones, y las marcas tomadas de
`brand/` como fuente única. Los tokens del sitio SHALL derivarse de los del
cliente de escritorio y una divergencia de valor MUST fallar la verificación. El
sitio SHALL seguir el tema claro u oscuro del sistema, SHALL exponer foco visible
y operación completa por teclado sin ejecutar código, MUST codificar todo estado
con símbolo o forma más palabra —el color MUST NOT ser el único portador de
significado—, MUST NOT animar el layout de sus bandas de aviso y SHALL honrar
las preferencias de movimiento reducido y alto contraste del sistema.

#### Scenario: Token divergente rehusado
- **WHEN** un token del sitio declara un valor distinto del que declara el cliente de escritorio
- **THEN** la verificación SHALL fallar nombrando el token y ambos valores

#### Scenario: Tema del sistema honrado
- **WHERE** el sistema del visitante declara preferencia de tema oscuro
- **THEN** el sitio SHALL renderizarse con la paleta oscura de los tokens

#### Scenario: Recorrido por teclado con foco visible
- **WHEN** el visitante recorre una página solo con teclado
- **THEN** cada elemento enfocable SHALL mostrar su indicador de foco
- **AND** ninguna acción SHALL depender del puntero ni de la ejecución de código

#### Scenario: Estado legible sin color
- **WHEN** una página comunica un estado o un nivel de integración
- **THEN** SHALL renderizar símbolo o forma más palabra además del color

### Requirement: Capturas reales desde un proyecto fixture
Las capturas del sitio SHALL mostrar ambas superficies reales —cliente de
escritorio y terminal— tomadas sobre un repositorio fixture temporal con el
agente simulado, nunca sobre un proyecto real ni bajo una cuenta de agente. Cada
captura MUST llevar texto alternativo descriptivo y su procedencia declarada
(versión del producto, plataforma y superficie), y MUST NOT mostrar rutas
personales, identidades, marcas de terceros ni material de credenciales.

#### Scenario: Captura sin alternativa textual rehusada
- **WHEN** una captura se publica sin texto alternativo descriptivo
- **THEN** la verificación SHALL fallar señalando la imagen

#### Scenario: Procedencia declarada por captura
- **WHEN** corre la verificación del sitio
- **THEN** cada captura SHALL declarar versión, plataforma y superficie

#### Scenario: Capturas tomadas de fixture y agente simulado
- **WHEN** se produce el material visual del sitio
- **THEN** las capturas SHALL provenir de un repositorio fixture temporal con el agente simulado
- **AND** SHALL NOT contener rutas personales, identidades ni marcas de terceros

### Requirement: Paridad de idiomas ES/EN del sitio
El sitio SHALL publicarse en español e inglés como árboles gemelos, con la raíz
en inglés y el español bajo su prefijo de idioma, declarando `lang` y la
alternancia `hreflang` en cada página (constitución §11). El conmutador de idioma
SHALL ser un enlace plano: MUST NOT existir detección por dirección de red,
redirección automática ni ejecución de código para elegir idioma. IF una página
carece de su gemela en el otro idioma, la verificación MUST fallar.

#### Scenario: Página sin gemela rehusada
- **IF** una página existe en un idioma y falta en el otro
- **THEN** la verificación SHALL fallar nombrando la página ausente

#### Scenario: Conmutador de idioma sin ejecución de código
- **WHEN** el visitante cambia de idioma
- **THEN** el cambio SHALL producirse siguiendo un enlace a la página gemela
- **AND** SHALL NOT intervenir detección por dirección de red ni redirección automática

### Requirement: Verificación del sitio como gate de CI
El proyecto SHALL verificar el sitio con un lint de la suite del workspace que
corre en las tres plataformas, sin red y sin navegador, y la CI MUST fallar
—bloqueando el merge y la publicación— cuando falte una página o sección
requerida, un enlace no resuelva, aparezca JavaScript o un origen externo, un
enlace de descarga traiga literal de versión o un nombre que el pipeline no
emita, un token divergiera del cliente de escritorio, falte una gemela de idioma
o se duplique documentación.

#### Scenario: Sección requerida ausente rompe la CI
- **WHEN** una página del sitio pierde una sección requerida
- **THEN** el lint SHALL fallar en la CI señalando página y sección

#### Scenario: Lint rojo impide publicar
- **IF** el lint del sitio falla
- **THEN** la publicación SHALL NOT ejecutarse
- **AND** el sitio publicado SHALL permanecer en su edición anterior
