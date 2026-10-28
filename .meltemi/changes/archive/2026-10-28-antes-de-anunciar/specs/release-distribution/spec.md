# release-distribution — delta

## ADDED Requirements

### Requirement: Changelog por release generado del archivo
La herramienta de release SHALL escribir en `CHANGELOG.md` la sección de una
versión con cada change archivada que ninguna otra sección lista, cada una con
su nombre y la frase que la resume tomada de su propuesta, y opcionalmente solo
las archivadas hasta una fecha, que SHALL ser la fecha de la sección. La
sección SHALL titularse con la versión en el formato que lee la extracción de
notas de la release, y escribir dos veces la misma versión SHALL dar el mismo
archivo. La documentación de releases SHALL declarar que cada release lista
las changes que archiva y la cadencia: una release al cerrar cada tanda, y
nunca más de un mes sin release mientras la rama principal avance.

#### Scenario: Cada release lista las changes que archiva
- **WHEN** se escribe la sección de una versión
- **THEN** SHALL listar cada change archivada que ninguna sección anterior lista
- **AND** SHALL NOT repetir una que ya figura en otra

#### Scenario: Las notas de la release salen de esa sección
- **WHEN** se piden las notas de una versión cuya sección se escribió
- **THEN** la extracción SHALL devolver esa sección
