# initial-docs Specification

## Purpose
TBD - created by archiving change documentacion-inicial. Update Purpose after archive.
## Requirements

### Requirement: README con las secciones mínimas
El repositorio SHALL contener un README que presente qué es Meltemi (plano de
control spec-driven), el estado honesto del proyecto, la arquitectura en un
vistazo, cómo instalar y el primer paso; MUST NOT nombrar productos de terceros
fuera de datos factuales de interoperabilidad y MUST enlazar el espejo breve en
español.

#### Scenario: Primer contacto completo
- **WHEN** un recién llegado lee el README
- **THEN** SHALL entender qué es, en qué estado está y cómo empezar

### Requirement: Quickstart verificado contra binarios
El quickstart SHALL llevar de cero al primer `propose` revisado en terminal, por
plataforma, y sus pasos scriptables MUST ejecutarse en CI contra los binarios
construidos: una desviación entre documento y producto MUST fallar el build.

#### Scenario: Quickstart en CI
- **WHEN** corre la verificación del quickstart
- **THEN** los pasos scriptables SHALL ejecutarse contra los binarios reales
- **AND** una salida divergente SHALL fallar la verificación

### Requirement: Referencia CLI generada desde la fuente
La referencia de subcomandos, flags y códigos de salida SHALL generarse desde la
gramática y la taxonomía del código (fuente única), y el mapa de teclas de la TUI
desde el keymap-como-dato; la regeneración MUST formar parte del pipeline y una
referencia desactualizada MUST detectarse.

#### Scenario: Referencia nunca desincronizada
- **WHEN** la gramática gana un subcomando sin regenerar la referencia
- **THEN** la verificación del pipeline SHALL fallar señalándolo

### Requirement: Notas de plataforma reales
La documentación SHALL incluir notas por plataforma con los hallazgos reales del
proyecto — incluido el mangling de variables tipo `MELTEMI_ENDPOINT` bajo
git-bash/MSYS en Windows, las rutas de datos por SO, el acceso remoto por túnel
SSH del socket local y la ruta accesible garantizada (`--json`, NO_COLOR, ASCII).

#### Scenario: La trampa de git-bash documentada
- **WHEN** un usuario de Windows consulta las notas de plataforma
- **THEN** SHALL encontrar la advertencia del mangling con su remedio

### Requirement: Guía de agentes verificada contra el registro
El repositorio SHALL incluir una guía de agentes enlazada desde el README que,
por cada entrada del registro de flota, explique qué instala el usuario, cómo se
detecta cada capa, el nivel de integración y qué significa, la configuración de
perfiles para varias suscripciones con ejemplos completos, y la solución de
problemas de detección por sistema operativo. El registro SHALL ser la única
fuente de los hechos de la guía y una verificación del pipeline MUST fallar
cuando guía y registro divergan: entrada sin sección, sección sin entrada, o
nivel y binarios distintos de los declarados. El enlace desde el README MUST
respetar la regla vigente de no nombrar productos de terceros fuera de datos
factuales de interoperabilidad.

#### Scenario: Cada entrada del registro tiene su sección con nivel y binarios
- **WHEN** corre la verificación de la guía de agentes
- **THEN** cada entrada del registro SHALL tener su sección en la guía
- **AND** la sección SHALL declarar el mismo nivel y los mismos binarios por capa que el registro

#### Scenario: Entrada nueva o renombrada falla la verificación
- **WHEN** el registro gana o renombra una entrada sin actualizar la guía
- **THEN** la verificación SHALL fallar señalando la entrada y la sección ausente

#### Scenario: Ejemplos de configuración de perfiles válidos
- **WHEN** la verificación revisa los ejemplos de perfiles de la guía
- **THEN** cada ejemplo SHALL parsear como configuración válida del proyecto
- **AND** SHALL NOT contener material secreto en claro

#### Scenario: Solución de problemas de detección por sistema operativo
- **WHEN** un usuario cuyo agente no se detecta consulta la guía
- **THEN** SHALL encontrar los síntomas y los remedios por sistema operativo, incluidos los shims de script en Windows

#### Scenario: Enlace desde el README sin nombrar productos
- **WHEN** corre el lint de documentación sobre el README
- **THEN** el enlace a la guía SHALL resolver a un archivo existente
- **AND** el README SHALL seguir sin nombrar productos de terceros

### Requirement: Guía de la capa empaquetada y de la vía de terceros
La guía de agentes SHALL explicar que la capa adaptador de las entradas con
adaptador propio viaja en los instaladores de Meltemi — sin comando de
instalación de terceros — y qué hacer cuando falta: reinstalar o reparar la
instalación de Meltemi. La guía SHALL documentar además la receta para
seguir usando un adaptador de terceros por configuración del usuario,
presentada como vía disponible y no recomendada, y SHALL decir que el
estatus y la nota legal que Meltemi muestra describen la vía que Meltemi
distribuye y no esa otra, remitiendo al lector a la licencia del adaptador y
a los términos del proveedor, que son los que sí la gobiernan. La guía MUST
NOT prestar a una vía de terceros el estatus ni la nota de la entrada. La
guía MUST NOT presentar la ruta propia como bendecida por proveedor alguno:
el estatus y la nota del registro se muestran tal cual, y toda actualización
de postura MUST citar fuente.

#### Scenario: La capa empaquetada explicada sin comando de terceros
- **WHEN** el lector consulta la sección de una entrada con adaptador propio
- **THEN** la guía SHALL decir que la capa adaptador viaja con los instaladores de Meltemi
- **AND** el remedio documentado ante su ausencia SHALL ser reinstalar o reparar Meltemi, no un comando de terceros

#### Scenario: Receta de adaptador de terceros por configuración
- **WHEN** el lector busca cómo usar un adaptador de terceros en lugar del propio
- **THEN** la guía SHALL mostrar la declaración por configuración del usuario con un ejemplo válido
- **AND** SHALL decir que el estatus y la nota del registro no cubren esa vía y remitir a los términos que sí la gobiernan, sin recomendarla

#### Scenario: Sin bendición inventada
- **WHEN** corre la verificación de coherencia entre registro y guía
- **THEN** el estatus y la nota legal de cada entrada SHALL coincidir con los declarados en el registro
- **AND** la guía SHALL NOT afirmar aprobación de proveedor que el registro no declare
