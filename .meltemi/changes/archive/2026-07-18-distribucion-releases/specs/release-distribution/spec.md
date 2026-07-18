## ADDED Requirements

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
