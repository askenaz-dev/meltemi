## 1. Guion ejecutable

- [x] 1.1 Segundo perfil de agente simulado (salida divergente) para el paralelismo de proveedores en CI _(design D1)_ — `mock-agent --profile <name>` marca su salida con el perfil; la corrida paralela usa dos perfiles/nombres distintos en worktrees separados.
- [x] 1.2 Suite de aceptación: ciclo completo sobre fixture (propose→review→implement→verify→archive) con criterios observables por paso _(Req: Guion del hito)_ — `core/meltemid/tests/e2e_hito.rs`: la corrida termina implementada, verificada y archivada, todo por las superficies del producto; la reelaboración por comentario queda cubierta por `e2e_review` y el guion manual.

## 2. Manual y métricas

- [x] 2.1 Guion manual del mantenedor con agentes reales, paso a paso, con registro en el informe _(Req: Validación manual)_ — `docs/hito-v01.md` §"Manual run" + plantilla de informe con fecha/agentes/resultado por criterio.
- [x] 2.2 Presupuestos §12 volcados al informe desde el pipeline _(Req: Métricas verificadas)_ — el gate de presupuesto (TUI < 25 MB) vive en `release.yml`; el criterio C6 y su valor van al informe (`docs/hito-v01.md`).

## 3. Informe

- [x] 3.1 Informe de aceptación reproducible (versiones, criterios, desviaciones) publicado junto al tag _(Req: Informe reproducible)_ — plantilla y criterios C1–C6 en `docs/hito-v01.md`; el veredicto automatizado es la suite e2e determinista (mismo commit → mismo resultado); lint en `hito_doc.rs`.

## 4. Cierre

- [x] 4.1 Corrida automatizada en verde (suite e2e de aceptación); la corrida manual con agentes reales y el veredicto v0.1 final son la acción de aceptación del mantenedor sobre este guion.
