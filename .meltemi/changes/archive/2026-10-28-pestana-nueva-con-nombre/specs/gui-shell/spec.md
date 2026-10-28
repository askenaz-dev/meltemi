# gui-shell — delta

## ADDED Requirements

### Requirement: La pestaña del compositor se nombra con palabras
La pestaña del compositor de una sesión nueva SHALL nombrarse «Nueva sesión»
—en el idioma de la superficie— en la tira de pestañas, en la miga y en la
sección de abiertas de la barra, y su id interno MUST NOT mostrarse en ninguna
de ellas.

#### Scenario: El compositor abierto no muestra su id
- **WHEN** el compositor de una sesión nueva está abierto
- **THEN** la miga y la sección de abiertas SHALL nombrarlo «Nueva sesión»
- **AND** ninguna superficie SHALL mostrar su id interno
