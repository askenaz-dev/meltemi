## Why

La culminación operativa (§5.2): `/implement` despliega agentes sobre `tasks.md`
y los conduce tarea a tarea con la disciplina completa — planificar/actuar,
supervisado/autónomo, checkpoints, permisos y commits. Todas sus piezas llegan
por separado (#9 permisos, #14 ciclo, #16 worktrees, #17 checkpoints, #18
commits); esta change las compone en el verbo que ejecuta el plan.

## What Changes

- **`/implement`**: toma la change activa, secuencia `tasks.md` por dependencias
  y despliega el/los agentes asignados tarea a tarea.
- **Planificar/actuar**: en modo planificar, el agente propone su plan por tarea
  y el humano lo aprueba antes de tocar nada; en actuar, ejecuta directo dentro
  de guardarraíles.
- **Supervisado/autónomo**: aprobación cambio a cambio, o autonomía dentro de
  reglas del proxy (#9) con checkpoints (#17) y commit por tarea (#18).
- **Composición**: cada tarea = checkpoint → turno(s) del agente en su worktree
  (#16) → verificación rápida (build/tests si la tarea los declara) → commit con
  trazabilidad → tick en `tasks.md`.
- **Progreso vivo**: la vista de Sesión muestra tarea actual/restantes; el
  streaming existente cubre el detalle.

## Capabilities

### New Capabilities
- `implement-command`: el despliegue orquestado de agentes sobre tasks.md.

### Modified Capabilities
- `sdd-authoring`: el ciclo gana su fase de ejecución.
- `cli-contract`: `implement` deja de estar reservado.

## Impact

- `core/meltemid` (orquestador de tareas — compone capacidades existentes),
  `tui/`. Es la change de integración: su design fija los contratos entre
  piezas; conviene escribirlo cuando #9/#16/#17/#18 tengan design real.

## Fuera de alcance

- Carreras multi-agente por tarea como flujo por defecto (existen vía #16; aquí
  la asignación estándar es 1 agente por tarea).
- Hooks por evento (fase 2).
