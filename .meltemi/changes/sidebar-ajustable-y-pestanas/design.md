# Design — sidebar-ajustable-y-pestanas

## Context

Verificado en el código el 2026-08-08, a partir de la captura y las tres frases
del mantenedor:

- La «línea divisoria» es `border-top: 1px solid var(--hair)` sobre `.section`
  (`Sidebar.svelte:493`), la fila del título PROYECTOS. **No hay elemento
  divisor.** `.section` además lleva `margin-bottom: calc(-1 * var(--sp-2))`
  (:495), de modo que la raya queda a 12 px de `nav` y a ~4 px del árbol: se lee
  como raya de encabezado, que es lo que es hoy.
- **No existe ningún control de arrastre en toda la superficie.** Un barrido de
  `desktop/ui/src` no encuentra `pointerdown`, `pointermove` ni
  `setPointerCapture`. El único oyente de la familia del ratón es la
  descartación por clic fuera de `Chip.svelte`.
- **No existe ningún estilo de barra de desplazamiento**: ni
  `scrollbar-width`, ni `scrollbar-color`, ni `::-webkit-scrollbar` en ningún
  archivo. Lo único que toca el pintado nativo es `color-scheme: light dark`
  (`app.css:61`), que hace que la barra siga el tema pero no cambia ancho ni
  botones de paso.
- `App.svelte:51` es `let detailSession: string | null = $state(null)`: **una**
  sesión visible. La instancia de `SessionDetail` no está envuelta en `{#key}`
  —a diferencia de `<Editor>`—, así que cambiar de sesión muta la prop sobre la
  misma instancia: el borrador sobrevive de una sesión a otra y un `session/log`
  rechazado deja el transcript anterior bajo la cabecera nueva.
- **El daemon soporta N vigilancias por conexión**: `server.rs:123` declara
  `let mut watched: HashSet<String>` por conexión, sin tope, y el hub entrega
  por `origin` o por pertenencia al conjunto. Es un `HashSet`, no un `Option`.
  Ese hecho es el que hace viable montar varias sesiones a la vez.
- **El Editor ya tiene una tira de pestañas** propia (`role="tablist"` en
  `Editor.svelte`), sin patrón ARIA completo: es el precedente de forma, no de
  corrección.
- `lanzador-conversacional` tiene un delta `## MODIFIED Requirements` sobre
  `### Requirement: Paridad de vistas y modelo de navegación` de `gui-shell`, y
  está bloqueado en la firma del mantenedor fundador. El motor de specs
  reemplaza el bloque completo de un requisito ante un MODIFIED: **dos deltas
  sin archivar sobre el mismo encabezado se pisan**.

## Goals / Non-Goals

**Goals**: que el usuario reparta el alto entre navegación y proyectos y ese
reparto se recuerde; que ninguna barra de desplazamiento de la superficie se
coma una columna; que varias sesiones estén abiertas a la vez sin que ninguna
pierda su lectura ni su borrador.

**Non-Goals**: el ancho de la barra; persistir las pestañas; rediseñar la
navegación; teclas globales nuevas; tocar la TUI; la auditoría de
intuitividad.

## Decisions

### D1 — Las tres peticiones son una change porque la primera fabrica la segunda

Darle a `nav` un alto puesto por el autor exige `min-height: 0; overflow-y:
auto` sobre él —un ítem de flex en columna no puede encogerse por debajo de su
contenido sin eso, y las siete entradas se desbordarían de la caja—. En el
momento en que eso entra, la columna de 216 px estrena **una segunda barra
clásica de Windows con sus flechas**, dentro de la misma barra lateral cuya
primera barra el mantenedor acaba de llamar fea. Entregar el divisor sin el
arreglo del scroll duplica el defecto. No es agrupación por comodidad: es que
la petición 1 no se puede entregar honestamente sin la 2.

La tercera viaja con ellas porque las tres son la misma frase del mantenedor
sobre la misma captura, y separarla obligaría a un segundo ciclo de spec para
una superficie que ya se está tocando. Si el alcance se demuestra excesivo
durante la implementación, el punto de corte declarado es el bloque 3
(pestañas), que no depende de los bloques 1 y 2.

### D2 — Un elemento separador propio, no `.section` como asa

`role="separator"` en su forma enfocable es un rol de widget, y `.section`
contiene un `<span>` de título y un `<button>` «Abrir carpeta…». Descendientes
interactivos dentro de un separador enfocable es ARIA inválido. Por tanto se
inserta un elemento dedicado entre `nav` y `.section`, y **el hairline se muda
del encabezado al control**: `.section` pierde su `border-top` y el separador
lo pinta con un `::after` centrado en un área de agarre de 12 px.

