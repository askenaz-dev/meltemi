## 1. Generación del commit

- [x] 1.1 Plantilla de mensaje desde tarea+requisitos (convención garantizada) + trailers `Meltemi-Task`/`Meltemi-Req`; guardia dura contra trailers de co-autoría _(Req: Trailers; Convención)_ — `commit::build_message` + `strip_coauthorship`; slug de requisito con folding de acentos.
- [x] 1.2 Flujo supervisado (presentar → aprobar/editar/rechazar, modal del shell) y autónomo (directo + evento) _(Req: Commit atómico)_ — `commit/task` con `confirm`: sin él previsualiza (mensaje + diff, no comete); con él aplica y emite `task_committed` en el JSONL. Título/cuerpo son las entradas editables del humano.

## 2. Verificación

- [x] 2.1 Post-commit: árbol limpio + alcance del commit contra el checkpoint; desviaciones visibles con paso correctivo _(Req: Atomicidad verificada)_ — `treeClean` + `scope_deviations` contra los archivos declarados (base: checkpoint de la tarea, o HEAD resuelto pre-commit); las desviaciones se informan, nunca bloquean (v0.1).
- [x] 2.2 Hooks del usuario respetados (fallo mostrado tal cual, sin `--no-verify`) _(Req: Atomicidad — hooks)_ — `git commit -F -` corre los hooks; el fallo se devuelve verbatim como error 4003; jamás se pasa `--no-verify`.

## 3. Tests y calidad

- [x] 3.1 Unit: plantilla y trailers (incluida la guardia anti-co-autoría), slug de requisito
- [x] 3.2 E2e en repos fixture: tarea → commit con trailers; supervisado (preview) no comete; desviación reportada; hook que falla se muestra _(constitución: jamás este repo)_
- [x] 3.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
