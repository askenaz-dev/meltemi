# release-distribution — delta

## ADDED Requirements

### Requirement: El compañero móvil en las dos rutas de build
La ruta de integración y la ruta de release SHALL compilar el APK del
compañero móvil para `arm64-v8a` con el toolchain pineado, y SHALL medirlo
contra su presupuesto declarado con el mismo valor en las dos; pasarlo MUST
fallar el build. El APK de la ruta de integración SHALL poder instalarse y
SHALL declarar donde se descarga que su firma es efímera y no prueba origen.
La release SHALL publicar el APK solo firmado con la clave de publicación y
verificado, con un nombre estable y dentro del `SHA256SUMS` que cubre la
atestación; IF la firma de publicación no está disponible, THEN la release
SHALL publicarse sin el APK y MUST NOT publicar un APK sin esa firma.

#### Scenario: Push a main deja el APK descargable
- **WHEN** la rama principal avanza
- **THEN** la ruta de integración SHALL dejar un APK instalable del compañero
- **AND** su aviso SHALL declarar que la firma es efímera y no prueba origen

#### Scenario: APK sobre el techo falla
- **IF** el APK supera su presupuesto en cualquiera de las dos rutas
- **THEN** ese build SHALL fallar nombrando el tamaño y el techo

#### Scenario: El APK publicado lleva la firma de publicación
- **WHEN** el pipeline publica una release con el APK
- **THEN** el APK SHALL estar firmado con la clave de publicación y verificado antes de publicar
- **AND** SHALL constar con su nombre estable en el `SHA256SUMS` que cubre la atestación

#### Scenario: Sin firma de publicación la release sale sin APK
- **IF** la firma de publicación del APK no está disponible
- **THEN** la release SHALL publicarse sin el APK
- **AND** SHALL NOT incluir un APK sin firmar ni con firma efímera
