# Tareas — rama-por-change

Vía completa. Un commit atómico por tarea, con referencia
`(rama-por-change N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite del crate
tocado. **Esta change se desarrolla en su propia rama (`rama-por-change`, el
flujo que ella misma hospeda) y aterriza en `main` al cerrar.**

## 1. El contrato

- [x] 1.1 `proto/meltemi-proto`: métodos `change/workspace` y `change/land`
  con sus params/results (`ChangeWorkspaceParams` con `branch` y `unique`
  opcionales y excluyentes; `ChangeWorkspaceResult` con ruta, rama y si fue
  reencuentro; `ChangeLandParams` con `confirm` y `branch` opcional;
  `ChangeLandResult` con previsualización de commits y archivos y el resultado
  del merge), schemas JSON y los tres casos de conformidad por campo opcional
  (design D4, D5) — gates: `cargo test -p meltemi-proto`

## 2. El daemon

- [x] 2.1 `change/workspace` en `core/meltemid`: rama con el nombre de la
  change desde la punta de la rama por defecto (detectada, no asumida),
  worktree en `.meltemi/worktrees/<change>/workspace`, registro append-only,
  idempotencia con `reencuentro` declarado, rehúso ante rama homónima ajena
  (solo en el camino implícito: nombrarla es consentir), la rama elegida con
  `branch`, el taller único con sufijo, y exclusión de la raíz gestionada vía
  `.git/info/exclude` (design D2, D3, D4) — escenarios «El primer taller se
  crea desde la rama por defecto», «Pedirlo de nuevo reencuentra, no falla»,
  «El taller sobre una rama elegida», «Un taller único no colisiona con
  nadie», «La rama ajena se rehúsa sin tocarse» y «El taller no ensucia el
  estado del árbol principal» — gates: suite del crate
- [x] 2.2 `change/land`: previsualización sin `confirm` (commits y archivos);
  con `confirm`, merge `--no-ff` a la rama por defecto; rehúsos con remedio
  ante taller sucio y ante conflictos, con `merge --abort` inmediato que deja
  la rama por defecto intacta (design D5) — escenarios «Sin confirmación, la
  previsualización», «Con confirmación, el aterrizaje limpio», «El conflicto
  se rehúsa y no deja el árbol a medias» y «El taller sucio no aterriza» —
  gates: suite del crate
- [x] 2.3 Retiro protegido: quitar un taller con commits que la rama por
  defecto no alcanza exige confirmación diciendo cuántos quedarían solo en la
  rama; el retiro nunca borra la rama (design D6) — escenarios «Retirar con
  commits sin aterrizar exige confirmación» y «Retirar el taller conserva la
  rama» — gates: suite del crate

## 3. Las superficies

- [x] 3.1 CLI: verbos `workspace <change> [--branch <rama>|--unique]` y `land <change> [--branch <rama>] [confirm]` con su
  render humano (la previsualización legible; el reencuentro dicho con
  palabras); paleta TUI; `registry.ts` GUI + `npm run gen:forms`; dos filas en
  `docs/paridad-nucleo.md`; `docs/referencia-cli.md` regenerada (design D7,
  gate bloqueante `tui/tests/parity.rs`) — gates: suite de `tui`, parity,
  `check:forms`

## 4. Cierre

- [x] 4.1 E2e en `core/meltemid/tests/` contra repos fixture temporales
  (incluido uno cuya rama por defecto no es `main`): ciclo completo
  workspace → commits → land con previsualización y con confirmación, el
  conflicto abortado, y el retiro protegido; `meltemi validate rama-por-change`
  limpio y `meltemi verify` con los doce escenarios enlazados (meta: cero
  marcas manuales); suite completa, clippy y fmt verdes; entrada en
  `docs/plan-de-cambios.md`; y el cierre practica lo que predica: esta rama
  aterriza en `main` con el flujo recién construido si ya funciona, o con git a
  mano documentándolo
