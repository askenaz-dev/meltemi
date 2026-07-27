# QA — Smoke visual conducido de la GUI (2026-07-27, pulido-pre-anuncio)

Verificación de `pulido-pre-anuncio` **mirando la ventana**, según el método
que estableció `gui-acabado-y-cierre-sdd` (docs/qa/2026-07-26-gui-acabado-smoke.md):
el apilado de icono sobre etiqueta fue invisible para todos los tests de
cableado mientras era visible al primer vistazo humano, así que la corrección
se verifica midiendo geometría real sobre el binario reconstruido, no leyendo
el código (design D5).

## Método

- Build de release con la CLI de Tauri (`ui/node_modules/.bin/tauri build
  --no-bundle`), con `additionalBrowserArgs: --remote-debugging-port=9333`
  **temporal** en `tauri.conf.json` (revertido antes del commit y el binario
  reconstruido limpio después; el binario publicado no expone puerto alguno).
- Driver Node (~200 líneas) sobre el WebSocket de CDP: `Runtime.evaluate`
  midiendo `getBoundingClientRect()` y `getComputedStyle` sobre el webview
  real; navegación por clicks reales sobre el DOM.
- Dos corridas: **A** contra el daemon real (0 sesiones), con un fixture
  temporal como proyecto activo (repo git con `.meltemi/` en el scratchpad,
  nunca este repo; `desktop-ui.json` respaldado y restaurado); **B** con
  `MELTEMI_ENDPOINT` apuntando a una ruta que ningún daemon puede enlazar,
  para el banner de daemon caído y el estado vacío de la Flota — estados
  reales, no fabricados.

Criterio de «una línea»: el svg del icono queda dentro de la caja del botón,
centrado verticalmente (desviación del centro ≤ 2 px) y la altura del botón es
la de una sola línea (~28–30 px; apilado mediría ≥ 40 px).

## Resultado: los siete botones del inventario, en una línea

| # | Botón (vista) | Alto | Desvío del centro del icono | Corrida |
|---|---|---|---|---|
| 1 | «Nueva sesión» — estado vacío de Sesiones | 29.9 px | 0 px | A |
| 2 | «Refrescar la detección» — estado vacío de Flota | 29.9 px | 0 px | B |
| 3 | «Refrescar la detección» — drawer de Flota | 29.9 px | 0 px | A |
| 4 | «Lanzar» — lanzador de sesión | 29.9 px | 0 px | A |
| 5 | «Abrir con…» — Editor | 28.5 px | 0 px | A |
| 6 | «Guardar (Ctrl+S)» — Editor | 28.5 px | 0 px | A |
| 7 | «Editar» ×2 — Ajustes (configuración efectiva) | 29.9 px | 0 px | A |

Nota: `getComputedStyle(...).display` reporta `flex` en los botones medidos
porque todos son hijos de contenedores flex/grid (la blockificación convierte
el `inline-flex` de la regla global); la regla que actúa es la del skin —
ningún componente re-declara.

## El par del estado vacío, a altura pareja

- «Nueva sesión» (icono+etiqueta) y «Ver la flota» (solo texto): **29.9 px
  ambos**, mismo renglón (top 221.2 en los dos); `align-items: center`
  computado en `.actions`.
- Forzando el envolvimiento (fila estrechada a 140 px): **29.9 px ambos**,
  renglones separados (tops 221.2 / 259.1) — ninguna acción se estira al
  envolver.
- El estado vacío de Flota computa el mismo `align-items: center` (corrida B).

## La etiqueta sin «(4)» y el atajo en su casa

- La acción del estado vacío de Sesiones se lee «Ver la flota» — sin número
  entre paréntesis (ES medido en vivo; EN cubierto por el wiring test del
  catálogo).
- El ítem Flota del sidebar conserva su `kbd` con «4» (medido: los cinco
  ítems con tecla renderizan su kbd 1–5).

## Sin regresión en el resto del inventario

- **Solo texto** (ganan `inline-flex` de la regla global, un único ítem flex
  anónimo): Ajustes tema/idioma (5 botones, 29.9 px uniformes), «Atrás» del
  Editor (28.5 px), «Cancelar» del lanzador (29.9 px), «Ver la flota»
  (29.9 px) — todos en una línea, alturas parejas con sus vecinos.
- **Varios hijos** (conservan sus overrides de gap, design D1): barra superior
  «Buscar o comandar + Ctrl K» (28.6 px, icono centrado, gap 8 px como antes),
  bandeja de permisos (28.5 px); banner de daemon caído «Reintentar ahora» y
  «Copiar diagnóstico» (27 px, icono centrado, gap 4 px — override --sp-1
  conservado); herramientas del Editor con gap 4 px conservado.
- **Árbol del Editor**: filas uniformes de 22.9 px — exactamente la medida
  sana del smoke del 2026-07-26 —, sin recorte; el twisty y el nombre ganan
  el gap de 8 px de la regla global (delta inventariado en la propuesta como
  riesgo asumido, sin efecto sobre altura ni recorte).

Nada estimado: cada cifra sale de `getBoundingClientRect()` /
`getComputedStyle` sobre el webview del binario reconstruido.

## Deuda declarada

Igual que en el precedente: el smoke es manual y por release; convertirlo en
gate de CI sigue fuera de alcance (declarado en la propuesta). El build que
acompañe el anuncio debe reconstruirse tras el merge de esta change.
