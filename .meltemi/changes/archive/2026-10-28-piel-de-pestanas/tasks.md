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
- [x] 1.2 Las medidas nuevas de la piel (alto de pestaña, radio superior, ancho
  de la franja de grupo) nacen en `tab-strip.ts` con el mismo trato, cada una
  en la tarea que la usa (design D8) — mismo escenario, cubierto por derivación
  <!-- 2026-08-09: `TAB_JOIN_PX` entró con 2.1, que es quien la usa. Las
  restantes llegan con 2.3 y 3.4. -->

## 2. La anatomía

- [x] 2.1 La tira gana su capa (`background`), las inactivas se vuelven
  transparentes y la activa toma la superficie del panel; se retira
  `border-color: var(--accent)` y la activa se une al panel con las curvas de
  pie por pseudo-elemento y `radial-gradient` (design D1, D2) — escenario «La
  activa se distingue por su forma»
  <!-- 2026-08-09: el test lee las **declaraciones**, no la prosa: la primera
  versión se cazó a sí misma, porque el comentario que explica qué evita la
  regla («no mask-composite, no @property») hacía fallar la aserción que
  buscaba esos nombres en el texto del bloque. Se retiran los comentarios antes
  de comprobar; el comentario útil se queda donde el próximo lector lo
  necesita. -->
- [x] 2.1b El contraste real entre la capa de la tira y el panel se mide en el
  smoke (4.2): `--surface` y `--surface-2` están a un paso de tono en el tema
  oscuro y el design D1 ya declaró la palanca si no separan (hairline o la
  sombra de un nivel, jamás un color fuera de los tokens)
- [x] 2.2 La activa conserva marca bajo `forced-colors`, con el anillo de foco
  global y `aria-selected` intactos (design D2) — escenario «La selección
  sobrevive a la sustitución de colores»
  <!-- 2026-08-09: `forced-colors` retira los rellenos y con ellos la costura,
  así que la selección cae a un borde en color de sistema (`Highlight`). Los
  otros dos portadores —`aria-selected` y el anillo `--focus`— son
  independientes de la piel y el test los exige por separado, para que un
  rediseño futuro no se lleve los tres de una vez. -->
- [x] 2.2b Comprobar la rama `forced-colors` en el smoke (4.2): un test de
  fuente prueba que la regla existe, no que el sistema la aplique como se
  espera
- [x] 2.3 Separador hairline entre inactivas contiguas por pseudo-elemento,
  oculto junto a la activa y junto a la pestaña bajo el puntero, sin desplazar
  nada al ocultarse (design D3) — escenario «Las inactivas comparten silueta»
  <!-- 2026-08-09: el separador vive en `::before` de las **inactivas**, que es
  el pseudo-elemento que la activa dedica a su costura: selectores disjuntos,
  sin colisión. Se apaga por color y nunca por `display`, y el test lo exige
  por el lado negativo —un `display: none` en un separador movería cada pestaña
  posterior cada vez que el puntero cruza la tira—. Se apaga también tras una
  etiqueta de grupo, donde el separador dibujaría una línea contra el borde de
  la etiqueta. -->
- [x] 2.4 La franja de color del grupo (design D7) queda con 3.4, que es su
  tarea; se anota aquí para que la anatomía no se dé por cerrada sin ella

## 3. El puntero, el cierre y el ancho

- [x] 3.1 Relleno de hover en las inactivas, puesto en el contenedor porque el
  botón interno no tiene borde que la regla global pueda alcanzar (design D4) —
  escenario «La inactiva responde al puntero»
  <!-- 2026-08-09: la paleta **no tiene un paso intermedio** entre la capa de
  la tira (`--surface-2`) y el panel (`--surface`), así que el hover usa la
  superficie del panel y la distinción con la activa la sostiene la costura,
  que es exactamente el portador que D2 eligió. Se descartó inventar un token
  —el lint de variables lo cazaría y el design system es normativo— y se
  descartó `color-mix`, que no está verificada en los tres webviews. **El smoke
  (4.2) debe confirmar que una pestaña bajo el puntero no se confunde con la
  activa**; si se confunden, el paso siguiente es un token de hover declarado
  en la paleta con sus dos temas, no un color suelto aquí. El test pinea además
  por qué el hover no puede quedar en la regla global: si el botón interno
  dejara de ser sin borde, la explicación caducaría en silencio. -->
- [x] 3.1b Confirmar en el smoke (4.2) que hover y activa se distinguen
- [x] 3.2 La X se revela con puntero y con foco, permanece en la activa y
  reserva su hueco siempre; `Delete` sigue cerrando (design D5) — escenario «El
  cierre se revela sin perder el camino de teclado»
  <!-- 2026-08-09: se oculta con `visibility`, no con `display`, por dos
  razones que se refuerzan: el hueco queda reservado (revelarla no encoge el
  rótulo ni corre a las vecinas) y `visibility: hidden` **la saca del orden de
  tabulación**, de modo que una X invisible no puede recibir el foco. El foco
  del botón de la pestaña la revela vía `:focus-within` y solo entonces es
  enfocable: el gesto nunca queda solo-puntero. `Delete` no se toca y el test
  lo exige, porque es el camino que no depende de nada de esto. -->
