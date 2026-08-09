# pestanas-como-chrome

> Vía completa (proposal → design → specs → tasks). El delta es **solo ADDED**
> sobre `gui-shell`, pero el alcance no es de un día: una tira que deja de
> envolver y aprende a desplazarse, controles de desbordamiento, agrupación con
> su modelo propio, y todo ello sin perder el patrón ARIA que la tira ya cumple.
> Se declina la vía rápida por alcance, no por forma (design D7).

## Why

El mantenedor lo pidió con una referencia y dos requisitos: «el sistema de tabs
quiero que se parezca al de google chrome. Además la posibilidad de agrupar tabs
y con botón `<` `>` si es que hay muchos tabs, igual que google chrome».

La tira que existe hoy —nacida en `sidebar-ajustable-y-pestanas`— cumple el
patrón ARIA completo y es correcta, pero **envuelve**: `flex-wrap: wrap`. Con
cinco sesiones abiertas la captura del mantenedor ya muestra dos filas, y la
segunda empuja el contenido hacia abajo. Es el comportamiento que Chrome no
tiene y que hace que una tira deje de leerse como una tira: la posición de una
pestaña cambia de renglón según cuántas haya, así que el ojo pierde el sitio
donde dejó la que estaba mirando.

**Y no hay forma de agrupar.** Con ocho sesiones abiertas de tres trabajos
distintos, la tira es una lista plana de identificadores cortados. Chrome
resolvió eso con grupos nombrados y de color, plegables, y es exactamente la
forma que el mantenedor nombró.

Lo que **no** se copia de Chrome es su forma de tratar el desbordamiento:
Chrome encoge las pestañas hasta que solo queda el favicon y nunca desplaza. El
mantenedor pidió explícitamente los botones `<` `>`, que es lo que hacen Edge y
Firefox, y es la petición la que manda sobre la referencia.

## What Changes

- **Una sola fila, siempre.** La tira deja de envolver. Las pestañas encogen
  hasta un mínimo legible y, pasado ese punto, la tira se desplaza en
  horizontal.
- **Controles `<` y `>` que solo existen cuando hacen falta**: aparecen cuando
  hay más pestañas de las que caben y desaparecen cuando dejan de sobrar. Cada
  uno se deshabilita en su extremo, para que el usuario sepa que llegó al final
  en vez de descubrirlo pulsando.
- **La pestaña activa siempre se ve.** Seleccionarla con el teclado, abrir una
  nueva o volver a una de fondo la trae al área visible sin que el usuario tenga
  que buscarla.
- **Grupos con nombre y color, plegables.** Una pestaña puede unirse a un grupo
  nuevo o existente y salirse de él; el grupo se pliega a su etiqueta y se
  despliega; plegado, declara cuántas pestañas guarda y NO las cierra.
- **La forma se acerca a la de Chrome**: pestañas contiguas sin separación, con
  las esquinas superiores redondeadas, y la activa unida al panel que gobierna.
  El color del grupo es una franja, nunca el único portador: el nombre del grupo
  viaja en el texto de cada pestaña que le pertenece.

## Capabilities

### Modified Capabilities

- `gui-shell`: + la tira de una sola fila con desbordamiento desplazable y sus
  controles, + la pestaña activa siempre visible, + los grupos de pestañas con
  su nombre legible como texto y su plegado que no cierra nada.

### New Capabilities

- Ninguna.

## Impact

- `desktop/ui/src/lib/tab-groups.ts` (nuevo, puro: el modelo de grupos y sus
  reglas), `desktop/ui/tests/tab-groups.test.ts` (nuevo),
  `desktop/ui/src/lib/components/TabStrip.svelte` (la fila, el desplazamiento y
  los grupos), `desktop/ui/src/lib/components/SessionTabs.svelte`,
  `desktop/ui/src/App.svelte` (el estado de los grupos), `messages.ts`,
  `desktop/tests/scenarios_shell.rs`.
- **Cero cambios en el daemon, en el contrato y en la TUI.** Sigue siendo cromo
  de una superficie: no nace deber de paridad §4.
- Cero dependencias nuevas.

## Fuera de alcance

- **Persistir grupos o pestañas entre arranques.** Las pestañas se olvidan por
  decisión escrita (`sidebar-ajustable-y-pestanas` D7) y un grupo de pestañas
  que no existen no tiene qué guardar. Si se persisten las pestañas, los grupos
  van en esa misma change.
- **Arrastrar para reordenar o para agrupar.** La agrupación se hace por menú,
  que es alcanzable con teclado; el arrastre es un gesto solo de puntero y
  necesita su propio equivalente accesible.
- **Desprender una pestaña a otra ventana**, fijar pestañas, silenciarlas.
- **Adoptar la tira en el Editor**: sigue siendo seguimiento anotado.
- **Copiar el encogido extremo de Chrome** (hasta solo el icono): el mantenedor
  pidió los controles de desplazamiento, y tener las dos cosas a la vez esconde
  el momento en que aparecen.
