# release-distribution — delta

## ADDED Requirements

### Requirement: El contrato viaja con cada release
Cada release SHALL publicar los JSON Schemas del contrato daemon ↔ clientes de
esa versión, con su índice y su licencia, como un asset de nombre estable
dentro del `SHA256SUMS` que cubre su atestación. El contenido del asset MUST
ser exactamente el del árbol del commit publicado, y el índice MUST nombrar
cada schema. La versión del protocolo que declara el índice SHALL coincidir con
la del código y con el directorio de los schemas. La documentación SHALL decir
que el asset es el contrato y no un SDK.

#### Scenario: El contrato publicado con la release
- **WHEN** el pipeline publica una release
- **THEN** el asset del contrato SHALL constar con su nombre estable en el `SHA256SUMS` que cubre la atestación

#### Scenario: El asset es el árbol
- **IF** la lista de archivos del asset difiere de los schemas del árbol más el índice y la licencia
- **THEN** el pipeline SHALL fallar mostrando la diferencia
- **AND** SHALL NOT publicar

#### Scenario: Un schema sin fila en el índice falla
- **IF** un schema del contrato no tiene fila en el índice
- **THEN** la verificación SHALL fallar nombrando el schema

#### Scenario: La versión del índice es la del protocolo
- **IF** la versión que declara el índice no es la del código, o no existe su directorio de schemas
- **THEN** la verificación SHALL fallar nombrando las dos versiones
