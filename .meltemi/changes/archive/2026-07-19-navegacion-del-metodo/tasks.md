## 1. Métodos de navegación

- [ ] 1.1 `change/list`: activas (artefactos presentes, tareas x/y, review decididos/total, verify verificados/total — lectores de estado existentes, cero escritura) + archivadas (nombre, fecha) con `limit` _(Req: Listado de changes con estado agregado; design D1/D2)_
- [ ] 1.2 `change/show` (artefactos + deltas tal cual) y `spec/list`/`spec/show` (capacidades y spec parseada); inexistente rehúsa con remedio _(Req: Mostrar changes y specs vivas; design D5)_
- [ ] 1.3 `sdd/validate`: por change (motor + `dry_run_diagnostics` extraídos del gate de archive) y sin argumento (verdad viva completa); solo lectura _(Req: Validación independiente del archivado; design D3)_

## 2. Contrato

- [ ] 2.1 `proto/`: métodos + tipos + schemas aditivos (`change/list`, `change/show`, `spec/list`, `spec/show`, `sdd/validate`) + conformance

## 3. CLI

- [ ] 3.1 `changes` (listado con columnas de estado), `show <change|spec>`, `validate [change]`, todos con `--json` _(paridad §4)_
- [ ] 3.2 Código de salida `14` en la taxonomía (`exit.rs` + tabla `EXIT_CODES` → la referencia CLI generada se regenera) y cableado en `validate` _(Modified: cli-contract; design D4)_

## 4. Tests y calidad

- [ ] 4.1 Unit: agregación de estado (parciales honestos: sin tasks.md, sin deltas), mapeo de salida 0/14
- [ ] 4.2 E2e sobre fixture `.meltemi/`: listado con una change en estado mixto (review parcial, verify parcial, tareas parciales) + archivadas; show de change y de spec viva; validate limpio (0) y con delta conflictivo (14, verdad viva intacta) _(constitución: fixtures temporales)_
- [ ] 4.3 `cargo clippy -- -D warnings`, `fmt --check`, tests y frescura de la referencia CLI en verde
