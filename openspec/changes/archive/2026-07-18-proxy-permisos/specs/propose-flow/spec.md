## ADDED Requirements

### Requirement: Honestidad del resultado ante denegaciones
El resultado de `propose` SHALL declarar cuántas peticiones de permiso fueron
denegadas durante el turno (`deniedPermissions`), y las superficies MUST
presentarlo de forma visible: la CLI en su salida humana y en el objeto `--json`,
advirtiendo que el artefacto puede estar incompleto. La palabra de estado del
turno MUST presentarse en forma estable en minúsculas y las rutas MUST
normalizarse al separador de la plataforma.

#### Scenario: Propose con permiso denegado lo declara
- **WHEN** un `propose` termina habiendo denegado al menos una petición del agente
- **THEN** el resultado SHALL incluir el conteo de denegaciones
- **AND** la salida humana SHALL advertir que la propuesta puede haber quedado incompleta

#### Scenario: Estado estable y rutas normalizadas
- **WHEN** la CLI presenta el resultado de un propose
- **THEN** la palabra de estado SHALL emitirse en minúsculas estables (p. ej. `completed`)
- **AND** la ruta del artefacto SHALL usar un separador uniforme de la plataforma
