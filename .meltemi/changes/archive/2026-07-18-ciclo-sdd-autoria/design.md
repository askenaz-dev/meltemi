## Context

`propose` es hoy andamio+delegación mínima; los demás verbos están reservados. El
motor (`meltemi-spec`) valida estructura+EARS+deltas; el shell tiene gates
modales y paleta; la flota, permisos, proyección y contexto de repo ya existen en
este punto del orden. Falta el ciclo de autoría con gates humanos y modo dual.

**Nota de provisionalidad**: los nombres RPC `sdd/*` y la forma exacta de los
gates se revalidan al frente de la cola contra lo aprendido en #7–#12.

## Goals / Non-Goals

**Goals:** `/constitution`, `/explore`, `/propose` completo, `/plan`; gate humano
por artefacto; validación del motor como puerta previa al gate; modo dual con
criterio escrito; superficies TUI/CLI.
**Non-Goals:** `/review` rico (#15), `/implement` (#20), `/verify`+`/archive`
(#19); detección semántica de contradicciones (#15).

## Decisions

### D1 — El ciclo como máquina de estados en el daemon
Una change en autoría avanza `proposal → specs (EARS) → design → tasks`; cada
artefacto: (1) el agente redacta, (2) el motor valida (estructura+EARS; deltas
aplicables en seco), (3) el humano decide en el gate (aprobar / comentar-y-reelaborar /
abortar). El estado vive en la change (archivo de estado en su carpeta), no en
memoria: sobrevive reinicios y es inspeccionable.

### D2 — Verbos y contrato (provisional)
`sdd/constitution`, `sdd/explore` (streaming de deliberación, **jamás escribe**),
`sdd/propose` (inicia/continúa el ciclo), `sdd/plan` (refina design y secuencia
tasks), más eventos de gate (`sdd/gate` pendiente→decisión, reutilizando el
patrón de cola de permisos: pendientes de primera clase, decidibles por RPC).

### D3 — Modo dual con criterio escrito de proporcionalidad
`spec-full`: gate por artefacto. `fast-forward`: el agente produce los cuatro
artefactos y hay un único gate final. Criterio por defecto (escrito en la spec):
elegible para fast-forward una change sin capacidades nuevas y sin deltas
MODIFIED/REMOVED (solo ADDED pequeños o docs); todo lo demás, spec-full. El
humano puede forzar cualquiera; la elección queda registrada en la change.

### D4 — El agente autor es el configurado; el motor es el árbitro
La autoría usa el agente del proyecto (catálogo #7) con las reglas de permisos
vigentes (#9: escribir dentro de `.meltemi/changes/<name>/` será típicamente una
regla allow de proyecto). Un artefacto que no pasa el motor vuelve al agente con
los diagnósticos como instrucción, sin consumir el gate humano.

### D5 — Superficies
TUI: acciones en la vista Proyecto + gates como modales de primera clase
(contrato ya vivo); progreso del ciclo visible por change. CLI: `explore` y
`plan` operativos; `propose` scriptable ejecuta spec-full por pasos (cada gate =
una invocación que muestra el artefacto y pide decisión) — sin TTY no hay gates
interactivos colgantes.

## Risks / Trade-offs

- **Agente redacta artefactos mediocres** → el motor rechaza formato; la calidad
  semántica la custodia el gate humano + #15 (review) después.
- **Gates fatigosos** → fast-forward con criterio escrito; comentarios reutilizan
  el bucle de reelaboración sin reiniciar el ciclo.
- **Nombres RPC provisionales** → revalidar al frente; el delta de cli-contract
  de esta change fija el mapeo definitivo en ese momento.

## Migration Plan

Aditivo sobre `propose-flow` (el andamio actual queda como primer paso del
ciclo). Los verbos nuevos se des-reservan en la gramática (delta acumulativo).

## Open Questions

- Umbral exacto de "pequeño" para fast-forward (constante inicial en la spec de
  proporcionalidad; ajustable por config de proyecto).
- Persistencia del estado del ciclo: nombre/forma del archivo en la change.
