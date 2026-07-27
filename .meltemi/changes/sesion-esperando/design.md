# Design — sesion-esperando

## Context

Dos datos que el daemon ya tiene y no publica: el bloqueo de una sesión en una
decisión humana (estado definido, renderizado por las tres superficies, jamás
fijado) y el gate pendiente de una change (persistido en `.cycle-state.json`,
reportado solo a quien invocó el verbo). Ninguno exige contrato nuevo más allá
de dos campos aditivos.

## Decisions

### D1 — La espera se cuenta, no se conmuta
Un booleano «esperando» se rompe con dos peticiones simultáneas de la misma
sesión: la primera en resolverse devolvería la sesión a `active` mientras la
segunda sigue bloqueada. La registry lleva un **contador de esperas** por
sesión: `0→1` fija `WaitingPermission`, `1→0` restituye `Active`. Las
transiciones viven en la registry (`begin_waiting`/`end_waiting`), no en el
llamador, para que el invariante no dependa de que cada sitio recuerde
decrementar simétricamente.

### D2 — Restituir `Active`, no «el estado anterior»
Al terminar la espera se fija `Active` explícitamente en vez de recordar y
restaurar el estado previo. El único estado desde el que se puede escalar es
`Active` (una sesión escala mientras corre su turno), y `set_state` no toca
sesiones ya desregistradas, así que una sesión que terminó o se canceló
durante la espera no resucita a `Active`. Guardar y restaurar un estado
arbitrario sería más código para el mismo resultado, con más formas de
equivocarse.

### D3 — El gate se lee, no se duplica
`navigate::aggregate` lee `CycleState::load(change_dir)` como ya lee tareas,
review y verify: una lectura más del disco en el mismo lugar. No se persiste
un índice nuevo ni se cachea nada — el ciclo ya es la fuente de verdad, y
duplicarla es cómo se desincroniza. Una change sin `.cycle-state.json`
(mayoría: las que no nacieron de `sdd/propose`) declara `gatePending: false`
sin artefacto, que es la verdad, no un hueco.

Las changes archivadas se agregan con `review`/`verify` en cero por decisión
previa (su estado está congelado); el gate sigue la misma regla: el histórico
no tiene decisiones pendientes.

### D4 — `gatePending` requerido, `gateArtifact` opcional
El booleano siempre es computable, así que el contrato lo exige y ningún
cliente necesita defenderse de su ausencia. El artefacto solo existe cuando
hay gate: se omite en vez de viajar vacío, la misma regla de honestidad que
`archivedAt` y `expiresInSeconds`.

### D5 — Sin RPC nuevo, y sin agregador prematuro
Tentación descartada: un `waiting/list` que cruce permisos y gates. Con
`permission/pending` (ya multi-cliente) y `change/list` (ahora completo) una
superficie compone «qué me espera» sin ayuda del daemon. Un método nuevo
tendría que justificar por qué el cliente no puede — y hoy puede.
