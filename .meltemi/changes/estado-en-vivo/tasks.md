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
- [x] 2.4 `server.rs`: la conexión se suscribe al abrirse, empuja la **foto**
  —una afirmación por sesión conocida— al completarse `initialize`, y al recibir
  `RecvError::Lagged` la vuelve a empujar entera en vez de dejar el hueco
  registrado solo del lado del servidor; el orden suscribir→fotografiar es
  deliberado y el comentario lo dice (design D3, D4) — escenarios «Una conexión
  nueva recibe el estado actual antes de cualquier transición» y «Un rezago se
  repara reafirmando el estado, no con un hueco» — gates: suite de `meltemid`
  <!-- 2026-09-26: la foto y la reparación son **la misma función**, y eso no es
  economía sino la consecuencia de D1: una afirmación entera sirve igual para
  abrir que para reparar. El brazo del rezago la llama con el mismo argumento que
  `initialize`.
  Los tests de este tramo registran sesiones **directamente en el registro**, sin
  agente: lo que se prueba es quién recibe qué, y un agente añadiría subproceso,
  apretón de manos y turno a una pregunta que no los necesita. Eso destapó algo
  que conviene dejar escrito: `session/list` arma sus filas desde el índice en
  disco y solo le pide el estado vivo al registro, así que una sesión registrada
  a mano **no aparece en la lista** —medido, no supuesto: la aserción falló en
  «la sesión está listada», no en un desacuerdo—. El acuerdo entre canal y lista
  se afirma donde de verdad pueden discrepar (el estado del registro contra la
  afirmación que publica) y de punta a punta en 2.5.
  Un test más de los que no estaban pedidos y hacen falta: una conexión que no
  completó el apretón de manos **no recibe nada**, que es el deny-by-default de
  §3 llegando también aquí; y la afirmación lleva cinco campos y ningún sexto,
  comprobado leyendo las claves del objeto en el cable y verificando que la línea
  de comandos del agente no viaja. -->
- [x] 2.5 e2e con `mock-agent`: un cliente abre una sesión y **otro** cliente
  conectado al mismo daemon recibe sus transiciones sin pedirlas, y el estado
  que recibe coincide con `session/list` en ese instante. Y la medición que el
  design promete: varias sesiones contra varios clientes, comprobando que en
  uso normal nadie se rezaga y que quien se rezaga queda con estado válido
  (design D5) — escenario «Una transición llega a un cliente que no inició la
  sesión» — gates: suite de `meltemid`
  <!-- 2026-09-26: **este test encontró un agujero real y esa es su
  justificación entera.** Los flujos de un solo turno (`propose`, la rama de
  reanudación) no terminan con `set_state(Ended)`: finalizan **quitando** la
  sesión del registro (`session_finalize`). Sin anuncio en esa salida, el
  segundo cliente quedaba con «esperando instrucción» y no volvía a saber nada
  nunca — exactamente la mentira congelada que esta change viene a terminar, y
  reintroducida por la puerta de atrás. `deregister` ahora afirma `ended` al
  salir. Lo encontró el test diciendo «me dijeron que esperaba y después nada».
  Dos suposiciones mías que el test también corrigió: completar el turno **no**
  termina la sesión, la aparca con su agente intacto; y `session/cancel` es una
  **notificación**, no una petición, así que pedirle respuesta devuelve «method
  not found».
  La medición de la tormenta: 5 clientes × 20 sesiones × 4 transiciones = 400
  notificaciones, en 0,32 s, sin que nadie se rezague. El techo del canal es
  1024 y el uso ordinario no se le acerca, así que D5 —sin coalescencia hasta
  que algo la pida— se queda con una cifra detrás. -->

## 3. Las superficies

- [x] 3.1 GUI: un caso `session/state` en el `switch` que ya despacha los
  empujones del daemon (`stores.ts`), que parchea `allSessions` por
  `sessionId` y vuelve a estrechar `sessions`; el sondeo queda como
  reconciliación al conectar y al cambiar de proyecto. **Ningún componente se
  toca** y las guardias de `scenarios_shell.rs` siguen intactas (design D7) —
  escenarios «El contador de la barra cuenta una sesión de otro cliente» y «El
  sondeo solo reconcilia» — gates: suite de `meltemi-desktop`, tests de la UI
  <!-- 2026-09-26: se cumplió lo que D7 prometía —**ningún componente tocado**—
  y el escenario lo pinea por los dos lados: que la tienda toma la afirmación, y
  que los cuatro indicadores siguen derivando de ese único array. Si mañana uno
  de ellos se buscara su propia fuente, parchear la tienda dejaría de ser el
  arreglo entero y este test lo diría.
  Un caso que el design no había previsto: una sesión que **otro cliente acaba
  de abrir** no tiene fila en la tienda, y la afirmación es delgada a propósito
  —lleva dónde está la sesión, no todo lo que una fila muestra—. Inventarla con
  cinco campos habría puesto una fila a medias en la barra lateral; se pide la
  lista una vez y desde ahí las afirmaciones la mantienen al día.
  `svelte-check` limpio (150 archivos) y los 82 tests de la UI verdes; el taller
  necesitaba `npm ci` primero, que no toca el lockfile. -->
