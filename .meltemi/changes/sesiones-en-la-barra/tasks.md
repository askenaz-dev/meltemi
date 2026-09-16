# Tareas — sesiones-en-la-barra

Vía completa. Un commit atómico por tarea, con referencia
`(sesiones-en-la-barra N.M)` y sin trailers de co-autoría. Gates del repo en
cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, la suite del
crate tocado, y en `desktop/ui` `npm run check` + `npm test` + el lint de
i18n. **Esta change se desarrolla en su propio taller
(`meltemi workspace sesiones-en-la-barra`) y aterriza en `main` con
`meltemi land` al cerrar.** Nada toca el daemon ni el contrato: no hay deber
de paridad §4, y la TUI entra por paridad de superficies.

## 1. La partición

- [x] 1.1 `desktop/ui/src/lib/tree.ts`: `bucketSessions` — la tabla de cuatro
  cubetas en orden de señal, pura y con test unitario en
  `desktop/ui/tests/tree.test.ts`; la cubeta de detenidas acotada a las más
  recientes con la cuenta total; claves `nav.bucket.*` ES/EN nuevas en
  `messages.ts` (la barra de estado tiene tres con `{n}` incrustado, no sirven)
  (design D1) — escenarios «Cuatro estados, cuatro cubetas en orden de señal» y
  «Una cubeta vacía no ocupa sitio» — gates: `npm test`
  <!-- 2026-09-15: `BUCKET_OF` es un `Record` sobre la unión, como `LIVE_STATE`:
  un estado nuevo del contrato es error de compilación **aquí**, una vez, en vez
  de caerse de todas las cubetas y desaparecer de la barra sin que nada lo note.
  Y la cubeta de detenidas **ordena ella misma** por fecha antes de acotar: su
  tope promete «las más recientes», y una promesa que depende de que el llamador
  haya ordenado se rompe el día que alguien la llama directo. Las cubetas vivas
  conservan el orden que reciben y no se acotan — lo que se acumula para siempre
  es lo terminado, y es lo único que empujaría el árbol fuera de pantalla. -->

## 2. La barra

- [x] 2.1 `Sidebar.svelte`: las cubetas dentro de cada proyecto (cabecera con
  glifo, palabra y cuenta; filas con avatar, título, pastilla de suscripción y
  marca de abierta; «Ver las {n}» hacia Sesiones), `stateGlyph` alineado con la
  tabla del design system (● para `waiting_permission`), sin una sola palabra
  de animación en el archivo (design D1, D7) — escenarios «La fila dice el
  título y si está abierta», «Las detenidas no desbordan la barra» y «Un cambio
  de estado salta de cubeta sin animarse»; conserva el pin vivo «Dos
  suscripciones del mismo agente distinguibles»
  (`scenarios_multiproyecto.rs:285`) — gates: suite de cableado
  <!-- 2026-09-15: **el design se equivocó en un hecho**. Daba por existente una
  guardia que prohíbe toda palabra de animación en `Sidebar.svelte`
  (`scenarios_multiproyecto.rs:328`); ese archivo ya no existe —se fundió en
  `scenarios_shell.rs`— y la prohibición **solo cubría la bandeja**, no la
  barra. Así que la guardia se escribe aquí, que es justo lo que el escenario
  «Un cambio de estado salta de cubeta sin animarse» pedía: ahora la barra
  contiene la cubeta donde aterriza un permiso, y una fila que se deslizara
  sería movimiento bajo el cursor mientras se decide.
  El glifo de `waiting_permission` pasa de `‖` a `●`: la barra era la única
  superficie que lo escribía a su manera, y el test lee **las dos** fuentes
  —`docs/ux/design-system.md` y la función— en vez de fiarse de una lista
  escrita en el test. -->
