# gui-shell — delta

## ADDED Requirements

### Requirement: Las pestañas y listas nombran el trabajo

Donde la superficie nombra una sesión —pestañas, lista de sesiones, encabezado
del detalle, árbol y recientes— SHALL mostrar su título cuando lo tenga, con la
identidad del agente conservada como marca visual. El identificador de la
sesión NO SHALL desaparecer: SHALL permanecer accesible junto al título o en el
texto emergente, que además SHALL seguir diciendo el proyecto al que pertenece.
Una sesión sin título SHALL nombrarse como antes de esta capacidad, sin
sustituto inventado.

#### Scenario: La pestaña dice de qué trata la sesión

- **WHEN** una sesión con título está abierta en una pestaña
- **THEN** el rótulo de la pestaña SHALL ser su título
- **AND** el identificador completo y el proyecto SHALL seguir en su texto
  emergente

#### Scenario: Una sesión sin título se nombra como antes

- **WHEN** una sesión carece de título
- **THEN** SHALL nombrarse con su agente y su identificador corto

### Requirement: El proyecto se antepone solo cuando hace falta

Cuando las pestañas abiertas pertenecen a más de un proyecto, el rótulo de cada
una SHALL anteponer el nombre de su proyecto. Cuando todas pertenecen al mismo,
NO SHALL anteponerlo.

#### Scenario: El proyecto se antepone ante ambigüedad

- **WHEN** las pestañas abiertas cruzan más de un proyecto
- **THEN** cada rótulo SHALL anteponer el nombre de su proyecto

#### Scenario: Con un solo proyecto el rótulo no lo repite

- **WHEN** todas las pestañas abiertas pertenecen al mismo proyecto
- **THEN** ningún rótulo SHALL anteponer el nombre del proyecto
