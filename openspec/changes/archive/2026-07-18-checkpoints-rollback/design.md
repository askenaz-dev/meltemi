## Context

La autonomía es aceptable solo si deshacer es barato (§4.6). Los worktrees (#16)
aíslan; falta la instantánea pre-tarea y la reversión granular con alcance
honesto, enganchada a la clasificación del proxy de permisos (#9, ya viva).

## Goals / Non-Goals

**Goals:** checkpoint automático pre-tarea; listado y reversión granular por
tarea; alcance honesto (qué NO se revierte, alimentado por las decisiones de
permisos); eventos en el log.
**Non-Goals:** sandbox propio (fase 2); snapshot de estado externo (jamás se
promete); deshacer commits ya publicados por el usuario.

## Decisions

### D1 — Checkpoints como refs técnicas, no stashes
Antes de cada tarea, commit técnico del estado del worktree bajo
`refs/meltemi/checkpoints/<change>/<tarea>` (índice y untracked incluidos vía
commit temporal), invisible en las ramas del usuario. Sin objetos exóticos: puro
git, inspeccionable con herramientas estándar.

### D2 — Reversión granular por tarea
Revertir la tarea T = restaurar el worktree de T a su checkpoint (reset duro del
worktree gestionado + limpieza de untracked creados después, con confirmación
modal). Tareas en otros worktrees no se tocan. Si T ya produjo commit (#18), la
reversión lo deja fuera de la rama gestionada (reset), nunca reescribe ramas del
usuario.

### D3 — Alcance honesto alimentado por permisos
Junto a cada checkpoint, el daemon acumula las operaciones aprobadas durante la
tarea que actúan fuera del árbol (comandos ejecutados, accesos de red del agente
— según la clasificación de la petición de permiso). La UX de reversión lista
esas operaciones como **irreversibles** antes de confirmar: el usuario sabe
exactamente qué no vuelve.

### D4 — Registro
Eventos `checkpoint_created` y `checkpoint_restored` en el JSONL con la ref y la
tarea; la reversión referencia las irreversibles mostradas.

## Risks / Trade-offs

- **Untracked pesados** (builds) → el commit técnico respeta gitignore; lo
  ignorado ni se captura ni se limpia.
- **Clasificación imperfecta de irreversibles** → se listan las aprobadas fuera
  del árbol de forma conservadora; mejor sobre-avisar.

## Migration Plan

Aditivo sobre #16/#9. Refs propias eliminables sin rastro (`refs/meltemi/*`).

## Open Questions

- Retención de checkpoints tras archivar la change (¿podar refs?): propuesto
  podar al archivar con confirmación; fijar al frente de la cola.
