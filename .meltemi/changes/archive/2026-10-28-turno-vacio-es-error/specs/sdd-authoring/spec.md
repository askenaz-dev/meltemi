# sdd-authoring — delta

## ADDED Requirements

### Requirement: Los verbos de autoría dicen si algún turno fue vacío
WHEN algún turno de autoría que corre una llamada a `sdd/explore`,
`sdd/propose`, `sdd/plan`, `sdd/constitution` o `sdd/gate` se declara vacío, su
resultado SHALL traer `emptyTurn` en `true`, y SHALL omitir el campo cuando
todos sus turnos llevaron trabajo o la llamada no corrió ninguno.

#### Scenario: Un gate cuyo turno de autoría fue vacío lo declara
- **WHEN** un `sdd/gate` corre el turno de autoría del siguiente artefacto y ese turno se declara vacío
- **THEN** el resultado del gate SHALL traer `emptyTurn` en `true`

#### Scenario: La vía rápida declara el vacío de cualquiera de sus turnos
- **WHEN** un `sdd/propose` en vía rápida corre sus turnos de autoría y uno de ellos se declara vacío
- **THEN** el resultado SHALL traer `emptyTurn` en `true`
