## Why

La promesa central de orquestación (§6.3): **N agentes sobre M tareas sin pisarse**,
incluida la carrera de dos agentes sobre la misma tarea. Hoy toda sesión trabaja
sobre el repo tal cual; dos agentes concurrentes serían un choque garantizado. El
aislamiento por worktrees de git es el mecanismo; la mezcla es un **merge asistido
por humano**, nunca automágico.

## What Changes

- **Worktrees aislados por tarea/sesión**: creación, ciclo de vida y limpieza de
  worktrees gestionados por el daemon (nomenclatura estable, estado consultable).
- **Asignación N×M**: lanzar agentes sobre tareas con su worktree; la columna
  worktree/rama reservada en la tabla de Sesiones se llena.
- **Carreras**: la misma tarea a ≥2 agentes en worktrees separados, resultados
  comparables lado a lado.
- **Merge asistido**: presentación de diffs en competencia, elección de base y
  aplicación selectiva de parches; los conflictos se minimizan secuenciando en
  `tasks.md` las tareas que comparten archivos (análisis de solapamiento).
- **Contrato**: métodos de worktree y asignación (aditivos).

## Capabilities

### New Capabilities
- `worktree-orchestration`: aislamiento, asignación, carreras y merge asistido.

### Modified Capabilities
- `acp-session`: una sesión nace ligada a un worktree (cwd del agente).
- `tui-shell`: la columna y acciones reservadas de worktree se materializan.

## Impact

- `core/meltemid` (git worktrees vía CLI de git del usuario), `proto/`, `tui/`.
- Requiere repos git; en repos no-git, degradación honesta (sesión sin
  aislamiento, advertida).

## Fuera de alcance

- Checkpoints/rollback (#17) y commit por tarea (#18) — se montan sobre esto.
- Ejecución remota multi-máquina (fase 3+).
