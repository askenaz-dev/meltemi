# Tareas — estado-en-vivo

Vía completa. Un commit atómico por tarea, con referencia
`(estado-en-vivo N.M)` y **sin trailers de co-autoría**. Gates del repo en cada
tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite del crate tocado, y `cargo deny check bans` donde
cambie un `Cargo.toml`. **Esta change se desarrolla en su propio taller
(`meltemi workspace estado-en-vivo`) y aterriza en `main` con `meltemi land` al
cerrar.**

Orden deliberado: el contrato primero, el daemon después, y las tres
superficies al final. Así cada superficie se escribe contra un canal que ya
existe y ya está probado, en vez de contra una promesa.

## 1. El contrato

- [x] 1.1 `proto/`: constante `SESSION_STATE` en el módulo `methods` (una sola
  línea con `= "session/state"`, porque el gate de paridad parsea texto);
  `SessionStateParams` con `sessionId`, `projectRoot`, `state`, `since` y
  `pendingPermissions`; esquema `proto/schemas/v1/session-state.schema.json`
  con su `$defs.notificationParams`; `assert_conforms` en
  `proto/meltemi-proto/tests/conformance.rs`. Sin campo de contenido alguno
  (design D1, D9) — escenario «La notificación no lleva contenido del
  proyecto» — gates: suite de `meltemi-proto`
  <!-- 2026-09-26: el esquema nuevo entra además a la comparación del enum de
  estados, que hasta hoy recorría dos archivos y ahora recorre tres: la
  notificación lleva el enum, así que debe quedar sujeta a la misma regla que
  los dos que ya existían. Los tres rechazos del test son los que muerden solos:
  una afirmación sin `since`, una con `since` que no es fecha, y un estado que
  nadie declaró. -->
- [x] 1.2 Paridad: `session/state` en `DAEMON_INITIATED` de
  `tui/tests/parity.rs` y su fila en la tabla de tráfico iniciado por el daemon
  de `docs/paridad-nucleo.md`, con las cinco columnas. **No** entra en la
  paleta de la TUI ni en el registro de la GUI, y el test de la dirección
  contraria es la razón (design D9) — gates: `cargo test -p meltemi --test parity`
  <!-- 2026-09-26: el gate estaba en rojo antes de tocarlo, con las tres
  aserciones nombrando `session/state` —sin casa en la paleta, sin casa en el
  registro de la GUI, y ausente de la matriz—, así que muerde sin necesidad de
  neutralizar nada. La columna CLI de la fila nueva nombra `sessions --follow`,
  que llega en la tarea 3.3 de esta misma change: la matriz describe el estado
  final, y la change aterriza de una pieza. La prosa de la sección gana la razón
  por la que estas dos notificaciones llegan a todos y las otras no: su carga
  afirma un estado entero, no un cambio. -->

## 2. El daemon

- [x] 2.1 `session.rs`: el registro gana `state_since` por sesión, en RFC 3339
  UTC, fijado en **cada** una de las cinco mutaciones de estado (`register`,
  `set_state`, `begin_waiting`, `end_waiting`, `begin_idle`); `summaries()` lo
  expone junto al conteo de permisos en espera que ya lleva. `idle_since` no se
  toca: es monótono y mide otra cosa (design D6) — gates: suite de `meltemid`
  <!-- 2026-09-26: **dos correcciones a lo que esta tarea creía.** (1) El
  registro no guardaba la raíz del proyecto: `session/list` la lee del índice en
  disco y solo le pide el estado vivo al registro. La notificación tiene que
  llevarla, así que `register` gana un tercer argumento y la raíz viaja desde
  los seis sitios de registro, con la misma expresión con la que el índice la
  escribe (`display().to_string()`) para que las dos grafías no se separen —si
  se separaran, el estrechamiento por proyecto de la GUI dejaría de casar.
  (2) `summaries()` **no** lleva el conteo de permisos ni lo lleva
  `SessionInfo`; ambos salen de la entrada del registro, así que la afirmación
  se construye ahí y `SessionInfo` no se toca. Un archivo de esquema menos que
  mover.
  El instante se sella **al entrar** y no en cada asignación: varios sitios
  fijan `Active` sobre una sesión ya activa, y un `since` que se moviera en cada
  uno reiniciaría el cronómetro que la superficie muestra. Comprobado que el
  test muerde: sin la guarda, falla con un segundo de diferencia. -->
- [x] 2.2 `events.rs`: el hub gana un **segundo canal** `broadcast` para el
  estado, con su propio sobre y sin la regla de `delivers` —toda conexión
  iniciada recibe toda transición, sin origen y sin vigilancia—, y un test que
  pinea que dos suscriptores reciben la misma afirmación una vez cada uno
  (design D2) — escenario «Una transición llega a un cliente que no inició la
  sesión» — gates: suite de `meltemid`
  <!-- 2026-09-26: el canal de estado **no lleva sobre**. El del evento sella el
  origen porque `delivers` lo necesita; aquí no hay nada por lo que filtrar, y
  sellar un origen que nadie usa sería una invitación a que alguien filtre por
  él mañana. Se añade además un test de que los dos canales no se cruzan: un
  evento que apareciera en el canal de estado habría pasado de contrabando por
  delante de `delivers`. -->
