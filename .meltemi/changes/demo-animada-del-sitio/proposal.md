# demo-animada-del-sitio

> Vía rápida (fast-forward): un procedimiento de captura scriptado que
> produce los medios de la demostración en `docs/media/`. Es la mitad de esta
> change que sigue en el IDE: el sitio vive en meltemi-web y su página, su CSS
> y su lint son una change de allí (ver la nota). Nace del comparativo con
> Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brecha P4).

> **Nota 2026-10-24**, tras la mudanza del sitio:
>
> - **Se parte en dos.** Aquí queda lo que produce los medios: el script de
>   captura, la grabación y la revisión de fugas. A meltemi-web pasa lo que los
>   muestra: el vídeo embebido sin JavaScript, el CSS que lo oculta bajo
>   `prefers-reduced-motion` y el lint de peso y de procedencia. El IDE ya
>   entregaba así sus figuras fijas: las escribe en `docs/media/` y la web las
>   copia (`docs/ux/capturas.md`).
> - **El sitio no tiene hoy el lint que esta propuesta daba por hecho.** Las
>   reglas «alt y procedencia obligatorias» eran del lint de `site/` y se fueron
>   con él; los tests de meltemi-web no miran las figuras (comprobado el
>   2026-10-24). La change `site-figures-provenance` de meltemi-web los repone
>   para las dos figuras actuales, y el embebido del vídeo espera a ella.
> - **«La portada» es la página del IDE.** Las figuras viven en `/ide/`
>   («The surfaces, as they actually render»). La portada de la familia no
>   cambia; si la demo debe subir a ella, es decisión del mantenedor y de la
>   change de la web.

## Why

La página de Orca es la app en movimiento: un transcript que avanza, un
spinner, un cronómetro, una terminal que compila. La de Meltemi es una
captura fija de la vista Sesiones y un SVG de la TUI — reales, reproducibles
(`docs/ux/capturas.md`), y quietos. El único momento en que Meltemi dice
«un agente está trabajando bajo gobierno» —el anillo, la tarjeta de permiso
que llega, la calle que avanza— no aparece en el sitio, que es donde un
visitante decide en treinta segundos.

Las reglas de las figuras no lo impiden: piden que sean capturas reales de
las superficies reales sobre un fixture con el agente simulado, sin
identidades ni nombres de terceros, con texto alternativo y procedencia. Un
vídeo o un SVG animado cumplen lo mismo si se producen con el mismo script.

## What Changes

- **`scripts/capture-demo.ps1`**: sobre el fixture y el agente simulado que
  ya usa `capture-desktop.ps1`, conduce una secuencia corta —compositor,
  anillo trabajando, tarjeta de permiso, transcript con herramientas, tablero
  con dos calles— y la graba como WebM más un póster PNG del primer frame, en
  `docs/media/`. La TUI se graba desde el renderizador como SVG animado
  (frames del mismo `capture_svg`), también a `docs/media/`. El design decide
  si hay gemelo `.sh`: la superficie de escritorio que se graba es una
  ventana de Windows, y `capturas.md` ya dice que en macOS y Linux la captura
  es manual y el pie lo declara.
- **`docs/ux/capturas.md`** gana el procedimiento de la demostración y su
  revisión antes de confirmar: ninguna ruta personal, identidad, nombre de
  terceros ni credencial en los fotogramas, y el peso dentro del presupuesto
  que fija el design antes de grabar.
- **Del lado del sitio, en meltemi-web y fuera de esta change**: el
  `<video muted loop playsinline>` con el póster, el fallback sin movimiento
  y el lint de peso, procedencia en el pie y ninguna fuente externa. Primero
  `site-figures-provenance`; el embebido, cuando haya medios que copiar.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `initial-docs`: + requisito «Procedimiento de captura de la demostración,
  reproducible y sin fugas».

## Impact

- Archivos: `scripts/capture-demo.ps1` (nuevo), `docs/media/`,
  `docs/ux/capturas.md`, el ejemplo `capture_svg` de `tui/` si el SVG animado
  lo necesita.
- Cero dependencias nuevas en el producto; la grabación usa herramientas del
  mantenedor (el design nombra cuáles y cómo se reproduce).
- Riesgo: peso del vídeo frente a la promesa de un sitio ligero; el
  presupuesto se fija antes de grabar y la web lo repite en su lint.
- Sin gate externo. La parte de la web depende de esta (necesita los medios) y
  de `site-figures-provenance`.

## Fuera de alcance

- La página, el CSS y el lint del sitio: meltemi-web.
- JavaScript, reproductores externos o analítica de reproducción: spec
  `product-site` de meltemi-web y constitución §9.
- Un demo interactivo dentro del producto: `demo-guiada`.
