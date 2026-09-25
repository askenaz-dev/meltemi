# Tasks — acceso-remoto-afinado

Toda tarea cuyo gate lanza binarios corre antes `cargo build --workspace`: las
pruebas del arranque cargan el `meltemid` ya construido.

## 1. El arranque solo en el endpoint propio

- [x] 1.1 `connect_or_start` lanza el daemon solo en el endpoint propio
  (`paths::default_endpoint()`, ahora público) o con `MELTEMI_START_HERE`
  (ausente, vacía o `0` es no), y en otro caso rehúsa de inmediato con el
  diagnóstico y los dos remedios; unit tests de la regla y del texto (design
  D1, D2, D3) — gates: suite de `meltemi-client`

- [x] 1.2 Pruebas con el binario real: el bootstrap arranca `meltemid` en un
  endpoint aislado con la bandera, una segunda conexión lo reutiliza con el
  binario apuntado a uno inexistente, y sin la bandera el endpoint explícito se
  rehúsa aunque el binario esté a mano; la CLI arranca bajo demanda con la
  bandera y, sin ella, un endpoint explícito caído sale con `10` y el remedio;
  `tui/tests/unreachable.rs` fija la bandera para seguir probando un fallo de
  arranque; `scripts/capture-desktop.ps1` la fija (design D4, y el Context:
  quién depende del accidente) —
  escenarios «Primer cliente arranca el daemon», «Segunda invocación reutiliza
  la instancia», «Un endpoint ajeno que no responde no arranca nada», «Arrancar
  en un endpoint explícito se pide», «Arranque bajo demanda al no haber
  daemon», «Fallo de arranque se reporta como inalcanzable», «Un endpoint
  explícito caído se reporta sin arrancar nada» — gates: `cargo build
  --workspace`, las pruebas nuevas, `-p meltemi --test unreachable`

- [x] 1.3 (Hallazgo de la revisión propia) La prueba de la GUI «Fallo de
  arranque diagnosticado» (`desktop/tests/bridge.rs`) sigue pasando —un endpoint
  de prueba sin daemon queda inalcanzable con su endpoint en el diagnóstico—,
  pero su comentario contaba con un arranque que ya no se intenta; dice ahora
  por qué no se arranca nada ahí (design D1) — gates: `-p meltemi-desktop
  --test bridge`

## 2. El túnel

- [x] 2.1 `tui/src/tunnel.rs`: el latido y `ExitOnForwardFailure` en el
  comando y en el snippet con su nota, `ssh_args` como única fuente de lo
  impreso y lo ejecutado; `--exec` ejecuta esos argumentos y su final se decide
  en una función pura (estado `0` termina con `0`; otro estado o una señal, con
  `10` y lo que significa); unit tests (design D5, D6) — escenarios «El túnel
  lleva latido y cae si no puede reenviar», «Un túnel que muere se reporta» —
  gates: suite de `meltemi`

## 3. Documentación y smoke

- [x] 3.1 `docs/acceso-remoto.md`: «Windows como PC servido», «Desde el
  teléfono, hoy» (absorbe «Aprobar desde el teléfono, hoy»: verbos, TUI por
  `ssh -t` con 80×24, `MELTEMI_BELL`, lo que no hay) y «Cuánto se espera al
  humano»; `docs/plataformas.md` sin contradecir al helper; la bandera
  documentada junto al aislamiento; tests de docs (design D7) — escenarios «El
  PC Windows servido está documentado con lo no medido», «El teléfono de hoy
  está documentado con sus límites», «Cada espera tiene su ajuste y su valor»,
  «Plataformas y el helper dicen lo mismo» — gates: tests de docs

- [x] 3.2 El smoke `docs/qa/2026-09-25-acceso-por-tunel-smoke.md`: el rehúso
  y el arranque con la bandera medidos con el binario del taller, lo heredado de
  los e2e, y lo pendiente con lo que falta; la frase «funciona todo» pasa a ser
  lo esperado con enlace al smoke; test de docs (design D8) — escenarios «El
  smoke dice qué se midió y qué no», «La documentación no promete lo no
  medido» — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate acceso-remoto-afinado` limpio, `meltemi verify
  acceso-remoto-afinado` completo, gate del workspace (`cargo test --workspace
  --no-fail-fast`, clippy, fmt), el daemon que arranquen `validate`/`verify`/`land`
  detenido, estado de la change en `docs/plan-de-cambios.md`, `meltemi land
  acceso-remoto-afinado` y `cargo clean` del taller
