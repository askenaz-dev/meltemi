# registro-de-la-sesion-de-calle

> Vía rápida. 6º de la tanda de `release-para-un-desarrollador` (2026-10-28).
> Nace de dos hallazgos del 2026-09-26: el 3 de la corrida integrada
> (`docs/qa/2026-09-26-corrida-integrada.md`, «`meltemi log` no encuentra el
> registro de una sesión de worktree») y el 5 de la aceptación del hito v0.1
> (`docs/qa/2026-09-26-hito-v01-aceptacion.md`, «Los worktrees de tarea no se
> excluyen del árbol principal»).

## Why

Una sesión de `worktree/dispatch` —una calle de la carrera— queda en el índice
con el worktree como raíz, a propósito: es el árbol donde corrió el turno y la
clave con que el tablero une una calle con su sesión (`tablero-de-carrera` D2).
Pero su registro se escribe bajo la clave del proyecto principal. Un cliente
que pide `session/log` con la raíz que el listado le dio busca bajo la clave
del worktree, no encuentra nada y recibe una página vacía con código `0`. En la
corrida integrada, los permisos de los tres despachos hubo que leerlos a mano de
los archivos. Le pasa al verbo `log` del CLI, al detalle de sesión de la TUI y
al de la GUI por igual, porque los tres piden lo mismo.

Lo mismo vale al reanudar esa sesión: la nueva se registraría bajo la clave del
worktree, y el listado del proyecto dejaría de verla.

Y `worktree/assign` y `worktree/dispatch` crean sus worktrees dentro de
`.meltemi/worktrees/`, pero solo el taller de una change añade esa raíz a
`.git/info/exclude`. En un proyecto donde nunca se abrió un taller, un
`git add -A` en el árbol principal recoge cada calle como un repositorio
embebido.

## What Changes

- **El registro de una sesión de calle se lee por su calle.** El daemon sabe a
  qué proyecto pertenece una ruta de worktree gestionado por su forma
  (`<raíz>/.meltemi/worktrees/<change>/<calle>`), y `session/log`, la búsqueda
  de una sesión por raíz y la reanudación usan la clave de ese proyecto. La
  raíz que el índice guarda para la calle no cambia: el tablero la sigue usando.
- **Una calle terminada se puede reanudar.** Un despacho termina marcando su
  sesión como terminada sin sacarla del registro vivo, y `session/direct` la
  rehusaba como «activa pero no dirigible» mientras el daemon viviera. Ahora
  solo rehúsa una sesión que sigue corriendo.
- **La raíz gestionada queda excluida desde el primer worktree**, sea de tarea
  o de taller, por la misma vía local de siempre: `.git/info/exclude`, nunca el
  `.gitignore` del usuario.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `session-history`: + el registro de una sesión de calle se lee por su calle.
- `worktree-orchestration`: + la raíz gestionada excluida desde el primer
  worktree de tarea.

## Impact

- `core/meltemid/src/worktrees.rs`, `core/meltemid/src/server.rs` y sus tests;
  un e2e nuevo contra un fixture temporal con el agente simulado.
- Cero dependencias, cero RPC, cero cambios en el contrato: la misma petición
  con la misma raíz ahora encuentra lo que antes no.

## Fuera de alcance

- Mover el registro de la calle bajo la clave del worktree: la clave del
  proyecto es la que agrupa sus sesiones, y la calle desaparece cuando se
  limpia.
- Reescribir el índice de sesiones ya grabadas: no hace falta, la lectura es la
  que cambia.
