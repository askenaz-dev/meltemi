# integration-levels — delta

## ADDED Requirements

### Requirement: El nivel verificado de un adaptador propio se re-otorga al re-anclar

WHEN se re-ancla el esquema vendorizado de un adaptador propio a una versión
nueva del CLI, la corrida de conformidad manual contra el binario real SHALL
repetirse, y el nivel verificado SHALL otorgarse solo con las comprobaciones
de su nivel medidas contra esa versión. El registro de la corrida SHALL
nombrar la versión del CLI, la fecha, la política de aprobación efectiva
observada y, para cada comprobación fallida, las palabras del proveedor.

#### Scenario: El re-anclaje exige la corrida real
- **WHEN** se re-ancla el volcado de un adaptador propio
- **THEN** la corrida de conformidad manual SHALL repetirse contra la versión anclada
- **AND** el nivel verificado SHALL constar solo si sus comprobaciones pasaron

#### Scenario: Una comprobación fallida se registra con las palabras del proveedor
- **WHEN** una comprobación de la corrida real falla
- **THEN** el registro SHALL citar el motivo que dio el proveedor