- [x] 3.2b Comprobar en el smoke (4.2) que revelar la X no mueve las vecinas
  ni recorta el rótulo, medido
- [x] 3.3 El estado se comprime a su glifo dentro de la pestaña y su palabra
  pasa al nombre accesible y al emergente, en `SessionTabs.svelte` (design D6)
  — escenario «El estado no gasta ancho en repetirse» — gates: `npm run
  lint:i18n`
  <!-- 2026-08-09: `StatusBadge` gana una forma `compact` en vez de nacer un
  segundo badge —el estado se dice en un solo sitio para las cinco
  condiciones—, y `TabItem` gana `state` para que la palabra comprimida llegue
  al nombre accesible de la **pestaña**, no solo del badge. La composición de
  ese nombre salió a `accessibleName()`: antes vivía inline y solo sabía del
  grupo; ahora suma lo que la pestaña comprima, y devuelve `undefined` cuando
  no hay nada que añadir para no repetir el contenido. Hubo que enmendar dos
  aserciones vecinas que pineaban la forma literal (`<StatusBadge
  state={info.state} />` y la expresión inline del `aria-label`); ambas
  garantías siguen exigidas, ahora por su nueva forma. -->
- [x] 3.3b Comprobar en el smoke (4.2) que el rótulo recupera ancho real con
  seis pestañas del mismo agente, medido
- [x] 3.4 Franja del color del grupo en cada pestaña miembro, con el nombre del
  grupo intacto en el nombre accesible (design D7) — escenario «La pertenencia
  se ve en la pestaña, no solo en la etiqueta»
  <!-- 2026-08-09: la franja es el **borde superior**, no un pseudo-elemento:
  `::before` y `::after` ya están tomados por el separador y por la costura, y
  una tercera capa habría exigido un elemento nuevo en el marcado. **Todas** las
  pestañas reservan el ancho de la franja (transparente si no hay grupo), de
  modo que entrar a un grupo no desplaza la pestaña contra sus vecinas.
  `GROUP_BAND_PX = 3` y no 2 por una razón concreta y verificable: el `2px` del
  swatch ya vive en esta hoja de estilo, y el guardián de la tarea 1.1 —que
  prohíbe el literal de cualquier medida del módulo— habría disparado con
  razón. -->
- [x] 3.4b Comprobar los cuatro tonos de grupo en el smoke (4.2), en los dos
  temas

## 4. Cierre

- [x] 4.1 `meltemi validate piel-de-pestanas` limpio y `meltemi verify` con los
  ocho escenarios enlazados (meta: cero marcas manuales); gates del frontend
  verdes y `cargo clippy`/`fmt` limpios
- [x] 4.2 Smoke visual CDP sobre el **binario de release** con capturas y
  medidas: los dos temas, `forced-colors` en la pasada, la activa unida al
  panel, el hover de una inactiva, la X revelándose por foco, la franja de
  grupo y el contraste real entre la capa de la tira y el panel (design D1: si
  no separa, se aplica la palanca declarada y se anota). Nota en
  `docs/qa/2026-08-09-piel-de-pestanas-smoke.md`
  <!-- 2026-08-09: conducido sobre binario propio (CARGO_TARGET_DIR aparte,
  para no cerrar la GUI del mantenedor) contra un fixture con endpoint, datos y
  config propios — el daemon real no se detuvo ni se consultó. Resultados: las
  seis comprobaciones cerradas, **el riesgo de contraste confirmado** (1.142:1
  en oscuro; palanca D1 aplicada = línea de base) y **un defecto que ningún
  test de fuente podía ver**: los apagadores del separador alcanzaban la
  costura de la activa —mismo pseudo-elemento, misma especificidad, más
  tarde—, así que su pie izquierdo no se pintaba cuando era primera o seguía a
  una etiqueta de grupo. Corregido con `:not(.active)` y pineado. Dos hallazgos
  de método en la nota (la receta CDP necesita patch **y** user data folder
  propio) y un hallazgo ajeno anotado en el backlog (el CLI no canonicaliza la
  raíz del proyecto). -->

> **Medido en el smoke (2026-08-09)**, para que cada marca de arriba diga qué
> la sostiene: contraste de capas **1.142:1 oscuro / 1.124:1 claro** (no
> separan → palanca D1 aplicada como línea de base); `forced-colors` presente
> en la hoja cargada (`.tab.active { border-color: highlight }`); hover contra
> activa resuelto **por la línea de base**, que solo la activa pierde; revelar
> la X deja las medidas idénticas al píxel (134.725 / 87.575 / 474.325);
> rótulos de 87–91 px sin truncar con seis pestañas del mismo agente; y la
> banda de grupo medida sobre un grupo creado por la vía real, con su ancho
> reservado en todas las pestañas.
