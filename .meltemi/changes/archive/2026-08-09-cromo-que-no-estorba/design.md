# Design — cromo-que-no-estorba

## Context

Verificado en el código el 2026-08-08:

- `Drawer.svelte:71-78` declara `.body { overflow: auto }` —ambos ejes— en un
  panel de `width: 268px`. Un solo hijo cuyo contenido mínimo no quepa produce
  la barra horizontal que se ve bajo todo el cajón en la captura.
- `stores.ts:159-166`: `pushNotice` apila y `dismissNotice` quita; **no hay
  caducidad de ningún tipo**. El comentario del tipo dice «persistent notices
  (permission expiries, session errors): never silent», que es la razón correcta
  para *esos* avisos y se aplicó a todos.
- La spec viva acota esa obligación: «Toda auto-denegación por vencimiento
  (`permission/timeout`) MUST superficializarse con un aviso persistente que no
  se descarta en silencio», con su escenario «Vencimiento anunciado». Habla de
  vencimientos y errores, no de confirmaciones.
- `Palette.svelte:149` es `<div class="scrim" role="presentation"
  onkeydown={onKeydown}>`: velo sin `onclick`. `ProjectSwitcher.svelte:42` es
  `<div class="scrim" role="presentation" onclick={onClose}>`. Dos velos, dos
  comportamientos.

## Goals / Non-Goals

**Goals**: que ninguna superficie flotante se desplace de lado; que una
confirmación no se quede clavada; que el gesto de cerrar funcione en todos los
velos por igual.

**Non-Goals**: tocar la persistencia de los avisos de vencimiento o error;
rediseñar el banner de daemon; las pestañas ni la flota.

## Decisions

### D1 — El cajón parte, no desplaza

`overflow-y: auto; overflow-x: hidden` en el cuerpo, más `min-width: 0` en sus
hijos directos para que una ruta larga pueda partirse en vez de empujar. Un
panel de ancho fijo que se desplaza de lado esconde la mitad de cada línea tras
un gesto que nadie hace en un panel de detalle.

Alternativa descartada: ensanchar el cajón. Mueve el problema al primer
contenido más largo y le quita ancho al contenido principal, que es de quien es.

### D2 — La caducidad se decide por consecuencia, no por antigüedad

Dos clases, y la frontera es la que la spec ya trazó:

- **Transitorio** (`info`): confirma algo que el usuario acaba de hacer y ya ve
  —«enlace creado», «proyecto abierto»—. Se retira solo a los **6 segundos**.
- **Persistente** (`warn`, `danger`): vencimientos, denegaciones, errores de
  sesión, fallos de operación. **No caduca**; solo se va con un gesto. Esto es
  obligación de spec, no preferencia, y el test lo pinea por el lado negativo:
  ningún temporizador puede alcanzar a un aviso que no sea `info`.

Se descartó «caducan todos a los N segundos» —viola la spec— y «caduca el más
antiguo cuando llega el cuarto» —convierte un error en algo que desaparece
porque hubo suerte con el orden.

**La cuenta se detiene bajo el puntero y bajo el foco.** Un aviso que se
desvanece mientras alguien lo está leyendo es peor que uno que se queda: la
información se pierde sin que nadie decida. Al salir el puntero, la cuenta
**reinicia** en vez de reanudar, que es lo que hace que un aviso releído dure lo
que dura leerlo.

El temporizador vive en el store, no en el componente: el componente se desmonta
al cambiar de vista y llevárselo allí haría que un aviso sobreviviera para
siempre por haber navegado. Se cancela al descartar, para que nada intente
retirar un identificador que ya no existe.

### D3 — Un velo es un velo: cierra

El velo de la paleta gana `onclick`. Y para que no vuelva a divergir, el test no
comprueba la paleta: **barre `desktop/ui/src` y exige que todo elemento con
`class="scrim"` lleve un manejador de clic**. Es la misma forma que el lint de
variables de estilo — cerrar la clase, no el caso.

Cuidado con el falso positivo del clic que burbujea: el velo y el panel son
hermanos, no anidados, en los dos componentes, así que un clic dentro del panel
no llega al velo. No hace falta comprobar el objetivo del evento, y el test no
pide una guardia que aquí sería ruido.

## Risks / Trade-offs

- **Seis segundos es un número.** Corto para un texto largo, largo para uno
  corto. Se elige uno solo y se documenta; la pausa bajo el puntero es lo que
  vuelve el número poco importante.
- **Un aviso informativo que importaba** podría irse antes de leerse. Mitigado
  por el historial: el contador de excedentes ya existe y el aviso solo
  desaparece de la vista, no del registro de la sesión.
- **`overflow-x: hidden` puede recortar** un contenido que no sepa partirse. Por
  eso va con `min-width: 0` y el `overflow-wrap` que las rutas ya usan; el smoke
  mira el cajón con la ruta más larga que la vista produce.

## Migration Plan

Aditivo y reversible: una propiedad CSS, un temporizador por tono y un atributo.
Nada persiste, nada del contrato se mueve.

## Open Questions

- ¿Debería el usuario poder ajustar la duración? No hasta que alguien lo pida:
  una preferencia por un número que nadie ha discutido es cromo sobre cromo.
