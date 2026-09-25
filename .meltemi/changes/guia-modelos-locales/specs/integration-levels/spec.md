# integration-levels — delta

## ADDED Requirements

### Requirement: Pierna de proveedor local en la conformidad manual
El procedimiento de conformidad manual SHALL incluir una pierna de proveedor
local: un agente de nivel 1 contra un modelo servido en la propia máquina,
ejercida a mano y por opt-in sobre la sesión, el streaming, los permisos y la
cancelación, registrando la versión del agente, el modelo y los tiempos
medidos. Mientras nadie la haya corrido en vivo, la pierna SHALL figurar como
pendiente junto a lo verificado en forma, y MUST NOT presentarse como aprobada.

#### Scenario: La pierna declara sus criterios y lo que registra
- **WHEN** el mantenedor consulta la corrida manual
- **THEN** SHALL encontrar la pierna de proveedor local con sus cuatro criterios y lo que debe registrar

#### Scenario: Sin corrida en vivo la pierna queda pendiente
- **WHERE** la pierna no se ha corrido contra un modelo local vivo
- **THEN** SHALL figurar como pendiente con lo verificado en forma
- **AND** MUST NOT presentarse como aprobada
