# tui-shell — delta

## ADDED Requirements

### Requirement: La lista de sesiones muestra el título

La lista de sesiones del shell SHALL mostrar el título de cada sesión que lo
tenga, junto al identificador y al agente que ya muestra, recortándolo al ancho
disponible sin desplazar las demás columnas. Una sesión sin título SHALL
mostrarse como antes de esta capacidad.

#### Scenario: La lista de sesiones muestra el título

- **WHEN** la lista muestra una sesión con título
- **THEN** SHALL mostrar su título junto al identificador y al agente
- **AND** SHALL recortarlo al ancho disponible sin desplazar las demás columnas