Consecuencia visual deliberada, dicha y no escondida: la línea baja unos 6 px y
el título PROYECTOS unos 12. Ese es el punto — una raya que ahora divide debe
leerse como divisor, no como encabezado. El smoke mide ambos desplazamientos.

Alternativas descartadas: (a) hacer arrastrable el borde de `.section` sin
elemento propio — ARIA inválido y sin foco; (b) un asa que aparece al pasar el
puntero — es exactamente lo que esta superficie ya rechazó por escrito («un
control que aparece bajo el puntero es un control que el teclado no alcanza»).

### D3 — El reparto se recuerda donde ya se recuerdan el tema y el pliegue

`nav_split: Option<u32>` en `desktop-ui.json`, junto a `nav_collapsed`, tema y
geometría, tras `#[serde(default)]` para que todo perfil existente cargue. Una
preferencia de disposición pertenece con las otras preferencias de disposición.

Se persiste **al soltar el puntero, nunca durante el movimiento**: la superficie
escribe el objeto entero en cada mutación mientras el anfitrión hace
lectura-modificación-escritura de la geometría de ventana. Escribir por cada
`pointermove` serían ~200 escrituras por arrastre y multiplicaría las
probabilidades de esa carrera, que ya existe y que esta change no debe
alimentar.

**Reajuste al encoger la ventana**: un reparto recordado que ya no cabe se
acota al que la barra puede dar, con el mismo espíritu que la geometría de
ventana que ya se valida contra las pantallas que existen. El efecto que reajusta
compara antes de escribir (`if (fixed !== navSplit)`), porque sin esa
desigualdad escribe el valor que acaba de leer y no converge.

**Suelos por los dos lados y el caso apretado**: la aritmética vive en un módulo
puro con `MIN_NAV_PX` y `MIN_TREE_PX`. Cuando la barra es demasiado corta para
satisfacer ambos, **las entradas conservan su mínimo y el árbol se queda con lo
que reste**, desplazándose con scroll. Es una decisión, no un accidente: la
navegación es el camino de vuelta a todo lo demás.

### D4 — Propiedades estándar de scrollbar, una sola declaración de cada una

`scrollbar-color: var(--text-faint) transparent` en `:root`, junto a
`color-scheme: light dark`, y `scrollbar-width: thin` en un selector universal.

**Corrección tras medir sobre el binario empaquetado.** La primera versión de
esta decisión puso ambas en `:root` afirmando que ambas heredan. **Solo hereda
`scrollbar-color`.** El smoke lo desmintió con el valor computado del árbol:
`scrollbar-color` llegó desde la raíz y `scrollbar-width` computó `auto`, con la
barra clásica y sus botones de paso intactos — es decir, la regla se veía
correcta y no hacía nada. De ahí la forma final: el color una vez, heredado; el
ancho declarado para todo elemento, que es la única manera de que una sola regla
alcance a todos los desplazadores. El test pinea las dos mitades, incluida la
negativa: el ancho **no** debe quedar en `:root`.

Las demás razones siguen en pie: (a) los cuatro bloques de tema redefinen
`--text-faint`, de modo que el color sigue al tema sin una segunda declaración;
(b) una regla local en `Sidebar.svelte` dejaría el transcript, la paleta y
Ajustes con la barra gorda —exactamente la incoherencia que un design system
existe para evitar— y además quedaría sujeta al podado de selectores no usados
de Svelte.

**Segunda corrección tras fotografiar la barra.** La versión anterior rechazaba
la familia `::-webkit-scrollbar` invocando el presupuesto de compatibilidad
entre webviews. Al ampliar la captura sobre el binario empaquetado, la barra
angosta **conserva sus botones de flecha** en WebView2: `scrollbar-width: thin`
la estrechó de ~17 px a ~10 y redondeó el pulgar, pero los botones —la parte más
fea de lo que el mantenedor señaló— siguen ahí, y **ninguna propiedad estándar
los retira**.

El presupuesto nombra sus tres motores objetivo: WebView2 (Chromium), WKWebView
(Safari) y WebKitGTK. Los tres implementan esa familia: está **dentro** de esa
intersección, no fuera. Y no queda nada apoyado en ella: las propiedades
estándar siguen dando el ancho y el color, así que un motor que ignore el bloque
recibe igualmente la barra angosta. Se usa solo para quitar cromo que ninguna
propiedad estándar quita. El requisito se redactó con esa frontera —los
selectores de motor no pueden ser el único portador del ancho ni del color— y el
test la pinea por los dos lados.