- [x] 2.2 La sección «Abiertas (n)»: `Sidebar.svelte` recibe `openSessions` y
  `activeSession`, lista las pestañas en orden de tira, selecciona y cierra
  por el mismo `closeTab`, marca la actual por forma y palabra, `Delete`
  cierra la fila enfocada; plegada, la sección se va con el árbol (design D3)
  — escenarios «Seleccionar en la barra trae la pestaña al frente», «Cerrar
  desde la barra cae en la vecina», «Cerrar no es olvidar» y «Plegada, la tira
  sigue siendo el camino» — gates: suite de cableado
  <!-- 2026-09-15: las filas se derivan de `openSessions`, **no** del listado de
  sesiones: una pestaña cuya sesión el listado todavía no alcanzó sigue abierta,
  y dejarla sin fila haría que la barra y la tira discreparan sobre qué existe.
  El test comprueba «Cerrar no es olvidar» **por el lado negativo**: lee el
  cuerpo de `closeSessionTab` y exige que no contenga ningún verbo de terminar
  sesión — así el escenario no depende de que alguien se acuerde. -->

## 3. Las pestañas

- [x] 3.1 La lista deja de ser pestaña: `SessionTabs.svelte` y
  `session-tabs.ts` sin el centinela `__list__`; `App.svelte` mantiene
  `activeSession === null` como «la tabla en pantalla»; los pines de
  `desktop/tests/scenarios_shell.rs` que fijaban `id: LIST` + `closable: false`
  (:1419) y los paneles ocultos con clave (:1341) se reescriben nombrando el
  escenario nuevo, y se añade el pin del renombre —tras el archivo,
  `.meltemi/specs/gui-shell/spec.md` contiene «La lista es la vista, no una
  pestaña» y no «La lista es la primera pestaña y nunca se cierra»— porque la
  guardia `migration.rs` no cubre `gui-shell` (design D2) — escenario «La lista
  es la vista, no una pestaña» y los diez restantes del bloque MODIFIED, ya
  cubiertos, re-pineados — gates: suite de cableado
  <!-- 2026-09-15: quitar el centinela dejó al descubierto algo que el design no
  había mirado: `TabStrip` ponía `tabindex=0` **solo** en la pestaña
  seleccionada, y con la lista en pantalla ninguna lo estaría — un tablist sin
  nada en el orden de tabulación, o sea sin manera de entrar con el teclado.
  Ahora el tabindex sigue a un `focusIndex` que cae a la primera pestaña cuando
  no hay selección: ARIA pide exactamente una pestaña tabulable, nunca que una
  esté seleccionada. El panel de la lista deja además de declararse `tabpanel`
  y de decir que lo etiqueta `tab-__list__`: sin la pestaña, eso era una promesa
  al lector de pantalla que ya no sostenía nada. Y la clave `sessions.tabs.list`
  —«Lista»— se borra del catálogo. -->
- [x] 3.2 La sesión nueva como pestaña: pestaña `__new__` con el compositor;
  `openComposer` abre-o-enfoca sin cambiar de vista y da el foco; al recibir
  `session_started` la pestaña se convierte en la de la sesión; la llegada
  sin última vista es esa pestaña; cerrarla con borrador pide decisión con
  `ConfirmDialog`; `home` sigue siendo el `ViewId` de la puerta (design D4) —
  escenarios «Pedir una sesión nueva abre su pestaña y da el foco», «Pedirla
  de nuevo enfoca, no duplica», «Enviar convierte la pestaña en la sesión»,
  «Llegar es llegar a la pestaña nueva» y «Cerrar con borrador pide decisión»
  — gates: suite de cableado
  <!-- 2026-09-15: la adopción es un reductor puro (`adoptTab`) con su test, no
  una asignación en el shell, y devuelve `null` cuando no hay compositor del que
  adoptar — una sesión puede nacer desde donde nunca hubo uno, y ese `null` es
  lo que manda al llamador de vuelta a la puerta única. Dos detalles que el
  design no había decidido y que el código obligó a decidir: (1) si la sesión
  arranca mientras lees **otra** pestaña, la pestaña nace igual pero el frente
  no se mueve — llevarte sería que la superficie decidiera dónde miras; (2) el
  compositor solo toma el cursor mientras **es** la pestaña al frente, porque el
  panel queda montado detrás de otra y un campo oculto que roba el foco escribe
  donde nadie ve. -->

