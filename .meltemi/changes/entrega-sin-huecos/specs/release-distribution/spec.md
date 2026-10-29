# release-distribution — delta

## ADDED Requirements

### Requirement: El estado de cada capacidad tiene dueño
Cada capacidad que el producto declara en su documentación o en su sitio
(núcleo, escritorio, compañero móvil, SDK) SHALL tener inventariados su estado
verificable y la change que cierra lo que le falta, y una brecha de entrega
SHALL NOT quedar sin change que la nombre. La política de versiones SHALL
decir cómo sube la versión una release que marca un hito probado de punta a
punta.

#### Scenario: Cada brecha con su dueño
- **WHEN** se revisa el inventario de la tanda de entrega
- **THEN** cada capacidad declarada SHALL tener su estado y la change que cierra su brecha, o la constancia de que no tiene ninguna

#### Scenario: La política dice qué hace un hito
- **WHEN** se lee la política de versiones
- **THEN** SHALL decir que un hito probado de punta a punta sube el minor aunque nada en él rompa
