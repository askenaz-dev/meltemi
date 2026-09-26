# Tareas — via-ssh-del-cliente

Vía completa. Un commit atómico por tarea, con referencia
`(via-ssh-del-cliente N.M)` y **sin trailers de co-autoría**. Gates del repo en
cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite del crate tocado, y `cargo deny check bans` donde
cambie un `Cargo.toml`. **Esta change se desarrolla en su propio taller y
aterriza en `main` con `meltemi land` al cerrar.**

## 1. El transporte

- [x] 1.1 `core/meltemi-client`: el partidor de comandos —programa y
  argumentos, comillas dobles agrupan, sin shell— con sus tests de mesa
  (comando simple, argumento entrecomillado, comillas sin cerrar, cadena
  vacía) (design D2) — escenario «El comando se lanza sin intérprete
  intermedio» — gates: suite de `meltemi-client`
  <!-- 2026-09-26: un caso que la tarea no listaba y que el partidor tiene que
  acertar: **un par de comillas vacío es un argumento vacío de verdad**. Quien
  escribe `-o ""` quiere pasar una cadena vacía, y descartarla habría cambiado el
  comando del usuario en silencio. También se añade el nombre abreviado para el
  cromo, que es de la tarea 2.2 pero pertenece a este módulo: muestra el programa
  sin su ruta y el primer argumento, que para el caso que existe es el host. -->
- [x] 1.2 `transport.rs`: la forma `exec:` del endpoint; lanza el proceso
  dentro de un `Scope` de `meltemi-process`, une su stdin y su stdout en un
  `Stream`, recoge las últimas líneas de su `stderr` en un buffer acotado, y
  al soltarse termina el proceso y el ámbito; `meltemi-client` gana la
  dependencia del crate del ámbito, justificada aquí (design D1, D6, D7) —
  escenarios «El cliente habla por el comando que el usuario nombró» y «Al
  cerrar el cliente no queda el proceso» — gates: suite de `meltemi-client`,
  `cargo deny check bans`
- [x] 1.3 El rehúse con causa: un proceso que muere antes del apretón de manos
  produce un error que nombra el comando, adjunta lo que escribió en `stderr` y
  dice el remedio; Meltemi no añade ninguna opción al comando del usuario
  (design D5) — escenario «Un comando que no llega a hablar el protocolo se
  rehúsa con su causa» — gates: suite de `meltemi-client`
  <!-- 2026-09-26: **1.2 y 1.3 aterrizan en el mismo commit y separarlas habría
  sido falso.** El rehúse con causa no es un añadido al transporte: es lo que el
  transporte hace cuando la salida del proceso termina. Un `ExecStream` que
  reportara fin de flujo limpio ahí estaría tirando la única explicación que
  existe, y por tanto no hay una versión intermedia del transporte que valga la
  pena cometer.
  **D9 nuevo, descubierto al escribir el test**: el endpoint se **quita** del
  entorno del hijo. Un comando que ejecute a su vez `meltemi` heredaría este
  endpoint y marcaría contra sí mismo; con `ssh` remoto no pasa, con cualquier
  ayudante local sí. El test de eso enseñó además una asimetría de plataforma:
  `cmd` deja `%VAR%` literal cuando la variable no existe, así que la aserción
  correcta es por ausencia del valor, no por un par de corchetes vacío.
  Se declaran además las features de tokio que el crate **ya usaba** por
  unificación de otro crate (`macros`, y `rt` para el recolector): antes
  `cargo build -p meltemi-client` a solas no compilaba, y ahora sí. No entra
  ninguna dependencia nueva a la clausura; `cargo deny check bans` verde. -->
- [x] 1.4 `bootstrap.rs`: un endpoint `exec:` **jamás** arranca un daemon,
  escrito explícitamente y con su test, en vez de depender de que la
  comparación con el endpoint propio no coincida por accidente (design D4) —
  escenario «El comando falla y no se arranca nada» — gates: suite de
  `meltemi-client`
  <!-- 2026-09-26: **no era un accidente inofensivo, era un agujero.** La regla
  vieja es «el endpoint propio, o donde `MELTEMI_START_HERE` lo pida», y con esa
  variable puesta un endpoint `exec:` **sí** habría arrancado un daemon local.
  El test lo pinea con las tres formas de pedirlo. Y el rehúse gana su propia
  redacción, porque las dos salidas que ofrece el otro no sirven aquí: no hay
  túnel que revisar y el opt-in acaba de cerrarse, así que ofrecerlo habría sido
  peor que no ofrecer nada. -->

## 2. Ámbito y superficies

