<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — barra-de-estado-agentica (2026-08-10)

Sobre el **binario de release**, receta de
`docs/qa/2026-08-09-piel-de-pestanas-smoke.md`: patch de puerto revertido al
terminar, user data folder propio y nuevo, binarios y endpoint aparte de los
del mantenedor. Recargando antes de medir y sin inyectar DOM.

Fixture con `.meltemi/` **copiado** del repositorio, de modo que la barra lee
changes reales sin que el smoke toque el árbol vivo.

## La barra, a 1200 px

```
▸ connected  v0.1.1  \\.\pipe\meltemid-smoke-bar  barfix  17 change(s)  0 working
```

Los cuatro segmentos nuevos están: proyecto por su nombre corto, el recuento de
changes, y el desglose de sesiones. Altura de la barra: 28 px.

## La prioridad de ceder ancho, a 900 px

La ventana en su mínimo declarado, medida segmento a segmento:

| segmento | 1200 px | 900 px |
|---|---|---|
| versión | visible | **cedido** |
| endpoint | visible | **cedido** |
| proyecto | visible | **cedido** |
| changes | visible | visible |
| sesiones | visible | visible |
| conexión | visible | visible |

Es exactamente el orden que el design declaró, y —lo que importa— **la conexión
y el recuento de lo que espera no ceden nunca**. Esto era lo que el diseño
señalaba como imposible de comprobar desde una hoja de estilos: aquí está
medido en el binario, no leído en el CSS.

## Lo que no se pudo medir, dicho

**La rama de la compuerta no se ejercitó**: ninguna change del fixture tenía
`gatePending` en el momento del smoke, así que la barra mostró su otra rama —el
recuento de changes activas—, que es la correcta para ese estado. La rama del
gate queda probada por su test de fuente y sin confirmación conducida; se dice
en vez de insinuar que se vio.

Tampoco se ejercitó el segmento de consumo: el proyecto del fixture no tiene
sesiones con consumo medido, que es justamente el caso mayoritario que la
change decidió tratar con silencio en vez de con un cero.

## Reversión

Patch de `additionalBrowserArgs` retirado y su ausencia verificada antes de
commitear. Los procesos del mantenedor no se detuvieron.
