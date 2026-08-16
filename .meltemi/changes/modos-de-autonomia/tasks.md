# modos-de-autonomia — tasks

> Vía completa. El motor primero y con su matriz completa de tests, porque es
> donde una esquina mal resuelta se convierte en un permiso concedido que nadie
> pidió. Las superficies después: son la parte que se ve, no la que decide.

## 1. La postura, en el motor

- [x] 1.1 El tipo del modo en el contrato, con los tres nombres y **ninguno
  más**; sin modo declarado no hay composición (design D3) — escenario «Sin
  modo, la resolución es la de siempre»
- [x] 1.2 La composición en tres reglas, en orden: el `deny` del usuario gana
  siempre; lo irreversible y lo fuera del árbol escalan en todo modo; el resto
  lo decide el modo (design D2) — escenarios «El deny del usuario sobrevive a
  cualquier modo» y «Lo irreversible escala aunque el modo sea autónomo»
- [x] 1.3 Manual **retira** lo concedido —no solo deja de conceder—, que es lo
  que un bundle de reglas no podía expresar (design D1) — escenario «Manual
  retira lo que las reglas concederían»
- [x] 1.4 La contención de Semi con la ruta que los hechos ya llevan, y sus tres
  esquinas: ruta ausente, ruta fuera del árbol, y sesión sin worktree (design
  D4) — escenario «Semi concede solo lo contenido»
- [x] 1.5 La matriz completa modo × decisión-de-reglas × clasificación, pineada
  celda a celda: es la clase de tabla en la que un hueco es un permiso regalado
- [x] 1.6 Ningún modo omite el proxy: test que recorre los modos admitidos y lo
  exige (design D6) — escenario «Ningún modo omite el proxy»

## 2. La deuda que hay que pagar antes de montar encima

- [x] 2.1 `allow_meltemi_writes()` se acota de verdad a `.meltemi/` con el
  `path_prefix` que el motor ya tiene, en vez de devolver `allow_all()` (design
  D5), con su test
  <!-- 2026-08-16: la deuda era más honda de lo que decía. Primero, el mock **no
  llevaba la ruta** en su petición, así que cualquier acotación por ruta parecía
  romperlo: se le añadió, porque un agente real sí la lleva y el mock estaba
  sub-modelando el protocolo. Segundo, y el fondo: un prefijo **relativo**
  `.meltemi` no puede acotar una ruta **absoluta**, de modo que acotaba nada y
  toda escritura de autoría escalaba. Ahora el bound es el `.meltemi/` del
  proyecto, absoluto, y sus tres tests de autoría vuelven a pasar concediendo
  solo lo que el nombre prometía. -->

## 3. El contrato y el registro

- [x] 3.1 Campo aditivo de modo en `session/start` y `worktree/dispatch`, con la
  conformidad de tres vías y `gen:forms` commiteado; **no** en `session/direct`
  (design D8)
- [x] 3.2 El arranque registra el modo, y cada decisión de permiso registra bajo
  cuál se tomó — escenarios «El arranque registra el modo» y «Cada decisión dice
  bajo qué modo se tomó»
- [x] 3.3 Un modo desconocido se rehúsa nombrando los válidos, jamás degrada —
  escenario «Un modo desconocido se rehúsa con los válidos»

## 4. Las superficies

- [x] 4.1 GUI: elección en el lanzador y chip del modo junto al compositor —
  escenario «El modo se elige al lanzar y se ve en la sesión»
- [x] 4.2 GUI: con sesión libre sin worktree, semi **no** se presenta como
  contención; se nombra el ámbito real (design D4) — escenario «Semi sin
  worktree dice cuál es su ámbito real»
  <!-- 2026-08-16: la GUI no puede distinguir worktree de proyecto —`SessionInfo`
  no lo dice— pero no le hace falta: **una sesión lanzada desde esta superficie
  es siempre libre**, y una libre no crea worktree. El aviso aplica siempre que
  el modo sea semi, y la razón queda escrita donde se lee. Donde `semi` sí
  significa contención de verdad es en `worktree/dispatch`, que también ganó el
  campo. -->
- [x] 4.3 TUI: elección al arrancar y declaración con símbolo y palabra —
  escenario «El terminal declara el modo de la sesión»
- [x] 4.4 CLI: flag de modo con su ayuda nombrando los admitidos
- [x] 4.5 i18n es/en de todo lo nuevo, con el lint como guardián

## 5. Cierre

- [x] 5.1 E2e contra el mock: una sesión en cada modo, y la misma petición
  resolviéndose distinto — la prueba de que el modo hace algo
- [x] 5.2 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend y paridad revisada
