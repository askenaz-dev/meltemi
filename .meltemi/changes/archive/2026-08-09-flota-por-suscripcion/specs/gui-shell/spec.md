# gui-shell — delta

## ADDED Requirements

### Requirement: La flota se lee por agente y por suscripción

La vista Flota SHALL presentar cada agente del catálogo seguido de las
suscripciones enlazadas a él, y cada fila de suscripción SHALL declarar **como
texto** de qué agente lo es. Cada agente con suscripciones SHALL declarar
cuántas tiene. Una suscripción cuyo agente subyacente no esté en el catálogo NO
SHALL desaparecer del listado: SHALL mostrarse marcada, con el identificador que
declara.

#### Scenario: Varias suscripciones del mismo agente se leen juntas

- **WHEN** un agente del catálogo tiene varias suscripciones enlazadas
- **THEN** SHALL presentarse seguido de todas ellas
- **AND** cada una SHALL declarar como texto de qué agente lo es
- **AND** el agente SHALL declarar cuántas tiene

#### Scenario: La suscripción sin agente conocido no desaparece

- **WHEN** una suscripción declara un agente que no está en el catálogo
- **THEN** SHALL listarse igualmente, marcada
- **AND** SHALL mostrar el identificador que declara

#### Scenario: La relación no depende de la sangría

- **WHEN** una fila de suscripción se lee como texto
- **THEN** el agente al que pertenece SHALL estar en su contenido
- **AND** NO SHALL depender de su posición ni de su sangría
