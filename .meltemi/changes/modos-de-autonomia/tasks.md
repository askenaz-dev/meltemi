# modos-de-autonomia — tasks

> Vía completa. El motor primero y con su matriz completa de tests, porque es
> donde una esquina mal resuelta se convierte en un permiso concedido que nadie
> pidió. Las superficies después: son la parte que se ve, no la que decide.

## 1. La postura, en el motor

- [ ] 1.1 El tipo del modo en el contrato, con los tres nombres y **ninguno
  más**; sin modo declarado no hay composición (design D3) — escenario «Sin
  modo, la resolución es la de siempre»
- [ ] 1.2 La composición en tres reglas, en orden: el `deny` del usuario gana
  siempre; lo irreversible y lo fuera del árbol escalan en todo modo; el resto
  lo decide el modo (design D2) — escenarios «El deny del usuario sobrevive a
  cualquier modo» y «Lo irreversible escala aunque el modo sea autónomo»
- [ ] 1.3 Manual **retira** lo concedido —no solo deja de conceder—, que es lo
  que un bundle de reglas no podía expresar (design D1) — escenario «Manual
  retira lo que las reglas concederían»
- [ ] 1.4 La contención de Semi con la ruta que los hechos ya llevan, y sus tres
  esquinas: ruta ausente, ruta fuera del árbol, y sesión sin worktree (design
  D4) — escenario «Semi concede solo lo contenido»
- [ ] 1.5 La matriz completa modo × decisión-de-reglas × clasificación, pineada
  celda a celda: es la clase de tabla en la que un hueco es un permiso regalado
- [ ] 1.6 Ningún modo omite el proxy: test que recorre los modos admitidos y lo
  exige (design D6) — escenario «Ningún modo omite el proxy»

## 2. La deuda que hay que pagar antes de montar encima

- [ ] 2.1 `allow_meltemi_writes()` se acota de verdad a `.meltemi/` con el
  `path_prefix` que el motor ya tiene, en vez de devolver `allow_all()` (design
  D5), con su test

## 3. El contrato y el registro

- [ ] 3.1 Campo aditivo de modo en `session/start` y `worktree/dispatch`, con la
  conformidad de tres vías y `gen:forms` commiteado; **no** en `session/direct`
  (design D8)
- [ ] 3.2 El arranque registra el modo, y cada decisión de permiso registra bajo
  cuál se tomó — escenarios «El arranque registra el modo» y «Cada decisión dice
  bajo qué modo se tomó»
- [ ] 3.3 Un modo desconocido se rehúsa nombrando los válidos, jamás degrada —
  escenario «Un modo desconocido se rehúsa con los válidos»

## 4. Las superficies

- [ ] 4.1 GUI: elección en el lanzador y chip del modo junto al compositor —
  escenario «El modo se elige al lanzar y se ve en la sesión»
- [ ] 4.2 GUI: con sesión libre sin worktree, semi **no** se presenta como
  contención; se nombra el ámbito real (design D4) — escenario «Semi sin
  worktree dice cuál es su ámbito real»
- [ ] 4.3 TUI: elección al arrancar y declaración con símbolo y palabra —
  escenario «El terminal declara el modo de la sesión»
- [ ] 4.4 CLI: flag de modo con su ayuda nombrando los admitidos
- [ ] 4.5 i18n es/en de todo lo nuevo, con el lint como guardián

## 5. Cierre

- [ ] 5.1 E2e contra el mock: una sesión en cada modo, y la misma petición
  resolviéndose distinto — la prueba de que el modo hace algo
- [ ] 5.2 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend y paridad revisada
