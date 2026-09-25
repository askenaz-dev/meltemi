# Tasks — verbos-del-agent-boss

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. La gramática

- [x] 1.1 `tui/src/cli.rs`: los seis verbos (`permissions`, `approve`, `deny`,
  `watch`, `log`, `cancel`), `--option` como flag del verbo —solo en `approve`
  y `deny`, rehusado en los demás—, exactamente un id por decisión con el
  remedio de las reglas en el rehúso, el pin de `FLAGS`, la ayuda (y el párrafo
  de permisos que dice cómo decidir sin terminal) y `docs/referencia-cli.md`
  regenerada; la taxonomía describe el primer uso del `12`; unit tests (design
  D1, D3, D4) — escenario «Dos ids se rehúsan sin decidir ninguno» — gates:
  suite de `meltemi`, frescura de la referencia

## 2. Decidir

- [x] 2.1 El núcleo puro de decidir (`tui/src/decide.rs`): la opción de cada
  verbo —la aprobación de una vez cuando es una sola, el rechazo del agente o
  cancelar, `--option` restringido a la clase del verbo y a lo que el agente
  ofreció, con sus rehúsos listando las opciones—, el listado con su resumen,
  columnas por contenido, opciones y texto del agente escapado, y la línea de
  la decisión; unit tests (design D2, D8, D10) — escenarios «Varias
  aprobaciones exigen nombrar una», «El listado escapa el texto del agente» —
  gates: suite de `meltemi`

- [x] 2.2 `permissions`, `approve` y `deny` sobre `permission/pending` y
  `permission/decide`, con los códigos `12` y `2` y sus mensajes; e2e
  `tui/tests/permission_verbs.rs` con el daemon en proceso y el simulado: una
  sesión que abre un cliente crudo que no responde el empuje, listada y
  decidida desde la CLI —aprobación simple, rechazo, dos aprobaciones que exigen
  `--option`, opción ajena o de la clase contraria, petición ya decidida,
  vencida, inexistente y `all`— (design D2, D4) — escenarios «permissions
  muestra lo que espera y desde cuándo», «approve elige la aprobación simple del
  agente», «deny elige el rechazo del agente o cancela», «approve rehúsa una
  opción que el agente no ofreció», «Una petición que ya no espera no se
  decide», «Un comodín no es un id» — gates: `cargo build --workspace`, `-p
  meltemi --test permission_verbs`

- [x] 2.3 Remoto sin terminal: el binario real, con stdio en tuberías y sin TTY,
  lista y aprueba una petición de una sesión abierta por otra superficie
  (design D11) — escenario «Aprobar sin terminal desde otra conexión» — gates:
  `-p meltemi --test permission_verbs`

## 3. Seguir, leer y cancelar

- [x] 3.1 `tui/src/follow.rs`: la línea del registro con los escapes de JSON, el
  escritor por formato (JSON Lines, flujo YAML, objeto de error que cierra el
  flujo), `log` (localizar la sesión y paginar `session/log`) y `watch`
  (declarar, localizar, emitir, consultar la vida cada 2 s, cerrar con `10` si
  la conexión cae), con su despacho junto a `bridge` en `lib.rs`; `cancel` en
  `run.rs` (notificación y `status` por la misma conexión); unit tests (design
  D5, D6, D7, D8, D9) — escenarios «Una línea del flujo lleva el texto del
  agente como escape JSON», «Un flujo en YAML es un documento por registro» —
  gates: suite de `meltemi`

- [ ] 3.2 E2e `tui/tests/session_verbs.rs`: una sesión `detach` abierta por un
  cliente crudo, seguida por el binario real sin TTY mientras la CLI aprueba su
  petición y la cancela; `watch` termina con el cierre y `log` guarda lo mismo;
  `watch` y `cancel` de la sesión terminada; ids desconocidos; matar un `watch`
  no cancela; y con la prueba como daemon, el daemon que muere a mitad del flujo
  y la sesión que desaparece sin cierre (design D6, D7, D9, D12) — escenarios
  «watch termina con la sesión», «cancel detiene la sesión como la TUI», «Lo que
  watch emite es lo que log guarda», «Seguir o cancelar una sesión que ya
  terminó no es un error», «Una sesión desconocida es error de contrato», «Dejar
  de seguir no cancela la sesión», «watch no sobrevive a su daemon», «watch no
  sobrevive a la sesión aunque el cierre no llegue», «Un error a mitad del flujo
  cierra con un objeto», «Un flujo emite un objeto por línea», «Seguir sin
  terminal» — gates: `-p meltemi --test session_verbs`

## 4. Documentación

- [ ] 4.1 `docs/acceso-remoto.md` gana «Aprobar desde el teléfono, hoy» (la
  receta, sin túnel, una petición por vez, `watch` para seguir, qué muestra la
  cola y dónde está el resto) con su test de docs; `docs/paridad-nucleo.md`
  nombra los verbos en las cinco filas; el pin del escenario restablecido
  «Éxito en JSON emite un objeto» (design D10, D11) — escenario «La receta de
  aprobar desde el teléfono está documentada» — gates: tests de docs, paridad

## 5. Cierre

- [ ] 5.1 `meltemi validate verbos-del-agent-boss` limpio, `meltemi verify
  verbos-del-agent-boss` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  verbos-del-agent-boss` y `cargo clean` del taller
