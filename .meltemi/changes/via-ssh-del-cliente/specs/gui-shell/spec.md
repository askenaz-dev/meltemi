# gui-shell — delta

## ADDED Requirements

### Requirement: La GUI dice cuándo no está hablando con su propia máquina

WHERE la GUI hable con el daemon a través de un comando, su cromo SHALL
mostrarlo de forma legible, con el comando abreviado. Una superficie que
callara la diferencia haría que una acción sobre la máquina equivocada
pareciera una acción sobre la propia.

#### Scenario: El cromo nombra la vía

- **WHEN** la GUI está conectada a través de un comando
- **THEN** su barra de estado muestra que habla por esa vía, con el comando abreviado
