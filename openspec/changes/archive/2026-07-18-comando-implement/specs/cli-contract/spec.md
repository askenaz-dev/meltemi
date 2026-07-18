## MODIFIED Requirements

### Requirement: Gramática de subcomandos y reserva
El binario `meltemi` SHALL exponer una gramática de subcomandos estable, toda
operativa: `status`, `propose`, `stop`, `fleet`, `project`, `sessions`,
`explore`, `plan`, `constitution`, `review`, `verify`, `archive`, `implement`,
`version`, `help`. Un token fuera de la gramática MUST tratarse como error de
uso.

#### Scenario: Subcomando operativo reconocido
- **WHEN** se invoca `meltemi status`
- **THEN** el binario SHALL despachar el subcomando `status` en modo scriptable

#### Scenario: El ciclo completo es operativo
- **WHEN** se invoca `meltemi implement` con el daemon accesible
- **THEN** el binario SHALL despacharlo como subcomando operativo
- **AND** ningún subcomando del ciclo SDD SHALL permanecer reservado

#### Scenario: Subcomando desconocido
- **WHEN** se invoca `meltemi` con un token que no pertenece a la gramática
- **THEN** el binario SHALL emitir un error de uso por stderr y terminar con el código de error de uso

### Requirement: Mapeo comando↔método RPC
Cada subcomando respaldado por RPC SHALL enviar `initialize` como primer mensaje y
luego el método correspondiente: `status`→`status`, `propose`→`propose`,
`stop`→`shutdown`, `fleet`→`fleet/list`, `project`→`context/project`,
`sessions`→`session/list`, `explore`→`sdd/explore`, `plan`→`sdd/plan`,
`constitution`→`sdd/constitution`, `review`→`sdd/review`, `verify`→`sdd/verify`,
`archive`→`sdd/archive`, `implement`→`sdd/implement`. Los subcomandos locales
(`version`, `help`) MUST NOT abrir conexión con el daemon.

#### Scenario: initialize precede a todo método RPC
- **WHEN** un subcomando respaldado por RPC abre una conexión con el daemon
- **THEN** el binario SHALL enviar `initialize` antes de cualquier otro método

#### Scenario: implement despliega el plan
- **WHEN** se invoca `meltemi implement` con el daemon accesible
- **THEN** el binario SHALL invocar el método `sdd/implement` y presentar el progreso por tarea

#### Scenario: Los subcomandos locales no tocan el daemon
- **WHEN** se invoca `meltemi version` o `meltemi help`
- **THEN** el binario SHALL responder localmente
- **AND** SHALL NOT abrir conexión con el daemon
