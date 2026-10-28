# cli-contract — delta

## ADDED Requirements

### Requirement: El modo se declara al arrancar desde la CLI

Los verbos que arrancan una sesión SHALL admitir declarar el modo, y su ayuda
SHALL nombrar los modos admitidos. Un nombre que no sea uno de ellos SHALL
rehusarse con los válidos, jamás degradarse a uno.

#### Scenario: Un modo desconocido se rehúsa con los válidos

- **WHEN** se arranca una sesión con un modo que no existe
- **THEN** SHALL rehusarse nombrando los modos admitidos
- **AND** NO SHALL arrancar con otro modo
