## MODIFIED Requirements

### Requirement: Presupuestos de huella de la GUI
El instalador de la GUI SHALL mantenerse por debajo de 15 MB en toda plataforma
con verificación bloqueante en el pipeline, sostenido por no embeber motor de
navegador en artefacto alguno; el arranque hasta shell interactivo SHALL quedar
por debajo de 1 segundo y la memoria en reposo por debajo de 80 MB en el hardware
de referencia, medidos y publicados por release en la documentación de QA. La
medición publicada SHALL cubrir el instalador de cada plataforma que la release
publique.

#### Scenario: Gate de tamaño del instalador
- **WHEN** un build de release produce un instalador que excede el presupuesto
- **THEN** el pipeline SHALL fallar el gate de tamaño

#### Scenario: Medición publicada por release
- **WHEN** se publica una release con GUI
- **THEN** las notas de QA SHALL incluir arranque y memoria en reposo medidos por plataforma

#### Scenario: Tamaño de instalador medido por plataforma publicada
- **WHEN** se publica una release con GUI
- **THEN** las notas de QA SHALL registrar el tamaño medido del instalador de cada plataforma publicada
- **AND** SHALL NOT declarar como medido un tamaño que nadie midió
