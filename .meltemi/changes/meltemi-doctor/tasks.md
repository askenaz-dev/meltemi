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
- [x] 1.2 `meltemi doctor [--json]`: gramática, ayuda, referencia regenerada,
  salida `0`/`14` (design D7) — escenarios «El diagnóstico funciona sin daemon
  y no arranca ninguno» y «El diagnóstico se lee como JSON»; gates: e2e contra
  el binario y la referencia fresca
  <!-- 2026-10-28: `Command::Doctor` en la gramática y la ayuda, despachado sin
  el arranque bajo demanda; el JSON lleva `clean`, así que una fila que falla
  sale con `14` por la misma regla que `validate`. Ejecutado a mano contra un
  endpoint vacío (informe entero, `14`, nada arrancado) y contra el daemon
  aislado. Esa corrida encontró dos defectos de 1.1, corregidos con su test:
  un agente cuya única capa encontrada es un adaptador propio de Meltemi —que
  viaja en toda instalación— salía como instalación a medias en falla, y es un
  agente no instalado; y el remedio repetía un comando que su frase ya
  nombraba. `tui/tests/doctor.rs`: sin daemon (con `MELTEMI_START_HERE=1`
  puesto) el informe sale entero, sale con `14` y no aparece ni socket ni log;
  el JSON trae cada fila con id, estado, hechos y palabras; con argumentos es
  error de uso. Referencia del CLI regenerada (`docs` 20/20). -->

## 2. Las superficies

- [x] 2.1 TUI: `doctor`/`diagnostico` en la paleta, compuesto en el bucle de
  conexión también desconectado, y pintado bajo la flota — escenario «La paleta
  muestra el diagnóstico, también sin daemon»; gates: tests del reductor y del
  render
  <!-- 2026-10-28: `doctor`, `diagnostico` y `diagnóstico` en la paleta llevan a
  la flota con `Effect::Doctor`; el bucle de conexión lo compone en una tarea
  propia (la recogida bloqueante en un hilo aparte), conectado o en la espera
  entre reintentos, y lo manda como `Update::Doctor`; la flota lo pinta con
  las palabras del idioma de la superficie y el glifo o su gemelo. Al escribir
  el test apareció que la flota sin catálogo —el caso sin daemon— no pintaba
  nada más que «cargando», tampoco las máquinas que su propio comentario dice
  que se leen sin conexión: ahora pinta las máquinas y el diagnóstico debajo.
  Tests del reductor (tres palabras) y del render (Unicode y ASCII, sin daemon);
  suite de `meltemi` 349/349, clippy limpio. -->
- [ ] 2.2 GUI: comando de Tauri que compone con las respuestas que la vista pide
  al daemon, sección en Ajustes con las filas en es/en y «Copiar» —
  escenario «Ajustes muestra el diagnóstico y lo copia»; gates: `npm run
  check`, `lint:i18n`, tests del frontend y del escritorio

## 3. Docs y cierre

- [ ] 3.1 La FAQ y la plantilla de error mandan a `doctor` — escenario «Lo
  primero que pide la FAQ es el diagnóstico»; `meltemi validate` y `verify` con
  los seis escenarios; entrada en `docs/plan-de-cambios.md`
