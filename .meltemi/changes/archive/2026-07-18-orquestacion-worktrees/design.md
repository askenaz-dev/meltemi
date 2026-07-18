## Context

Toda sesión trabaja hoy sobre el repo tal cual: dos agentes concurrentes se
pisarían. La promesa (§6.3) es N agentes × M tareas en worktrees aislados, con
carreras y merge asistido por humano. El histórico de sesiones (#8) ya existe en
este punto del orden; la columna worktree/rama de la tabla de Sesiones está
reservada desde el shell.

**Nota de provisionalidad**: la forma exacta de asignación tarea↔agente se
revalida contra el design real de #14 (tasks.md del ciclo) al frente de la cola.

## Goals / Non-Goals

**Goals:** ciclo de vida de worktrees gestionados; sesión ligada a worktree;
asignación N×M; carreras con resultados comparables; merge asistido mínimo en
terminal; degradación honesta en repos no-git.
**Non-Goals:** checkpoints (#17) y commits por tarea (#18); resolución
automática de conflictos (jamás automágica); ejecución multi-máquina.

## Decisions

### D1 — Git del usuario, sin libgit2
Todas las operaciones (worktree add/list/remove, diff) invocan el `git` del
usuario (proceso hijo, salida parseada). Cero dependencias nativas nuevas; el
comportamiento coincide con el git que el usuario ya conoce. Versión mínima de
git verificada al primer uso con diagnóstico claro.

### D2 — Worktrees gestionados con nomenclatura estable
`<repo>/.meltemi/worktrees/<change>/<tarea>-<agente>` sobre ramas
`meltemi/<change>/<tarea>-<agente>`. El daemon registra qué worktrees son suyos
(estado en datos); jamás toca worktrees que no creó. Limpieza segura: solo
worktrees gestionados, con confirmación si hay cambios sin commitear.

### D3 — Sesión nace en su worktree
La asignación crea (o reutiliza) el worktree y lanza la sesión con cwd en él; la
tabla de Sesiones llena su columna reservada. Una sesión sin asignación (flujo
actual) sigue corriendo sobre el repo: compatibilidad intacta, advertida cuando
hay más de una sesión simultánea en el mismo árbol.

### D4 — Carreras etiquetadas
La misma tarea a ≥2 agentes = un worktree por agente, sesiones etiquetadas como
competidoras de la misma tarea. Al terminar, el resultado por agente queda como
diff contra la base común.

### D5 — Merge asistido mínimo viable en terminal
Vista de comparación: diffs en competencia lado a lado (o conmutables en anchos
menores, per reflow del shell), elección de base y aplicación selectiva **por
archivo** (v0.1; por hunk llega con la superficie de revisión de código). Nada se
mezcla sin decisión humana explícita.

### D6 — Secuenciación por solapamiento
Antes de asignar en paralelo, el daemon calcula solapamiento de archivos
declarado entre tareas (de `tasks.md` de #14/plan) y serializa las que comparten
archivos, informándolo.

## Risks / Trade-offs

- **Deriva de la base durante la carrera** → la base común queda fijada al crear
  los worktrees (misma revisión); el merge asistido opera contra ella.
- **Worktrees huérfanos tras caídas** → registro propio + limpieza segura listable.
- **Espacio en disco** → visible por worktree en la vista; limpieza a un gesto.

## Migration Plan

Aditivo. El flujo sin worktree se conserva. Reversión: eliminar worktrees
gestionados (git worktree remove) sin tocar el repo.

## Open Questions

- Aplicación selectiva por hunk (¿aquí o con la revisión de código?): v0.1 por
  archivo, decisión anotada.
