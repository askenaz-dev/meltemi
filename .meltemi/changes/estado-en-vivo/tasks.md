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

- [ ] 1.1 `proto/`: constante `SESSION_STATE` en el módulo `methods` (una sola
  línea con `= "session/state"`, porque el gate de paridad parsea texto);
  `SessionStateParams` con `sessionId`, `projectRoot`, `state`, `since` y
  `pendingPermissions`; esquema `proto/schemas/v1/session-state.schema.json`
  con su `$defs.notificationParams`; `assert_conforms` en
  `proto/meltemi-proto/tests/conformance.rs`. Sin campo de contenido alguno
  (design D1, D9) — escenario «La notificación no lleva contenido del
  proyecto» — gates: suite de `meltemi-proto`
- [ ] 1.2 Paridad: `session/state` en `DAEMON_INITIATED` de
  `tui/tests/parity.rs` y su fila en la tabla de tráfico iniciado por el daemon
  de `docs/paridad-nucleo.md`, con las cinco columnas. **No** entra en la
  paleta de la TUI ni en el registro de la GUI, y el test de la dirección
  contraria es la razón (design D9) — gates: `cargo test -p meltemi --test parity`

## 2. El daemon

- [ ] 2.1 `session.rs`: el registro gana `state_since` por sesión, en RFC 3339
  UTC, fijado en **cada** una de las cinco mutaciones de estado (`register`,
  `set_state`, `begin_waiting`, `end_waiting`, `begin_idle`); `summaries()` lo
  expone junto al conteo de permisos en espera que ya lleva. `idle_since` no se
  toca: es monótono y mide otra cosa (design D6) — gates: suite de `meltemid`
- [ ] 2.2 `events.rs`: el hub gana un **segundo canal** `broadcast` para el
  estado, con su propio sobre y sin la regla de `delivers` —toda conexión
  iniciada recibe toda transición, sin origen y sin vigilancia—, y un test que
  pinea que dos suscriptores reciben la misma afirmación una vez cada uno
  (design D2) — escenario «Una transición llega a un cliente que no inició la
  sesión» — gates: suite de `meltemid`
- [ ] 2.3 `session.rs` + los cinco puntos de mutación: publicar la afirmación
  al canal en cada transición, con el estado entero y no un delta; un test por
  transición que compruebe que lo publicado coincide con lo que `session/list`
  devolvería en ese instante (design D1) — escenario «La notificación afirma el
  estado entero y es idempotente» — gates: suite de `meltemid`
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
