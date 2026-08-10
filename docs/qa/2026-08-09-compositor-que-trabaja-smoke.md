<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — compositor-que-trabaja (2026-08-09)

Medición sobre el **binario de release** con la GUI conducida por CDP. Receta de
`docs/qa/2026-08-09-piel-de-pestanas-smoke.md`: patch de puerto revertido al
terminar, `WEBVIEW2_USER_DATA_FOLDER` propio y nuevo, binarios y endpoint
aparte de los del mantenedor.

## Qué se midió y de qué manera

Esta change tiene una dificultad que conviene decir antes de los números: **el
agente simulado termina su turno en segundos**, así que el estado que enciende
la luz dura poco. Las mediciones son de dos clases y el informe las separa en
vez de mezclarlas:

1. **En su condición real** — la luz del compositor del Home, capturada en la
   ventana entre enviar y `session_started`, que es exactamente cuando el
   gancho `running` la enciende. El driver muestrea rápido hasta encontrarla.
2. **Del estilo aplicado** — la rama de movimiento reducido, pedida al motor
   real con `Emulation.setEmulatedMedia` y leída del documento vivo, con la
   clase forzada en el DOM porque la ventana de trabajo es demasiado breve para
   coincidir con el cambio de media. Se declara que la clase se forzó: lo que
   se mide ahí es **qué hace la regla**, no cuándo se aplica — eso último lo
   prueban los tests de fuente.

## La luz, en su condición real

Capturada en la ventana de trabajo del compositor del Home, sin forzar nada:

| | valor medido |
|---|---|
| clase | `wind svelte-5wo4x7` (el ámbito de Svelte, aplicado) |
| capa | `position: absolute`, `z-index: -1` — detrás del compositor opaco |
| bucle | `composer-wind 2.4s` |
| pintura | `conic-gradient(… rgb(37, 99, 235) 60deg …)` — `--mel-aegean` |
| sobredimensión | `1524.8px` de ancho para un compositor de ~760 px |

Es decir: la capa está donde debe, gira a la cadencia declarada, viste la marca
y su gradiente es el doble del marco, que es lo que evita que la rotación
descubra una esquina.

## Movimiento reducido: retirada, no congelada

Pedido al motor real con `prefers-reduced-motion: reduce`, y con la luz en su
condición real —no inyectada—:

| | valor medido |
|---|---|
| la luz | **`display: none`** |
| el marco | `border-color: rgb(96, 165, 250)` (`--accent`) |
| la animación | `1e-05s` |

Ese último número es la prueba del argumento que el design hizo por escrito: el
kill-switch global **sí** acortó la duración a diez microsegundos, y eso es
todo lo que sabe hacer. Si la change se hubiera apoyado en él, la luz seguiría
en `display: block` con su degradado pintado y detenido en una posición
arbitraria. La regla propia la retira, y el marco de acento sostiene el estado.

## La deuda de marca

`button.primary` pinta ahora
`linear-gradient(135deg, rgb(37, 99, 235), rgb(34, 211, 238))` —
`--mel-aegean` → `--mel-wind`, sin el literal que había en su lugar. El tono
final del degradado cambia, que era el único efecto visual colateral declarado.

## Trampa del método, para no repetirla

La primera corrida midió una luz **sin estilos** y estuvo a punto de reportarse
como defecto. La causa era el propio driver: una versión anterior **inyectaba**
un `<span class="wind">` en el DOM para poder leer la rama de movimiento
reducido fuera de la ventana de trabajo, y ese nodo —sin la clase de ámbito que
Svelte añade— sobrevivió a las mediciones siguientes, que no recargaban la
página. Un driver que inyecta nodos contamina todo lo que venga después.

La regla que queda: **recargar antes de medir, y no inyectar DOM propio**. Si
un estado es demasiado breve para alcanzarlo, se provoca de verdad (aquí,
enviando desde el compositor) en vez de fabricarlo.

## Reversión

Patch de `additionalBrowserArgs` retirado y su ausencia verificada antes de
commitear. La GUI y el daemon del mantenedor no se detuvieron.
