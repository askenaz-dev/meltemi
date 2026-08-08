# Tareas — tablero-de-carrera

Orden: contrato primero (1), daemon después (2), superficies en paralelo
sobre el contrato ya verde (3 GUI, 4 TUI), cierre transversal (5). Un
commit atómico por tarea, con referencia `(tablero-de-carrera N.M)` y sin
trailers de co-autoría. Gates del repo en cada tarea: `cargo clippy -- -D
warnings`, `cargo fmt --check` y la suite del crate tocado.

## 1. Contrato: la calle que se declara

- [x] 1.1 Campos aditivos por calle en `WorktreeCompetitorDiff` (`source`,
  `profile`, `level`, `sessionId`, `committed`, `sha`, `baseRev`) y
  `session_id` en `WorktreeDispatchResult`, todos `Option` con
  `#[serde(default, skip_serializing_if)]`; propiedades en
  `proto/schemas/v1/worktree.schema.json` sin entrar a `required`, sin
  tocar `taskTicked const:false` (design D1) — gates: `cargo test -p
  meltemi-proto`
- [x] 1.2 Conformidad de tres vías para cada campo nuevo: presente
  conforme, omitido conforme, y byte-igualdad de la forma omitida con la
  previa a la change; rechazo de degenerados (cadena vacía) (design D1)
  — gates: `cargo test -p meltemi-proto`

## 2. Daemon: el despacho asienta y el diff agrega

- [x] 2.1 El despacho escribe registro de índice al abrir y al concluir,
  con `level`, `agent_id` y `profile` reales; e2e: la sesión de un
  despacho aparece completa en `session/list` sin reconstrucción, y la
  reconstrucción desde logs recupera la procedencia del evento
  `AgentResolved` (design D2) — escenarios «Sesión de despacho listada
  completa» y «La red de seguridad recupera la procedencia» — gates:
  `cargo test -p meltemid`
- [x] 2.2 `WorktreeDispatchResult.session_id` poblado y agregación de
  procedencia en el handler del diff: unión por igualdad exacta
  `record.project_root == ManagedWorktree.path` tomando el registro más
  reciente por calle; raíz de entrada canonicalizada, rutas almacenadas
  jamás re-canonicalizadas; `baseRev` propio por calle (design D1, D2) —
  escenarios «La calle declara procedencia, sesión y estado», «Los campos
  aditivos no rompen al cliente anterior» y «Bases divergentes visibles
  por calle» — gates: `cargo test -p meltemid`
- [x] 2.3 CLI: `render_race` muestra procedencia, sesión y estado por
  calle (ausencia como «—», nunca inventada); referencia CLI regenerada si
  la salida documentada cambia (design D1, D2) — gates: `cargo test -p
  meltemi`

## 3. GUI: el drill-in de revisión se vuelve tablero

- [x] 3.1 Extraer `fileSections`/`hunksOf` de `Review.svelte` a un módulo
  compartido con test unitario TS; actualizar las aserciones de fuente que
  leen `Review.svelte` (design D3) — gates: `npm test` en `desktop/ui`,
  `cargo test -p meltemi-desktop` (tests de cableado)
- [x] 3.2 Tablero: cabecera por calle con procedencia (chips agente +
  perfil), estado turno/commit/checkpoint con señal+palabra, y calles lado
  a lado sobre el diff compartido; estado vacío honesto con camino a
  asignar; strings nuevas en `messages.ts` ES y EN (design D3) —
  escenarios «Calles lado a lado con procedencia visible» y «Carrera sin
  competidores, estado vacío honesto» — gates: suite de cableado +
  `npm run check:forms`
- [x] 3.3 Acciones de carrera desde el tablero: despachar turno, revertir
  (con `ConfirmDialog`, honrando `dangerous`), commit y merge por archivo
  vía formularios tipados; cancelar la confirmación no envía nada (design
  D3) — escenario «Acción destructiva solo con confirmación explícita» —
  gates: suite de cableado
- [x] 3.4 Vida del tablero: al concluir un turno propio (stream de eventos
  de sesión), re-consultar el diff y actualizar la calle sin recargar;
  refresco manual conservado para despachos ajenos, con la limitación
  declarada en la superficie (design D5) — escenario «El tablero refleja
  el turno concluido» — gates: suite de cableado

## 4. TUI: la casa reservada se abre

- [x] 4.1 Generalizar el drill (`drilled: bool` → superficie drill
  enumerada) sin cambiar el comportamiento de Sesiones; verbo `race`
  des-reservado con su arm en `reduce_palette` (design D4) — escenario «El
  verbo de carrera abre el tablero» — gates: `cargo test -p meltemi`
- [x] 4.2 Tablero en el shell: calles con estado en glifo+palabra (gemelos
  ASCII) y procedencia por calle (agente y perfil, ausencia visible), diff
  con paneo existente y tope declarado; variantes Effect/Command/Update y
  render con tests de reducer y de buffer, incluida la presentación ASCII
  (design D4) — escenario «El tablero degrada a ASCII sin perder
  significado» — gates: `cargo test -p meltemi`
- [x] 4.3 Despacho desde el tablero con el patrón de petición larga (peer
  clonado, task aparte del bucle); e2e vivo con daemon efímero: el tick
  sigue refrescando mientras corre el turno y el tablero refleja la
  conclusión (design D4, D5) — escenario «El despacho no congela el
  shell» — gates: `cargo test -p meltemi`

## 5. Cierre: paridad, smoke y verificación

- [x] 5.1 Matriz de paridad: punteros de vista actualizados para las filas
  de worktree/checkpoint (GUI «registry + tablero», TUI «race»), sin filas
  nuevas; coherencia con los tests de docs (design D6) — gates: `cargo
  test -p meltemi --test parity --test docs`
- [x] 5.2 Smoke visual CDP del tablero (GUI construida, calles reales de
  una carrera fixture) con informe publicado en `docs/qa/` (design D6)
- [x] 5.3 `meltemi validate tablero-de-carrera` limpio y `meltemi verify
  tablero-de-carrera` con los doce escenarios enlazados a sus tests (meta:
  cero marcas manuales); suite completa, clippy y fmt verdes en las tres
  plataformas (design D6)
