# initial-docs — delta

## ADDED Requirements

### Requirement: La guía de agentes declara la versión verificada de cada adaptador propio con rango

La guía de agentes SHALL declarar, para cada entrada pilotada por un adaptador
propio que declara un rango soportado, la versión mínima del CLI oficial que el
adaptador soporta, el remedio cuando el CLI instalado es más viejo o más nuevo
que la versión verificada, la compuerta que el adaptador configura en el CLI y
lo que esa compuerta no cubre, y el nombre con que el adaptador se presenta
ante el CLI. Una verificación del pipeline MUST fallar cuando la versión mínima
de la guía difiera de la que el adaptador declara.

#### Scenario: La guía y el adaptador declaran la misma versión mínima
- **WHEN** corre la verificación de la guía de agentes
- **THEN** la versión mínima de cada entrada con rango SHALL coincidir con el piso que declara su adaptador
- **AND** una divergencia SHALL fallar la verificación
