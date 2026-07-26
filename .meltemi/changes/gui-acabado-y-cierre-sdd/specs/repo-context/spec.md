# repo-context — delta

## ADDED Requirements

### Requirement: Metadirectorio de git fuera del mapa
El mapa del repositorio (`repo/map`) SHALL excluir el metadirectorio `.git`
en cualquier nivel del árbol, sin consumirlo del presupuesto de truncado,
mientras los directorios ocultos que sí son contexto (como `.meltemi/`)
SHALL seguir listándose.

#### Scenario: Metadirectorio de git fuera del mapa
- **WHEN** se construye el mapa de un repositorio git
- **THEN** ninguna entrada SHALL pertenecer a `.git`
- **AND** `.meltemi/` SHALL seguir presente en el mapa
