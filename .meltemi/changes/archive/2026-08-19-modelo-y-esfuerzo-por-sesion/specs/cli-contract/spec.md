# cli-contract — delta

## ADDED Requirements

### Requirement: Modelo y esfuerzo se declaran al arrancar desde la CLI

Los verbos que arrancan una sesión SHALL admitir declarar modelo y esfuerzo, y
su ayuda SHALL decir que son cadenas del proveedor que el núcleo no interpreta.
Un valor vacío SHALL rehusarse; NO SHALL enviarse una cadena vacía como si
fuera una elección.

#### Scenario: Un valor vacío se rehúsa en vez de viajar

- **WHEN** se arranca una sesión con un modelo vacío
- **THEN** SHALL rehusarse
- **AND** NO SHALL arrancar con una cadena vacía
