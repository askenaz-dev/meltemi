## MODIFIED Requirements

### Requirement: Instaladores de la GUI de escritorio
El pipeline de release SHALL producir instaladores firmados de la GUI por
plataforma — MSI en Windows, DMG en macOS, paquete `.deb` en Linux — con la
misma custodia de firmas y los mismos gates del pipeline existente. Ningún
artefacto publicado SHALL embeber un motor de navegador: el runtime de webview
del sistema se aprovecha, se bootstrapea o se declara como dependencia del
paquete, de modo que el instalador SHALL mantenerse por debajo de 15 MB en toda
plataforma, y el tamaño MUST verificarse como gate bloqueante del pipeline.
WHERE un formato de empaquetado sea autocontenido por construcción y no permita
declarar el motor como dependencia externa, el pipeline MUST NOT publicarlo.

#### Scenario: Instalador firmado por plataforma
- **WHEN** el pipeline publica una release con GUI
- **THEN** cada plataforma SHALL recibir su instalador firmado bajo la custodia documentada

#### Scenario: Presupuesto de tamaño como gate
- **WHEN** un build de release produce un instalador de GUI que excede 15 MB
- **THEN** el pipeline SHALL fallar el gate de tamaño

#### Scenario: Formato autocontenido no se publica
- **IF** un formato de empaquetado embebe el motor de navegador en el artefacto
- **THEN** el pipeline SHALL NOT producirlo ni publicarlo
- **AND** la documentación de descargas SHALL NOT nombrarlo

#### Scenario: El paquete declara el motor del sistema
- **WHEN** el pipeline produce el paquete de Linux
- **THEN** el paquete SHALL declarar el runtime de webview del sistema entre sus dependencias
- **AND** la instalación en una máquina sin ese runtime SHALL fallar nombrando lo que falta, en vez de instalar y no arrancar
