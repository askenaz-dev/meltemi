# worktree-orchestration — delta

## ADDED Requirements

### Requirement: Taller de change en su propia rama

El daemon SHALL ofrecer, a demanda, el taller de una change: por defecto, una
rama con el nombre de la change creada desde la punta de la rama por defecto, y
un worktree gestionado con nomenclatura estable dentro de la raíz gestionada
del proyecto. La petición MAY nombrar la rama del taller — si la nombrada no
existe se crea desde la punta de la rama por defecto; si existe, la elección
explícita SHALL entenderse como consentimiento para trabajar sobre ella. La
petición MAY pedir en su lugar un taller único: rama y worktree con un sufijo
único, de modo que varios talleres de la misma change coexistan sin pisarse.
La petición por defecto SHALL ser idempotente — si el taller ya existe y es
gestionado, se devuelve con su ruta y su rama, declarando que es un
reencuentro; un taller único SHALL ser siempre una creación nueva. WHERE exista
una rama homónima que el daemon no creó y la petición no la haya nombrado
explícitamente, la petición SHALL rehusarse con diagnóstico y remedio, y el
daemon MUST NOT tocarla. La raíz gestionada SHALL quedar excluida del estado de
git del árbol principal por vía local, sin modificar el `.gitignore` versionado
del usuario.

#### Scenario: El primer taller se crea desde la rama por defecto

- **WHEN** se pide el taller de una change que no lo tiene
- **THEN** el daemon SHALL crear la rama con el nombre de la change desde la
  punta de la rama por defecto
- **AND** SHALL crear su worktree gestionado y devolver ruta y rama

#### Scenario: Pedirlo de nuevo reencuentra, no falla

- **WHEN** se pide el taller de una change que ya lo tiene
- **THEN** el daemon SHALL devolver el existente con su ruta y su rama
- **AND** SHALL declarar que es un reencuentro, no una creación

#### Scenario: El taller sobre una rama elegida

- **WHEN** se pide el taller nombrando una rama
- **THEN** el worktree SHALL crearse sobre esa rama, creándola desde la punta
  de la rama por defecto si no existe
- **AND** nombrarla explícitamente SHALL valer como consentimiento aunque el
  daemon no la haya creado

#### Scenario: Un taller único no colisiona con nadie

- **WHEN** se pide un taller único para una change
- **THEN** su rama y su worktree SHALL llevar un sufijo único
- **AND** varios talleres de la misma change SHALL coexistir sin pisarse
- **AND** la respuesta SHALL declararlo creación, nunca reencuentro

#### Scenario: La rama ajena se rehúsa sin tocarse

- **WHERE** existe una rama con el nombre de la change que el daemon no creó
- **THEN** la petición SHALL rehusarse con diagnóstico y remedio
- **AND** la rama NO SHALL modificarse

#### Scenario: El taller no ensucia el estado del árbol principal

- **WHEN** existe al menos un taller gestionado
- **THEN** el estado de git del árbol principal NO SHALL listar la raíz
  gestionada como contenido sin seguimiento
- **AND** el `.gitignore` versionado del usuario NO SHALL haberse modificado

### Requirement: Aterrizaje del taller con decisión explícita

El daemon SHALL fusionar la rama del taller en la rama por defecto únicamente
con confirmación explícita; sin ella, SHALL previsualizar qué commits
aterrizarían y qué archivos tocan. La fusión SHALL conservar la forma de la
change en el grafo. El daemon SHALL rehusarse con diagnóstico y remedio cuando
el taller tenga cambios sin commitear, y cuando la fusión produzca conflictos —
en cuyo caso SHALL abortar la fusión dejando la rama por defecto intacta, y
MUST NOT resolver conflicto alguno por su cuenta.

#### Scenario: Sin confirmación, la previsualización

- **WHEN** se pide aterrizar sin confirmación
- **THEN** el daemon SHALL responder los commits que aterrizarían y los
  archivos que tocan
- **AND** NO SHALL fusionar nada

#### Scenario: Con confirmación, el aterrizaje limpio

- **WHEN** se pide aterrizar con confirmación y la fusión aplica limpia
- **THEN** la rama del taller SHALL quedar fusionada en la rama por defecto
- **AND** la forma de la change SHALL quedar visible en el grafo

#### Scenario: El conflicto se rehúsa y no deja el árbol a medias

- **WHERE** la fusión produce conflictos
- **THEN** el daemon SHALL abortar la fusión dejando la rama por defecto
  intacta
- **AND** SHALL rehusar con diagnóstico y el remedio de resolver en el git del
  usuario

#### Scenario: El taller sucio no aterriza

- **WHERE** el taller tiene cambios sin commitear
- **THEN** el aterrizaje SHALL rehusarse con diagnóstico y remedio
- **AND** nada SHALL fusionarse

### Requirement: El taller sin aterrizar no se pierde en silencio

Retirar el taller de una change cuya rama contiene commits que la rama por
defecto no alcanza SHALL exigir confirmación explícita, y el aviso SHALL decir
cuántos commits quedarían solo en la rama. Retirar el taller SHALL retirar el
worktree y NO SHALL borrar la rama.

#### Scenario: Retirar con commits sin aterrizar exige confirmación

- **WHERE** la rama del taller tiene commits que la rama por defecto no alcanza
- **WHEN** se pide retirar el taller sin confirmación
- **THEN** el daemon SHALL rehusar diciendo cuántos commits quedarían solo en
  la rama

#### Scenario: Retirar el taller conserva la rama

- **WHEN** se retira el taller de una change
- **THEN** el worktree gestionado SHALL desaparecer
- **AND** la rama SHALL permanecer
