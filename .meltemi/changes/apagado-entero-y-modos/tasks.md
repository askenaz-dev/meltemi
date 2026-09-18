# Tareas — apagado-entero-y-modos

Vía completa. Un commit atómico por tarea, con referencia
`(apagado-entero-y-modos N.M)` y sin trailers de co-autoría. Gates del repo
en cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite del crate tocado, y `cargo deny check bans`
donde cambie un `Cargo.toml`. **Esta change se desarrolla en su propio
taller (`meltemi workspace apagado-entero-y-modos`) y aterriza en `main` con
`meltemi land` al cerrar.** Las secciones 1 y 2 no comparten archivo
(design D9): cada commit es de una sola.

## 1. Apagado entero

- [x] 1.1 Crate `core/meltemi-process` (biblioteca): `Scope` con `new`,
  `adopt(&tokio::process::Child)`, `adopt_pid(u32)`, `end()` y `Drop` que
  cierra el handle; Windows con Job Object y
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, resto de plataformas un no-op
  declarado; feature `Win32_System_JobObjects` en la entrada `windows-sys`
  del workspace; cabecera SPDX; test `#[cfg(windows)]` con un `.cmd` real
  cuyo nieto (`ping -n 60 127.0.0.1`) desaparece al terminar el ámbito, y
  su gemelo inerte en las otras plataformas (design D1, D2, D5) —
  escenario «Un lanzador intermedio no deja huérfanos» (la parte del
  mecanismo; el daemon lo re-ejercita en 1.3) — gates: suite del crate,
  `cargo deny check bans`
  <!-- 2026-09-18: **la API queda solo con `adopt_pid`**. El design pedía
  además `adopt(&tokio::process::Child)`, y el hijo que el daemon tiene que
  adoptar es un `async_process::Child` —el crate ACP lanza con `async-process`,
  no con tokio—, así que un `adopt` tipado habría servido a uno solo de los dos
  llamadores y el otro habría necesitado igual la vía del identificador. A
  cambio, el contrato escribe lo que el tipo ya no puede garantizar: el llamador
  debe seguir sujetando su `Child`, porque un identificador cuyo proceso ya fue
  cosechado puede quedar reasignado y adoptaríamos a un desconocido.
  El test del nieto costó un falso fallo antes de medir bien: `OpenProcess`
  **también abre un proceso ya terminado** cuyo lanzador no lo ha cosechado, así
  que «se puede abrir» no es «sigue vivo». Se pregunta por el código de salida
  (`GetExitCodeProcess` → `STATUS_PENDING`), que es la pregunta real. El
  mecanismo estaba bien desde la primera corrida: el nieto ya moría. -->
- [x] 1.2 Adaptadores: `supervisor.rs` lanza dentro de un `Scope`,
  `ProcessControl::kill` termina el ámbito, y `kill_on_drop` deja de ser una
  promesa del destructor; `tests/process_lifecycle.rs` gana el caso del
  `.cmd` que ignora el fin de entrada hasta agotar la gracia; la auditoría
  de dependencias sigue verde con el crate nuevo en la clausura (design D1,
  D2, D5) — escenarios «El intermediario del proveedor no deja huérfanos» y
  «El adaptador cae y el proveedor no le sobrevive» — gates: suite de
  `meltemi-adapters`
  <!-- 2026-09-18: el orden del apagado es **primero el hijo, después el
  ámbito**. Al revés, el ámbito mata al hijo y el `kill` posterior de tokio
  opera sobre un proceso que ya no está; así el hijo se mata y se cosecha
  mientras existe, y el ámbito recoge lo que había debajo. Un ámbito que no se
  puede abrir **rehúsa el lanzamiento**, con remedio propio y no el de «instala
  el CLI»: el CLI está, y lo que falló fue que la plataforma no dejó a este
  proceso hacerse cargo de lo que lanza. Comprobado que el test muerde
  neutralizando `scope.end()`: el nieto sobrevive y el test lo dice. El de
  soltar sigue verde con `end()` neutralizado, y debe: lo que prueba es el
  cierre del handle, no la llamada. -->
