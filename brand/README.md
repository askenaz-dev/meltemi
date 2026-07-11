# Meltemi — identidad V2

Esta iteración hace explícita la conexión marítima sin convertir el logo en una ilustración náutica.

## Concepto elegido

El símbolo combina un trazo de viento con un casco mínimo y tiene cuatro lecturas:

1. El trazo principal conserva una **m** minúscula.
2. Sus dos arcos funcionan como **velas asimétricas**.
3. Una forma curva y afilada completa el **casco**.
4. El extremo derecho es simultáneamente **proa y ráfaga de viento**.

La V1 monoline era elegante, pero la navegación solo aparecía después de explicarla. En esta V2, la `m` y el barco son visibles sin añadir mástil, olas, bandera o ancla. El casco se mantiene separado por una franja de aire para evitar que el conjunto parezca una cara o una ilustración pesada.

## Archivos

- `meltemi-mark-gradient.svg` — símbolo principal transparente.
- `meltemi-mark-mono-dark.svg` — símbolo monocromático para superficies claras.
- `meltemi-mark-mono-light.svg` — símbolo monocromático para superficies oscuras.
- `meltemi-lockup-dark.svg` — lockup horizontal para fondos oscuros.
- `meltemi-lockup-light.svg` — lockup horizontal para fondos claros.
- `meltemi-app-icon.svg` — tratamiento para app/launcher.
- `meltemi-app-icon-512.png` — exportación PNG de 512 px.
- `meltemi-brand-board.svg` y `meltemi-brand-preview.png` — lámina de presentación.
- `meltemi-alternate-monoline-hull.svg` — alternativa más cercana al símbolo original.

El wordmark está convertido a contornos vectoriales desde Inter SemiBold. No depende de una fuente instalada.

## Uso recomendado

- Usar el símbolo completo desde `16px`; en ese tamaño, preferir color sólido.
- Usar el degradado desde `24px` en adelante.
- Mantener como espacio libre mínimo el ancho del recorte de la vela pequeña.
- Para el lockup, no bajar de `120px` de ancho.
- Mantener visible la separación entre las velas y el casco; no unir ambos elementos manualmente.
- No añadir mástil, olas, brújula, ancla, sombra, glow ni contenedor circular.

## Prompt actualizado

> Minimal flat vector logo mark for “Meltemi”, an open-source Agentic IDE that orchestrates AI coding agents. Draw a bold continuous lowercase “m” with two asymmetric wind-filled arches that also read as a small foresail and a taller mainsail. Add one separate, minimal tapered curve underneath as the hull of a fast sailboat, leaving a deliberate strip of negative space between sails and hull. Let the right end of the m rise northeast as both prow and gust. The first reading is “m in motion”; the second is “sailboat driven by wind”. Compact near-square silhouette, rounded terminals, readable at 16 px, flat 2D. Gradient from Aegean blue #2563EB at lower left to wind cyan #22D3EE at upper right. Transparent background, centered 1:1 artboard. --v 7 --style raw --ar 1:1 --no text, wordmark, anchor, compass, flag, mast, literal waves, face, smile, circle, frame, shadow, glow, texture, mockup, photorealism, 3D
