# Tasks — meltemi-doctor

Vía rápida. Un commit atómico por tarea, con referencia `(meltemi-doctor N.M)`
y sin trailers de co-autoría. Cero dependencias nuevas; cero RPC.

## 1. El informe y el verbo

- [ ] 1.1 `meltemi_client::doctor`: hechos locales con tope, respuestas del
  daemon sin arrancarlo, filas con id, estado, hechos y remedio, palabras en
  inglés y en español, texto con glifo y palabra y JSON (design D1–D6) —
  escenario «Cada problema dice su remedio»; gates: tests del módulo
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
