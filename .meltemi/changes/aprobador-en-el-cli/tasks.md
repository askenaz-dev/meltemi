# Tasks — aprobador-en-el-cli

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y el simulado

- [x] 1.1 El push lleva su id: `requestId` en `PermissionRequestParams` y en
  `permission.schema.json` (aditivo), `spawn_live_push` lo llena con el id de la
  cola, caso de conformidad y e2e de `e2e_permisos` que compara el id del push
  con el de la entrada (design D4) — escenario «La petición en vivo nombra su
  id» — gates: `cargo build --workspace`, suites de `meltemi-proto` y
  `meltemid`

- [x] 1.2 `mock-agent --titled-permission`: su petición de permiso lleva
  `kind: edit` y un título; apagado por omisión, con su unit test (design D10)
  — gates: `cargo build -p mock-agent`, suite de `mock-agent`

## 2. El aprobador

- [ ] 2.1 Núcleo puro en `tui/src/approver.rs`: la pregunta desde el push (D2),
  la traducción tecla → respuesta (dígito, `d`/Esc, `r` + `y`, Ctrl+C, resto
  sin efecto), y las funciones compartidas con el shell para elegir permiso,
  rechazo y regla propuesta (`live.rs` las usa); unit tests (design D2, D3, D5)
  — escenarios «Sin herramienta no se ofrece regla» — gates: suite de
  `meltemi` (tui)

- [ ] 2.2 Fuentes de teclas: la de terminal (modo crudo sin pantalla
  alternativa, sondeo en tramos, descarte de lo tecleado antes, solo
  pulsaciones) y la programada para pruebas, con la traducción de eventos de
  crossterm a teclas probada por unit test (design D7) — gates: suite de
  `meltemi` (tui), clippy

- [ ] 2.3 El aprobador en el CLI: `execute_with` y `dispatch_with` con el
  aprobador (la biblioteca deniega por omisión), `main.rs` lo activa con stdin
  y stderr en terminal; `handle_incoming` pregunta de a una, responde el push,
  persiste por `permission/decide`, retira ante `permission/timeout` o un
  `permission/changed` sin su id, y ante Ctrl+C cancela la sesión y el comando
  sale con `13`; aviso honesto sin terminal; e2e en `tui/tests/approver.rs` con
  la fuente programada (design D1, D3, D5, D6, D9) — escenarios «Una tecla elige
  una opción del agente», «Denegar elige el rechazo del agente», «Sin terminal
  se deniega como hasta ahora», «La salida de máquina queda intacta con el
  aprobador activo», «Ctrl+C deniega y cancela el turno», «Una pregunta que
  vence se retira del terminal», «Una pregunta decidida en otra superficie se
  retira del terminal», «La regla del terminal gobierna la siguiente sesión» —
  gates: suite de `meltemi` (tui)

## 3. Gramática y documentación

- [ ] 3.1 La gramática rechaza toda bandera que conceda permisos, con su test;
  la ayuda de `session` describe el aprobador y `docs/referencia-cli.md` se
  regenera; `docs/sesion-libre.md` (sección Terminal), `docs/quickstart.md` y
  `docs/accesibilidad.md` describen las teclas y la denegación sin terminal;
  `docs/paridad-nucleo.md` anota que el CLI responde `permission/request` y usa
  `permission/decide` (design D1, D8) — escenario «Ninguna bandera concede
  permisos» — gates: suite de `meltemi` (tui), tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate aprobador-en-el-cli` limpio, `meltemi verify
  aprobador-en-el-cli` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/
  `land` detenido, estado de la change en `docs/plan-de-cambios.md`, y
  `meltemi land aprobador-en-el-cli`
