## ADDED Requirements

### Requirement: Instaladores de la GUI de escritorio
El pipeline de release SHALL producir instaladores firmados de la GUI por
plataforma — MSI en Windows, DMG en macOS, AppImage y deb en Linux — con la
misma custodia de firmas y los mismos gates del pipeline existente. El
instalador SHALL mantenerse por debajo de 15 MB por plataforma — el runtime de
webview del sistema se aprovecha o se bootstrapea, MUST NOT embeberse — y el
tamaño MUST verificarse como gate bloqueante del pipeline.

#### Scenario: Instalador firmado por plataforma
- **WHEN** el pipeline publica una release con GUI
- **THEN** cada plataforma SHALL recibir su instalador firmado bajo la custodia documentada

#### Scenario: Presupuesto de tamaño como gate
- **WHEN** un build de release produce un instalador de GUI que excede 15 MB
- **THEN** el pipeline SHALL fallar el gate de tamaño