Alcance decidido y no implícito: es una regla **de toda la superficie**. El
mantenedor nombró la barra lateral porque es donde 17 px duelen dentro de 200,
pero la misma barra está en el transcript y en Ajustes. El requisito se escribe
de toda la superficie para que el escenario corresponda al código.

Contrapartida declarada: `thin` son ~11 px en Chromium, un objetivo de agarre
más pequeño que los ~17 de hoy para quien arrastra el pulgar en vez de usar la
rueda. Se acepta porque los botones de paso desaparecen —que son los que hacen
ilegible el árbol— y porque la rueda y el teclado no cambian.

### D5 — Pestañas contenidas en la vista Sesiones, y ADDED por eso

Tres formas se diseñaron en paralelo: (a) una tira **sobre la región principal**
donde las vistas numeradas y las sesiones son pares; (b) la vista Sesiones
convertida en un **espacio de trabajo con pestañas**; (c) las pestañas
**contenidas en la vista Sesiones**, con la lista como primera pestaña.

Se elige (c) por cuatro razones, cada una comprobable:

1. **Es la única que no colisiona con un delta bloqueado por una firma.** (a) y
   (b) modifican `### Requirement: Paridad de vistas y modelo de navegación`, el
   mismo encabezado que `lanzador-conversacional` ya modifica en su delta sin
   archivar. El motor reemplaza el bloque completo ante un MODIFIED: uno de los
   dos pisaría al otro, en silencio. Y `lanzador-conversacional` está bloqueado
   en la firma del mantenedor fundador, que ninguna change puede darse a sí
   misma. (c) es solo-ADDED y no colisiona con nada.
2. **El radio de impacto es medible y es el menor.** La cadena de rutado queda
   una rama **más corta**, no más larga: la rama suelta del detalle se pliega
   dentro de la de Sesiones. `Sessions.svelte` no se toca —se monta como hijo—,
   y las tres cadenas que la suite pinea con más fuerza (la vista inicial, la
   lista de vistas con clave y el rango de dígitos 1–5) sobreviven literales.
3. **Su riesgo central se retiró leyendo el daemon.** Las tres formas marcaron
   como bloqueante «¿puede una conexión sostener N `session/watch` a la vez?».
   Está verificado: `server.rs:123` es un `HashSet` por conexión, sin tope. Eso
   valida específicamente la política de montar todo, que es la que más se apoya
   en ello.
4. **Arregla dos fallos vivos por construcción y no por parche.** Con
   `{#each openSessions as tab (tab.sessionId)}` cada sesión tiene **su propia
   instancia**, así que el borrador que salta de sesión y el transcript que
   sobrevive a un `session/log` fallido desaparecen sin escribir una sola
   guardia.

Lo que no se afirma: que (c) sea la más terminada. (a) persistía las pestañas y
(c) no. Ese corte se paga en D7 con razones de arranque, no de gusto.

### D6 — Montar todas, ocultar las de fondo; la vigilancia se queda donde está

Los paneles inactivos se ocultan con `hidden`, no se desmontan: es lo que
conserva transcript, búsqueda y borrador sin copiarlos a ningún sitio.

Dos consecuencias que hay que escribir porque son fáciles de olvidar:

- Un subárbol `hidden` reporta `scrollHeight` 0, así que el anclado al pie que
  hace el transcript es un no-op mientras la pestaña está de fondo. Hace falta
  un efecto que reancle **al volver al frente**, o la pestaña se abre por la
  línea 1.
- El autoenfoque del compositor se dispara una vez por sesión. Con N paneles
  montados, cada panel de fondo lo ejecuta **desde un subárbol `hidden` donde
  `.focus()` no hace nada**, y marca la sesión como ya enfocada — de modo que
  esa pestaña **nunca** enfocaría su compositor al venir al frente. La guarda
  por `active` no es cosmética: sin ella el fallo es silencioso y permanente.

La suscripción `session/watch` **se queda dentro del componente de detalle**, una
por instancia, con su limpieza. Levantarla a una capa del shell crearía dos
dueños de un conjunto por conexión que es idempotente: la desuscripción de uno
ensordece al otro, y el síntoma —«el transcript se paró»— no lo atrapa ninguna
prueba actual.

