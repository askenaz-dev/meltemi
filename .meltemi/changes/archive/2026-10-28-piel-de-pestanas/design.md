# Design — piel-de-pestanas

## Context

Verificado en `desktop/ui/src/lib/components/TabStrip.svelte` y
`desktop/ui/src/lib/tab-strip.ts` el 2026-08-09, contra las capturas del
mantenedor al lado de Chrome:

- **La tira no tiene capa propia.** `.strip` y `.tabs` no declaran `background`
  (`:225-247`), así que las pestañas flotan sobre el fondo del panel. Las
  inactivas son `--surface-2` (`:259`) y la activa `--surface` (`:263`) — que es
  el color del panel. En el tema oscuro son `#1a2540` contra `#111a2e`: un paso
  de tono, sin una capa que los separe. La anatomía de Chrome funciona al revés
  de como está montado: la **tira** es la superficie oscura, la **activa** toma
  el color del panel y se funde con él, y las inactivas se recuestan sobre la
  tira.
- **La activa se marca con acento**: `.tab.active { border-color: var(--accent) }`
  (`:262`). Un rectángulo con borde de acento es la forma de un campo de texto
  enfocado, no la de una pestaña levantada.
- **Cero respuesta al puntero**, y la causa es concreta: la regla global es
  `button:hover:not(:disabled) { border-color: var(--text-faint) }`
  (`app.css:237-239`), pero `.tab button` declara `border: 0` (`:313`). El hover
  cambia el color de un borde que no existe. No hay ninguna regla `:hover` en
  todo el componente.
- **Cada pestaña carga su propia caja** (`border: 1px solid var(--border)`,
  `:254`) y **la X está siempre visible** en las N pestañas a la vez
  (`:202-206`, sin condición).
- **Cada pestaña dice glifo + palabra de estado + agente + hash** (el `mark` y
  el `label` que `SessionTabs` compone): con seis sesiones del mismo agente, la
  tira repite la misma palabra seis veces y el rótulo pierde el ancho que
  necesita.
- **El patrón que la propuesta invoca no está cableado.** `tab-strip.ts` define
  `MIN_TAB_PX = 96` y `MAX_TAB_PX = 240` «out of the CSS so a test can read
  them», y el CSS **repite los dos números a mano** (`min-width: 96px`,
  `max-width: 240px`, `:252-253`). El import de `MIN_TAB_PX` quedó sin usar y
  hubo que retirarlo el 2026-08-09 para desbloquear el gate. Hay una sola fuente
  declarada y dos copias reales.

## Goals / Non-Goals

**Goals**: que la tira se lea como la de un navegador —capas, no cajas
apiladas—; que la selección se distinga por forma y no por color de acento; que
el puntero obtenga respuesta; que el cierre no ocupe N veces la tira; que la
pertenencia a un grupo se vea sin recorrerla; y que las medidas tengan un solo
dueño.

**Non-Goals**: qué dice cada pestaña (es `titulo-de-sesion`); el comportamiento
de desbordamiento, grupos y teclado (`pestanas-como-chrome` los fijó y esta
change no los enmienda); persistir pestañas; arrastrar para reordenar.

## Decisions

### D1 — La tira es una capa, y la activa se funde con el panel

`.tabs` gana `background: var(--surface-2)`; las inactivas se vuelven
**transparentes** —se recuestan sobre esa capa en vez de dibujar su propia
caja— y la activa toma `var(--surface)`, que es exactamente el color del panel
que gobierna. Así la separación la produce la capa y no un borde por pestaña,
que es lo que hace legible a Chrome.

La honestidad de contraste se dice aquí: `--surface` y `--surface-2` están a un
paso de tono en el tema oscuro. **La verificación es el smoke sobre el binario,
con medida**, no la vista de esta decisión. Si no separan, la palanca es un
hairline superior en la tira o la sombra de un solo nivel que el design system
reserva a flotantes — **jamás un color nuevo fuera de los tokens**, que el lint
de variables CSS de `panel-opaco-y-nav-plegable` rechazaría con razón.

### D2 — La selección se marca por forma; el acento se retira

`border-color: var(--accent)` sale. La activa queda unida al panel por la
ausencia de borde inferior que ya tiene, más **curvas de unión** en sus dos pies
hechas con pseudo-elementos y `radial-gradient` — dentro del presupuesto
cross-webview declarado (nada de `mask-composite`, nada de `@property`, nada
scroll-driven).

