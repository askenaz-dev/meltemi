# Tareas — maquinas-remotas

Vía completa. Un commit atómico por tarea, con referencia
`(maquinas-remotas N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny check`
y la suite del crate tocado. **Esta change se desarrolla en su propio taller
(`meltemi workspace maquinas-remotas`) y aterriza en `main` con `meltemi
land` al cerrar.** Ninguna tarea añade una dependencia, toca `deny.toml`,
añade un método al contrato ni modifica el daemon: si alguna lo necesitara, la
change se detiene y vuelve a design (design D7).

## 1. La lista

- [ ] 1.1 `core/meltemi-client/src/machines.rs`: `[[client.machines]]`
  (`name`, `endpoint`) leída de la configuración **de usuario** por el mismo
  camino que `configured_endpoint` —directorio de configuración y ninguna raíz
  de proyecto—, con `endpoint = "local"` resuelto al default de la plataforma,
  diagnóstico con remedio ante valor inválido, y **rechazo de todo campo de
  material de claves** (`identity-file`, `password`, `passphrase`, `port`,
  `user`) nombrando dónde vive eso (design D1) — escenarios «Las máquinas se
  leen del usuario y nunca de un repositorio» y «La lista no lleva material de
  claves» — gates: suite de `meltemi-client`

## 2. Cambiar de máquina

- [ ] 2.1 `desktop/src/bridge.rs`: el endpoint deja de ser constante del actor
  y pasa a variable del bucle; `BridgeCommand::Switch(String)` lo cambia y
  corta la conexión viva para que el bucle reconecte, con el emisor de avisos y
  el backoff intactos donde están (design D2) — escenario «Cambiar de máquina
  reconecta y lo dice» — gates: suite de `meltemi-desktop`
- [ ] 2.2 `core/meltemi-client/src/machines.rs` + `desktop/src/lib.rs`: probar
  una conexión —abrir, `initialize`, anotar versión y latencia, **cerrar**— con
  su propio límite de tiempo y sin arrancar nada, y el comando Tauri que la
  expone (design D3) — escenario «Probar una conexión no arranca nada» —
  gates: suite de `meltemi-client`

## 3. Las superficies

- [ ] 3.1 CLI: verbo `machines` con su render humano y `--json`/`--yaml`, que
  **lista y no cambia**, diciendo por dónde se cambia; `docs/referencia-cli.md`
  regenerada (design D6) — escenarios «machines imprime la lista y cuál está
  activa» y «Sin máquinas configuradas, dice cómo se configuran» — gates:
  suite de `tui`, parity
- [ ] 3.2 TUI: `machines`/`maquinas` y `machine <nombre>`/`maquina <nombre>` en
  la paleta con sus `Effect`, su `Command` en `conn.rs`, su estado y su render,
  la máquina activa en la cabecera junto al `vía …`, y el rehúso que nombra las
  que sí están; cadenas ES/EN (design D6) — escenarios «Máquinas desde la
  paleta» y «Cambiar de máquina desde la paleta reconecta» — gates: suite de
  `tui`
- [ ] 3.3 GUI: sección «Máquinas» en `Settings.svelte` (lista con endpoint,
  activa marcada, probar con versión y latencia, cambiar con la guarda de
  trabajo sin guardar **reutilizada**, no una nueva) y el nombre de la máquina
  activa en `StatusBar.svelte` junto al `vía …`; presencia **a pedido** cruzada
  por nombre con `identity/status`, sin sondeo y sin inventar cuando no cruza;
  ES/EN (design D4, D6) — escenarios «Ajustes lista las máquinas con su
  endpoint», «Probar una máquina dice si respondió y cuánto tardó», «Cambiar de
  máquina avisa del trabajo sin guardar», «Sin máquinas configuradas el cromo
  no cambia» y «La presencia se muestra a pedido y no es una prueba de
  conexión» — gates: suite de cableado, `npm run check`, lint de i18n

## 4. Cierre

- [ ] 4.1 `docs/acceso-remoto.md`: el selector pasa de nota de fase 3 a sección
  presente, y la nota queda acotada a lo único que sigue siendo nota (los
  certificados SSH del bastión); **reescribir el bloque de fase 3 de
  `the_rendezvous_pattern_is_documented_with_its_boundary` en `tui/tests/docs.rs`**
  conservando sus tres marcadores `// Scenario:`; e2e en
  `core/meltemi-client/tests/` contra dos endpoints de prueba (uno local, uno
  `exec:`) que cubra listar, probar y cambiar; `meltemi validate
  maquinas-remotas` limpio y `meltemi verify` con los dieciséis escenarios
  enlazados (meta: cero marcas manuales, incluidos los tres del bloque
  MODIFIED); suite completa, clippy, fmt y `cargo deny` verdes; smoke con la
  app viva de la sección y del cambio de máquina; entrada en
  `docs/plan-de-cambios.md`; y la rama aterriza en `main` con `meltemi land
  maquinas-remotas confirm`
