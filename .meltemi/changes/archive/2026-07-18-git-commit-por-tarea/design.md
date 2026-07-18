## Context

Constitución §8: cada línea rastreable hasta su requisito. El mecanismo es el
commit atómico por tarea con trailers de trazabilidad. Los worktrees (#16) y los
checkpoints (#17) ya existen; este repo practica la convención a mano — el
producto la impone para el trabajo de agentes.

## Goals / Non-Goals

**Goals:** commit por tarea completada (propuesto en supervisado, directo en
autónomo); trailers `Meltemi-Task`/`Meltemi-Req`; convención de mensaje;
verificación de atomicidad (árbol limpio, alcance del commit).
**Non-Goals:** push/PR/forjas (el usuario publica); firmado (config del usuario
se respeta); reescritura de historia.

## Decisions

### D1 — Trailers de trazabilidad legibles por máquina
`Meltemi-Task: <change>/<n.m>` y `Meltemi-Req: <capability>/<requirement-slug>`
(0..n por commit). **Jamás trailers de co-autoría**: la autoría es la del
`git config` del usuario, intocada — regla de proyecto elevada a spec.

### D2 — Mensaje según la convención del repo
Título imperativo en inglés (≤ 72), cuerpo con qué/por qué, referencia
`(<change> <tarea>)`. Plantilla generada por el daemon desde la tarea y el
requisito; el agente puede proponer el cuerpo, la forma la garantiza Meltemi.

### D3 — Supervisado propone, autónomo comete
Supervisado: al completar la tarea, el commit se presenta (mensaje + resumen del
diff) y el humano aprueba/edita el mensaje/rechaza. Autónomo: commit directo
dentro de las reglas (#9), registrado. En ambos, evento en el JSONL.

### D4 — Atomicidad verificada
Tras el commit: árbol del worktree limpio; el commit contiene solo rutas tocadas
por la tarea (contra el checkpoint #17 como base de comparación). Desviaciones →
diagnóstico visible (nunca silencioso), con la opción de commit correctivo.

## Risks / Trade-offs

- **Hooks del usuario fallan el commit** → el fallo se muestra tal cual (nunca
  `--no-verify`); la tarea queda completada-sin-commit, estado visible.
- **Tareas que tocan lo imprevisto** → la verificación lo declara; el humano
  decide (es información, no bloqueo duro en v0.1).

## Migration Plan

Aditivo al ciclo de tarea de #16/#17. Sin modo asignado, nada cambia.

## Open Questions

- Slug exacto del requisito en `Meltemi-Req` (normalización compartida con #15).
