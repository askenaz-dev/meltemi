# gui-shell — delta

## ADDED Requirements

### Requirement: El harness efectivo se ve desde la Flota

La superficie de escritorio SHALL ofrecer, desde la Flota, la lectura del
harness que aplica a un agente, mostrando por cada pieza su capa de origen y su
ruta, y mostrando también lo pisado y lo inválido con su motivo.

La vista SHALL ser de solo lectura: la autoría de las piezas vive en archivos.

#### Scenario: La ficha de un agente muestra su harness con el origen

- **WHEN** se abre el harness de un agente desde la Flota
- **THEN** SHALL verse cada pieza con la capa de la que viene
- **AND** SHALL verse lo que no aplica, con el motivo por el que no aplica