Tres garantías que esto no puede perder, y por eso se escriben:

1. **El anillo de foco global `--focus` se conserva intacto**: forma y foco son
   cosas distintas y siguen siéndolo.
2. **`aria-selected` ya lleva la verdad a quien no ve la forma** (`:191`), y no
   se toca.
3. **Bajo `forced-colors`**, donde los fondos se sustituyen por los del sistema,
   la unión desaparece: la activa conserva ahí un borde que la marca. La
   selección jamás depende de un fondo que el sistema puede reemplazar.

### D3 — Separadores entre inactivas, y solo entre inactivas

Sin caja por pestaña, la silueta la da un hairline de 1 px entre vecinas
inactivas, que **se oculta junto a la activa y junto a la que está bajo el
puntero** — la regla de Chrome. Se implementa con un pseudo-elemento sobre la
pestaña, no con `border-left`, para poder apagarlo por vecindad
(`.tab:hover + .tab::before` y `.tab.active + .tab::before`) sin mover el
layout: un separador que aparece y desaparece no puede correr las pestañas de
sitio.

### D4 — El hover se pone donde el hover puede verse

La respuesta al puntero vive en `.tab:not(.active):hover` como **relleno**
—no como borde—, porque el borde del botón interno es `0` y la regla global no
puede alcanzarlo. Corregir la regla global sería peor: cambiaría el hover de
toda la superficie por un caso de la tira.

### D5 — El cierre se revela, y el teclado no depende del puntero

La X aparece con `:hover` y con `:focus-within` en las inactivas, y queda
**siempre visible en la activa**. La revelación por foco es lo que impide que
el gesto quede solo-puntero; y `Delete` sigue cerrando por el patrón ARIA
vigente, que esta change no toca. El hueco de la X se **reserva siempre**
(visibilidad, no display), para que revelarla no encoja el rótulo ni mueva a
las vecinas.

### D6 — El estado se comprime a glifo, y la palabra viaja al nombre accesible

Dentro de la pestaña queda el glifo; la palabra de estado entra al `aria-label`
y al `title`. Es exactamente lo que la regla de iconografía vigente permite
—etiqueta visible **o** accesible— y lo que devuelve al rótulo el ancho que la
repetición le comía. La píldora de no-leídos se queda: es un número con nombre,
no un color solo.

### D7 — El color del grupo gana franja, sin dejar de ser redundante

Una franja de 2 px en el borde superior de cada pestaña miembro, además de la
etiqueta de grupo que ya existe. El requisito vigente no se relaja: **el nombre
del grupo sigue viajando en el nombre accesible de cada pestaña** (`:194`), así
que el color nunca es el único portador. Los cuatro tonos son los del `swatch`
ya cableado (`--ok`, `--warn`, `--danger`, `--info`), sin tokens nuevos.

### D8 — Un solo dueño para cada número

Las medidas dejan de estar duplicadas: `TabStrip.svelte` publica las constantes
de `tab-strip.ts` como custom properties en su elemento raíz
(`style="--tab-min: {MIN_TAB_PX}px; --tab-max: {MAX_TAB_PX}px; …"`) y el CSS las
consume. Las medidas nuevas de esta piel —alto de pestaña, radio superior,
ancho de la franja de grupo— nacen en el módulo con el mismo trato. Así el
patrón que la propuesta invocaba existe de verdad: un test lee el número, la
hoja de estilo lo obedece, y nadie los desincroniza en silencio.

## Risks / Trade-offs

- **Las curvas de unión en WebKitGTK** son lo único incierto. Si el smoke las
  muestra imperfectas allí, la degradación es honesta —sin curvas, esquinas
  rectas— y se anota; no bloquea la change.
- **La activa fundida al panel depende del contraste entre dos tokens
  vecinos** (D1). Medido en el smoke, con la palanca ya elegida por si falla.
- Riesgo de regresión sobre `pestanas-como-chrome`: ninguna medida de
  desbordamiento cambia de valor, solo de dueño (D8), y sus tests siguen
  leyendo el módulo.

## Migration / Rollout

Cambio de piel en una sola superficie; sin contrato, sin daemon, sin datos
persistidos. Se despliega con la change y se verifica con el smoke visual CDP
sobre el binario de release, en los dos temas y con `forced-colors` en la
pasada.
