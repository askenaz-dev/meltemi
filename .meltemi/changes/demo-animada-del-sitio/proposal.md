# demo-animada-del-sitio

> Vía rápida (fast-forward): un procedimiento de captura scriptado, un medio
> autoalojado y su lugar en la portada, bajo las reglas que `product-site` ya
> impone (sin JavaScript, sin orígenes externos, lint como gate). Nace del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha P4).

## Why

La portada de Orca es la app en movimiento: un transcript que avanza, un
spinner, un cronómetro, una terminal que compila. La de Meltemi es una
captura fija de la vista Sesiones y un SVG de la TUI — reales, reproducibles
(`docs/ux/capturas.md`), y quietos. El único momento en que Meltemi dice
«un agente está trabajando bajo gobierno» —el anillo, la tarjeta de permiso
que llega, la calle que avanza— no aparece en el sitio, que es donde un
visitante decide en treinta segundos.

Las reglas del sitio no lo impiden: piden que las figuras sean capturas
reales de las superficies reales sobre un fixture con el agente simulado,
sin identidades ni nombres de terceros, con texto alternativo y
procedencia. Un vídeo o un SVG animado cumplen lo mismo si se producen con
el mismo script.

## What Changes

- **`scripts/capture-demo.ps1`** (y su gemelo `.sh`): sobre el fixture y el
  agente simulado que ya usa `capture-desktop.ps1`, conduce una secuencia
  corta —compositor, anillo trabajando, tarjeta de permiso, transcript con
  herramientas, tablero con dos calles— y la graba como WebM autoalojado
  (`site/media/`) más un póster PNG del primer frame. La TUI se graba desde
  el renderizador como SVG animado (frames del mismo `capture_svg`).
- **La portada lo embebe sin JS**: `<video muted loop playsinline>` con el
  póster; bajo `prefers-reduced-motion` el vídeo se oculta por CSS y queda
  el póster. Leyenda con la procedencia y la palabra «simulado», como exige
  la política de capturas.
- **El lint del sitio aprende**: presupuesto de tamaño para medios,
  procedencia obligatoria en el pie, ninguna fuente externa.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `product-site`: + requisito «Demostración animada autoalojada con
  procedencia, póster y fallback sin movimiento».

## Impact

- Archivos: `scripts/capture-demo.{ps1,sh}` (nuevos), `site/media/`,
  `site/index.html`, `site/es/index.html`, `site/site.css`, el lint del
  sitio, `docs/ux/capturas.md`.
- Cero dependencias nuevas en el producto; la grabación usa herramientas
  del mantenedor (el design nombra cuáles y cómo se reproduce).
- Riesgo: peso del vídeo frente a la promesa de un sitio ligero; el
  presupuesto se fija en el lint antes de grabar.

## Fuera de alcance

- JavaScript, reproductores externos o analítica de reproducción: spec
  `product-site` y constitución §9.
- Un demo interactivo dentro del producto: `demo-guiada`.
