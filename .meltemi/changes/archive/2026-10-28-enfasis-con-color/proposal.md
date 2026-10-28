# enfasis-con-color

> Vía rápida (`fast-forward`: proposal, design, specs y tasks de una vez). Nace
> de la revisión del sitio del 2026-10-28: la captura de la TUI que meltemi.dev
> muestra en `/ide/` («The surfaces, as they actually render») enseña bloques
> blancos con texto negro sobre fondo oscuro, y así se ve también la TUI en un
> terminal con color. El mantenedor pidió mejorarlo.

## Why

La TUI marca todo lo que debe destacar —el banner, los contadores de la
cabecera, el título del panel enfocado, la fila seleccionada y las cabeceras de
grupo— con un único estilo, `emphasis()`: vídeo inverso más negrita. El inverso
es la elección correcta cuando no hay color (`NO_COLOR`, `TERM=dumb`): funciona
en cualquier terminal y no pinta fondo, como exige la spec `tui-shell`
(«Accesibilidad — NO_COLOR»). Pero cuando el terminal sí tiene color, el
inverso convierte cada énfasis en un bloque blanco que domina la pantalla y no
dice nada de la marca; y como cabeceras de grupo y selección comparten el
estilo, en la vista de sesiones hay cinco bloques iguales a la vez.

La marca tiene un azul (Egeo, `#2563eb`, `--mel-aegean` en el sistema de
diseño) que la GUI ya usa como acento. En el terminal cabe igual: fondo azul y
texto blanco en negrita para lo que debe destacar como bloque, y azul en
negrita sobre el fondo del propio terminal para los títulos y cabeceras de
grupo, que no necesitan bloque. El significado sigue en el glifo, la etiqueta
y la disposición, nunca en el color (spec «Accesibilidad — nunca solo color»):
sin color, todo vuelve exactamente al inverso de hoy.

## What Changes

- **`Presentation` sabe si el terminal anuncia color de 24 bits**
  (`COLORTERM=truecolor|24bit`): con él, el azul exacto de la marca; sin él,
  el más cercano de la paleta de 256 colores (índice 27). Con `NO_COLOR`,
  `--no-color` o `TERM=dumb` no cambia nada: ni color ni fondo.
- **Dos estilos en vez de uno.** `emphasis()` —el bloque: banner, contadores,
  título enfocado, fila seleccionada— pinta el azul de fondo con texto blanco en
  negrita cuando hay color; `heading()` —títulos de vista y cabeceras de
  grupo— pinta negrita en azul sin fondo. Sin color, los dos son el inverso de
  siempre.
- **La captura lleva los colores.** `capture_svg` escribe el color de fondo y de
  texto de cada tramo como atributos `fill`, y la negrita como `font-weight`,
  así que `docs/media/tui-sessions.svg` enseña la TUI como se ve en un terminal
  con color. Se regenera.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `tui-shell`: + requisito «Énfasis con el color de la marca» (dos estilos,
  acento por capacidad del terminal, monocromo intacto, captura con color).

## Impact

- Archivos: `tui/src/shell/present.rs`, `tui/src/shell/render.rs`,
  `tui/examples/capture_svg.rs`, `docs/media/tui-sessions.svg`.
- Cero dependencias nuevas; el daemon no se toca; cero RPC.
- meltemi-web copia la captura a `/ide/` (su README lo documenta).

## Fuera de alcance

- Un tema configurable o más colores: un acento basta y la spec manda que el
  color sea decorativo.
- La GUI: ya usa el acento.
