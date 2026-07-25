# release-distribution Specification

## Purpose
TBD - created by archiving change distribucion-releases. Update Purpose after archive.
## Requirements

### Requirement: Versionado con política escrita
El proyecto SHALL versionar con SemVer pre-1.0 y versión única de workspace, con
una política escrita que defina qué constituye ruptura (contrato `proto/`,
gramática CLI, formato de artefactos); todo release SHALL originarse de un tag y
su pipeline.

#### Scenario: Ruptura exige minor
- **WHEN** un release incluye un cambio de contrato definido como ruptura
- **THEN** la versión SHALL incrementar el minor pre-1.0
- **AND** el cambio SHALL constar en las notas del release

### Requirement: Pipeline de release con gates duros
El pipeline SHALL construir y probar en Windows, macOS y Linux con gates
obligatorios: suite completa, clippy sin warnings, formato, auditoría de
dependencias y presupuestos de rendimiento medidos (tamaño del binario de la TUI
y arranque); cualquier gate rojo MUST abortar el release sin publicar artefacto
alguno.

#### Scenario: Presupuesto excedido aborta
- **IF** el binario de la TUI supera su presupuesto en el gate
- **THEN** el release SHALL abortarse
- **AND** ningún artefacto SHALL publicarse

### Requirement: Artefactos firmados con custodia documentada
Cada artefacto publicado SHALL acompañarse de su checksum y firma verificables,
y el procedimiento de custodia de la clave (generación, almacenamiento, rotación
y revocación) MUST estar documentado y en manos del mantenedor.

#### Scenario: Verificación por el usuario
- **WHEN** un usuario descarga un binario de release
- **THEN** SHALL poder verificar checksum y firma con instrucciones publicadas

### Requirement: Instalador auditable
El proyecto SHALL ofrecer un instalador de una línea por plataforma cuyo script
sea corto, legible y con hash publicado, junto a instrucciones manuales
equivalentes; la instalación SHALL colocar `meltemi` y `meltemid` y crear el
alias `mel`.

#### Scenario: Instalación con alias
- **WHEN** el instalador concluye
- **THEN** `meltemi`, `meltemid` y el alias `mel` SHALL quedar disponibles en el PATH del usuario

### Requirement: Espacios de nombres reservados
Los crates `meltemi`, `meltemid` y `meltemi-proto` SHALL quedar publicados en el
registro de crates — el contrato real y placeholders honestos que apuntan al
repositorio — para asegurar el espacio de nombres del proyecto.

#### Scenario: Crates apuntan al proyecto
- **WHEN** alguien consulta los crates del proyecto
- **THEN** SHALL encontrar el contrato publicado y los placeholders con su referencia al repositorio

### Requirement: Instaladores de la GUI de escritorio
El pipeline de release SHALL producir instaladores firmados de la GUI por
plataforma — MSI en Windows, DMG en macOS, AppImage y deb en Linux — con la
misma custodia de firmas y los mismos gates del pipeline existente. El
instalador SHALL mantenerse por debajo de 15 MB por plataforma — el runtime de
webview del sistema se aprovecha o se bootstrapea, MUST NOT embeberse — y el
tamaño MUST verificarse como gate bloqueante del pipeline.

#### Scenario: Instalador firmado por plataforma
- **WHEN** el pipeline publica una release con GUI
- **THEN** cada plataforma SHALL recibir su instalador firmado bajo la custodia documentada

#### Scenario: Presupuesto de tamaño como gate
- **WHEN** un build de release produce un instalador de GUI que excede 15 MB
- **THEN** el pipeline SHALL fallar el gate de tamaño
