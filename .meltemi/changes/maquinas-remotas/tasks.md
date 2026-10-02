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

- [x] 1.1 `core/meltemi-client/src/machines.rs`: `[[client.machines]]`
  (`name`, `endpoint`) leída de la configuración **de usuario** por el mismo
  camino que `configured_endpoint` —directorio de configuración y ninguna raíz
  de proyecto—, con `endpoint = "local"` resuelto al default de la plataforma,
  diagnóstico con remedio ante valor inválido, y **rechazo de todo campo de
  material de claves** (`identity-file`, `password`, `passphrase`, `port`,
  `user`) nombrando dónde vive eso (design D1) — escenarios «Las máquinas se
  leen del usuario y nunca de un repositorio» y «La lista no lleva material de
  claves» — gates: suite de `meltemi-client`
  <!-- 2026-10-02: el rechazo de material de claves se escribió **ruidoso y no
  silencioso**, que es lo contrario de lo que primero parece prudente: si
  `identity-file` se ignorara, el usuario creería que su clave está en uso y no
  tendría forma de ver por qué no pasa nada. Se rehúsa la máquina entera y se
  nombra dónde vive ese dato (`~/.ssh/config`, el agente de claves). Siete campos
  en la lista, comparados sin distinguir mayúsculas, porque `IdentityFile` es
  como lo escribe `ssh`.
  `endpoint = "local"` es azúcar con razón: un named pipe con el SID del usuario
  dentro no es algo para reescribir a mano, y el primero que lo teclee mal tendrá
  un cliente que habla con nada en silencio.
  La garantía de ámbito es **estructural**, y el test la fija como tal: `read`
  toma un directorio de configuración y ningún argumento más, así que no existe
  parámetro por el que una raíz de proyecto pueda llegar. Es la misma forma que
  `configured_endpoint`, y es lo que tendría que cambiar para romper la regla.
  La prueba de conexión **no pasa por `bootstrap`**: no es que `may_start` ya
  rehúse los `exec:` —que lo hace—, es que probar no debe poder arrancar nada ni
  por accidente, así que conecta directo. Probar una máquina apagada **falla**, y
  eso es la respuesta correcta.
  Dos roturas deliberadas, dos mordidas: vaciar la lista de campos rehusados y
  quitar la resolución de `local`.
  Gates: fmt, clippy de `meltemi-client` en 0, 51 tests del crate verdes. -->

## 2. Cambiar de máquina

- [x] 2.1 `desktop/src/bridge.rs`: el endpoint deja de ser constante del actor
  y pasa a variable del bucle; `BridgeCommand::Switch(String)` lo cambia y
  corta la conexión viva para que el bucle reconecte, con el emisor de avisos y
  el backoff intactos donde están (design D2) — escenario «Cambiar de máquina
  reconecta y lo dice» — gates: suite de `meltemi-desktop`
- [x] 2.2 `core/meltemi-client/src/machines.rs` + `desktop/src/lib.rs`: probar
  una conexión —abrir, `initialize`, anotar versión y latencia, **cerrar**— con
  su propio límite de tiempo y sin arrancar nada, y el comando Tauri que la
  expone (design D3) — escenario «Probar una conexión no arranca nada» —
  gates: suite de `meltemi-client`
  <!-- 2026-10-02: el actor ya sabía reconectar, así que cambiar de máquina es
  **usar lo que hay** y no construir nada: el endpoint deja de ser constante
  capturada y pasa a variable del bucle. Dos sitios lo atienden y los dos
  importaban. En conexión viva, se **suelta** en vez de drenar: lo que está en
  vuelo pertenece a la máquina que se deja, y una respuesta suya llegando después
  del cambio sería una respuesta sobre el ordenador equivocado. Durante el
  backoff —el caso que un usuario sí pisa: la máquina actual caída, el cliente
  reintentando, y él eligiendo otra— el cambio **corta la espera y reinicia el
  retardo**, porque hacerle cumplir el castigo de la mala suerte de la máquina
  anterior es el cliente penalizándolo por su propio calendario. `drain_or_stop`
  pasó de `bool` a un enum de tres, que es lo que siempre fue.
  La máquina activa se guarda en el proceso nativo y no se le pregunta al puente:
  tiene que poder responderse **entre conexiones**, que es justo cuando alguien
  abre la lista a mirar.
  Dos roturas deliberadas, dos mordidas, una por cada sitio. Y un detalle que el
  grep cazó: el literal del pipe muerto había perdido una barra al escribirlo
  desde Python (`\.\pipe` en vez de `\.\pipe`), con lo que el test pasaba por
  la razón equivocada —ruta malformada en vez de pipe sin nadie al otro lado—.
  Gates: fmt, clippy del workspace 0, los 5 tests del puente verdes. -->

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