## 4. Los acordes

- [x] 4.1 `Home.svelte`, `SessionDetail.svelte`, `Palette.svelte`: guardia
  `!event.shiftKey` en los tres `Ctrl+Enter`; `Ctrl+Shift+Enter` encola
  (`session/direct` sin `interrupt`); `Ctrl+Enter` con turno en vuelo y texto
  releva (`interrupt: true`), y con el compositor vacío no ofrece nada; el
  botón primario se llama «Interrumpir y enviar» y «Encolar» está a la vista
  con turno en vuelo, ambos con `kbd`; `help.keys` los nombra; ES/EN (design
  D5) — escenarios «Ctrl+Enter despacha cuando la sesión espera»,
  «Ctrl+Shift+Enter encola y nunca interrumpe», «Ctrl+Enter mientras trabaja
  avisa antes y releva», «Con el compositor vacío no hay nada que relevar»,
  «Los acordes viejos no se pisan» y «Enter sigue siendo salto de línea» —
  gates: suite de cableado
  <!-- 2026-09-15: la condición vive en un `$derived` con nombre (`relays`) y
  **las teclas y las etiquetas la leen de ahí las dos**: separadas, tarde o
  temprano una diría una cosa y la otra haría otra. El acorde que encola se
  comprueba primero, porque un acorde que es superconjunto de otro tiene que
  comprobarse antes. Y con turno en vuelo ningún control se llama «Enviar» —el
  par es «Encolar» y «Interrumpir y enviar»—, que es exactamente lo que mantiene
  cierto palabra por palabra el «Enviar no interrumpe» de
  `conversational-session`; el test lo comprueba por el lado negativo.
  La paleta gana `!event.shiftKey`: si no, el acorde de encolar significaría dos
  cosas distintas según dónde esté el foco. -->

## 5. El terminal

- [x] 5.1 `tui/src/shell/render.rs`: cabeceras de cubeta dentro de cada
  proyecto con la misma tabla de D1 (glifo con gemelo ASCII, palabra, cuenta),
  cursor intacto; cadenas ES/EN en `tui/src/shell/messages.rs`; test de paridad
  de la tabla TS↔Rust que falle si divergen orden o estados; el pie de página
  del `direct` sigue nombrando `Tab`/`Enter` (design D1, D6) — escenarios
  «Cubetas dentro del proyecto en el terminal», «El cursor no se entera de las
  cubetas» y «Enviar y encolar con las teclas del terminal» — gates: suite de
  `tui`
  <!-- 2026-09-15: la tabla vive en `tui/src/shell/buckets.rs` con un `match`
  **exhaustivo** a propósito: un estado nuevo del contrato no compila hasta que
  tenga cubeta, que es la misma garantía que el `Record` sobre la unión le da al
  escritorio. El pin de paridad (`tui/tests/parity.rs`) lee **los dos archivos**
  y compara orden y estados; comprobé que muerde cambiando una rama a mano —
  falla nombrando el estado y las dos respuestas. El escenario «Enviar y encolar
  con las teclas del terminal» ya estaba servido y **no se toca**: el gemelo del
  terminal es `Tab` (alterna relevo) + `Enter`, y la spec viva prohíbe depender
  de combinaciones Ctrl que el TTY captura. -->

## 6. Cierre

- [ ] 6.1 `docs/ux/design-system.md`: dos frases en «Shell architecture», una
  en «Focus and keyboard» y dos filas en «Status vocabulary»
  (`waiting_instruction` ❯ / `>`; `interrupted` ■ / `x`, detenida), con change
  y fecha; `meltemi validate sesiones-en-la-barra` limpio y `meltemi verify`
  con los treinta y cuatro escenarios enlazados (meta: cero marcas manuales); smoke conducido sobre el
  binario release con captura en `docs/qa/`; suite completa, clippy y fmt
  verdes; entrada en `docs/plan-de-cambios.md`; y la rama aterriza en `main`
  con `meltemi land sesiones-en-la-barra confirm`
