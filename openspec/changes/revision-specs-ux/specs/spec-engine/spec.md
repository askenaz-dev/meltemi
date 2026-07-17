## ADDED Requirements

### Requirement: Diagnóstico de requisito duplicado
El motor SHALL detectar como diagnóstico un requisito ADDED cuyo nombre
normalizado coincide con uno existente de la capacidad tras aplicar el delta, con
ubicación en el delta que lo introduce.

#### Scenario: Duplicado detectado
- **WHEN** un delta añade un requisito cuyo nombre normalizado ya existe en la capacidad
- **THEN** la validación SHALL reportar el duplicado con archivo y línea

### Requirement: Diagnóstico de modificación sin efecto
El motor SHALL detectar como diagnóstico un delta MODIFIED cuyo contenido es
idéntico al requisito vivo (statement y escenarios), señalando que la operación
no produce cambio alguno.

#### Scenario: No-op señalado
- **WHEN** un delta MODIFIED replica exactamente el requisito vivo
- **THEN** la validación SHALL reportar la modificación sin efecto

### Requirement: Diagnóstico de referencia colgante
El motor SHALL detectar como diagnóstico una mención explícita a un requisito
(«Requirement: nombre») que no existe en la capacidad tras aplicar el delta.

#### Scenario: Referencia rota
- **WHEN** un requisito menciona por nombre a otro que el delta elimina
- **THEN** la validación SHALL reportar la referencia colgante con su ubicación
