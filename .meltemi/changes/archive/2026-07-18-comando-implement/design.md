## Context

La composición final: `/implement` despliega agentes sobre `tasks.md` con toda la
disciplina ya construida — reglas de permisos (#9), ciclo y gates (#14),
worktrees (#16), checkpoints (#17) y commits por tarea (#18). Esta change
integra; casi no inventa.

**Nota de provisionalidad máxima**: es la change de integración — su design se
revalida obligatoriamente al frente de la cola contra los designs reales de sus
cinco dependencias.

## Goals / Non-Goals

**Goals:** despliegue tarea a tarea por dependencias; planificar/actuar por
tarea; supervisado/autónomo dentro de reglas; progreso vivo; interrupción segura.
**Non-Goals:** carreras como flujo por defecto (existen vía #16); hooks (fase
2); reintentos automáticos multi-agente (futuro con evidencia).

## Decisions

### D1 — El ciclo de tarea, compuesto
Por tarea elegible (dependencias satisfechas): checkpoint (#17) → turno(s) del
agente en su worktree (#16) con permisos según reglas (#9) → verificación rápida
si la tarea declara comando → commit con trazabilidad (#18) → tick en `tasks.md`.
El bucle lo conduce el daemon; el estado de progreso persiste en la change.

### D2 — Planificar/actuar por tarea
En modo planificar, el agente propone su plan de la tarea (texto corto) y el
humano lo aprueba como gate antes de tocar nada; en actuar, ejecuta directo. El
modo se elige por change con override por tarea.

### D3 — Supervisado/autónomo son los modos de #9/#18
Supervisado: permisos escalan al humano y el commit se propone. Autónomo:
permisos por reglas y commit directo — dentro de guardarraíles; sin reglas
definidas, autónomo degrada a supervisado con aviso (jamás autonomía por
accidente).

### D4 — Progreso e interrupción
Eventos de progreso por tarea (inicio/fin/estado) sobre el streaming existente;
la vista de Sesión muestra tarea actual y restantes. Interrumpir (`x` ya
reservada) entre tareas deja estado consistente (tick + commit de lo completado);
interrumpir a mitad de tarea cancela la sesión de esa tarea y su worktree queda
para revertir (#17) o inspeccionar.

## Risks / Trade-offs

- **Complejidad de integración** → cada pieza llega ya especificada y testeada;
  aquí se prueban las costuras (e2e de composición).
- **Autonomía accidental** → D3: degradación a supervisado sin reglas, con aviso.

## Migration Plan

Aditivo: verbo final des-reservado (la gramática queda sin reservados).

## Open Questions

- Paralelismo por defecto de tareas independientes (¿secuencial v0.1 y paralelo
  opt-in?): propuesto secuencial por defecto; revalidar con #16 en producción.
