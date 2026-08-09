# Tareas — sidebar-ajustable-y-pestanas

Vía completa. Un commit atómico por tarea, con referencia
`(sidebar-ajustable-y-pestanas N.M)` y sin trailers de co-autoría. Gates del
repo en cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la
suite del crate tocado. Los bloques 1 y 2 son inseparables (design D1); el
bloque 3 es el punto de corte declarado si el alcance se demuestra excesivo.

## 1. El reparto que el usuario decide

- [x] 1.1 `desktop/ui/src/lib/nav-split.ts` (módulo puro, cabecera SPDX):
  `MIN_NAV_PX`, `MIN_TREE_PX`, `STEP_PX`, `clampNavHeight` y `stepNavHeight`,
  con el caso apretado resuelto a favor de la navegación; `desktop/ui/tests/
  nav-split.test.ts` con `node --test` cubriendo suelo, techo, ventana apretada
  y aritmética del paso (design D3) — escenario «El reparto tiene suelo por los
  dos lados» — gates: `npm test`
- [x] 1.2 El separador en `Sidebar.svelte`: elemento propio con
  `role="separator"`, `tabindex="0"`, nombre accesible, `aria-orientation`,
  `aria-valuenow`/`min`/`max` y `aria-controls` sobre la navegación; arrastre
  con captura de puntero; ArrowUp/ArrowDown/Home/End; el hairline se muda de
  `.section` al control; `nav` gana `min-height: 0` y `overflow-y: auto`; el
  separador se retira con el árbol en el riel; strings ES/EN (design D2) —
  escenarios «Arrastrar la línea reparte el alto», «El reparto se ajusta con el
  teclado», «Plegada la barra, no hay reparto que hacer» y «Ninguna entrada se
  pierde al encoger la navegación» — gates: suite de cableado
- [ ] 1.3 Persistencia: `navSplit` en `ui-state.ts` con `setNavSplit`, escritura
  al soltar y no durante el arrastre; `nav_split: Option<u32>` tras
  `#[serde(default)]` en `desktop/src/uistate.rs` con su test de defaults
  extendido; reajuste contra la ventana que existe, con la desigualdad que lo
  hace converger (design D3) — escenarios «El reparto se recuerda, el primer
  arranque no» y «Una ventana más pequeña no deja el reparto inservible» —
  gates: `cargo test -p meltemi-desktop`

## 2. La barra de desplazamiento que no se come la columna

- [ ] 2.1 `desktop/ui/src/app.css`: `scrollbar-width: thin` y
  `scrollbar-color: var(--text-faint) transparent` dentro de `:root`, con el
  comentario que justifica propiedades estándar frente a los selectores de un
  solo motor; test que pinea el par, que aparece una sola vez y que la
  superficie no usa selectores específicos de motor (design D4) — escenarios
  «El árbol de proyectos desplaza sin comerse la columna» y «La barra sigue el
  tema sin una segunda declaración» — gates: `cargo test -p meltemi-desktop`

## 3. Varias sesiones a la vez

- [ ] 3.1 `desktop/ui/src/lib/session-tabs.ts` (módulo puro, cabecera SPDX):
  `SessionTab`, `MAX_SESSION_TABS`, `openTab` (abrir-o-enfocar, rehúsa al tope
  sin desalojar), `closeTab` (cae en la vecina de la izquierda), `markUnread` y
  `clearUnread`; `desktop/ui/tests/session-tabs.test.ts` con `node --test`
  cubriendo las tres ramas de cierre, el duplicado y el tope (design D6) —
  escenarios «Abrir dos veces la misma sesión enfoca, no duplica», «Cerrar la
  pestaña activa cae en la vecina» y «El tope se rehúsa nombrando el remedio» —
  gates: `npm test`
- [ ] 3.2 `TabStrip.svelte` genérico con el patrón ARIA completo
  (`tablist`/`tab`/`tabpanel`, `tabindex` rotatorio, flechas con ciclo, Home,
  End, Delete, foco además de selección) y la enmienda de la guardia de
  `tabindex`: el detector se ensancha para ver la retirada dinámica del orden de
  tabulación y se abre la excepción estrecha para `role="tab"`, con el test
  positivo que la cobra (design D8) — escenario «La tira se recorre entera con
  el teclado» — gates: suite de cableado
- [ ] 3.3 `SessionTabs.svelte`: la lista como primera pestaña no cerrable, cada
  sesión resuelta contra el listado completo, estado con símbolo y palabra,
  contador de no leídos con nombre accesible; strings ES/EN (design D5) —
  escenarios «La lista es la primera pestaña y nunca se cierra» y «El estado de
  cada pestaña se lee sin color» — gates: suite de cableado
- [ ] 3.4 `App.svelte`: `openSessions` + `activeSession` en lugar de la única
  sesión visible, la rama suelta del detalle plegada dentro de la de Sesiones,
  los cuatro puntos de entrada por una sola función, Esc devolviendo a la lista
  sin cerrar nada, y las pestañas sobreviviendo a navegar a otra vista sin
  persistirse al reiniciar (design D5, D7) — escenarios «Abrir una segunda
  sesión no reemplaza la primera» y «Cambiar de vista no cierra las pestañas;
  reiniciar sí las olvida» — gates: suite de cableado
- [ ] 3.5 `SessionDetail.svelte`: props `active` y `onActivity`; aviso de
  actividad cuando no está en pantalla; reanclado al pie al volver al frente
  (un subárbol oculto reporta alto 0); y la guarda del autoenfoque sin la cual
  cada panel de fondo se marca como enfocado desde donde `.focus()` no hace
  nada y esa pestaña **nunca** enfoca su compositor; la suscripción por sesión
  **no** se mueve (design D6) — escenarios «Una pestaña de fondo conserva su
  lectura y su borrador», «La pestaña de fondo dice que llegó algo» y «Cada
  sesión abierta lee su propio registro y su propio flujo» — gates: suite de
  cableado

## 4. Cierre

- [ ] 4.1 `meltemi validate sidebar-ajustable-y-pestanas` limpio y
  `meltemi verify` con los veinte escenarios enlazados (meta: cero marcas
  manuales); suite completa, clippy y fmt verdes; smoke visual conducido sobre
  el binario de release con captura —arrastre y teclado del separador, la barra
  angosta, y cuatro sesiones en pestañas con su borrador intacto al volver—,
  reportando además el consumo con una, cuatro y ocho pestañas contra el
  presupuesto en reposo; nota de QA en `docs/qa/` y entrada en
  `docs/plan-de-cambios.md` con los dos seguimientos nombrados
