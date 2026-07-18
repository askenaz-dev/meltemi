## Why

El hito v0.1 tiene un enunciado preciso (§10): *"un desarrollador lleva una
funcionalidad de idea a código íntegramente en terminal, con specs revisables,
usando dos agentes de proveedores distintos en paralelo"*. Si eso no está escrito
como spec ejecutable, el hito se declararía por sensación. Esta change lo
convierte en el **escenario de aceptación** del MVP: la última verificación antes
de llamar v0.1 a algo.

## What Changes

- **El escenario del hito como spec EARS**: guion completo — `/propose` de una
  idea real → revisión de specs (`/review`) → `/implement` con **dos agentes de
  proveedores distintos en paralelo** (worktrees) → `/verify` → `/archive` — con
  criterios observables por paso.
- **Ejecución automatizable donde se puede**: contra dos `mock-agent` con
  personalidades distintas en CI (sin red ni agentes reales, constitución); el
  guion manual equivalente con agentes reales queda documentado para la
  validación del mantenedor.
- **Métricas del hito verificadas**: presupuestos de §12 aplicables en terminal
  (arranque < 1 s, binario TUI < 25 MB) medidos en el pipeline de release (#23).
- **Informe de aceptación**: artefacto reproducible (qué se corrió, qué pasó,
  desviaciones) que acompaña al tag v0.1.

## Capabilities

### New Capabilities
- `v01-acceptance`: el escenario de aceptación del hito y su informe.

### Modified Capabilities
- _Ninguna_ (si el guion revela huecos, vuelven como deltas a la change dueña —
  esa es precisamente su función).

## Impact

- Suite e2e de aceptación (fixtures + dos mocks), documentación del guion
  manual. Depende de todo el backlog anterior: es la última change del plan.

## Fuera de alcance

- Métricas de adopción (estrellas/contribuidores): objetivos, no aceptación.
- Cualquier feature nueva: esta change solo verifica y reporta.
