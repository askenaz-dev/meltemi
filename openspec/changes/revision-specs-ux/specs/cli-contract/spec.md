## MODIFIED Requirements

### Requirement: Gramática de subcomandos y reserva
El binario `meltemi` SHALL exponer una gramática de subcomandos estable: los
subcomandos operativos (`status`, `propose`, `stop`, `fleet`, `project`,
`sessions`, `explore`, `plan`, `constitution`, `review`, `version`, `help`) y los
subcomandos del ciclo SDD reservados (`implement`, `verify`, `archive`). Un
subcomando reservado MUST reconocerse como parte de la gramática y no como error
de uso.

#### Scenario: Subcomando operativo reconocido
- **WHEN** se invoca `meltemi status`
- **THEN** el binario SHALL despachar el subcomando `status` en modo scriptable

#### Scenario: Subcomando reservado no es error de uso
- **WHEN** se invoca un subcomando reservado aún no implementado (p. ej. `meltemi verify`)
- **THEN** el binario SHALL informar por stderr que el subcomando está reservado y aún no implementado
- **AND** SHALL terminar con un código distinto del de subcomando desconocido

#### Scenario: Subcomando desconocido
- **WHEN** se invoca `meltemi` con un token que no pertenece a la gramática
- **THEN** el binario SHALL emitir un error de uso por stderr y terminar con el código de error de uso

### Requirement: Mapeo comando↔método RPC
Cada subcomando respaldado por RPC SHALL enviar `initialize` como primer mensaje y
luego el método correspondiente: `status`→`status`, `propose`→`propose`,
`stop`→`shutdown`, `fleet`→`fleet/list`, `project`→`context/project`,
`sessions`→`session/list`, `explore`→`sdd/explore`, `plan`→`sdd/plan`,
`constitution`→`sdd/constitution`, `review`→`sdd/review`. Los subcomandos locales
(`version`, `help`) MUST NOT abrir conexión con el daemon.

#### Scenario: initialize precede a todo método RPC
- **WHEN** un subcomando respaldado por RPC abre una conexión con el daemon
- **THEN** el binario SHALL enviar `initialize` antes de cualquier otro método

#### Scenario: review conduce la checklist
- **WHEN** se invoca `meltemi review` con el daemon accesible
- **THEN** el binario SHALL invocar el método `sdd/review` y presentar la checklist

#### Scenario: Los subcomandos locales no tocan el daemon
- **WHEN** se invoca `meltemi version` o `meltemi help`
- **THEN** el binario SHALL responder localmente
- **AND** SHALL NOT abrir conexión con el daemon
