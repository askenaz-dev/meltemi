# Tareas — piel-de-pestanas

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(piel-de-pestanas N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea que toque `desktop/ui`: `npm run check`, `npm run lint:i18n`,
`npm test` y `npm run build` en `desktop/ui`, más la suite de cableado del
crate tocado si lo hubiera.

## 1. Las medidas, antes que la piel

- [x] 1.1 `TabStrip.svelte` publica como propiedades personalizadas las medidas
  que `tab-strip.ts` declara, de modo que el CSS deje de repetir `96px` y
  `240px` a mano (design D8) — escenario «La hoja de estilo no repite lo que el
  módulo declara» — gates: `npm test`, `npm run check`
  <!-- 2026-08-09: el test `the_strips_measurements_have_a_single_owner` no
  comprueba dos números concretos: **deriva** de `tab-strip.ts` todo valor
  `*_PX` declarado y exige que ninguno aparezca literal en la hoja de estilo,
  así que las medidas de la piel que lleguen en 2.x quedan cubiertas por
  construcción. Nacen con su tarea y no aquí, para no repetir el defecto que
  esta tarea corrige: una constante declarada sin uso es la mitad de una
  duplicación. Hubo que enmendar la aserción vecina de `pestanas-como-chrome`,
  que exigía `min-width: 96px` **literal** —es decir, exigía la duplicación—;
  su escenario sigue cubierto, ahora a través del elemento. -->
- [ ] 1.2 Las medidas nuevas de la piel (alto de pestaña, radio superior, ancho
  de la franja de grupo) nacen en `tab-strip.ts` con el mismo trato, cada una
  en la tarea que la usa (design D8) — mismo escenario, cubierto por derivación

## 2. La anatomía

- [ ] 2.1 La tira gana su capa (`background`), las inactivas se vuelven
  transparentes y la activa toma la superficie del panel; se retira
  `border-color: var(--accent)` y la activa se une al panel con las curvas de
  pie por pseudo-elemento y `radial-gradient` (design D1, D2) — escenario «La
  activa se distingue por su forma»
- [ ] 2.2 La activa conserva marca bajo `forced-colors`, con el anillo de foco
  global y `aria-selected` intactos (design D2) — escenario «La selección
  sobrevive a la sustitución de colores»
- [ ] 2.3 Separador hairline entre inactivas contiguas por pseudo-elemento,
  oculto junto a la activa y junto a la pestaña bajo el puntero, sin desplazar
  nada al ocultarse (design D3) — escenario «Las inactivas comparten silueta»

## 3. El puntero, el cierre y el ancho

- [ ] 3.1 Relleno de hover en las inactivas, puesto en el contenedor porque el
  botón interno no tiene borde que la regla global pueda alcanzar (design D4) —
  escenario «La inactiva responde al puntero»
- [ ] 3.2 La X se revela con puntero y con foco, permanece en la activa y
  reserva su hueco siempre; `Delete` sigue cerrando (design D5) — escenario «El
  cierre se revela sin perder el camino de teclado»
- [ ] 3.3 El estado se comprime a su glifo dentro de la pestaña y su palabra
  pasa al nombre accesible y al emergente, en `SessionTabs.svelte` (design D6)
  — escenario «El estado no gasta ancho en repetirse» — gates: `npm run
  lint:i18n`
- [ ] 3.4 Franja del color del grupo en cada pestaña miembro, con el nombre del
  grupo intacto en el nombre accesible (design D7) — escenario «La pertenencia
  se ve en la pestaña, no solo en la etiqueta»

## 4. Cierre

- [ ] 4.1 `meltemi validate piel-de-pestanas` limpio y `meltemi verify` con los
  ocho escenarios enlazados (meta: cero marcas manuales); gates del frontend
  verdes y `cargo clippy`/`fmt` limpios
- [ ] 4.2 Smoke visual CDP sobre el **binario de release** con capturas y
  medidas: los dos temas, `forced-colors` en la pasada, la activa unida al
  panel, el hover de una inactiva, la X revelándose por foco, la franja de
  grupo y el contraste real entre la capa de la tira y el panel (design D1: si
  no separa, se aplica la palanca declarada y se anota). Nota en
  `docs/qa/2026-08-XX-piel-de-pestanas-smoke.md`
