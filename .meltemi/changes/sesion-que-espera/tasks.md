# sesion-que-espera — tasks

> Vía completa. El orden es el del riesgo: primero el borde del turno (donde
> vive la espera y donde una carrera mal cerrada no se ve en los tests), después
> el contrato, las cotas, y al final las superficies —que son muchas y casi
> ninguna es error de compilación (design D6).

## 1. El primitivo que la cola no tiene

- [x] 1.1 `InstructionQueue` gana su señal: un `Notify` propio, señalado por
  `enqueue`, `interrupt_with` y `mark_cancelled` **bajo el mismo lock que muta
  el estado y después de mutarlo** — la disciplina que `redirigir-turno` dejó
  escrita y probada (design D1)
- [x] 1.2 `take_or_wait()` sustituye a `take_or_close()` como verbo del borde:
  comprueba y se registra en la señal **sin soltar el lock entre ambas cosas**,
  que es la única forma de que no exista ventana. `close()` sobrevive para los
  finales de verdad (design D1) — escenario «Encolar y despertar no dejan
  ventana»
  <!-- 2026-08-15: el design pedía «registrarse en la señal sin soltar el lock»,
  que es algo que `Notify` no puede dar literalmente —el registro ocurre al
  primer poll, ya sin el lock—. Lo que cierra la ventana de verdad es
  **`notify_one`, que guarda un permiso cuando nadie espera**: el mutador señala
  con el lock tomado y el borde comprueba antes de esperar, así que una
  instrucción que cae entre medio deja permiso y la espera vuelve al instante.
  `notify_waiters` —el que usa `redirigir-turno`— habría perdido ese despertar.
  El verbo quedó partido en dos (`try_take` + `wait_for_work`) en vez de un
  `take_or_wait` monolítico, para que el `select!` de las cotas viva donde
  tiene que vivir. -->
- [x] 1.3 El test de la ventana: una instrucción que llega exactamente mientras
  el borde entra en espera se despacha igual, y la sesión no queda dormida con
  trabajo en la cola

## 2. El borde del turno espera

- [x] 2.1 El bucle de `run_session` espera en vez de romper cuando la cola queda
  vacía, dentro del scope de `connect_with` — porque retornar *es* matar al
  agente (design D1) — escenario «La sesión sobrevive al turno y espera»
- [x] 2.2 La espera compone en un `select!` con las tres salidas: instrucción
  nueva, cancelación (el mismo `cancel` que ya usa el apagado ordenado), y las
  cotas de la 4 — escenario «La sesión en espera se cancela como cualquier otra»
- [ ] 2.3 El estado se declara **después** de que la espera humana haya
  terminado, nunca antes: `end_waiting` restituye a `Active`
  incondicionalmente y pisaría el estado nuevo (design D7). Test que pinea el
  orden
- [x] 2.4 Una sesión sin cola de instrucciones (`instruction_queue: None`) sigue
  siendo de un solo turno, sin esperar: el arranque de autoría no gana una
  espera que nadie pidió

## 3. El contrato

- [x] 3.1 `SessionState::WaitingInstruction` en `meltemi-proto` y en **los dos**
  JSON Schemas que duplican el enum, con el test que los compara — hoy nada lo
  guarda, aunque el idioma de ese test ya está escrito dos veces en el repo
  (design D6)
- [x] 3.2 `session/start` gana el parámetro aditivo de desacople, con la
  conformidad de tres vías (presente, omitido, y la forma omitida byte a byte
  idéntica) y `gen:forms` commiteado (design D2) — escenarios «Arranque
  desacoplado responde con el identificador» y «Sin pedirlo, el arranque
  responde como siempre»
  <!-- 2026-08-15: el test `the_free_session_verb_maps_to_session_start` colgó a
  los 30 s en cuanto el bucle aprendió a esperar, y eso **corrigió el design**:
  yo había separado «cuándo respondes» y «la sesión se aparca» en dos decisiones
  (D2 y D3) cuando son **una sola pregunta** —¿queda alguien con quien hablar?—.
  Por eso `detach` gobierna las dos cosas: quien espera el resultado pidió un
  turno y su desenlace, y aparcarle la sesión colgaría justamente la llamada que
  quería la respuesta. La CLI no desacopla y por tanto no espera: su test volvió
  a pasar sin tocarlo. -->
- [x] 3.3 El handler responde temprano copiando la forma que **ya existe en el
  mismo archivo**: la rama `queued`/`relayed` de `session/direct` responde con
  `status: null` y difiere el desenlace al stream (design D2)
