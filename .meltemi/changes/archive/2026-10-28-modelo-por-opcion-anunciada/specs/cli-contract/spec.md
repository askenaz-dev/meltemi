# cli-contract — delta

## ADDED Requirements

### Requirement: El CLI dice la palanca que rigió o por qué se rehusó
WHEN un subcomando arranca una sesión con modelo o esfuerzo, su salida humana
SHALL decir el modelo y el esfuerzo que rigieron, y un rehúso de palanca SHALL
imprimirse con la palanca, lo pedido, la razón y lo que el agente anunció o
dejó, a lo sumo veinte valores y cuántos más, con todo texto del agente escapado
antes de llegar al terminal. Con `--json`, stdout SHALL seguir llevando un único
objeto.

#### Scenario: El CLI dice el modelo que rigió
- **WHEN** una sesión arranca con un modelo que rige
- **THEN** la salida humana SHALL decir ese modelo

#### Scenario: El rehúso se imprime acotado y escapado
- **WHEN** un rehúso trae más de veinte valores anunciados o texto con caracteres de control
- **THEN** el CLI SHALL imprimir a lo sumo veinte y cuántos más
- **AND** SHALL escapar los caracteres de control