- [x] 2.1 Ámbito usuario: el endpoint con forma de comando se lee del entorno o
  de la configuración de usuario y **nunca** del `.meltemi/config.toml` de un
  repositorio; test que pone uno en un repositorio de prueba y comprueba que se
  ignora y que no se ejecuta nada (design D3) — escenario «Un repositorio no
  decide qué comando corre tu cliente» — gates: suite del crate que resuelve la
  configuración
  <!-- 2026-09-26: la vía de usuario que la propuesta nombraba **no existía**: el
  cliente resolvía el endpoint solo de `MELTEMI_ENDPOINT` y del valor por defecto
  de la plataforma, y ninguna configuración entraba. Eso hacía la garantía cierta
  por vacío, pero dejaba sin resolver el caso que importa: una GUI abierta desde
  un icono nunca vio la variable exportada en un shell. Así que se añade el
  lector, y **la frontera está en la firma**: recibe un directorio de
  configuración y ningún directorio de proyecto, así que no hay camino por el que
  un repositorio la alcance. `toml` entra como dependencia directa del crate, ya
  estaba en la clausura por el daemon y `cargo deny check bans` sigue verde.
  Trampa medida: parsear un documento como `toml::Value` **compila y devuelve
  nada**; hay que pedir `toml::Table`. Costó un test rojo antes de costarle a
  alguien una configuración ignorada en silencio. -->
- [x] 2.2 Las tres superficies: el CLI opera con el endpoint nuevo en cualquier
  verbo, y el cromo de la TUI y de la GUI muestra la vía con el comando
  abreviado (design D8) — escenarios «Un verbo cualquiera opera a través del
  comando», «El cromo nombra la vía» y «El cromo de la TUI nombra la vía» —
  gates: suite de `meltemi`, suite de `meltemi-desktop`
  <!-- 2026-09-26: **el e2e del CLI encontró otro agujero, y este era del
  proyecto entero, no de esta change.** El lector del peer hacía
  `Ok(None) | Err(_) => break` y después fallaba toda petición en vuelo con
  «connection closed», tirando el error del transporte. Con eso, todo el
  diagnóstico que la tarea 1.3 construyó —lo que `ssh` dijo, el remedio— **no
  llegaba al usuario**: el rehúse que veía era «connection closed» a secas. Ahora
  el lector guarda la razón y la propaga, que sirve a cualquier transporte que
  sepa explicarse; un socket que cierra sigue diciendo lo de siempre porque no
  tiene nada que añadir.
  El fixture del e2e es un **script** que fija el endpoint y corre
  `meltemi bridge`, que es lo que hace `ssh mi-pc meltemi bridge` una vez hecha
  la conexión. Probarlo con un `ssh` real necesitaría host, clave y red, y sería
  probar OpenSSH. En Windows el script es un `.cmd`, así que de paso ejercita el
  ámbito de proceso con `cmd.exe` en medio.
  La regla del cromo vive **dos veces** —en Rust para la TUI y en un módulo hoja
  de TypeScript para la GUI— porque el front no puede llamar al crate; las dos
  parten el comando con la misma regla y las dos tienen test. -->
- [x] 2.3 `docs/acceso-remoto.md` gana «la GUI contra tu PC» con el
  `~/.ssh/config` mínimo y la advertencia de que el `ssh` del usuario decide
  claves y hosts conocidos; `docs/plataformas.md` deja de contradecirlo —
  gates: `cargo test -p meltemi --test docs`
  <!-- 2026-09-26: la sección dice en voz alta las tres cosas que Meltemi **no**
  hace —no añade opciones, no gestiona claves, no da terminal para una
  passphrase— y la consecuencia incómoda que se sigue de la tercera: la primera
  conexión a un host nuevo va a fallar, con el remedio de una vez en un terminal.
  Mejor decirlo que dejar que alguien lo descubra pensando que está roto. -->

## 3. Cierre

- [x] 3.1 `meltemi validate via-ssh-del-cliente` limpio; `meltemi verify` con
  los nueve escenarios enlazados; suite completa con `--no-fail-fast`, clippy y
  fmt verdes; `cargo deny check` verde; entrada en `docs/plan-de-cambios.md`; y
  la rama aterriza en `main` con `meltemi land via-ssh-del-cliente confirm`
  <!-- 2026-09-26: `validate` limpio; `verify` **9/9**; `fmt --check`,
  `clippy --workspace --all-targets -D warnings`, `cargo deny check` y
  `cargo test --workspace --no-fail-fast` verdes (117 binarios, 1.347 tests, cero
  fallos). Nota para el próximo taller nuevo: `cargo test --workspace` **no**
  construye los binarios que los e2e lanzan; hay que correr
  `cargo build --workspace --bins` antes, o 33 suites fallan por «mock-agent
  binary not found» y no por el código. -->