- [ ] 3.4 `session/direct` sobre una sesión en espera despacha de inmediato en
  vez de tomar el camino de reanudación — escenario «Instrucción a una sesión
  que espera se despacha de inmediato»

## 4. Las cotas, y el final honesto

- [x] 4.1 `idle-timeout` y `max-idle-sessions` en la config, con el idioma que
  el repositorio ya usa —`Option<T>` crudo, default en el accesor con
  `unwrap_or`, diagnóstico con remedio que **conserva el default**, como
  `no-client-grace`— y defaults conservadores (design D4)
- [x] 4.2 La tercera cota **no se escribe**: `no_clients_sustained` ya existe y
  se compone en el mismo `select!` (design D4) — escenario «Sin clientes
  sostenidamente, la espera termina»
- [x] 4.3 Al vencer, finalize con `reason` estable en inglés (`idle_timeout`),
  jamás `completed` fingido — escenario «La espera vencida termina con su
  motivo»
- [x] 4.4 El tope cierra la espera **más antigua** y lo dice; el arranque nuevo
  no se rehúsa, porque rehusar castiga al usuario por sesiones que ya no mira
  (design D4) — escenario «El tope de esperas cierra la más antigua»
- [x] 4.5 El motivo se **traduce en cada superficie**, con su clave ES/EN: la
  GUI hoy imprime el string crudo del payload en el transcript y la constitución
  §11 obliga a internacionalizar (design D5). El lint de i18n es el guardián

## 5. Las superficies — enumeradas, porque el compilador no avisa

- [x] 5.1 El guardián primero: un test que recorre el enum del contrato y exige
  que **cada** estado tenga símbolo y palabra en **cada** mapa de superficie, de
  la clase que atrapó la omisión en `redirigir-turno` (design D6) — escenario
  «Ninguna superficie omite el estado de espera»
- [x] 5.2 Los tres sitios que **sí** son error de compilación:
  `session_state_label` de la TUI, el `Record<SessionState, …>` de
  `StatusBadge.svelte`, y el catálogo ES/EN
- [x] 5.3 Los sitios que **aceptarían el estado nuevo en silencio y lo pintarían
  mal**, uno por uno: el glifo del sidebar, los dos contadores de la barra de
  estado, `LIVE` de `tree.ts`, `isLive` de `Sessions.svelte`, `live_sessions`
  del daemon, `is_historical` de la TUI, y la pestaña de la tira
  <!-- 2026-08-15: en vez de parchear cada lista positiva, la pregunta «¿está
  viva?» quedó en **un solo lugar por lado**: `SessionState::is_live()` en el
  contrato (match exhaustivo ⇒ error de compilación al añadir un estado) y
  `LIVE_STATE: Record<SessionState, boolean>` en un módulo hoja de TS (el
  `Record` sobre la unión es exhaustivo ⇒ error de tipo). El árbol, la tabla, la
  barra lateral y el compositor lo preguntan; ya no lo listan. `is_historical`
  de la TUI pasó a ser la negación de esa respuesta. La barra de estado ganó una
  **tercera** cifra en vez de doblar el significado de una existente: una sesión
  entre turnos también te espera, pero no te debe nada. -->
- [x] 5.4 El compositor de la GUI queda vivo y sin rótulo de reanudación en
  espera — escenario «El compositor no muere al terminar el turno»
- [x] 5.5 Se comprueba que el anillo **sigue oscuro sin tocarlo**: lo gobierna
  una comparación literal con su test, así que el estado nuevo queda apagado por
  construcción — escenario «Esperar no enciende el indicador de trabajo»
- [ ] 5.6 TUI: estado con símbolo y palabra, y dirección ofrecida como sobre una
  activa — escenario «El shell dice que la sesión espera»
- [x] 5.7 CLI: el default sigue bloqueando, y la ayuda dice qué **no** se verá al
  desacoplar, porque `session/watch` y `session/log` son huecos declarados de la
  superficie scriptable (design D3) — escenario «Arrancar desde la CLI sigue
  mostrando el desenlace»

## 6. El e2e y el cierre

- [ ] 6.1 E2e contra el mock: turno, espera declarada, instrucción que despierta
  sin relanzar el subproceso, y cancelación que termina la espera
- [ ] 6.2 E2e de la cota: espera vencida que finaliza con su motivo y queda
  reanudable
- [ ] 6.3 QA del reposo con N sesiones esperando, medido y escrito — el riesgo
  mayor que el proposal declaró es este y no se cierra suponiendo
- [ ] 6.4 `validate` limpio, `verify` con todos los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend, y `docs/paridad-nucleo.md` revisado
  (el desacople es parámetro, no verbo: se comprueba que no haga falta fila
  nueva)
