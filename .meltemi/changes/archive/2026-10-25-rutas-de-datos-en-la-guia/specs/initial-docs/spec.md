# initial-docs — delta

## ADDED Requirements

### Requirement: La guía de plataformas dice dónde guarda el producto
La guía de plataformas SHALL nombrar, para Windows, macOS y Linux, la carpeta
de datos (registros de sesión y métricas locales) y la de configuración
(ajustes y reglas de permisos) que el producto usa, y MUST coincidir con las
que el código resuelve en la plataforma donde corre la verificación: una
carpeta que la guía nombre para la plataforma de la verificación y que el
código no resuelva MUST fallar el pipeline.

#### Scenario: Las carpetas de cada plataforma
- **WHEN** el lector busca dónde guarda Meltemi sus datos y su configuración
- **THEN** SHALL encontrar en la guía la carpeta de datos y la de configuración de Windows, macOS y Linux

#### Scenario: La guía coincide con el código
- **WHEN** corre la verificación de la guía de plataformas en una de las tres plataformas
- **THEN** las carpetas que el código resuelve en ella SHALL ser las que la guía nombra para esa plataforma
- **AND** la verificación SHALL fallar si difieren
