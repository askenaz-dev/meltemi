# Tasks — corrida-integrada

Toda tarea que lanza el daemon corre antes `cargo build --workspace`: la
corrida usa los binarios y los adaptadores de este checkout.

## 1. La corrida

- [x] 1.1 `scripts/smoke-agents.ps1` y `scripts/smoke-agents.sh`: el daemon
  aislado por las cinco variables, el fixture temporal con su regla de
  `read`/`edit` por prefijo, la flota detectada, la matriz de siete verbos por
  agente listo, las medidas sacadas de las respuestas y los registros, el
  informe escrito en `docs/qa/` con los no instalados declarados, y el
  desmontaje que solo detiene lo que lanzó; ensayada contra el agente simulado
  antes de gastar un turno real; un test pinea el aislamiento, la matriz y que
  ningún workflow la corre (design D1–D5) — escenarios «La corrida no toca el
  daemon del usuario», «La corrida no corre en CI» — gates: el test nuevo, la
  corrida contra el simulado

- [x] 1.2 La corrida contra los CLIs de esta máquina (Claude Code 2.1.280,
  OpenCode 1.18.32, Codex 0.156.1): `docs/qa/2026-09-26-corrida-integrada.md` (fecha UTC de la corrida)
  con la tabla, los hallazgos y los no medidos; un test pinea el informe
  (design D3–D5) — escenarios «Cada agente listo pasa por la misma matriz»,
  «Lo no instalado se declara» — gates: tests de docs

## 2. El hito y la conformidad

- [ ] 2.1 El hito v0.1 con Claude Code y OpenCode sobre un fixture mínimo, con
  la revisión y las marcas de verificación por `meltemi bridge`:
  `docs/qa/2026-09-26-hito-v01-aceptacion.md` (fecha UTC) con veredicto por criterio y los
  presupuestos medidos, y `docs/hito-v01.md` enlazándolo; un test pinea ambos
  (design D6) — escenarios «El informe de aceptación existe con su veredicto»,
  «El hito enlaza su informe» — gates: tests de docs

- [ ] 2.2 `docs/conformidad-manual.md`: la pierna de Claude Code 2.1.280 con
  `conformance_real`, la de OpenCode 1.18.32 con prompt desde la matriz, y los
  nivel 1 no instalados declarados como no medidos; `docs/agentes.md` gana la
  sección de la última corrida por agente y la regla de repetirla (design D7,
  D8) — gates: tests de docs, la guía de agentes

## 3. Cierre

- [ ] 3.1 `meltemi validate corrida-integrada` limpio, `meltemi verify
  corrida-integrada` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  corrida-integrada` y `cargo clean` del taller
