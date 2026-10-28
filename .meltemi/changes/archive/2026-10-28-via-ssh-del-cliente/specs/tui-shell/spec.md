# tui-shell — delta

## ADDED Requirements

### Requirement: La TUI dice cuándo no está hablando con su propia máquina

WHERE la TUI hable con el daemon a través de un comando, su cromo SHALL
mostrarlo de forma legible, con el comando abreviado, por la misma razón que
la GUI.

#### Scenario: El cromo de la TUI nombra la vía

- **WHEN** la TUI está conectada a través de un comando
- **THEN** su cromo muestra que habla por esa vía, con el comando abreviado
