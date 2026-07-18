## 1. Checkpoints

- [x] 1.1 Commit técnico pre-tarea bajo `refs/meltemi/checkpoints/...` (rastreado + untracked no ignorado; jamás ramas del usuario) _(Req: Checkpoint automático)_ — índice scratch (`GIT_INDEX_FILE`) + `commit-tree`/`update-ref`; el índice y las ramas del usuario no se tocan.
- [x] 1.2 Listado por contrato (change/tarea → ref, momento) _(Req: Listado de checkpoints)_ — `checkpoint/list` con registro propio (ref, tarea, agente, worktree, momento, irreversibles).

## 2. Reversión

- [x] 2.1 Restauración granular del worktree de una tarea (reset + limpieza de untracked posteriores) sin tocar otros worktrees _(Req: Reversión granular)_ — `reset --hard <ref>` + `clean -fd` (sin `-x`: lo ignorado se preserva); e2e verifica multi-worktree intactos.
- [x] 2.2 Confirmación modal con alcance declarado _(Req: Reversión — confirmación; Alcance honesto)_ — `checkpoint/revert` exige `confirm`; sin él devuelve el alcance (qué NO se revierte) como vista previa. La superficie modal del shell (revision-specs-ux) consume ese alcance.

## 3. Alcance honesto

- [x] 3.1 Acumular operaciones aprobadas fuera del árbol por tarea (clasificación desde las peticiones de permiso) y listarlas como irreversibles _(Req: Alcance honesto)_ — clasificador `RequestFacts::is_out_of_tree`; ledger `checkpoint/record-op` que el proxy alimenta al aprobar (la atribución sesión↔tarea se completa en `comando-implement`).
- [x] 3.2 Eventos `checkpoint_created`/`checkpoint_restored` en el JSONL — envolvente `SessionEvent` en `.meltemi/checkpoints/events.jsonl`.

## 4. Tests y calidad

- [x] 4.1 Unit: refs técnicas correctas, gitignore respetado, clasificación acumulada
- [x] 4.2 E2e en repos fixture: checkpoint→cambios del agente→reversión restaura exacto; multi-worktree intactos; irreversibles listadas cuando hubo comando aprobado _(constitución: jamás este repo)_
- [x] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
