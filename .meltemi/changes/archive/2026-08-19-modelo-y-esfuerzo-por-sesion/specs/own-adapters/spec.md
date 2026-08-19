# own-adapters — delta

## ADDED Requirements

### Requirement: Cada adaptador traduce lo que su proveedor documenta

Cada adaptador propio SHALL traducir modelo y esfuerzo a lo que su proveedor
acepta oficialmente, en el punto donde ese proveedor lo acepta, y SHALL rehusar
con diagnóstico lo que no. NO SHALL inventar una traducción no verificada
contra la versión pineada del proveedor.

#### Scenario: El adaptador manda la palanca donde su proveedor la acepta

- **WHEN** una sesión declara modelo para un adaptador que lo admite
- **THEN** SHALL viajar en el punto que el protocolo de ese proveedor define

#### Scenario: Lo no verificado se rehúsa en vez de inventarse

- **WHERE** el adaptador no tiene verificado que su proveedor acepte una palanca
- **THEN** SHALL rehusarla con ese motivo
- **AND** NO SHALL enviar una traducción adivinada
