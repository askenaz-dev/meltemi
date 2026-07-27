# cli-contract — delta

## ADDED Requirements

### Requirement: Flag de agente en los verbos que arrancan sesión
La gramática SHALL admitir una flag `--agent <id|perfil>` en los subcomandos que
arrancan una sesión de agente y cuyo método la acepta, y el parser global MUST
dejarla pasar a su subcomando en vez de rechazarla como flag desconocida: una
flag anunciada en la referencia y rechazada por el parser sería una promesa
incumplida de la gramática. Omitirla SHALL comportarse exactamente como hoy, y su
valor MUST llegar al daemon tal cual se escribió, sin normalizar mayúsculas. La
referencia CLI generada SHALL enumerarla donde aplique.

#### Scenario: Arranque con agente nombrado desde la CLI
- **WHEN** se invoca un subcomando de arranque de sesión con `--agent` y un id del catálogo
- **THEN** el binario SHALL enviar ese agente al método correspondiente
- **AND** el valor SHALL llegar sin alteración de mayúsculas

#### Scenario: La flag no es rechazada por el parser global
- **WHEN** se invoca un subcomando con `--agent` en cualquier posición de la línea
- **THEN** el binario SHALL NOT terminar con error de flag desconocida

#### Scenario: Sin la flag el comportamiento no cambia
- **WHEN** se invoca el mismo subcomando sin `--agent`
- **THEN** el daemon SHALL usar el agente configurado del proyecto
- **AND** la salida SHALL conservar su forma vigente

## MODIFIED Requirements

### Requirement: Mapeo comando↔método RPC
Cada subcomando respaldado por RPC SHALL enviar `initialize` como primer mensaje y
luego el método correspondiente: `status`→`status`, `propose`→`propose`,
`stop`→`shutdown`, `fleet`→`fleet/list`, `project`→`context/project`,
`sessions`→`session/list`, `session`→`session/start`, `explore`→`sdd/explore`,
`plan`→`sdd/plan`, `constitution`→`sdd/constitution`, `review`→`sdd/review`,
`verify`→`sdd/verify`, `archive`→`sdd/archive`, `implement`→`sdd/implement`,
`direct`→`session/direct`, `projects`→`project/list`,
`projects register`→`project/register`, `projects forget`→`project/forget`. Los
subcomandos locales (`version`, `help`) MUST NOT abrir conexión con el daemon. El
verbo de arranque de sesión libre MUST NOT nombrarse de forma que colisione con la
lectura del subcomando de apagado del daemon, y los subcomandos del registro de
proyectos MUST colgar de un verbo cuya gramática no confunda su discriminador con
una ruta.

#### Scenario: initialize precede a todo método RPC
- **WHEN** un subcomando respaldado por RPC abre una conexión con el daemon
- **THEN** el binario SHALL enviar `initialize` antes de cualquier otro método

#### Scenario: implement despliega el plan
- **WHEN** se invoca `meltemi implement` con el daemon accesible
- **THEN** el binario SHALL invocar el método `sdd/implement` y presentar el progreso por tarea

#### Scenario: El arranque de sesión libre tiene su subcomando
- **WHEN** se invoca el subcomando de arranque de sesión libre con una instrucción
- **THEN** el binario SHALL invocar el método de arranque de sesión libre
- **AND** SHALL presentar el identificador de la sesión creada y el desenlace del turno

#### Scenario: Alta y baja de proyecto desde la CLI
- **WHEN** se invoca el subcomando del registro de proyectos con su discriminador de alta o de baja y una ruta
- **THEN** el binario SHALL invocar el método de alta o de baja correspondiente
- **AND** el discriminador SHALL NOT interpretarse como la ruta del proyecto

#### Scenario: Los subcomandos locales no tocan el daemon
- **WHEN** se invoca `meltemi version` o `meltemi help`
- **THEN** el binario SHALL responder localmente
- **AND** SHALL NOT abrir conexión con el daemon
