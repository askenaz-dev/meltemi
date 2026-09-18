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

- [ ] 1.1 Crate `core/meltemi-process` (biblioteca): `Scope` con `new`,
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
- [ ] 1.2 Adaptadores: `supervisor.rs` lanza dentro de un `Scope`,
  `ProcessControl::kill` termina el ámbito, y `kill_on_drop` deja de ser una
  promesa del destructor; `tests/process_lifecycle.rs` gana el caso del
  `.cmd` que ignora el fin de entrada hasta agotar la gracia; la auditoría
  de dependencias sigue verde con el crate nuevo en la clausura (design D1,
  D2, D5) — escenarios «El intermediario del proveedor no deja huérfanos» y
  «El adaptador cae y el proveedor no le sobrevive» — gates: suite de
  `meltemi-adapters`
- [ ] 1.3 Daemon: `acp.rs` llama a `AcpAgent::spawn_process` él mismo,
  adopta el hijo por su `id()`, conecta con `Lines` sobre su stdio,
  replica del crate el colector de `stderr` y el vigilante del hijo
  (citando `acp_agent.rs:280-372` de 1.2.0), y anota pid y programa
  efectivo en el log; test de Windows que lanza `mock-agent` a través de
  un `.cmd` y comprueba que cancelar no deja el nieto; el e2e existente con
  `mock-agent` intacto (design D3, D4, D5) — escenarios «Un lanzador
  intermedio no deja huérfanos», «El daemon termina de golpe y nada le
  sobrevive» y «Cancelación de sesión» re-pineado — gates: suite de
  `meltemid`
- [ ] 1.4 `docs/agentes.md` (sección Windows: qué pasa con un shim y por qué
  ya no importa) y `docs/conformidad-manual.md` (comprobación opt-in contra
  un CLI real instalado por npm: abrir sesión, cancelar, listar procesos)
  (design D5) — gates: `cargo test -p meltemi --test docs`

## 2. Modos anunciados

- [ ] 2.1 `session_config::from_acp(config_options, modes)` con la
  precedencia de D6 y la síntesis de la opción de modo; el daemon recuerda
  por sesión que esa opción es de forma heredada; tests unitarios de las
  tres combinaciones (solo modos, solo opciones, las dos) — escenarios
  «Modos sin opción de configuración se ofrecen como opción» y «Con las dos
  formas gana la opción de configuración» — gates: suite de `meltemid`
- [ ] 2.2 `server.rs`: `session/set-config-option` sobre la opción heredada
  manda `session/set_mode` y fija el valor aceptado; `acp.rs`
  (`forward_update`) reconoce `current_mode_update` y
  `config_option_update`, actualiza lo anunciado y registra
  `ConfigOptionsAnnounced`; `available_commands_update` se ve y no se toma,
  con el motivo en el código (design D7) — escenarios «El modo elegido viaja
  por el verbo de modos», «Un cambio de modo del agente actualiza la opción»
  y «Una lista de opciones nueva reemplaza a la anterior» — gates: suite de
  `meltemid`, e2e con `mock-agent` (que gana un guion con modos y otro con
  `current_mode_update`)
- [ ] 2.3 Verificación manual, opt-in, contra cada agente de nivel 1
  instalado en este equipo: abrir una sesión ACP y anotar qué campo pobló
  —modos, opciones, ninguno— con versión y fecha, en
  `docs/conformidad-manual.md` (design D8). Si todos pueblan opciones, la
  nota del backlog lo dice y la síntesis se queda igual — gates: ninguno
  automático; el resultado se persiste como documento

## 3. Cierre

- [ ] 3.1 `meltemi validate apagado-entero-y-modos` limpio; `meltemi verify`
  con los diez escenarios enlazados (los dos de Windows con el test
  `#[cfg(windows)]` como su evidencia, declarado en la nota de la tarea);
  suite completa, clippy y fmt verdes en este equipo; `cargo deny check`
  verde; entrada en `docs/plan-de-cambios.md` con lo que la verificación
  manual encontró; y la rama aterriza en `main` con `meltemi land
  apagado-entero-y-modos confirm`
