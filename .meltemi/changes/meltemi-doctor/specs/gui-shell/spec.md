# gui-shell — delta

## ADDED Requirements

### Requirement: Diagnóstico en Ajustes
Ajustes SHALL tener una sección de diagnóstico que muestre el mismo informe que
`doctor`, fila por fila con su estado en palabra y símbolo y su remedio, en el
idioma de la interfaz, y un botón que lo copie al portapapeles. La sección NO
SHALL mandar el informe a ningún sitio.

#### Scenario: Ajustes muestra el diagnóstico y lo copia
- **WHEN** el usuario abre el diagnóstico en Ajustes
- **THEN** SHALL ver cada fila con su estado y su remedio
- **AND** copiarlo SHALL dejar el informe en el portapapeles y nada más
