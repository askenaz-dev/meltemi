# context-projection Specification

## Purpose
TBD - created by archiving change proyeccion-contexto. Update Purpose after archive.
## Requirements

### Requirement: Compilación determinista del contexto
El motor SHALL compilar constitución, rumbo y la change activa en un documento de
contexto de forma determinista: el mismo árbol de artefactos MUST producir
exactamente el mismo texto. El rumbo SHALL respetar su regla de inclusión:
`siempre` se inyecta íntegro; `por-patrón` y `manual` se referencian como
disponibles sin inyectarse.

#### Scenario: Misma entrada, mismo documento
- **WHEN** se compila dos veces el mismo árbol `.meltemi/`
- **THEN** el texto resultante SHALL ser idéntico byte a byte

#### Scenario: Inclusión por regla
- **WHERE** un archivo de rumbo declara `inclusion: por-patrón`
- **THEN** el documento SHALL referenciarlo como disponible
- **AND** SHALL NOT inyectar su contenido completo

### Requirement: Bloques gestionados que preservan al usuario
La proyección SHALL escribir únicamente dentro de marcadores gestionados con
huella de origen, y MUST preservar byte a byte todo contenido fuera de ellos. Si
el archivo destino no tiene marcadores, la proyección SHALL anexarlos al final;
la escritura MUST ser atómica.

#### Scenario: Contenido del usuario intacto
- **WHEN** se proyecta sobre un archivo con contenido del usuario fuera de los marcadores
- **THEN** ese contenido SHALL permanecer idéntico
- **AND** solo el interior de los marcadores SHALL cambiar

#### Scenario: Idempotencia
- **WHEN** se proyecta dos veces sin cambios en las fuentes
- **THEN** la segunda escritura SHALL dejar el archivo sin modificaciones

### Requirement: Destinos y variantes declarados en datos
Los destinos de proyección SHALL declararse en un mapa de datos versionado
(archivo base `AGENTS.md` siempre presente; variantes por formato de agente), y
los artefactos del método MUST NOT nombrar productos de terceros: los nombres
viven en los datos.

#### Scenario: Variante generada según el mapa
- **WHEN** el mapa de destinos declara una variante adicional
- **THEN** la proyección SHALL escribir esa variante con el mismo contenido compilado y sus marcadores propios

### Requirement: Proyección bajo demanda por contrato
El daemon SHALL exponer `context/project` (proyecto como parámetro) que compila y
escribe todos los destinos declarados, reportando qué archivos tocó y su huella;
el subcomando CLI `project` SHALL invocarlo siguiendo la disciplina scriptable
(`--json` con un objeto; códigos de la taxonomía).

#### Scenario: Regeneración reportada
- **WHEN** un cliente invoca `context/project` sobre un proyecto válido
- **THEN** la respuesta SHALL listar los destinos escritos con su huella

#### Scenario: CLI project
- **WHEN** se invoca `meltemi project --json`
- **THEN** el binario SHALL emitir exactamente un objeto JSON con el reporte

### Requirement: Dogfooding de la proyección
El propio repositorio de Meltemi SHALL usar la proyección generada como fuente de
su contexto raíz: la nota de proyección manual MUST retirarse y el bloque
gestionado MUST reflejar constitución y rumbo vigentes.

#### Scenario: AGENTS.md del repo gestionado
- **WHEN** se ejecuta la proyección sobre este repositorio
- **THEN** `AGENTS.md` SHALL contener el bloque gestionado actualizado
- **AND** el contenido manual restante SHALL permanecer intacto

### Requirement: Las reglas del proyecto viajan en el bloque gestionado

Las reglas de ámbito proyecto SHALL proyectarse dentro del bloque gestionado
existente de los destinos declarados, con su ámbito de aplicación expresado en
prosa junto a cada una. Todo lo de fuera del bloque SHALL seguir preservándose
byte a byte.

#### Scenario: Una regla del proyecto llega a los destinos del repositorio

- **WHEN** el proyecto declara una regla y se proyecta el contexto
- **THEN** SHALL aparecer dentro del bloque gestionado de cada destino
- **AND** SHALL decir a qué archivos aplica

### Requirement: Lo global del usuario jamás entra al repositorio

El contenido de los ámbitos globales del usuario SHALL proyectarse únicamente a
destinos de ámbito usuario, y NO SHALL entrar en archivo alguno del
repositorio.

La separación SHALL ser estructural —la compilación de lo que va al repositorio
no recibe las fuentes globales— y NO SHALL depender de una comprobación que una
refactorización pueda omitir.

#### Scenario: Proyectar con harness global deja el repositorio intacto

- **WHERE** el usuario tiene harness global declarado
- **WHEN** se proyecta el contexto de un proyecto
- **THEN** los archivos del repositorio NO SHALL contener ese contenido
- **AND** el estado del repositorio SHALL quedar sin cambios por ese contenido

### Requirement: Destinos de ámbito usuario solo donde estén verificados

El mapa de destinos SHALL admitir rutas de ámbito usuario por agente, cada una
con la fecha en que se verificó contra la documentación de su proveedor. Un
agente sin ruta verificada SHALL declararse como tal en la vista y NO SHALL
recibir escritura alguna.

La primera escritura sobre un archivo de ámbito usuario de un agente SHALL
requerir consentimiento explícito y quedar registrada. Meltemi SHALL escribir
únicamente dentro de su propio bloque gestionado y NO SHALL sobrescribir
contenido ajeno.

#### Scenario: Un agente sin destino verificado no recibe escritura

- **WHERE** un agente no tiene ruta de ámbito usuario verificada
- **THEN** SHALL declararse la ausencia
- **AND** NO SHALL escribirse archivo alguno para ese agente

#### Scenario: La primera escritura en config ajena se consiente

- **WHEN** Meltemi va a escribir por primera vez en el archivo de usuario de un
  agente
- **THEN** SHALL pedirse consentimiento explícito
- **AND** la decisión SHALL quedar registrada
