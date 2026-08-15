# acp-session — delta

## ADDED Requirements

### Requirement: Interrupción con relevo

La dirección de una sesión SHALL admitir pedir que el turno en vuelo se
interrumpa y que la instrucción la releve. La operación SHALL ser atómica: la
instrucción SHALL quedar encolada **antes** de que la interrupción se señale,
de modo que el borde del turno nunca observe la cola vacía entre ambas mitades.

Al drenar el turno interrumpido, la sesión SHALL continuar con la instrucción
que lo relevó, y NO SHALL terminar. Una cancelación de sesión SHALL seguir
terminándola, y un turno que el agente cancela por su cuenta —sin interrupción
pedida— SHALL seguir terminando la sesión como hasta ahora.

Una petición de permiso en vuelo al interrumpir SHALL resolverse como cancelada
y quedar registrada como tal: interrumpir NO SHALL dejar decisiones sin
desenlace en el registro. El registro SHALL permitir distinguir un turno que el
agente detuvo de uno que el usuario interrumpió.

Cuando una interrupción y una cancelación de sesión concurren, SHALL prevalecer
la cancelación.

#### Scenario: La instrucción releva al turno interrumpido

- **WHEN** se dirige una sesión activa pidiendo interrumpir
- **THEN** la instrucción SHALL quedar encolada antes de señalarse la
  interrupción
- **AND** al drenar el turno la sesión SHALL continuar con esa instrucción
- **AND** NO SHALL terminar

#### Scenario: Una cancelación sigue terminando la sesión

- **WHEN** se cancela una sesión
- **THEN** SHALL terminar, y ninguna instrucción encolada SHALL despacharse

#### Scenario: Un turno cancelado por el agente no continúa

- **WHEN** un turno termina cancelado sin que se haya pedido interrumpir
- **THEN** la sesión SHALL terminar como hasta ahora

#### Scenario: El permiso en vuelo se resuelve al interrumpir

- **WHILE** una petición de permiso espera una decisión
- **WHEN** el turno se interrumpe
- **THEN** la petición SHALL resolverse como cancelada
- **AND** SHALL quedar registrada con ese desenlace

#### Scenario: El registro dice quién detuvo el turno

- **WHEN** se lee el registro de una sesión interrumpida
- **THEN** SHALL poder distinguirse de un turno que el agente detuvo por su
  cuenta

#### Scenario: Cancelar gana a interrumpir

- **WHEN** una interrupción y una cancelación de sesión concurren
- **THEN** la sesión SHALL terminar
