## 1. Mapa

- [ ] 1.1 Dependencia `ignore` pineada y justificada (cargo-deny verde) _(design D1)_
- [ ] 1.2 `repo/map` en proto + handler con profundidad/límite y truncado declarado _(Req: Mapa del repositorio)_

## 2. Expansión

- [ ] 2.1 Parser de referencias `@` (con escape `@@`) y expansión determinista con límites por archivo/prompt y marcas de truncado/no-encontrado _(Req: Expansión determinista)_
- [ ] 2.2 Registro de expansiones (rutas+bytes) en el JSONL _(Req: Auditoría de expansiones)_

## 3. TUI

- [ ] 3.1 Autocompletado de `@` en el compositor contra `repo/map` (accesibilidad baseline) _(Req: Autocompletado)_

## 4. Tests y calidad

- [ ] 4.1 Unit: gitignore anidado, truncado declarado, escape, límites, no-encontrado no aborta
- [ ] 4.2 E2e: prompt con `@` llega expandido al mock-agent; log reconstruye el contexto
- [ ] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
