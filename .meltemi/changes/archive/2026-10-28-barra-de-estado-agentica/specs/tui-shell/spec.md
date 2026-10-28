# tui-shell — delta

## ADDED Requirements

### Requirement: El chrome del terminal dice la change y su compuerta

El chrome persistente SHALL mostrar, cuando el proyecto lleva el método, la
change que reclama una decisión junto al artefacto que la espera. La señal
SHALL respetar la prioridad vigente al reducirse el tamaño: SHALL cederse antes
que la conexión y que las decisiones pendientes.

#### Scenario: El chrome nombra la compuerta que espera

- **WHEN** una change reclama una decisión
- **THEN** el chrome SHALL nombrarla junto al artefacto que la espera

#### Scenario: La compuerta cede antes que la conexión

- **WHEN** el ancho disponible obliga a reducir el chrome
- **THEN** la señal de la compuerta SHALL cederse antes que la conexión y que
  las decisiones pendientes