**Tope de ocho, y rehúsa en vez de desalojar.** Ocho porque el árbol ya recorta
las sesiones por proyecto en ocho y una superficie no debería contradecir su
propio número. Rehúsa porque una pestaña de fondo puede tener un borrador sin
enviar y descartarlo por una regla invisible es peor que decir «cierra una para
abrir otra».

### D7 — Las pestañas no se persisten, y la condición de reapertura queda escrita

Tres razones concretas, ninguna de gusto: (i) restaurar ocho pestañas al
arrancar dispara ocho lecturas de registro y ocho vigilancias contra el
presupuesto de arranque bajo un segundo; (ii) el listado de sesiones aterriza de
forma asíncrona **después** del montaje, así que restaurar obliga a pintar
pestañas que el daemon no ha confirmado o a bloquear en el primer listado —un
problema de orden sin respuesta limpia; (iii) los identificadores caducos
reciben al usuario con un aviso de «tus pestañas desaparecieron», que es una mala
primera impresión de una función de persistencia.

Las pestañas **sí** sobreviven a navegar a otra vista y volver, que es el caso
que de verdad duele: aprobar un permiso no puede costar tres transcripts.

**Condición de reapertura**: si el mantenedor pide restaurarlas al arrancar, es
una change propia cuya primera tarea es medir el arranque con ocho pestañas.

### D8 — La guardia de `tabindex` se enmienda hacia arriba, no hacia abajo

El patrón ARIA de pestañas exige `tabindex` rotatorio: la activa a `0`, las
demás a `-1`. La guardia actual solo ve el **literal** `tabindex="-1"` sobre
etiquetas interactivas; una expresión como la que exige el patrón **hoy pasaría
sin ser vista**. Así que la enmienda tiene dos mitades y la primera es la que
importa: el detector se **ensancha** para ver también la retirada dinámica del
orden de tabulación, que hoy es un punto ciego. La excepción es estrecha —solo
elementos con `role="tab"`— y se cobra con un test positivo nuevo: la tira debe
recorrerse entera con flechas, Home, End y Delete, moviendo foco además de
selección.

Se dice aquí porque tocar un test cuyo trabajo es la alcanzabilidad por teclado
merece sospecha, y la defensa tiene que estar visible en el propio diff.

### D9 — Vía completa aunque el delta sea solo-ADDED

El criterio de vía rápida tiene dos mitades: la forma del delta (solo ADDED
sobre una capability existente) y el alcance (de un día). La forma califica; el
alcance no. Se declina explícitamente en vez de dejar que un revisor lo note.

## Risks / Trade-offs

- **Secuenciación, y es el riesgo mayor y no del todo controlable aquí.** Este
  sería el tercer delta sin archivar sobre `gui-shell`. Se mitiga siendo
  solo-ADDED con encabezados nuevos y distintos, que es la única forma que el
  motor puede fusionar sin pisar texto ajeno.
- **Primer uso de captura de puntero en este repositorio.** No hay patrón
  interno que copiar ni prueba previa de cómo se comporta bajo WebView2. El
  smoke conducido es la red.
- **Ocho transcripts montados contra el presupuesto de 80 MB en reposo**: sin
  medir. El smoke debe reportar consumo con una, cuatro y ocho pestañas, y si
  el número obliga, el tope de líneas del transcript deja de ser fuera de
  alcance y pasa a ser una change con evidencia.
- **`thin` reduce el objetivo de agarre** de la barra de ~17 a ~11 px. Aceptado
  en D4 con su razón.
- **El efecto de reajuste puede ciclar** si se escribe sin la desigualdad. Se
  fija con un test, no con confianza en la revisión.
- **Trampa de substring**: la suite falla si `Sidebar.svelte` contiene
  `transition:`, `animate:`, `flip`, `@keyframes` o `animation:`. La coincidencia
  es por substring, no por identificador: ningún comentario de este archivo
  puede contener la palabra suelta `flip`.

## Migration Plan

Aditivo y reversible. Un perfil sin `nav_split` arranca en el reparto de hoy y
la barra se ve exactamente igual salvo el hairline desplazado; un perfil sin
pestañas abiertas ve la lista de siempre. Nada del contrato `proto/` se mueve,
así que ningún cliente necesita coordinarse.

## Open Questions

- ¿Debe el tope de ocho ser configurable? Se omite: un número que nadie ha
  chocado todavía no merece una preferencia.
- ¿Adopta el Editor la tira genérica? Se anota como seguimiento en el backlog,
  no se hace aquí.
