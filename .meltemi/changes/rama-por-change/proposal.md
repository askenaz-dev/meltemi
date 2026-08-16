# rama-por-change

> Vía completa (proposal → design → specs → tasks). El delta es **solo ADDED**
> sobre una capability existente (`worktree-orchestration`, que ninguna change
> activa toca), pero el alcance no es de un día: dos métodos nuevos del
> contrato con sus schemas y conformidad, daemon, dos verbos del CLI, la fila
> de paridad con sus tres superficies, y e2e contra repos fixture. La forma
> calificaría para vía rápida; el alcance no.

## Why

El mantenedor instauró un flujo de trabajo (2026-08-16) y lo pidió con estas
palabras: cada change «en ramas separadas por change y al terminar [...] merge
a main». Se lo pidió a cada sesión de Claude que trabaja sobre este
repositorio, y las sesiones lo hacen **a mano**: `git worktree add -b
<change> ../repo-<change> main`, trabajar ahí, y fusionar al cerrar.

El porqué no es estético. Con varias sesiones commiteando en `main` desde el
mismo árbol, un `git add` amplio de una arrastra los archivos a medio escribir
de otra: ocurrió **tres veces en un solo día** (commits `94ecbab`, `d54a1aa` y
un tercero), siempre con el contenido intacto y los gates verdes, pero con la
trazabilidad de la constitución §8 —un commit atómico por tarea, con su
referencia— mezclada entre changes ajenas. El aislamiento por rama con
worktree propio elimina la clase entera de accidente: árboles distintos no
comparten índice.

Y aquí está el hueco: **Meltemi ya orquesta worktrees, pero en el eje
equivocado para este flujo**. `worktree-orchestration` crea worktrees por
*change, tarea y agente* para las carreras — varios agentes compitiendo sobre
la misma tarea, con merge asistido archivo por archivo. Lo que el flujo del
mantenedor necesita es el eje *change*: un taller aislado donde vive **toda**
la change, con su rama, y un aterrizaje a la rama por defecto cuando cierra.
Hoy eso se hace con git a mano, que funciona — pero es exactamente la clase de
disciplina repetible que este producto existe para hospedar, y cada sesión la
improvisa con sus propias convenciones de nombre y de limpieza.

## What Changes

- **`change/workspace`**: el daemon crea a demanda el taller de una change —
  una rama con el nombre de la change, desde la punta de la rama por defecto,
  y un worktree gestionado con nomenclatura estable. Pedirlo de nuevo devuelve
  el existente en vez de fallar: el verbo es «dame el taller», no «créalo».
  Una rama con ese nombre que Meltemi no creó produce rehúso honesto con
  remedio — el daemon jamás toca lo que no es suyo.
- **`change/land`**: fusiona la rama del taller en la rama por defecto, solo
  con confirmación explícita; sin `confirm` **previsualiza** (qué commits, qué
  archivos), como ya hacen `commit` y `revert`. Un taller con cambios sin
  commitear rehúsa; un merge con conflictos rehúsa con diagnóstico y remedio —
  los conflictos se resuelven en el git del usuario, jamás automáticamente.
- **El taller no ensucia el árbol principal**: la raíz gestionada queda
  excluida del estado de git por vía local (`.git/info/exclude`), sin tocar el
  `.gitignore` versionado del usuario.
- **Retirar un taller cuya rama tiene commits sin aterrizar exige
  confirmación**, extendiendo la regla que ya protege los cambios sin
  commitear: trabajo no fusionado no desaparece en silencio.
- **CLI**: `meltemi workspace <change>` y `meltemi land <change> [confirm]`.
  TUI y GUI los consumen por el registro obligatorio de métodos (paleta y
  registry), como manda la paridad §4; la vista dedicada no nace aquí.

## Capabilities

### Modified Capabilities

- `worktree-orchestration`: + tres requisitos ADDED — el taller de change en su
  propia rama (creación idempotente, propiedad, exclusión del estado), el
  aterrizaje con decisión explícita (previsualización, confirmación, rehúso
  ante conflictos), y la protección del taller sin aterrizar. El eje
  tarea×agente de las carreras no se toca: son requisitos hermanos, no
  reemplazos.

### New Capabilities

- Ninguna.

## Impact

- `proto/meltemi-proto` (dos métodos, params/results, schemas y conformidad),
  `core/meltemid` (`worktrees.rs`/`git.rs` ganan el eje change),
  `tui` (dos verbos + paleta), `desktop/ui` (registry + `gen:forms`),
  `docs/paridad-nucleo.md` (dos filas), `docs/referencia-cli.md` (regenerada),
  e2e en `core/meltemid/tests/` contra repos fixture.
- **Nace deber de paridad §4 y se paga aquí**: método nuevo del daemon →
  paleta TUI + registry GUI + fila de paridad, con el gate bloqueante
  (`tui/tests/parity.rs`) forzándolo.
- Cero dependencias nuevas: todo es el git del usuario, como las carreras.

## Fuera de alcance

- **Resolver conflictos de merge**: jamás. El rehúso nombra el remedio y el
  merge conflictivo es del humano en su git.
- **Mover las changes ya en `main`** a ramas retroactivamente: historia
  publicada no se reescribe.
- **Borrar la rama tras aterrizar**: el aterrizaje deja la rama; borrarla es
  decisión del usuario (o de una limpieza futura con su propia confirmación).
- **Una vista dedicada de talleres en la GUI**: la paleta la cubre por
  paridad; la vista es candidata futura si el uso la pide.
- **Cambiar las carreras** (eje tarea×agente): siguen exactamente como están.
