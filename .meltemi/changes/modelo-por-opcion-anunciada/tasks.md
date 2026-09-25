# Tasks — modelo-por-opcion-anunciada

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y el registro

- [x] 1.1 Contrato: `SessionLever`; los eventos `levers_applied` (el modelo y el
  esfuerzo que rigieron) y `lever_refused` (la palanca, lo pedido, la razón, lo
  que el agente anunció o dejó —a lo sumo 20— y cuántos más); el modelo y el
  esfuerzo que rigieron en la respuesta de `session/start`; y los datos del error
  2007; schemas y casos de conformidad (design D3, D4, D6) — gates: `cargo build
  --workspace`, suite de `meltemi-proto`

- [ ] 1.2 El registro declara las palancas y la regla por nombre se va: campo
  `levers` (conjunto completo; un nombre desconocido o repetido invalida el
  documento), `CatalogEntry`, `ResolvedAgent.levers`, `route_lever`; el reparto
  reemplaza a `levers_of` donde hoy se usa, con la vía anunciada rehusada antes
  de lanzar hasta 2.3, como hoy; prueba del registro embebido que hereda el
  escenario de `own-adapters`; las pruebas que pineaban el simulado por nombre
  pasan a un registro de prueba (design D2) — escenarios «Una entrada declara
  sus palancas de lanzamiento», «El nombre del binario no decide», «Una palanca
  que el agente no admite se rehúsa» — gates: suite de `meltemid`

## 2. Confirmar o rehusar antes del primer prompt

- [ ] 2.1 El simulado demuestra: `--config-clamp`, `--config-hang`,
  `--config-mode-shift`, `--config-twins`, `--echo-levers` y `--reasoning`,
  apagados por omisión y con sus unit tests (design D7) — gates: `cargo build -p mock-agent`, suite de
  `mock-agent`

- [ ] 2.2 `run_session` fija las palancas anunciadas antes del primer prompt: el
  modelo primero, el esfuerzo contra lo que quede anunciado, la última respuesta
  decide, el modo no se mueve, espera acotada y cancelable; rehúsa con
  `lever_refused` y un resultado tipado, y anota `levers_applied` con lo que
  rigió; unit tests de la elección de opción, de la validación y de la
  comparación (design D1, D3, D4) — gates: `cargo build --workspace`, suite de
  `meltemid`

- [ ] 2.3 La sesión libre reparte las palancas por vía, cierra `lever_refused`
  sin `turn_completed`, responde el 2007 con sus datos y lo deja en el log si
  está desacoplada; el evento de resolución y el registro de inicio llevan solo
  lo que viajó por bandera, el de cierre lo que rigió, y la fusión del índice lo
  toma; e2e con el simulado (design D3, D4, D6) — escenarios «Sin declaración la
  palanca es lo que el agente anuncia», «La opción anunciada fija el modelo antes
  del primer prompt», «La opción anunciada fija el esfuerzo antes del primer
  prompt», «Un valor no anunciado se rehúsa listando lo anunciado», «Sin una
  opción de la categoría la palanca se rehúsa al abrir», «Lo que el agente no
  confirma se rehúsa sin prompt», «Un cambio de modo al fijar la palanca se
  rehúsa», «Un agente que no contesta a tiempo se rehúsa», «Cancelar mientras se
  fija la palanca no envía prompt», «El modelo pedido viaja sin interpretarse»,
  «Lo que rigió queda en el registro», «Lo rehusado no figura como lo que rigió»
  — gates: suite de `meltemid`

- [ ] 2.4 Todos los verbos que arrancan un agente: resuelven y reparten las
  palancas antes de sus efectos, aplican las del perfil, y entregan el rehúso
  (`propose` retira su andamio intacto; `dispatch` e `implement` responden el
  2007, nunca «el agente se negó»); e2e de `propose`, un verbo de autoría,
  `dispatch` e `implement`, cada uno con su rehúso (design D4, D5) — escenarios
  «El modelo del perfil rige en todo verbo que arranca un agente», «La sesión
  pisa el default del perfil», «Un perfil sin declaración no impone nada», «El
  rehúso llega a quien lo pidió» — gates: suite de `meltemid`

## 3. Superficies y documentación

- [ ] 3.1 El CLI dice el modelo y el esfuerzo que rigieron e imprime el rehúso
  acotado y escapado (`approver::clean`); la transcripción de la TUI dice la
  palanca rehusada, en español e inglés (design D4, D6) — escenarios «El CLI
  dice el modelo que rigió», «El rehúso se imprime acotado y escapado», «El
  terminal dice la palanca rehusada», «El terminal muestra el modelo efectivo» —
  gates: suite de `meltemi` (tui)

- [ ] 3.2 La conversación de la GUI dice la palanca rehusada y el cierre
  `lever_refused`, en español e inglés (design D4) — escenarios «La
  conversación dice la palanca rehusada», «Se elige con búsqueda y se admite
  entrada libre», «La ficha no inventa lo que no sabe», «Cambiar en marcha se
  advierte» — gates: suite de `meltemi-desktop`, `svelte-check` y `node --test`
  de `desktop/ui` (con la unión temporal a los `node_modules` de `main`,
  retirada al terminar)

- [ ] 3.3 Documentación: `docs/agentes.md` (qué palanca toma cada adaptador, la
  vía anunciada y su confirmación), `docs/sesion-libre.md` (`--model`,
  `--effort`), la ayuda de `session` y `docs/referencia-cli.md` regenerada,
  `docs/paridad-nucleo.md` (los eventos y los campos nuevos) y la descripción
  del 2007 en el schema de errores — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate modelo-por-opcion-anunciada` limpio, `meltemi
  verify modelo-por-opcion-anunciada` completo, gate del workspace (`cargo test
  --workspace --no-fail-fast`, clippy, fmt), el daemon que arranquen
  `validate`/`verify`/`land` detenido, estado de la change en
  `docs/plan-de-cambios.md` con `selector-de-modelo-que-sabe` anotada, `meltemi
  land modelo-por-opcion-anunciada` y `cargo clean` del taller
