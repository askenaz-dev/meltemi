# Tasks — turno-vacio-es-error

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El simulado y el contrato

- [x] 1.1 `mock-agent --silent-turn` y `--silent-turn-at <N>`: el turno
  callado respeta `--turn-delay-ms` y responde con el motivo de un turno normal;
  la decisión es la función pura `silent_in(args, turn)` con su unit test
  (sin interruptor, todos los turnos, solo el turno `N`) (design D8) — gates:
  `cargo build -p mock-agent`, suite de `mock-agent`

- [x] 1.2 Contrato aditivo, con el workspace verde en el mismo commit: evento
  `turn_empty { stopReason }` en `SessionEventKind` y en
  `session-event.schema.json`; `emptyTurn` opcional (omitido si es falso) en
  `SessionStartResult`, `ProposeResult`, `SessionDirectResult`,
  `WorktreeDispatchResult` y `SddResult` y en sus esquemas; `emptyTurns` en
  `UsageActivity` y bajo `properties` de `activity`; el valor neutro en cada
  literal existente (daemon y tests de la TUI) y en los `match` exhaustivos;
  `turn_empty: { glyph: "∅", tone: "warn" }` en `EVENT_STYLE`; casos de
  conformidad que validan el evento y prueban que un `emptyTurn` falso no deja
  clave (design D2, D3, D5, D7) — gates: `cargo build --workspace`, clippy del
  workspace, suite de `meltemi-proto`, `scenarios_shell` de `meltemi-desktop`

## 2. El daemon mide y declara

- [x] 2.1 Medición por turno en `acp.rs`: `carries_work` puro con unit tests
  de lo que suma y de lo que no; contador en `HandlerState` sumado en
  `forward_update` y `passthrough_permission`, a cero en cada `PromptSent`;
  veredicto después de tomar el relevo y antes de la rama de cancelación, solo
  para `completed`, contador en cero y sin petición de parada en el turno;
  `SessionOutcome` gana `last_turn_empty` y `empty_turns` (design D1, D2) —
  escenarios «Un turno con cualquier palabra o acción no es vacío», «Un turno
  que solo piensa, actualiza una herramienta o su plan no es vacío», «Un
  fragmento de solo espacios no cuenta como trabajo», «Lo que describe la
  sesión no cuenta como trabajo» — gates: suite de `meltemid`

- [x] 2.2 Las respuestas lo dicen y el log lo registra de punta a punta:
  `emptyTurn` en `session/start` sin `detach`, `propose`, reanudación de
  `session/direct`, `worktree/dispatch` y `sdd/explore`, `sdd/propose`,
  `sdd/plan`, `sdd/constitution` y `sdd/gate` (`run_turn` y `author_artifact`
  devuelven si el turno fue vacío); e2e con el simulado callado y con el normal,
  uno de trabajo → vacío → trabajo por instrucciones dirigidas, y uno de un
  turno callado y demorado que el humano cancela y otro que interrumpe (design
  D2, D3) — escenarios «Turno sin palabra ni acción declarado vacío», «Un turno
  que no terminó no se declara vacío», «Un turno que el humano detuvo no se
  declara vacío», «Cada turno de una sesión de varios turnos se mide», «Un
  arranque cuyo turno fue vacío lo declara», «Un turno con trabajo no deja
  clave», «Sin estado no hay veredicto en la respuesta», «Propuesta con un
  turno vacío», «Despacho con un turno vacío», «Un gate cuyo turno de autoría
  fue vacío lo declara», «La vía rápida declara el vacío de cualquiera de sus
  turnos» — gates: suite de `meltemid`

- [ ] 2.3 Analítica: `facts_of_log` cuenta `turn_empty`, `accumulate` y
  `merge_activity` lo suman, y el e2e de analítica lo lee (design D7) —
  escenario «La actividad cuenta los turnos vacíos» — gates: suite de
  `meltemid`

## 3. Las superficies

- [ ] 3.1 CLI: `ExitCode::EmptyTurn = 15` y su fila en la tabla de códigos;
  `exit_for(&Command, &Value)` puro en `tui/src/lib.rs`, que da `15` a toda
  respuesta con `emptyTurn` en `true` y escribe la advertencia con remedio en
  stderr en modo humano; `meltemi usage` imprime los turnos vacíos cuando no
  son cero; `docs/referencia-cli.md` regenerado; unit tests de `exit_for` (con
  una respuesta de `plan`) y de `render_usage`, y e2e del CLI contra el simulado
  callado en modo humano y `--json` (design D4, D7) — escenarios «Turno vacío
  distinguible», «La advertencia va a stderr con su remedio», «La salida de
  máquina lleva el veredicto», «Un verbo de autoría sigue la misma regla» —
  gates: suite de `meltemi` (tui), test de frescura de la referencia

- [ ] 3.2 GUI: el pliegue convierte `turn_empty` en un aviso `empty`; la
  conversación lo pinta con `∅`, palabra y remedio; claves ES/EN; la vista
  Consumo gana la columna de turnos vacíos; casos en `conversation.test.ts` y
  sus pines con `// Scenario:` en `desktop/tests/scenarios_shell.rs`, y el pin
  de la columna en `core/meltemid/tests/scenarios_analytics.rs` (design D5, D7)
  — escenarios «El turno vacío se lee en la conversación», «Un turno con salida
  no muestra el aviso», «Las superficies muestran los turnos vacíos» — gates:
  `node --test`, `svelte-check`, suite de `meltemi-desktop`

- [ ] 3.3 TUI: `summarize_event` y `summarize_log_line` dicen «turno vacío»
  con el remedio desde el catálogo `Msg`, en ES y EN (design D6) — escenarios
  «El transcript nombra el turno vacío», «El detalle histórico nombra el turno
  vacío» — gates: suite de `meltemi` (tui)

- [ ] 3.4 `docs/paridad-nucleo.md`: el veredicto del turno vacío llega a las
  tres superficies por el evento y a los verbos del CLI por la respuesta; los
  métodos sin `emptyTurn` por declaración (`sdd/review-decide`,
  `sdd/implement`, arranques separados, instrucciones encoladas); y los turnos
  vacíos de la actividad en las dos superficies que la muestran (design D3–D7)
  — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate turno-vacio-es-error` limpio, `meltemi verify
  turno-vacio-es-error` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), estado de la change en
  `docs/plan-de-cambios.md`, y `meltemi land turno-vacio-es-error`
