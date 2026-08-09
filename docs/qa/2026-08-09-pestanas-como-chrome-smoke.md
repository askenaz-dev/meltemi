# Smoke conducido — `pestanas-como-chrome`

**Fecha**: 2026-08-09 · **Plataforma**: Windows 11, WebView2
**Binario**: `target/release/meltemi-desktop.exe`, sobre un repositorio fixture
temporal con `mock-agent` y directorios aislados. Puerto de depuración remoto
temporal, revertido al terminar.

## Resultado

| Escenario | Medido |
| --- | --- |
| Muchas pestañas no producen un segundo renglón | 6 pestañas, **1 fila** (una sola coordenada superior entre todas) |
| Los controles aparecen solo cuando sobran pestañas | desborda: sí → 2 controles renderizados |
| La pestaña activa nunca queda fuera de vista | la tira arranca desplazada 48 px: la activa se trajo sola |
| Una pestaña pertenece a un grupo y lo dice | nombre accesible: «mock-agent bc514e22 — Refactor» |
| Plegar guarda espacio, no trabajo | 6 → 6 paneles montados; la fila pasa de 6 a 5 pestañas |
| El grupo plegado declara cuántas guarda | etiqueta «Refactor (1)» |
| Sigue habiendo exactamente una pestaña seleccionada | 1 |

## Lo que el smoke encontró y esta change corrigió

**El bloque del menú quedó después de `</style>`.** Al añadirlo al final del
archivo acabó fuera del marcado, así que el menú no se renderizaba nunca: el
botón cambiaba su `aria-expanded` y no aparecía nada. No lo vio ningún test de
cableado —el marcado existe en el archivo, solo que en el sitio equivocado— ni
el compilador, que lo ignoró en silencio.

## Dos lecciones del método, anotadas

1. **El estado sobrevive entre corridas del conductor.** Una corrida dejó el
   menú abierto y la siguiente lo cerró al «abrirlo», con lo que el fallo
   pareció del producto. El conductor ahora cierra cualquier menú antes de
   abrir el suyo.
2. **Un evento `input` sintético no mueve un binding de Svelte.** El botón de
   envío seguía deshabilitado porque el valor nunca llegó al componente. Se
   escribe con `Input.insertText` —teclas de verdad—, que además es lo que hace
   una persona.

Ninguna de las dos era un defecto del producto, y por eso se anotan aquí en vez
de en el registro de cambios.
