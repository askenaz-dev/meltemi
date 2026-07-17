## 1. Checkpoints

- [ ] 1.1 Commit técnico pre-tarea bajo `refs/meltemi/checkpoints/...` (rastreado + untracked no ignorado; jamás ramas del usuario) _(Req: Checkpoint automático)_
- [ ] 1.2 Listado por contrato (change/tarea → ref, momento) _(Req: Listado de checkpoints)_

## 2. Reversión

- [ ] 2.1 Restauración granular del worktree de una tarea (reset + limpieza de untracked posteriores) sin tocar otros worktrees _(Req: Reversión granular)_
- [ ] 2.2 Confirmación modal con alcance declarado _(Req: Reversión — confirmación; Alcance honesto)_

## 3. Alcance honesto

- [ ] 3.1 Acumular operaciones aprobadas fuera del árbol por tarea (clasificación desde las peticiones de permiso) y listarlas como irreversibles _(Req: Alcance honesto)_
- [ ] 3.2 Eventos `checkpoint_created`/`checkpoint_restored` en el JSONL

## 4. Tests y calidad

- [ ] 4.1 Unit: refs técnicas correctas, gitignore respetado, clasificación acumulada
- [ ] 4.2 E2e en repos fixture: checkpoint→cambios del mock-agent→reversión restaura exacto; multi-worktree intactos; irreversibles listadas cuando hubo comando aprobado
- [ ] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
