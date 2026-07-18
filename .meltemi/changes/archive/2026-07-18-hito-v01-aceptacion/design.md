## Context

El hito v0.1 (§10) tiene enunciado preciso: una funcionalidad de idea a código
íntegramente en terminal, specs revisables, **dos agentes de proveedores
distintos en paralelo**. Esta change lo vuelve spec ejecutable e informe
reproducible — la última verificación antes del tag.

## Goals / Non-Goals

**Goals:** el guion del hito como spec EARS; ejecución automatizada en CI con
dos agentes simulados de perfiles distintos; guion manual documentado para la
validación real del mantenedor; métricas §12 verificadas; informe de aceptación.
**Non-Goals:** features nuevas (si el guion revela huecos, vuelven como deltas a
la change dueña — esa es su función); métricas de adopción (objetivos, no
aceptación).

## Decisions

### D1 — Dos mock-agents con personalidades distintas en CI
La constitución prohíbe agentes reales y red en CI: el "dos proveedores" se
ejercita con dos agentes simulados de perfiles divergentes (estilos de plan,
latencias, formas de salida) corriendo la carrera/paralelo real por worktrees.
La validación con agentes reales es el guion manual del mantenedor (documentado
paso a paso, con registro).

### D2 — El guion completo, de idea a archivo
`constitution` (si falta) → `propose` (idea real de fixture) → `review` (con al
menos un comentario→reelaboración) → `implement` con dos agentes en paralelo →
`verify` (tests vinculados) → `archive` (fusión + proyección). Cada paso con
criterios observables (spec) y captura para el informe.

### D3 — Informe de aceptación reproducible
Artefacto generado por la corrida: qué se ejecutó, versiones, resultados por
criterio, desviaciones. Acompaña el tag v0.1; regenerable por cualquiera con el
mismo commit.

## Risks / Trade-offs

- **El guion pasa con mocks y cojea con reales** → por eso el guion manual del
  mantenedor es parte de la aceptación (registrado en el informe).

## Migration Plan

Suite y documentación; cero features.

## Open Questions

- Idea de fixture canónica para el guion (pequeña pero no trivial): elegir al
  frente de la cola.
