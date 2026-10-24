# gui-shell — delta

## ADDED Requirements

### Requirement: La GUI configura y prueba el aviso de espera

La GUI SHALL mostrar en sus ajustes si el aviso de espera está configurado y
SHALL ofrecer enviar un aviso de prueba que recorra exactamente el mismo camino
que el aviso real. El comando SHALL ejecutarse desde el proceso nativo de la
aplicación y NO SHALL ejecutarse desde la vista web.

#### Scenario: La prueba usa el mismo camino que el aviso real

- **WHEN** el usuario envía un aviso de prueba desde los ajustes
- **THEN** se ejecuta el mismo comando con la clase de espera de prueba
- **AND** el resultado —enviado o fallido— se muestra donde se pidió
