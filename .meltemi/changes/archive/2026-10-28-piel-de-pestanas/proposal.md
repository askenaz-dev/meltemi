# piel-de-pestanas

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre `gui-shell` (la spec vigente
> pinea el comportamiento de la tira: una fila, desplazamiento, grupos; la
> forma visual no tiene requisito que enmendar), cero dependencias, cero
> contrato, cero daemon. Cromo de una superficie: no nace deber de paridad §4.

## Why

`pestanas-como-chrome` entregó la estructura que el mantenedor pidió — una
sola fila que se desplaza, controles de desbordamiento medidos, grupos con
nombre y color — y la piel quedó en el mínimo que la estructura necesitaba
para existir. El mantenedor volvió con la captura al lado de Chrome y el
veredicto es justo: «el manejo de tabs está feo». El diagnóstico, leído en
`TabStrip.svelte` contra las capturas, tiene cinco causas concretas:

1. **No hay capa de fondo para la tira.** Las pestañas flotan sobre el mismo
   fondo que el panel. Chrome funciona porque su tira es más oscura que el
   contenido y la pestaña activa comparte el color del panel y se funde con
   él; sin ese contraste de capas, la activa no tiene de dónde levantarse —
   en el tema oscuro, activa e inactivas quedan a un paso de tono
   (`--surface` #111a2e contra `--surface-2` #1a2540).
2. **La activa se marca con borde de acento** (`border-color: var(--accent)`),
   que se lee como un input con outline, no como una pestaña elevada.
3. **Cero hover.** El botón interno lleva `border: 0`, así que la regla global
   de hover cambia un borde invisible: pasar el puntero no responde nada.
4. **Cada pestaña carga su propia caja de 1 px** — se leen como botones
   pegados, no como una silueta continua; y la X de cerrar está siempre
   visible en las N pestañas a la vez.
5. **Cada pestaña dice glifo + palabra de estado + agente + hash**: con seis
   sesiones del mismo agente, todas las pestañas son idénticas salvo ocho
   caracteres de hex, y la palabra repetida come ancho que el rótulo necesita.

## What Changes

- **La tira gana su propia capa de fondo**, más oscura que el panel, y la
  pestaña activa toma el fondo del panel y se une a él sin borde inferior:
  la anatomía de Chrome, con **curvas de unión** en los pies de la activa
  hechas con pseudo-elementos y `radial-gradient` — dentro del presupuesto
  cross-webview (nada de `mask-composite`, nada de `@property`).
- **Las inactivas pierden su caja**: separador hairline de 1 px entre vecinas
  inactivas, que se oculta junto a la activa y junto a la pestaña bajo el
  puntero, como hace Chrome.
- **Hover honesto**: relleno redondeado suave sobre la inactiva bajo el
  puntero; la X de cerrar se revela en hover y en foco, y queda siempre
  visible en la activa. El camino de teclado no depende del puntero: `Delete`
  ya cierra (patrón ARIA vigente) y la X se revela cuando la pestaña tiene el
  foco — ningún gesto queda solo-puntero.
- **El estado se comprime a su glifo** dentro de la pestaña; la palabra viaja
  en el nombre accesible y en el tooltip (regla vigente de iconografía:
  etiqueta visible **o accesible**). La píldora de no-leídos se queda: es un
  número con nombre, no un color.
- **El color del grupo gana una franja** en el borde superior de cada pestaña
  miembro, además de la etiqueta actual: la pertenencia se ve sin recorrer la
  tira. El nombre sigue viajando en el nombre accesible (requisito vigente:
  el color jamás es el único portador).
- **El foco es inconfundible**: el anillo global `--focus` se conserva y la
  activa queda además marcada por forma (elevación + unión al panel), así que
  la selección sobrevive monocromo y forced-colors.
- **Las medidas nuevas** (alto de pestaña, radio superior, franja de grupo)
  entran como constantes a `tab-strip.ts`, donde un test puede leerlas —
  el patrón que la tira ya usa para `MIN_TAB_PX`.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «La tira de pestañas se lee como la de un
  navegador» — capa propia de la tira, activa unida al panel y marcada por
  forma, respuesta al puntero en las inactivas, cierre revelado sin perder el
  camino de teclado, estado comprimido a glifo con nombre accesible, franja
  de color del grupo. ADDED-only: los requisitos vigentes de la tira (una
  fila, controles, grupos) no se enmiendan.

## Impact

- Solo `desktop/ui`: `TabStrip.svelte`, `SessionTabs.svelte`, `tab-strip.ts`
  y sus tests. Cero dependencias, cero contrato, cero daemon, TUI intacta.
- i18n: sin cadenas nuevas previstas (la palabra de estado ya existe y pasa
  al nombre accesible).
- Verificación: tests de componente por escenario + **smoke visual CDP sobre
  el binario release** (método de la casa), con los dos temas y
  forced-colors en la pasada.
- Riesgo: bajo y localizado. Lo único delicado son las curvas de unión en
  WebKitGTK — si el smoke las muestra imperfectas allí, la degradación es
  honesta (sin curvas, esquinas rectas) y se anota, no se bloquea la change.

## Fuera de alcance

- **El rótulo de la pestaña** (hoy agente + hash): lo resuelve
  `titulo-de-sesion`, que trae el título derivado por el daemon. Esta change
  no toca qué dice la pestaña, solo cómo se viste.
- **Persistir pestañas o grupos entre arranques** (deuda declarada heredada
  de `sidebar-ajustable-y-pestanas` D7).
- **Arrastrar para reordenar**, desprender a otra ventana, fijar, silenciar:
  siguen fuera por las razones escritas en `pestanas-como-chrome`.
- **Adoptar la tira en el Editor**: sigue siendo seguimiento anotado.