- [x] 1.3 Daemon: `acp.rs` llama a `AcpAgent::spawn_process` él mismo,
  adopta el hijo por su `id()`, conecta con `Lines` sobre su stdio,
  replica del crate el colector de `stderr` y el vigilante del hijo
  (citando `acp_agent.rs:280-372` de 1.2.0), y anota pid y programa
  efectivo en el log; test de Windows que lanza `mock-agent` a través de
  un `.cmd` y comprueba que cancelar no deja el nieto; el e2e existente con
  `mock-agent` intacto (design D3, D4, D5) — escenarios «Un lanzador
  intermedio no deja huérfanos», «El daemon termina de golpe y nada le
  sobrevive» y «Cancelación de sesión» re-pineado — gates: suite de
  `meltemid`
  <!-- 2026-09-18: **dos desvíos del design, los dos declarados en el
  proposal.** (1) El requisito «el identificador de proceso y el programa
  efectivo SHALL constar en el log» no tenía dónde constar: `agent_resolved` y
  `session_started` se escriben **antes** del lanzamiento y no pueden llevar un
  pid que aún no existe. Se añade al contrato el evento `agent_process
  {pid, binary}` y su entrada en el schema — aditivo, una variante más de una
  unión etiquetada, ningún método ni tipo de petición tocado. (2) `meltemid`
  pasa a depender de `futures` de forma **directa**: el colector de `stderr`
  que se replica del crate ACP usa sus traits de lectura asíncrona. No entra
  nada nuevo a la clausura —ya viajaba como dependencia del propio crate ACP—
  y se pinea igual que el resto.
  El e2e nuevo pone el intermediario real en la cadena (`meltemid` → `cmd.exe`
  → `mock-agent`) y mide lo único que los demás e2e no pueden. **Comprobado que
  muerde**: neutralizando la adopción, el test falla con «process 33672
  outlived the session», y el shim y el `mock-agent` de debajo quedan vivos en
  la tabla de procesos tras completarse la sesión.
  **Corrección tras la suite completa**: el evento nuevo entró al contrato sin
  glifo en la transcripción de la GUI, y el test de paridad que exige uno por
  cada tipo declarado lo detectó en `cargo test --workspace` — no en la suite
  del daemon, que es la que esta tarea declaraba como gate. La GUI ya lo habría
  pintado con el glifo por defecto; el test existe para que un evento nuevo sea
  una decisión y no un fallback, y se tomó: el mismo tono tenue que
  `agent_resolved`, porque es el mismo tipo de hecho. La TUI no nombra tipos de
  evento, así que no había nada que añadir allí. -->
- [x] 1.4 `docs/agentes.md` (sección Windows: qué pasa con un shim y por qué
  ya no importa) y `docs/conformidad-manual.md` (comprobación opt-in contra
  un CLI real instalado por npm: abrir sesión, cancelar, listar procesos)
  (design D5) — gates: `cargo test -p meltemi --test docs`
  <!-- 2026-09-18: la comprobación manual quedó con **dos piernas**, no una.
  Cancelar la sesión ejercita el apagado ordenado; matar el daemon sin darle
  ocasión de limpiar ejercita la otra mitad —la que un destructor no puede
  prometer y el kernel sí—, y es precisamente la que ningún apagado ordenado
  toca. Se añade también el procedimiento de 2.3 (qué campo pobló cada agente
  para sus modos) en la misma página, porque su resultado se persiste ahí. -->

## 2. Modos anunciados

- [x] 2.1 `session_config::from_acp(config_options, modes)` con la
  precedencia de D6 y la síntesis de la opción de modo; el daemon recuerda
  por sesión que esa opción es de forma heredada; tests unitarios de las
  tres combinaciones (solo modos, solo opciones, las dos) — escenarios
  «Modos sin opción de configuración se ofrecen como opción» y «Con las dos
  formas gana la opción de configuración» — gates: suite de `meltemid`
  <!-- 2026-09-18: la función nueva se llama `announced` y **no** reemplaza a
  `from_acp`: la traducción pura sigue haciendo falta donde no hay campo de
  modos que leer (la respuesta a un cambio de opción, y la lista que el agente
  manda por su cuenta), y fundirlas habría obligado a pasar `None` en esos
  sitios para decir «aquí no aplica». Devuelve `Announced { options,
  mode_is_inherited }` en vez de una lista suelta, porque el dato que decide
  qué verbo fija la opción **no se puede leer de la opción**: un agente puede
  anunciar la suya con el mismo id y la misma categoría. El daemon lo guarda en
  `LiveConfig`. Se añade `set_current_mode`, que busca la opción de modo **por
  categoría y no por id**, porque las dos formas coinciden en la categoría y la
  nueva puede llamar a la opción como quiera. -->
