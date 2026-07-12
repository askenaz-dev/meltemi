# propose-flow Specification

## Purpose
TBD - created by archiving change fase-0-fundacion. Update Purpose after archive.
## Requirements
### Requirement: Inicialización de `.meltemi/`
Cuando se invoque `propose` en un repositorio sin `.meltemi/`, `meltemid` SHALL crear la estructura mínima (`.meltemi/changes/`) sin sobrescribir ningún archivo existente.

#### Scenario: Repositorio sin .meltemi/
- **WHEN** un cliente invoca `propose` en un repositorio que no contiene `.meltemi/`
- **THEN** `meltemid` crea `.meltemi/changes/` y continúa el flujo sin error

### Requirement: Andamiaje determinista de la propuesta
El método `propose(idea)` SHALL derivar un nombre kebab-case a partir de la idea, crear `.meltemi/changes/<nombre>/proposal.md` con el esqueleto estándar de forma determinista (sin intervención del agente), y SHALL fallar con un error claro si el directorio ya existe.

#### Scenario: Propuesta nueva
- **WHEN** un cliente invoca `propose` con una idea y el nombre derivado no existe
- **THEN** se crea `.meltemi/changes/<nombre>/proposal.md` con las secciones estándar antes de involucrar al agente

#### Scenario: Colisión de nombre
- **WHEN** el nombre derivado ya existe en `.meltemi/changes/`
- **THEN** el cliente recibe un error que incluye el nombre en conflicto y una sugerencia de alternativa, y no se modifica nada

### Requirement: Delegación del contenido al agente ACP
Tras el andamiaje, `meltemid` SHALL enviar a la sesión ACP un prompt con la idea y la ruta del esqueleto para que el agente complete `proposal.md`, con el directorio de trabajo en la raíz del repositorio y las escrituras del agente sujetas al passthrough de permisos de `acp-session`.

#### Scenario: Flujo completo de extremo a extremo
- **WHEN** un cliente invoca `propose` con una idea válida y aprueba las peticiones de permiso del agente
- **THEN** al finalizar el turno, `.meltemi/changes/<nombre>/proposal.md` contiene contenido generado por el agente y el cliente recibió el streaming del progreso y un resultado final con la ruta creada

### Requirement: Resultado final estructurado
Al completarse o fallar el flujo, el cliente SHALL recibir un resultado estructurado con el nombre del cambio, la ruta creada y el estado final del turno del agente.

#### Scenario: Reporte de finalización
- **WHEN** el turno del agente finaliza con éxito
- **THEN** la respuesta del método `propose` incluye nombre del cambio, ruta de `proposal.md` y estado `completed`

