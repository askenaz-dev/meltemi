# tui-shell — delta

## ADDED Requirements

### Requirement: Vínculo de suscripción en el shell

El shell interactivo SHALL ofrecer el vínculo de suscripción desde la paleta
de comandos con captura verbatim de sus argumentos: el nombre del vínculo
MUST llegar al daemon exactamente como se escribió, sin la minusculización de
la línea de paleta. El resultado — el gesto de login o el rehúso con su
remedio — SHALL quedar visible en los avisos del shell.

#### Scenario: El verbo de vínculo captura el nombre tal cual

- **WHEN** el usuario invoca el verbo de vínculo desde la paleta y escribe
  agente y nombre en el campo de captura
- **THEN** el nombre SHALL viajar al daemon sin alteración de mayúsculas
- **AND** el aviso SHALL traer el gesto de login compuesto

#### Scenario: El rehúso llega con su remedio al shell

- **IF** el vínculo rehúsa
- **THEN** el aviso del shell SHALL traer el diagnóstico y el remedio del
  daemon
- **AND** el shell MUST NOT descartar en silencio lo escrito