- [x] 2.2 `server.rs`: `session/set-config-option` sobre la opción heredada
  manda `session/set_mode` y fija el valor aceptado; `acp.rs`
  (`forward_update`) reconoce `current_mode_update` y
  `config_option_update`, actualiza lo anunciado y registra
  `ConfigOptionsAnnounced`; `available_commands_update` se ve y no se toma,
  con el motivo en el código (design D7) — escenarios «El modo elegido viaja
  por el verbo de modos», «Un cambio de modo del agente actualiza la opción»
  y «Una lista de opciones nueva reemplaza a la anterior» — gates: suite de
  `meltemid`, e2e con `mock-agent` (que gana un guion con modos y otro con
  `current_mode_update`)
  <!-- 2026-09-18: el agente simulado gana **tres** guiones y no dos:
  `--modes`, `--mode-drift` y `--options-drift`, porque el escenario de la
  lista nueva necesita una lista **distinta** de la del apretón de manos para
  poder distinguir «reemplazada» de «fusionada» sin leerle la mente al daemon.
  El discriminador del verbo también está construido en el simulado y no
  asertado de memoria: bajo `--modes` no anuncia opciones de configuración, así
  que su verbo de opciones responde lista vacía — un daemon que tomara el verbo
  equivocado volvería sin nada anunciado, y el test lo dice con esas palabras.
  **Comprobado que los tres muerden**: neutralizado el ruteo y las dos ramas de
  `forward_update`, los tres fallan.
  **Consecuencia declarada, no escondida** (`session.rs`): una lista nueva
  reemplaza entera, así que un agente que anunciara sus modos por el campo
  viejo **y** opciones por el nuevo perdería su selector de modo al cambiar
  cualquier otra opción, porque la lista que reemplaza no lleva modo. Es lo que
  cuesta «reemplazar entera, nunca fusionar» (D7), y volver a sintetizar ahí
  sería justo la fusión que esa regla prohíbe. Ningún agente instalado tiene
  esa forma (ver 2.3). -->
- [x] 2.3 Verificación manual, opt-in, contra cada agente de nivel 1
  instalado en este equipo: abrir una sesión ACP y anotar qué campo pobló
  —modos, opciones, ninguno— con versión y fecha, en
  `docs/conformidad-manual.md` (design D8). Si todos pueblan opciones, la
  nota del backlog lo dice y la síntesis se queda igual — gates: ninguno
  automático; el resultado se persiste como documento
  <!-- 2026-09-18: **corrida hecha**, y el resultado no es el que ninguna de
  las dos hipótesis del design anticipaba. De las seis entradas de nivel 1 del
  catálogo solo `opencode 1.14.33` está instalada en este equipo, y **puebla
  las dos formas a la vez**: `modes` con `build`/`plan`, y `configOptions` con
  `model` (317 valores) y `mode` (categoría `mode`, los mismos dos). Es decir:
  el caso de precedencia de D6 no era teórico, era el único caso real medible
  aquí, y fusionar habría dado dos selectores de modo. La síntesis del campo
  viejo **no queda ejercitada por ningún binario real** —haría falta un agente
  que puebla `modes` y no `configOptions`—, y la página lo dice con esas
  palabras en vez de dejarlo implícito; la cubren los unitarios y el simulado.
  Hallazgo lateral: `modelo-y-esfuerzo-por-sesion` D9 («ningún proveedor
  pineado anuncia opciones de sesión») ya no es cierto — no por error de
  entonces, sino por su fecha.
  La corrida abre sesión y no manda prompt, así que no gasta turno de
  proveedor. `claude 2.1.261` y `codex-cli 0.77.0` están instalados pero son
  de nivel 2 y quedan fuera del alcance que D8 fijó. -->

## 3. Cierre

- [x] 3.1 `meltemi validate apagado-entero-y-modos` limpio; `meltemi verify`
  con los diez escenarios enlazados (los dos de Windows con el test
  `#[cfg(windows)]` como su evidencia, declarado en la nota de la tarea);
  suite completa, clippy y fmt verdes en este equipo; `cargo deny check`
  verde; entrada en `docs/plan-de-cambios.md` con lo que la verificación
  manual encontró; y la rama aterriza en `main` con `meltemi land
  apagado-entero-y-modos confirm`
  <!-- 2026-09-18: `validate` limpio; `verify` 10/10 enlazados; `fmt --check`,
  `clippy --workspace --all-targets -D warnings`, `cargo deny check`
  (advisories, bans, licenses, sources) y `cargo test --workspace
  --no-fail-fast` verdes en este equipo: 1.090 tests, 99 binarios, cero
  fallos. La primera corrida completa **no** estaba verde: el evento
  `agent_process` no tenía glifo en la transcripción de la GUI, y un test de
  paridad lo exigía (corregido como seguimiento de 1.3). Y esa corrida se había
  detenido en el primer binario rojo, así que se repitió con `--no-fail-fast`
  para que ningún binario quedara sin informar. **Los dos escenarios de Windows tienen por evidencia tests
  `#[cfg(windows)]`**: «Un lanzador intermedio no deja huérfanos» (el mecanismo
  en `meltemi-process` y el daemon de punta a punta en
  `e2e_apagado_entero.rs`) y «El daemon termina de golpe y nada le sobrevive»
  (el cierre del handle, que es el mismo comportamiento del kernel que un
  crash). En macOS y Linux el ámbito es inerte por diseño, y esos tests no
  compilan allí en vez de pasar vacíos: la verificación que dan es de Windows,
  y así se declara. Lo que solo un CLI real puede confirmar queda en
  `docs/conformidad-manual.md`, con su procedimiento de dos piernas. -->
