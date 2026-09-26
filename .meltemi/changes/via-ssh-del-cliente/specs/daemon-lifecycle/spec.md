# daemon-lifecycle — delta

## ADDED Requirements

### Requirement: Un endpoint con forma de comando nunca arranca un daemon

WHERE el endpoint del cliente nombre un comando en lugar de un socket, el
cliente NO SHALL arrancar un daemon local bajo ninguna circunstancia, ni
siquiera cuando ese comando falle o no responda. Arrancar uno serviría una
máquina en el nombre de otra.

#### Scenario: El comando falla y no se arranca nada

- **WHEN** el endpoint nombra un comando y ese comando termina sin hablar el protocolo
- **THEN** el cliente rehúsa con su diagnóstico
- **AND** no queda ningún daemon arrancado por ese intento
