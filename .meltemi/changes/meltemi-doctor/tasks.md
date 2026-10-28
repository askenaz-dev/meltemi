# Tasks — meltemi-doctor

Vía rápida. Un commit atómico por tarea, con referencia `(meltemi-doctor N.M)`
y sin trailers de co-autoría. Cero dependencias nuevas; cero RPC.

## 1. El informe y el verbo

- [x] 1.1 `meltemi_client::doctor`: hechos locales con tope, respuestas del
  daemon sin arrancarlo, filas con id, estado, hechos y remedio, palabras en
  inglés y en español, texto con glifo y palabra y JSON (design D1–D6) —
  escenario «Cada problema dice su remedio»; gates: tests del módulo
  <!-- 2026-10-28: `Local::gather` (carpetas probadas escribiendo y borrando un
  archivo, `config.toml` parseado con las omisiones de sus máquinas, `git
  --version` y `ssh -V` con tres segundos, `sshd.exe` en Windows, la cola de
  64 KiB del log más nuevo), `ask_daemon` (conecta sin arranque, `initialize`,
  `status` y `fleet/list`, cinco segundos en total) y `compose`. Cada fila trae
  sus palabras en inglés y en español (design D3, ajustado al implementar: una
  sola fuente de frases para las tres superficies). Ocho tests: el agente a
  medias falla con el remedio y el comando de la flota, y toda fila que falla
  trae remedio en los dos idiomas; el daemon caído es una fila y el resto sigue;
  versiones distintas avisan con la salida; símbolos con gemelo ASCII y
  palabra; la carpeta se prueba sin dejar rastro; el log da sus últimas cinco
  líneas de error; un programa ausente es «missing»; una configuración rota
  falla y dice por qué. Clippy `-D warnings` limpio. -->
- [ ] 1.2 `meltemi doctor [--json]`: gramática, ayuda, referencia regenerada,
  salida `0`/`14` (design D7) — escenarios «El diagnóstico funciona sin daemon
  y no arranca ninguno» y «El diagnóstico se lee como JSON»; gates: e2e contra
  el binario y la referencia fresca

## 2. Las superficies

- [ ] 2.1 TUI: `doctor`/`diagnostico` en la paleta, compuesto en el bucle de
  conexión también desconectado, y pintado bajo la flota — escenario «La paleta
  muestra el diagnóstico, también sin daemon»; gates: tests del reductor y del
  render
- [ ] 2.2 GUI: comando de Tauri que compone con las respuestas que la vista pide
  al daemon, sección en Ajustes con las filas en es/en y «Copiar» —
  escenario «Ajustes muestra el diagnóstico y lo copia»; gates: `npm run
  check`, `lint:i18n`, tests del frontend y del escritorio

## 3. Docs y cierre

- [ ] 3.1 La FAQ y la plantilla de error mandan a `doctor` — escenario «Lo
  primero que pide la FAQ es el diagnóstico»; `meltemi validate` y `verify` con
  los seis escenarios; entrada en `docs/plan-de-cambios.md`
