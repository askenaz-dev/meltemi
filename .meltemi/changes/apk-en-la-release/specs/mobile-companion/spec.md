# mobile-companion — delta

## ADDED Requirements

### Requirement: Probado en un teléfono antes de publicarse
Antes de que el estado declarado del compañero diga «publicado», SHALL existir
un informe de una corrida en un teléfono Android real con el APK de una release:
instalarlo, conectar por el túnel SSH con la clave del compañero, ver la flota
y aprobar una petición de permiso. El informe SHALL nombrar el modelo, la
versión de Android y lo que no se pudo ejercer. Un paso no ejercido MUST NOT
escribirse como aprobado.

#### Scenario: La corrida en el teléfono queda escrita
- **WHEN** se corre el compañero en un teléfono real con el APK de una release
- **THEN** el informe SHALL registrar la instalación, la conexión por el túnel, la flota y una aprobación desde el teléfono, con el modelo y la versión de Android

#### Scenario: Sin informe del teléfono no se declara publicado
- **IF** no existe el informe de la corrida en un teléfono real
- **THEN** el estado declarado del compañero SHALL NOT decir «publicado»
