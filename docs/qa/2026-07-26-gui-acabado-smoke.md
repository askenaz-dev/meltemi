# QA — Smoke visual conducido de la GUI (2026-07-26)

Verificación de `gui-acabado-y-cierre-sdd` **mirando la ventana**, no leyendo
el código: los cuatro defectos de esta change eran invisibles para los tests
de cableado (`desktop/tests/*.rs` leen el fuente) y para el QA de presupuestos
(mide instalador/arranque/RAM sin inspeccionar el render). Método nuevo:
conducir el webview real de la app por Chrome DevTools Protocol.

## Método

- Build `dev` de `meltemi-desktop` + `meltemid` con
  `additionalBrowserArgs: --remote-debugging-port=9222` **temporal** en
  `tauri.conf.json` (revertido antes del commit; el binario publicado no
  expone puerto alguno).
- Fixture temporal (repo git con `.meltemi/config.toml` apuntando al
  `mock-agent`), nunca este repo.
- Driver Node de ~120 líneas sobre el WebSocket de CDP:
  `Runtime.evaluate` para medir cajas, `Page.captureScreenshot` para
  evidencia, `Input.dispatchKeyEvent` para la paleta.

## Resultado: los cuatro escenarios PASAN sobre el binario real

| Escenario (spec delta) | Antes (medido 2026-07-25) | Después (medido 2026-07-26) |
|---|---|---|
| La vista ocupa el alto disponible | `main` 252px de 900; barra de estado flotando en y=297 | `main` 727px de 800; barra de estado termina en y=800 (borde) |
| Filas del árbol sin recorte | filas de 13.5px con línea de 18.85px (texto cortado) | filas uniformes de 22.9px |
| Metadirectorio de git fuera del mapa | 314 entradas de `.git` en el árbol del fixture | 12 entradas, `.git` ausente, `.meltemi/` presente |
| Turno de autoría finalizado queda cerrado | toda autoría listaba «▲ interrumpida»; Consumo: TIEMPO ACTIVO 0s | `sdd/propose` completo lista «■ finalizada · 1s»; Consumo: 1s |

Nada estimado: cada cifra sale de `getBoundingClientRect()` sobre el webview
o del texto renderizado de la vista. Capturas del antes y el después en el
registro de la evaluación (sesión del 2026-07-25/26).

## Deuda declarada

El smoke es manual y por release. Convertirlo en gate de CI (arrancar la app,
recorrer las vistas, afirmar las invariantes de layout) queda apuntado como
change futura en el plan; esta nota es el precedente del método.