- [x] 2.3 `session.rs` + los cinco puntos de mutación: publicar la afirmación
  al canal en cada transición, con el estado entero y no un delta; un test por
  transición que compruebe que lo publicado coincide con lo que `session/list`
  devolvería en ese instante (design D1) — escenario «La notificación afirma el
  estado entero y es idempotente» — gates: suite de `meltemid`
  <!-- 2026-09-26: **anuncia el registro, no los sitios de llamada.** Hay una
  docena de sitios que mueven una sesión, y pedirle a cada uno que se acuerde de
  anunciar después es pedir el que se olvide — y un estado que se movió sin
  anunciarse es exactamente la mentira silenciosa que esta change viene a
  terminar. El registro guarda el hub y `DaemonState` le pasa **el mismo** que
  escuchan las conexiones: dos hubs serían dos silencios.
  Dos detalles que el test fija: `begin_waiting` anuncia **aunque el estado no
  se mueva**, porque el conteo es parte de la afirmación y una segunda petición
  esperando es noticia; y una sesión que el registro no tiene no anuncia nada, en
  vez de afirmar una sesión que no existe. `begin_idle` anuncia antes de aplicar
  el tope, porque esa sesión ya está esperando sea lo que sea que el tope decida
  de otra. Comprobado que muerde: sin el anuncio de `set_state`, el test falla en
  la primera transición que no llega. -->
- [ ] 2.4 `server.rs`: la conexión se suscribe al abrirse, empuja la **foto**
  —una afirmación por sesión conocida— al completarse `initialize`, y al recibir
  `RecvError::Lagged` la vuelve a empujar entera en vez de dejar el hueco
  registrado solo del lado del servidor; el orden suscribir→fotografiar es
  deliberado y el comentario lo dice (design D3, D4) — escenarios «Una conexión
  nueva recibe el estado actual antes de cualquier transición» y «Un rezago se
  repara reafirmando el estado, no con un hueco» — gates: suite de `meltemid`
- [ ] 2.5 e2e con `mock-agent`: un cliente abre una sesión y **otro** cliente
  conectado al mismo daemon recibe sus transiciones sin pedirlas, y el estado
  que recibe coincide con `session/list` en ese instante. Y la medición que el
  design promete: varias sesiones contra varios clientes, comprobando que en
  uso normal nadie se rezaga y que quien se rezaga queda con estado válido
  (design D5) — escenario «Una transición llega a un cliente que no inició la
  sesión» — gates: suite de `meltemid`

## 3. Las superficies

- [ ] 3.1 GUI: un caso `session/state` en el `switch` que ya despacha los
  empujones del daemon (`stores.ts`), que parchea `allSessions` por
  `sessionId` y vuelve a estrechar `sessions`; el sondeo queda como
  reconciliación al conectar y al cambiar de proyecto. **Ningún componente se
  toca** y las guardias de `scenarios_shell.rs` siguen intactas (design D7) —
  escenarios «El contador de la barra cuenta una sesión de otro cliente» y «El
  sondeo solo reconcilia» — gates: suite de `meltemi-desktop`, tests de la UI
- [ ] 3.2 TUI: un `Update` propio para la transición, que parchea la fila por su
  id; `refresh_sessions` deja de ser la fuente y pasa a reconciliar; el ticker
  de 2 s se queda para los relojes (design D7) — escenarios «La fila cambia al
  llegar la transición» y «El reloj sigue moviendo los relojes» — gates: suite
  de `meltemi`
- [ ] 3.3 CLI: `sessions --follow` con las convenciones de `follow` —un registro
  por línea en todo formato, un documento YAML por registro, error a medias
  cierra el flujo con su objeto—, empezando por la foto; `sessions` sin la
  bandera responde exactamente como hoy; `docs/referencia-cli.md` lo documenta
  (design D8) — escenarios «Cada transición es una línea», «El flujo termina con
  su daemon» y «Sin la bandera nada cambia» — gates: suite de `meltemi`

## 4. Cierre

- [ ] 4.1 `meltemi validate estado-en-vivo` limpio; `meltemi verify` con los
  doce escenarios enlazados; suite completa con `--no-fail-fast`, clippy y fmt
  verdes en este equipo; `cargo deny check` verde; entrada en
  `docs/plan-de-cambios.md` con lo que la medición de la tormenta encontró; y la
  rama aterriza en `main` con `meltemi land estado-en-vivo confirm`
