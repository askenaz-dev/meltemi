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

