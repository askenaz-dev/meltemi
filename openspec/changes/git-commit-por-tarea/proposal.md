## Why

"Cada línea de código debe poder rastrearse hasta el requisito que la originó"
(constitución §8). El mecanismo es el **commit atómico por tarea** (§6.8): un
commit por tarea completada, con un trailer de trazabilidad que apunta al
requisito EARS y a la tarea. Este repo ya practica la convención a mano; el
producto debe imponerla para el trabajo de los agentes.

## What Changes

- **Commit por tarea**: al completarse una tarea de `tasks.md` en su worktree,
  el daemon crea el commit atómico correspondiente (o lo propone, según modo
  supervisado/autónomo).
- **Trailer de trazabilidad**: `Meltemi-Task: <change>/<n.m>` y
  `Meltemi-Req: <capability>/<requirement>` — legibles por máquina, estables.
  **Nunca trailers de co-autoría**; la autoría es del usuario.
- **Convención de mensajes**: título imperativo en inglés, cuerpo con el qué/por
  qué, referencia `(<change> <tarea>)` — la convención de este repo, formalizada.
- **Verificación**: el daemon valida que el árbol queda limpio tras cada tarea y
  que el commit contiene solo lo tocado por esa tarea (disciplina atómica).

## Capabilities

### New Capabilities
- `git-per-task`: commit atómico, trailers de trazabilidad y convención.

### Modified Capabilities
- `worktree-orchestration`: el ciclo de tarea culmina en commit (o propuesta).

## Impact

- `core/meltemid` (git plumbing vía CLI del usuario), `tui/` (revisión del
  commit propuesto antes de aplicar, en modo supervisado).

## Fuera de alcance

- Push/PR/forjas remotas (el usuario decide cuándo y cómo publica).
- Firmado de commits (configuración del usuario; se respeta la suya).
- Revisión de diffs línea a línea enriquecida (GUI fase 2).
