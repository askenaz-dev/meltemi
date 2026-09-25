# Tasks — turno-vacio-es-error

## 1. El simulado y el contrato

- [ ] 1.1 `mock-agent --silent-turn`: termina cada turno con `end_turn` sin
  enviar actualización ni pedir permiso, apagado por omisión (design D8) —
  gates: suite de `mock-agent`

- [ ] 1.2 Contrato aditivo: evento `turn_empty { stopReason }` en
  `SessionEventKind` y en `session-event.schema.json`; `emptyTurn` opcional en
  `SessionStartResult`, `ProposeResult`, `SessionDirectResult`,
  `WorktreeDispatchResult` y `SddResult` y en sus esquemas; `emptyTurns` en
  `UsageActivity` y bajo `properties` de `activity` en
  `analytics.schema.json`; los literales de la conformidad al día, un caso que
  valida el evento y otro que prueba que un `emptyTurn` falso no deja clave
  (design D2, D3, D7) — gates: suite de `meltemi-proto`

## 2. El daemon mide y declara

- [ ] 2.1 Medición por turno en `acp.rs`: contador en `HandlerState`, sumado en
  `forward_update` y `passthrough_permission`, puesto a cero en cada
  `PromptSent` y leído al resolver la respuesta; `turn_empty` escrito solo para
  `completed` con el contador en cero; `SessionOutcome` gana `last_turn_empty`
  y `empty_turns`; unit tests de lo que suma y de lo que no (design D1, D2) —
  escenarios «Turno sin palabra ni acción declarado vacío», «Un turno con
  cualquier palabra o acción no es vacío», «Lo que describe la sesión no
  cuenta como trabajo», «Un turno que no terminó no se declara vacío» — gates:
  suite de `meltemid`

- [ ] 2.2 Las respuestas lo dicen: `emptyTurn` en `session/start` esperando,
  `propose`, reanudación de `session/direct`, `worktree/dispatch` y los verbos
  `sdd/*` (algún turno de la llamada); e2e con el simulado callado y con el
  normal, y uno de sesión que espera con un turno intermedio vacío (design D3)
  — escenario «Cada turno de una sesión que espera se mide» — gates: suite de
  `meltemid`

- [ ] 2.3 Analítica: `facts_of_log` cuenta `turn_empty`, `accumulate` y
  `merge_activity` lo suman, y el e2e de analítica lo lee (design D7) —
  escenario «La actividad cuenta los turnos vacíos» — gates: suite de
  `meltemid`

## 3. Las superficies

- [ ] 3.1 CLI: `ExitCode::EmptyTurn = 15` en `tui/src/exit.rs` y en la tabla de
  códigos; advertencia con remedio en stderr para `session`, `propose`,
  `direct`, `dispatch` y `explore`; salida con `15` en modo humano y `--json`;
  `meltemi usage` muestra los turnos vacíos cuando no son cero;
  `docs/referencia-cli.md` regenerado (design D4, D7) — escenarios «Turno
  vacío distinguible», «La advertencia va a stderr con su remedio», «La salida
  de máquina lleva el veredicto» — gates: suite de `meltemi` (tui) y e2e del
  CLI contra el simulado callado

- [ ] 3.2 GUI: el pliegue convierte `turn_empty` en un aviso `empty`; la
  conversación lo pinta con `◌`, palabra y remedio; claves ES/EN; la vista
  Consumo muestra `emptyTurns`; casos en `conversation.test.ts` y su pin en
  `desktop/tests/scenarios_shell.rs` (design D5, D7) — escenarios «El turno
  vacío se lee en la conversación», «Un turno con salida no muestra el aviso»
  — gates: `node --test`, `svelte-check`, suite de `meltemi-desktop`

- [ ] 3.3 TUI: `summarize_event` dice «turno vacío» con el remedio desde el
  catálogo `Msg`, en ES y EN (design D6) — escenario «El transcript nombra el
  turno vacío» — gates: suite de `meltemi` (tui)

- [ ] 3.4 `docs/paridad-nucleo.md`: el veredicto del turno vacío como campo que
  llega a las tres superficies, y los turnos vacíos de la actividad a las dos
  que la muestran (design D4–D7) — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate turno-vacio-es-error` limpio, `meltemi verify
  turno-vacio-es-error` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), estado de la change en
  `docs/plan-de-cambios.md`, y `meltemi land turno-vacio-es-error`
