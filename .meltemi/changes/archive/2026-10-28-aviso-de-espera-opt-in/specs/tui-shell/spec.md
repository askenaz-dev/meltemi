# tui-shell — delta

## ADDED Requirements

### Requirement: La TUI emite y prueba el aviso de espera

La TUI SHALL emitir el aviso configurado cuando algo empiece a esperar, con el
mismo contrato de contenido que la GUI, y SHALL ofrecer enviar un aviso de
prueba desde su paleta.

#### Scenario: La paleta de la TUI envía un aviso de prueba

- **WHEN** el usuario pide el aviso de prueba desde la paleta
- **THEN** se ejecuta el comando configurado con la clase de espera de prueba
- **AND** si no hay comando configurado, se dice sin fingir que se envió algo
