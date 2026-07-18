## ADDED Requirements

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