- [x] 3.2 TUI: un `Update` propio para la transición, que parchea la fila por su
  id; `refresh_sessions` deja de ser la fuente y pasa a reconciliar; el ticker
  de 2 s se queda para los relojes (design D7) — escenarios «La fila cambia al
  llegar la transición» y «El reloj sigue moviendo los relojes» — gates: suite
  de `meltemi`
  <!-- 2026-09-26: el ticker **deja de pedir la lista** y se queda con los
  relojes y las vitales del daemon, que es lo que el escenario exige; la lista
  sigue pidiéndose al conectar, en un refresco explícito, y cuando una afirmación
  nombra una sesión sin fila. Para distinguir ese caso el actor de conexión lleva
  un conjunto de ids conocidas —local, como `watched`— y `refresh_sessions`
  devuelve las que trajo para **sembrarlo**: sin eso, la foto de apertura del
  daemon habría costado una consulta por sesión, que es peor que el sondeo que
  acabamos de quitar.
  La campana gana su caso: una sesión que termina es justo el momento para el
  que existe, y ahora llega como afirmación propia en vez de notarse en el
  siguiente sondeo. La regla vieja suena por un refresco entero que contenga una
  sesión terminada; se deja como está, porque el sondeo que la alimentaba ya no
  es quien encuentra los finales. -->
- [x] 3.3 CLI: `sessions --follow` con las convenciones de `follow` —un registro
  por línea en todo formato, un documento YAML por registro, error a medias
  cierra el flujo con su objeto—, empezando por la foto; `sessions` sin la
  bandera responde exactamente como hoy; `docs/referencia-cli.md` lo documenta
  (design D8) — escenarios «Cada transición es una línea», «El flujo termina con
  su daemon» y «Sin la bandera nada cambia» — gates: suite de `meltemi`
  <!-- 2026-09-26: **dos guardianes del repositorio saltaron y los dos tenían
  razón.** La gramática rehúsa toda bandera que no liste, así que `--follow`
  tuvo que declararse; y un test pinea la lista **entera** con el comentario
  «adding one changes this test, in front of whoever reviews it», precisamente
  para que ninguna bandera entre sin que alguien la vea. Queda con su razón al
  lado: lee, no concede.
  El verbo acepta la bandera **a los dos lados de la raíz**, porque las dos
  lecturas son naturales y rehusar una sería una regla que nadie puede adivinar.
  Con raíz filtra por la misma regla que la GUI —separadores y mayúsculas
  normalizados, y un worktree bajo la raíz cuenta—, porque un verbo que acepta
  una raíz al listar y la ignora al seguir sería una trampa.
  No hace falta declarar interés ni vigilar la vida de nada: el daemon afirma
  todas las sesiones a toda conexión, y este flujo no tiene una sesión a la que
  sobrevivir. El e2e juega al daemon para escenificar las tres cosas que ningún
  daemon real hace a pedido: foto, transición y desaparecer. Salida 10 al
  desaparecer, comprobada. -->

## 4. Cierre

- [x] 4.1 `meltemi validate estado-en-vivo` limpio; `meltemi verify` con los
  doce escenarios enlazados; suite completa con `--no-fail-fast`, clippy y fmt
  verdes en este equipo; `cargo deny check` verde; entrada en
  `docs/plan-de-cambios.md` con lo que la medición de la tormenta encontró; y la
  rama aterriza en `main` con `meltemi land estado-en-vivo confirm`
  <!-- 2026-09-26: `validate` limpio; `verify` **12/12**; `fmt --check`,
  `clippy --workspace --all-targets -D warnings`, `cargo deny check` y
  `cargo test --workspace --no-fail-fast` verdes en este equipo (116 binarios de
  test, cero fallos).
  El escenario del reloj llegó al final y por eso: la primera corrida de `verify`
  dio 11/12 y lo nombró. Se cubre observando al actor de conexión contra un
  daemon real durante dos tics: si el tic siguiera sondeando, empujaría una lista
  nueva cada dos segundos. Comprobado que muerde —devolviendo el sondeo al tic,
  el test falla con esas palabras— y que el tic **sigue vivo**, porque un test que
  solo comprobara el silencio pasaría igual si el reloj hubiera desaparecido. -->
