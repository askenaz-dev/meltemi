## 1. Generación del commit

- [ ] 1.1 Plantilla de mensaje desde tarea+requisitos (convención garantizada) + trailers `Meltemi-Task`/`Meltemi-Req`; guardia dura contra trailers de co-autoría _(Req: Trailers; Convención)_
- [ ] 1.2 Flujo supervisado (presentar → aprobar/editar/rechazar, modal del shell) y autónomo (directo + evento) _(Req: Commit atómico)_

## 2. Verificación

- [ ] 2.1 Post-commit: árbol limpio + alcance del commit contra el checkpoint; desviaciones visibles con paso correctivo _(Req: Atomicidad verificada)_
- [ ] 2.2 Hooks del usuario respetados (fallo mostrado tal cual, sin `--no-verify`) _(Req: Atomicidad — hooks)_

## 3. Tests y calidad

- [ ] 3.1 Unit: plantilla y trailers (incluida la guardia anti-co-autoría), slug de requisito
- [ ] 3.2 E2e en repos fixture: tarea del mock-agent → commit con trailers; supervisado rechaza y no comete; desviación reportada; hook que falla se muestra
- [ ] 3.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
