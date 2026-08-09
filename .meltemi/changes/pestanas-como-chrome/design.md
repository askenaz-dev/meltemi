# Design — pestanas-como-chrome

## Context

Verificado el 2026-08-09:

- `TabStrip.svelte:110` declara `.tabs { flex-wrap: wrap }`. Con cinco sesiones
  abiertas la captura del mantenedor muestra dos filas.
- La tira ya cumple el patrón ARIA de pestañas: `tablist`/`tab`/`tabpanel`,
  `tabindex` rotatorio, flechas con ciclo, Home, End y Delete, con el manejador
  en cada pestaña. Nada de eso se toca: se le añade encima.
- `.tab { max-width: 260px }` es el único límite de ancho; no hay mínimo, así
  que hoy las pestañas no encogen: envuelven.
- Las pestañas no se persisten (`sidebar-ajustable-y-pestanas` D7), y el
  conjunto abierto vive en `App.svelte` como `openSessions`.

## Goals / Non-Goals

**Goals**: una sola fila que se desplace cuando no quepa, con controles
explícitos; que la pestaña activa nunca quede fuera de vista; grupos con nombre,
color y plegado, legibles como texto.

**Non-Goals**: persistir nada; arrastrar; desprender a otra ventana; tocar el
daemon, el contrato o la TUI.

## Decisions

### D1 — Una fila, encoger hasta un mínimo, luego desplazar

`flex-wrap: nowrap` y `overflow-x: auto` en la tira; cada pestaña con
`min-width` y `flex: 0 1 auto`, de modo que primero encogen —el texto se corta
con elipsis— y solo cuando alcanzan el mínimo empieza el desplazamiento.

El mínimo se elige para que quepan la marca de estado y unos caracteres del
nombre: una pestaña que no dice nada no es una pestaña, es un botón sin
etiqueta. La cifra vive en el módulo puro con las demás, no incrustada en el
CSS, para que el test la pueda leer.

### D2 — Los controles aparecen cuando sobran pestañas, y no antes

`<` y `>` se renderizan **solo** si el contenido desborda. Un control
permanentemente presente y a veces inerte enseña a ignorarlo; uno que aparece
cuando hace falta es información en sí mismo.

Cada uno se deshabilita en su extremo. Se prefiere deshabilitar a esconder:
esconder mueve el resto de la tira justo cuando el usuario está pulsando ahí.

La medición del desbordamiento usa `ResizeObserver` sobre la tira y su
contenido. Alternativa descartada: recalcular en cada render — el ancho depende
del layout y leerlo durante el render es exactamente cómo se provoca un bucle de
lectura y escritura.

### D3 — La pestaña activa se trae a la vista, siempre

Un efecto que, cuando cambia la activa, llama a `scrollIntoView` con
`block: "nearest", inline: "nearest"` sobre su elemento. `nearest` es
deliberado: mueve lo mínimo, así que una pestaña ya visible no salta.

Sin esto, las flechas del patrón ARIA mueven el foco a una pestaña fuera de
pantalla y el usuario queda tecleando en algo que no ve — un fallo de
accesibilidad, no de estética.

### D4 — El grupo es un modelo propio, y es puro

`tab-groups.ts` guarda: identificador, nombre, color y plegado, más la
pertenencia de cada pestaña. Reglas:

- Una pestaña pertenece **a lo sumo** a un grupo.
- Un grupo sin pestañas se destruye solo: un grupo vacío es un nombre sin nada
  detrás, y dejarlo obliga al usuario a limpiarlo a mano.
- Plegar un grupo **no cierra nada**: las pestañas siguen abiertas, sus paneles
  siguen montados y sus borradores intactos. Plegar es una decisión sobre el
  espacio, nunca sobre el trabajo.
- Si la pestaña activa está dentro de un grupo que se pliega, la actividad pasa
  a la primera pestaña fuera de él, o a la lista si no queda ninguna. Un panel
  visible cuya pestaña no se ve sería una superficie mintiendo sobre dónde está
  el usuario.

Los colores salen de los tokens del design system, no de una paleta nueva.

### D5 — El color nunca es el único portador

La franja de color identifica el grupo de un vistazo; el **nombre del grupo
viaja en el texto accesible de cada pestaña que le pertenece** («Sesión X —
grupo Refactor»). Es la misma regla que los estados con símbolo y palabra, y la
que hace que la agrupación signifique algo para quien no distingue los colores.

Plegado, el grupo declara **cuántas pestañas guarda**, también como texto.

### D6 — Agrupar se hace por menú, no arrastrando

Cada pestaña ofrece un menú con: crear grupo nuevo, unirse a uno existente,
salir del grupo. Es alcanzable con teclado sin inventar equivalentes. El
arrastre queda fuera de alcance precisamente porque su equivalente accesible es
otra conversación, y media función accesible es una función que excluye.

### D7 — Vía completa, aunque el delta sea solo ADDED

La forma del delta calificaría para vía rápida; el alcance —dos módulos, una
tira reescrita en su layout, un modelo de grupos con su menú, y su smoke— no.

## Risks / Trade-offs

- **`ResizeObserver` es la primera vez en este repositorio.** Es API estándar en
  los tres motores objetivo, pero su comportamiento bajo WebView2 se comprueba
  en el smoke, no se supone.
- **La elipsis puede dejar dos pestañas con el mismo texto visible.** Mitigado
  por el `title` completo, que ya existe, y por la marca de estado.
- **Un grupo plegado esconde pestañas con trabajo sin enviar.** Por eso el
  recuento es texto y el plegado nunca cierra; aun así, es un lugar donde algo
  puede quedar olvidado, y el diseño lo dice en vez de fingir que no.
- **El menú por pestaña añade un control a una fila ya densa.** Aparece en la
  pestaña activa y al enfocar, no en todas a la vez.

## Migration Plan

Aditivo: sin grupos, la tira se comporta como hoy salvo que ya no envuelve.
Nada persiste, nada del contrato se mueve.

## Open Questions

- ¿Atajo de teclado para plegar un grupo? Se omite: el conjunto de teclas de la
  GUI sigue siendo materia de su propia revisión.
